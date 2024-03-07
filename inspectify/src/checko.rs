pub mod config;
mod db;
mod git;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use ce_shell::Analysis;
use color_eyre::eyre::Context;
use driver::{Driver, Hub, Job, JobId, JobKind};
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
    group_repos: tokio::sync::Mutex<IndexMap<(GroupName, Analysis), GroupRepo>>,
    group_states: tokio::sync::Mutex<IndexMap<(GroupName, Analysis), Arc<GroupState>>>,
    events_tx: crate::history_broadcaster::Sender<CheckoEvent>,
    events_rx: crate::history_broadcaster::Receiver<CheckoEvent>,
}

#[derive(Debug, Clone)]
pub enum CheckoEvent {
    JobAssigned {
        group: GroupName,
        kind: JobKind,
        job_id: JobId,
    },
}

#[derive(Debug, Clone)]
pub enum GroupDriver {
    Missing { reason: String },
    Driver(Driver<InspectifyJobMeta>),
}

pub struct GroupState {
    driver: GroupDriver,
    compile_job: Option<Job<InspectifyJobMeta>>,
    active_jobs: Mutex<Vec<Job<InspectifyJobMeta>>>,
}

#[derive(Debug, Clone)]
pub struct GroupRepo {
    pub path: PathBuf,
    pub git_hash: Option<String>,
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

        let (events_tx, events_rx) = crate::history_broadcaster::channel(1024);

        Ok(Self {
            hub,
            path,
            db,
            groups_config: groups,
            programs_config: programs,
            group_repos: Default::default(),
            group_states: Default::default(),
            events_tx,
            events_rx,
        })
    }

    pub fn events(&self) -> crate::history_broadcaster::Receiver<CheckoEvent> {
        self.events_rx.resubscribe()
    }

    pub fn groups_config(&self) -> &config::GroupsConfig {
        &self.groups_config
    }

    pub fn programs_config(&self) -> config::CanonicalProgramsConfig {
        self.programs_config.canonicalize().unwrap()
    }

    #[tracing::instrument(skip(self))]
    pub async fn group_repo(
        &self,
        config: &GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<GroupRepo> {
        let key = (config.name.clone(), analysis);

        let gs = self.group_repos.lock().await.get(&key).cloned();
        if let Some(repo) = gs {
            Ok(repo)
        } else {
            let repo = self.update_group_repo(config, analysis, deadline).await?;
            let mut group_repos = self.group_repos.lock().await;
            group_repos.insert(key, repo.clone());
            Ok(repo)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn group_state(
        &self,
        config: &GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<impl Future<Output = Arc<GroupState>> + 'static> {
        let key = (config.name.clone(), analysis);

        let gs = self.group_states.lock().await.get(&key).cloned();
        let state = if let Some(gs) = gs {
            gs
        } else {
            let state = self.build_group_state(config, analysis, deadline).await?;
            let mut group_states = self.group_states.lock().await;
            group_states.insert(key, Arc::clone(&state));
            state
        };

        Ok(async move {
            if let Some(job) = &state.compile_job {
                job.wait().await;
            }
            state
        })
    }

    #[tracing::instrument(skip(self))]
    async fn build_group_state(
        &self,
        config: &GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<Arc<GroupState>> {
        tracing::debug!("building group state");
        match self.build_group_driver(config, analysis, deadline).await {
            Ok(driver) => {
                tracing::debug!("ensuring compile job");
                let compile_job = driver.ensure_compile(InspectifyJobMeta {
                    group_name: Some(config.name.clone()),
                });
                tracing::debug!("group state built successfully");
                let state = Arc::new(GroupState {
                    driver: GroupDriver::Driver(driver),
                    compile_job,
                    active_jobs: Mutex::new(Vec::new()),
                });
                Ok(state)
            }
            Err(err) => {
                tracing::error!(?err, "could not build group driver");
                let state = Arc::new(GroupState {
                    driver: GroupDriver::Missing {
                        reason: format!("{:?}", err),
                    },
                    compile_job: None,
                    active_jobs: Mutex::new(Vec::new()),
                });
                Ok(state)
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn update_group_repo(
        &self,
        config: &config::GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<GroupRepo> {
        match (&config.git, &config.path) {
            (Some(git), then_path) => {
                let group_path = self
                    .path
                    .join("groups")
                    .join(format!("{}-{analysis:?}", config.name));

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
                    git::checkout_latest_before(&group_path, deadline).await?;
                }

                let path = if let Some(then_path) = then_path {
                    group_path.join(then_path)
                } else {
                    group_path
                };

                let git_hash = git::hash(&path).await?;

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

    #[tracing::instrument(skip(self))]
    async fn build_group_driver(
        &self,
        config: &config::GroupConfig,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<Driver<InspectifyJobMeta>> {
        let group_git = self.group_repo(config, analysis, deadline).await?;
        let driver = Driver::new_from_path(
            self.hub.clone(),
            &group_git.path,
            group_git
                .path
                .join(config.run.as_deref().unwrap_or("run.toml")),
        )?;
        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    pub async fn work(&self) -> color_eyre::Result<()> {
        let inputs = self.programs_config.inputs().collect_vec();

        let work_queue = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let mut groups = self
            .groups_config
            .groups
            .iter()
            .map(|g| Arc::new(g.clone()))
            .collect_vec();

        loop {
            groups.shuffle(&mut rand::thread_rng());

            self.group_repos.lock().await.clear();
            self.group_states.lock().await.clear();

            for g in &groups {
                for i in &inputs {
                    tracing::debug!(name=?g.name, analysis=?i.analysis(), "creating run for");
                    let run = db::Run::new(Arc::clone(g), i.clone())?;
                    work_queue.lock().await.push(run);
                }
            }

            work_queue.lock().await.shuffle(&mut rand::thread_rng());

            loop {
                let mut join_set = tokio::task::JoinSet::new();

                for run in work_queue.lock().await.drain(..) {
                    let Some(input) = run.input() else {
                        tracing::error!(name=?run.group_config, "could not get input for run");
                        continue;
                    };

                    let deadline = self
                        .programs_config
                        .deadlines
                        .get(&input.analysis())
                        .and_then(|d| {
                            let time = d.time?;
                            let date = time.date?;
                            let time = time.time?;
                            let date: chrono::NaiveDate = chrono::NaiveDate::from_ymd_opt(
                                date.year as _,
                                date.month as _,
                                date.day as _,
                            )?;
                            let time = chrono::NaiveTime::from_hms_opt(
                                time.hour as _,
                                time.minute as _,
                                time.second as _,
                            )?;
                            Some(chrono::NaiveDateTime::new(date, time))
                        });
                    let group_repo = self
                        .group_repo(&run.group_config, input.analysis(), deadline)
                        .await?;

                    if let Some(git_hash) = &group_repo.git_hash {
                        if let Some(data) = self.db.get_cached_run(
                            &db::CacheKeyInput {
                                group_name: &run.group_config.name,
                                git_hash,
                                input: &input,
                            }
                            .key(),
                        )? {
                            let job = self.hub.add_finished_job(data);
                            self.events_tx
                                .send(CheckoEvent::JobAssigned {
                                    group: run.group_config.name.clone(),
                                    kind: JobKind::Analysis(input),
                                    job_id: job.id(),
                                })
                                .unwrap();
                            continue;
                        }
                    }

                    let group_state = self
                        .group_state(&run.group_config, input.analysis(), deadline)
                        .await?;
                    let db = self.db.clone();
                    let events_tx = self.events_tx.clone();
                    join_set.spawn(
                    async move {
                        tracing::debug!(name=?run.group_config, "getting driver from group state");
                        let group_state = group_state.await;
                        let GroupDriver::Driver(driver) = &group_state.driver else {
                            tracing::error!(name=?run.group_config, "group driver missing");

                            return Ok::<_, color_eyre::Report>(());
                        };
                        tracing::debug!(name=?run.group_config, "starting run");
                        // db.start_run(run.id)?;
                        let job = driver.exec_job(
                            &input,
                            InspectifyJobMeta {
                                group_name: Some(run.group_config.name.clone()),
                            },
                        );
                        group_state.active_jobs.lock().unwrap().push(job.clone());
                        events_tx
                            .send(CheckoEvent::JobAssigned {
                                group: run.group_config.name.clone(),
                                kind: JobKind::Analysis(input.clone()),
                                job_id: job.id(),
                            })
                            .unwrap();
                        tracing::debug!(name=?run.group_config, "waiting for job");
                        job.wait().await;
                        tracing::debug!(name=?run.group_config, "job finished");
                        group_state
                            .active_jobs
                            .lock()
                            .unwrap()
                            .retain(|j| j.id() != job.id());

                        if let Some(git_hash) = &group_repo.git_hash {
                            db.insert_cached_run(
                                &db::CacheKeyInput {
                                    group_name: &run.group_config.name,
                                    git_hash,
                                    input: &input,
                                }
                                .key(),
                                &job.data(),
                            )?;
                        }

                        Ok::<_, color_eyre::Report>(())
                    }
                    .in_current_span(),
                );
                }

                if join_set.is_empty() {
                    break;
                }

                while let Some(res) = join_set.join_next().await {
                    if let Err(err) = res? {
                        tracing::error!(?err, "error in join set");
                    }
                }
            }

            tracing::info!("waiting for next batch of runs");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
