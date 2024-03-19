mod compression;
pub mod config;
mod db;
mod git;
pub mod scoreboard;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use ce_core::ValidationResult;
use ce_shell::{Analysis, Input};
use color_eyre::eyre::Context;
use driver::{Driver, Hub, Job, JobState};
use indexmap::IndexMap;
use itertools::Itertools;
use rand::seq::SliceRandom;
use tracing::Instrument;

use crate::endpoints::InspectifyJobMeta;

use self::config::{GroupConfig, GroupName};

pub struct Checko {
    hub: Hub<InspectifyJobMeta>,
    path: PathBuf,
    db: db::CheckoDb,
    groups_config: config::GroupsConfig,
    programs_config: config::ProgramsConfig,
    last_finished: std::sync::Mutex<Option<chrono::DateTime<chrono::FixedOffset>>>,
    group_states: tokio::sync::Mutex<IndexMap<(GroupName, Analysis), GroupState2>>,
}

#[derive(Debug, Clone)]
pub enum GroupDriver {
    Missing { reason: String },
    Driver(Driver<InspectifyJobMeta>),
}

pub struct GroupState {
    driver: GroupDriver,
    compile_job: Option<Job<InspectifyJobMeta>>,
}

#[derive(Debug, Clone)]
pub struct GroupRepo {
    pub path: PathBuf,
    pub git_hash: Option<String>,
}

#[derive(Default, Clone)]
pub struct GroupState2 {
    inner: Arc<tokio::sync::RwLock<GroupState2Inner>>,
}
#[derive(Default)]
pub struct GroupState2Inner {
    latest_hash: Option<String>,
    status: GroupStatus,
    results: BTreeMap<ce_shell::Hash, JobState>,
}

#[derive(
    tapi::Tapi, Debug, Default, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize,
)]
pub enum GroupStatus {
    #[default]
    Initial,
    CheckingForUpdate,
    Compiling,
    Testing,
    CompilationError,
    Finished,
}

impl GroupState2 {
    pub async fn status(&self) -> GroupStatus {
        self.inner.read().await.status
    }

    pub async fn set_status(&self, status: GroupStatus) -> GroupStatus {
        std::mem::replace(&mut self.inner.write().await.status, status)
    }

    pub async fn latest_hash(&self) -> Option<String> {
        self.inner.read().await.latest_hash.clone()
    }

    pub async fn update_latest_hash(&self, hash: Option<&str>) -> bool {
        let mut inner = self.inner.write().await;
        if inner.latest_hash.as_deref() == hash {
            return false;
        }
        inner.latest_hash = hash.map(|h| h.to_string());
        true
    }

    pub async fn results(&self) -> BTreeMap<ce_shell::Hash, JobState> {
        self.inner.read().await.results.clone()
    }

    pub async fn set_result(&self, hash: ce_shell::Hash, state: JobState) {
        self.inner.write().await.results.insert(hash, state);
    }
}

struct GroupToTest {
    group: Arc<config::GroupConfig>,
    analysis: Analysis,
    repo: GroupRepo,
    state: GroupState2,
    driver: GroupDriver,
}

impl Checko {
    #[tracing::instrument(skip(hub))]
    pub fn open(hub: Hub<InspectifyJobMeta>, path: &Path) -> color_eyre::Result<Self> {
        let path = dunce::canonicalize(path)
            .wrap_err_with(|| format!("could not canonicalize path: '{}'", path.display()))?;
        tracing::debug!(?path, "opening checko");

        let runs_db_path = path.join("runs.db3");
        let groups_path = dunce::canonicalize(path.join("groups.toml"))
            .wrap_err_with(|| format!("missing groups.toml at '{}'", path.display()))?;
        let programs_path = dunce::canonicalize(path.join("programs.toml"))
            .wrap_err_with(|| format!("missing programs.toml at '{}'", path.display()))?;

        tracing::debug!(?runs_db_path, ?groups_path, ?programs_path, "checko paths");

        let db = db::CheckoDb::open(&runs_db_path).wrap_err("could not open db")?;
        let groups = config::read_groups(groups_path)?;
        let programs = config::read_programs(programs_path)?;

        Ok(Self {
            hub,
            path,
            db,
            groups_config: groups,
            programs_config: programs,
            last_finished: Default::default(),
            group_states: Default::default(),
        })
    }

    pub fn groups_config(&self) -> &config::GroupsConfig {
        &self.groups_config
    }

    pub fn programs_config(&self) -> config::CanonicalProgramsConfig {
        self.programs_config.canonicalize().unwrap()
    }

    fn group_path(&self, config: &GroupConfig, analysis: Analysis) -> PathBuf {
        self.path
            .join("groups")
            .join(format!("{}-{analysis:?}", config.name))
    }

    #[tracing::instrument(skip(self))]
    async fn update_group_repo(
        &self,
        config: &config::GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::DateTime<chrono::FixedOffset>>,
    ) -> color_eyre::Result<GroupRepo> {
        match (&config.git, &config.path) {
            (Some(git), then_path) => {
                let group_path = self.group_path(config, analysis);

                std::fs::create_dir_all(&group_path).wrap_err_with(|| {
                    format!(
                        "could not create group directory: '{}'",
                        group_path.display()
                    )
                })?;

                let git_pull_result = tokio_retry::Retry::spawn(
                    tokio_retry::strategy::FixedInterval::new(Duration::from_secs(5)).take(15),
                    || git::clone_or_pull(git, &group_path),
                )
                .await;

                git_pull_result?;

                if let Some(deadline) = deadline {
                    let _found_any = git::checkout_latest_before(
                        git,
                        &group_path,
                        deadline,
                        &self.groups_config.ignored_authors,
                    )
                    .await?;
                    // TODO: perhaps do something since we did not find any
                    // meaningful commits, but for now, we just go on with the
                    // latest commit
                }

                let path = if let Some(then_path) = then_path {
                    group_path.join(then_path)
                } else {
                    group_path
                };

                let git_hash = git::hash(&path, None).await?;

                Ok(GroupRepo {
                    path,
                    git_hash: Some(git_hash),
                })
            }
            (_, Some(path)) => {
                let path = dunce::canonicalize(PathBuf::from(path))
                    .wrap_err_with(|| format!("could not canonicalize group path: '{}'", path))?;
                Ok(GroupRepo {
                    path,
                    git_hash: None,
                })
            }
            _ => todo!(),
        }
    }

    pub fn last_finished(&self) -> Option<chrono::DateTime<chrono::FixedOffset>> {
        *self.last_finished.lock().unwrap()
    }

    async fn groups_to_test(
        &self,
        analysis_inputs: &BTreeMap<Analysis, Arc<Vec<Input>>>,
    ) -> color_eyre::Result<Vec<GroupToTest>> {
        let mut groups = self
            .groups_config
            .groups
            .iter()
            .map(|g| Arc::new(g.clone()))
            .collect_vec();

        groups.shuffle(&mut rand::thread_rng());

        // TODO: use async streams for this instead

        let mut compile_join_set = tokio::task::JoinSet::new();

        for (&a, inputs) in analysis_inputs {
            let inputs = Arc::clone(inputs);
            let deadline = self.programs_config.deadlines.get(&a).and_then(|d| d.time);

            for g in &groups {
                let g = Arc::clone(g);
                let gs = self
                    .group_states
                    .lock()
                    .await
                    .entry((g.name.clone(), a))
                    .or_default()
                    .clone();
                let repo = {
                    let prev = gs.set_status(GroupStatus::CheckingForUpdate).await;
                    let repo = self.update_group_repo(&g, a, deadline).await?;
                    if !gs.update_latest_hash(repo.git_hash.as_deref()).await {
                        gs.set_status(prev).await;
                        continue;
                    }
                    repo
                };

                let mut need_work = false;

                if let Some(git_hash) = repo.git_hash.as_ref().as_ref() {
                    for input in inputs.iter() {
                        let key = db::CacheKeyInput {
                            group_name: &g.name,
                            git_hash,
                            input,
                        }
                        .key();
                        if let Some(job_data) = self.db.get_cached_run(&key)? {
                            gs.set_result(input.hash(), job_data.state).await;
                        } else {
                            need_work = true;
                        }
                    }
                } else {
                    need_work = true;
                }

                if !need_work {
                    gs.set_status(GroupStatus::Finished).await;
                    continue;
                }

                tracing::info!(name=?g.name, ?a, "rerunning tests");

                let driver = match Driver::new_from_path(
                    self.hub.clone(),
                    &repo.path,
                    repo.path.join(g.run.as_deref().unwrap_or("run.toml")),
                ) {
                    Ok(driver) => {
                        gs.set_status(GroupStatus::Compiling).await;
                        tracing::debug!("ensuring compile job");
                        let compile_job = driver.ensure_compile(InspectifyJobMeta {
                            group_name: Some(g.name.clone()),
                        });
                        tracing::debug!("group state built successfully");
                        Arc::new(GroupState {
                            driver: GroupDriver::Driver(driver),
                            compile_job,
                        })
                    }
                    Err(err) => {
                        gs.set_status(GroupStatus::CompilationError).await;
                        tracing::error!(?err, "could not build group driver");
                        Arc::new(GroupState {
                            driver: GroupDriver::Missing {
                                reason: format!("{:?}", err),
                            },
                            compile_job: None,
                        })
                    }
                };

                let inputs = Arc::clone(&inputs);
                compile_join_set.spawn(
                    async move {
                        let res = if let Some(job) = &driver.compile_job {
                            let state = job.wait().await;
                            match state {
                                JobState::Succeeded => Some(GroupToTest {
                                    analysis: a,
                                    group: g.clone(),
                                    repo,
                                    state: gs.clone(),
                                    driver: driver.driver.clone(),
                                }),
                                _ => None,
                            }
                        } else {
                            None
                        };

                        if res.is_some() {
                            gs.set_status(GroupStatus::Testing).await;
                        } else {
                            gs.set_status(GroupStatus::CompilationError).await;
                            for input in inputs.iter() {
                                gs.set_result(input.hash(), JobState::Failed).await;
                            }
                        }
                        res
                    }
                    .in_current_span(),
                );
            }
        }

        let mut groups_to_test = Vec::new();

        while let Some(res) = compile_join_set.join_next().await {
            if let Some(gtt) = res? {
                groups_to_test.push(gtt);
            }
        }

        Ok(groups_to_test)
    }

    // #[tracing::instrument(skip(self))]
    pub async fn work(&self) -> color_eyre::Result<()> {
        let analysis_inputs: BTreeMap<_, _> = self
            .programs_config
            .inputs()
            .map(|(analysis, inputs)| (analysis, Arc::new(inputs.collect_vec())))
            .collect();

        loop {
            let groups_to_test = self.groups_to_test(&analysis_inputs).await?;

            let mut testing_join_set = tokio::task::JoinSet::new();

            for gtt in groups_to_test {
                let driver = match gtt.driver {
                    GroupDriver::Driver(driver) => driver,
                    GroupDriver::Missing { reason } => {
                        tracing::error!(name=?gtt.group.name, ?reason, "group driver missing");
                        continue;
                    }
                };
                for i in analysis_inputs[&gtt.analysis].iter() {
                    let key = gtt.repo.git_hash.as_ref().as_ref().map(|git_hash| {
                        db::CacheKeyInput {
                            group_name: &gtt.group.name,
                            git_hash,
                            input: i,
                        }
                        .key()
                        .into_owned()
                    });

                    let db = self.db.clone();
                    let input = i.clone();
                    let g = Arc::clone(&gtt.group);
                    let gs = gtt.state.clone();
                    let job = driver.exec_job(
                        &input,
                        InspectifyJobMeta {
                            group_name: Some(g.name.clone()),
                        },
                    );
                    testing_join_set.spawn(
                        async move {
                            job.wait().await;

                            let state = {
                                let output = input.analysis().output_from_str(&job.stdout());
                                let validation = match (job.state(), &output) {
                                    (JobState::Succeeded, Ok(output)) => {
                                        Some(match input.validate_output(output) {
                                            Ok(output) => output,
                                            Err(e) => ValidationResult::Mismatch {
                                                reason: format!("failed to validate output: {e:?}"),
                                            },
                                        })
                                    }
                                    (JobState::Succeeded, Err(e)) => {
                                        Some(ValidationResult::Mismatch {
                                            reason: format!("failed to parse output: {e:?}"),
                                        })
                                    }
                                    _ => None,
                                };

                                match (job.state(), validation) {
                                    (
                                        JobState::Succeeded,
                                        Some(
                                            ValidationResult::CorrectNonTerminated { .. }
                                            | ValidationResult::CorrectTerminated,
                                        ),
                                    ) => JobState::Succeeded,
                                    (
                                        JobState::Succeeded,
                                        Some(ValidationResult::Mismatch { .. }),
                                    ) => JobState::Warning,
                                    (JobState::Succeeded, Some(ValidationResult::TimeOut)) => {
                                        JobState::Timeout
                                    }
                                    (state, _) => state,
                                }
                            };

                            if let Some(key) = key {
                                db.insert_cached_run(
                                    &key,
                                    &driver::JobData {
                                        state,
                                        ..job.data().clone()
                                    },
                                )?;
                            }

                            gs.set_status(GroupStatus::Finished).await;
                            gs.set_result(input.hash(), state).await;

                            Ok::<_, color_eyre::eyre::Error>(())
                        }
                        .in_current_span(),
                    );
                }
            }

            while let Some(res) = testing_join_set.join_next().await {
                if let Err(err) = res? {
                    tracing::error!(?err, "error in join set");
                }
            }

            self.hub.clear();

            *self.last_finished.lock().unwrap() = Some(chrono::Utc::now().fixed_offset());
            tracing::info!("waiting for next batch of runs");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
