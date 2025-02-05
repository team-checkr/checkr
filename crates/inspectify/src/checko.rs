mod compression;
pub mod config;
mod db;
pub mod scoreboard;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use ce_core::ValidationResult;
use ce_shell::{Analysis, Input};
use color_eyre::{eyre::Context, Result};
use driver::{Driver, Hub, Job, JobState};
use futures_util::{StreamExt, TryStreamExt};
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

pub struct GroupState {
    driver: Driver<InspectifyJobMeta>,
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

#[derive(Clone)]
struct GroupToTest {
    group: Arc<config::GroupConfig>,
    analysis: Analysis,
    repo: GroupRepo,
    state: GroupState2,
    driver: Driver<InspectifyJobMeta>,
}

impl GroupToTest {
    fn cache_key(&self, input: &Input) -> Option<db::CacheKey<'static>> {
        Some(
            db::CacheKeyInput {
                group_name: &self.group.name,
                git_hash: self.repo.git_hash.as_ref()?,
                input,
            }
            .key()
            .into_owned(),
        )
    }
    async fn test_input(&self, db: &db::CheckoDb, input: &Input) -> Result<()> {
        let job = self.driver.exec_job(
            input,
            InspectifyJobMeta {
                group_name: Some(self.group.name.clone()),
            },
        );
        job.wait().await;

        let state = compute_validated_job_state(&job);

        if let Some(key) = self.cache_key(input) {
            let data = driver::JobData {
                state,
                ..job.data().clone()
            };
            db.insert_cached_run(&key, &data)?;
        }

        self.state.set_status(GroupStatus::Finished).await;
        self.state.set_result(input.hash(), state).await;

        Ok(())
    }
}

impl Checko {
    #[tracing::instrument(skip(hub))]
    pub fn open(hub: Hub<InspectifyJobMeta>, path: &Path) -> Result<Self> {
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

    async fn group_state(&self, g: &GroupConfig, a: Analysis) -> GroupState2 {
        self.group_states
            .lock()
            .await
            .entry((g.name.clone(), a))
            .or_default()
            .clone()
    }

    async fn group_states(&self) -> Vec<(GroupName, Analysis, GroupState2)> {
        self.group_states
            .lock()
            .await
            .iter()
            .map(|((g, a), gs)| (g.clone(), *a, gs.clone()))
            .collect_vec()
    }

    #[tracing::instrument(skip(self))]
    async fn update_group_repo(
        &self,
        config: &config::GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::DateTime<chrono::FixedOffset>>,
    ) -> Result<GroupRepo> {
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
                    || gitty::clone_or_pull(git, &group_path),
                )
                .await;

                git_pull_result?;

                if let Some(commit) = config.commit.get(&analysis) {
                    gitty::checkout_commit(&group_path, commit).await?;
                } else if let Some(deadline) = deadline {
                    let _found_any = gitty::checkout_latest_before(
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
                    group_path.join(then_path.as_str())
                } else {
                    group_path
                };

                let git_hash = gitty::hash(&path, None).await?;

                Ok(GroupRepo {
                    path,
                    git_hash: Some(git_hash),
                })
            }
            (_, Some(path)) => {
                let path = dunce::canonicalize(PathBuf::from(path.as_str()))
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
        self: &Arc<Self>,
        analysis_inputs: &BTreeMap<Analysis, Arc<Vec<Input>>>,
    ) -> Result<Vec<GroupToTest>> {
        let mut groups = self.groups_config.groups.clone();

        groups.shuffle(&mut rand::rng());

        let mut compile_join_set = tokio::task::JoinSet::<Result<Option<_>>>::new();

        for ((&a, inputs), g) in analysis_inputs
            .iter()
            .cartesian_product(groups.iter().cloned())
        {
            let inputs = Arc::clone(inputs);
            let deadline = self.programs_config.deadlines.get(&a).and_then(|d| d.time);
            let db = self.db.clone();
            let hub = self.hub.clone();
            let checko = self.clone();

            // NOTE: We do the cloning sequentially, because we want to be nice
            // to the remote server
            let gs = checko.group_state(&g, a).await;
            let repo = checko.update_group_repo(&g, a, deadline).await?;

            compile_join_set.spawn(
                async move {
                    let prev_status = gs.set_status(GroupStatus::CheckingForUpdate).await;
                    if !gs.update_latest_hash(repo.git_hash.as_deref()).await {
                        gs.set_status(prev_status).await;
                        return Ok(None);
                    }

                    let mut need_work = false;

                    if let Some(git_hash) = repo.git_hash.as_ref() {
                        for input in inputs.iter() {
                            let key = db::CacheKeyInput {
                                group_name: &g.name,
                                git_hash,
                                input,
                            }
                            .key();
                            if let Some(job_data) = db.get_cached_run(&key)? {
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
                        return Ok(None);
                    }

                    tracing::info!(name=?g.name, ?a, "rerunning tests");

                    let driver = match Driver::new_from_path(
                        hub,
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
                                driver,
                                compile_job,
                            })
                        }
                        Err(err) => {
                            gs.set_status(GroupStatus::CompilationError).await;
                            tracing::error!(?err, "could not build group driver");
                            return Ok(None);
                        }
                    };

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
                    Ok(res)
                }
                .in_current_span(),
            );
        }

        let mut groups_to_test = Vec::new();

        while let Some(res) = compile_join_set.join_next().await {
            if let Some(gtt) = res?? {
                groups_to_test.push(gtt);
            }
        }

        Ok(groups_to_test)
    }

    #[tracing::instrument(skip(self))]
    pub async fn work(self: &Arc<Self>) -> Result<()> {
        let analysis_inputs: BTreeMap<_, _> = self
            .programs_config
            .inputs()
            .map(|(analysis, inputs)| (analysis, Arc::new(inputs.collect_vec())))
            .collect();

        loop {
            let groups_to_test = self.groups_to_test(&analysis_inputs).await?;
            self.run_group_tests(groups_to_test, &analysis_inputs)
                .await?;

            self.hub.clear();

            *self.last_finished.lock().unwrap() = Some(chrono::Utc::now().fixed_offset());
            tracing::info!("waiting for next batch of runs");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    async fn run_group_tests(
        &self,
        groups_to_test: Vec<GroupToTest>,
        analysis_inputs: &BTreeMap<Analysis, Arc<Vec<Input>>>,
    ) -> Result<(), color_eyre::eyre::Error> {
        let tests = groups_to_test.iter().flat_map(|gtt| {
            analysis_inputs[&gtt.analysis]
                .iter()
                .cloned()
                .map(move |input| {
                    let gtt = gtt.clone();
                    let db = self.db.clone();
                    tokio::spawn(async move { gtt.test_input(&db, &input).await })
                })
        });
        tokio_stream::iter(tests)
            .then(|test| async { test.await? })
            .try_collect()
            .await
    }
}

fn compute_validated_job_state(job: &Job<InspectifyJobMeta>) -> JobState {
    let input = match job.kind() {
        driver::JobKind::Compilation => return job.state(),
        driver::JobKind::Analysis(input) => input,
    };

    let output = input.analysis().output_from_str(&job.stdout());
    let validation = match (job.state(), &output) {
        (JobState::Succeeded, Ok(output)) => Some(match input.validate_output(output) {
            Ok(output) => output,
            Err(e) => ValidationResult::Mismatch {
                reason: format!("failed to validate output: {e:?}"),
            },
        }),
        (JobState::Succeeded, Err(e)) => Some(ValidationResult::Mismatch {
            reason: format!("failed to parse output: {e:?}"),
        }),
        _ => None,
    };

    match (job.state(), validation) {
        (
            JobState::Succeeded,
            Some(
                ValidationResult::CorrectNonTerminated { .. } | ValidationResult::CorrectTerminated,
            ),
        ) => JobState::Succeeded,
        (JobState::Succeeded, Some(ValidationResult::Mismatch { .. })) => JobState::Warning,
        (JobState::Succeeded, Some(ValidationResult::TimeOut)) => JobState::Timeout,
        (state, _) => state,
    }
}
