pub mod config;
mod db;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ce_shell::Analysis;
use color_eyre::eyre::{Context, ContextCompat};
use driver::{Driver, Hub, Job, JobId, JobKind};
use indexmap::IndexMap;
use itertools::Itertools;
use tracing::Instrument;

use crate::endpoints::InspectifyJobMeta;

pub struct Checko {
    hub: Hub<InspectifyJobMeta>,
    path: PathBuf,
    db: db::CheckoDb,
    groups_config: config::GroupsConfig,
    programs_config: config::ProgramsConfig,
    group_repos: tokio::sync::Mutex<IndexMap<(String, Analysis), GroupRepo>>,
    group_states: tokio::sync::Mutex<IndexMap<(String, Analysis), Arc<GroupState>>>,
    events_tx: crate::history_broadcaster::Sender<CheckoEvent>,
    events_rx: crate::history_broadcaster::Receiver<CheckoEvent>,
}

#[derive(Debug, Clone)]
pub enum CheckoEvent {
    JobAssigned {
        group: String,
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
    config: config::GroupConfig,
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
        group_name: &str,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<GroupRepo> {
        let key = (group_name.to_string(), analysis);

        let gs = self.group_repos.lock().await.get(&key).cloned();
        if let Some(repo) = gs {
            Ok(repo)
        } else {
            let config = self
                .groups_config
                .groups
                .iter()
                .find(|g| g.name == group_name)
                .wrap_err_with(|| format!("group '{}' not found", group_name))?;
            let repo = self.update_group_repo(config, analysis, deadline).await?;
            let mut group_repos = self.group_repos.lock().await;
            group_repos.insert(key, repo.clone());
            Ok(repo)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn group_state(
        &self,
        group_name: &str,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<impl Future<Output = Arc<GroupState>> + 'static> {
        let key = (group_name.to_string(), analysis);

        let gs = self.group_states.lock().await.get(&key).cloned();
        let state = if let Some(gs) = gs {
            gs
        } else {
            let state = self
                .build_group_state(group_name, analysis, deadline)
                .await?;
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
        group_name: &str,
        analysis: Analysis,
        deadline: Option<chrono::NaiveDateTime>,
    ) -> color_eyre::Result<Arc<GroupState>> {
        tracing::debug!(?group_name, "building group state");
        let config = self
            .groups_config
            .groups
            .iter()
            .find(|g| g.name == group_name)
            .wrap_err_with(|| format!("group '{}' not found", group_name))?;
        match self.build_group_driver(config, analysis, deadline).await {
            Ok(driver) => {
                tracing::debug!(?group_name, "ensuring compile job");
                let compile_job = driver.ensure_compile(InspectifyJobMeta {
                    group_name: Some(group_name.to_string()),
                });
                tracing::debug!(?group_name, "group state built successfully");
                let state = Arc::new(GroupState {
                    config: config.clone(),
                    driver: GroupDriver::Driver(driver),
                    compile_job,
                    active_jobs: Mutex::new(Vec::new()),
                });
                Ok(state)
            }
            Err(err) => {
                tracing::error!(?err, "could not build group driver");
                let state = Arc::new(GroupState {
                    config: config.clone(),
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

                let git_ssh_command = "ssh -o ControlPath=~/.ssh/cm_socket/%r@%h:%p -o ControlMaster=auto -o ControlPersist=60";

                let git_pull_result: color_eyre::Result<()> = tokio_retry::Retry::spawn(
                    tokio_retry::strategy::FixedInterval::new(std::time::Duration::from_secs(5))
                        .take(15),
                    || async {
                        if !group_path.join(".git").try_exists().unwrap_or(false) {
                            tracing::info!(?git, "cloning group git repository");
                            let status = tokio::process::Command::new("git")
                                .arg("clone")
                                .arg(git)
                                .args(["."])
                                .env("GIT_SSH_COMMAND", git_ssh_command)
                                .current_dir(&group_path)
                                .stderr(std::process::Stdio::inherit())
                                .stdout(std::process::Stdio::inherit())
                                .status()
                                .await
                                .wrap_err_with(|| {
                                    format!("could not clone group git repository: '{git}'")
                                })?;
                            tracing::debug!(code=?status.code(), "git clone status");
                            if !status.success() {
                                return Err(color_eyre::eyre::eyre!("git clone failed"));
                            }
                        } else {
                            tracing::info!(?git, "pulling group git repository");
                            let status = tokio::process::Command::new("git")
                                .arg("pull")
                                .env("GIT_SSH_COMMAND", git_ssh_command)
                                .current_dir(&group_path)
                                .stderr(std::process::Stdio::inherit())
                                .stdout(std::process::Stdio::inherit())
                                .status()
                                .await
                                .wrap_err_with(|| {
                                    format!("could not pull group git repository: '{git}'")
                                })?;
                            tracing::debug!(code=?status.code(), "git pull status");
                            if !status.success() {
                                return Err(color_eyre::eyre::eyre!("git pull failed"));
                            }
                        }

                        Ok(())
                    },
                )
                .await;

                git_pull_result?;

                let date: Option<&str> = None;
                // checkout latest commit before a set date
                if let Some(date) = date {
                    let commit_rev_bytes = std::process::Command::new("git")
                        .args(["rev-list", "-n", "1"])
                        .arg(format!("--before={date}"))
                        .arg("HEAD")
                        .current_dir(&group_path)
                        .output()
                        .wrap_err_with(|| format!("could not get latest commit before {date}"))?
                        .stdout;
                    let commit_rev = std::str::from_utf8(&commit_rev_bytes).unwrap();
                    std::process::Command::new("git")
                        .arg("checkout")
                        .arg(commit_rev)
                        .current_dir(&group_path)
                        .stderr(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .output()
                        .wrap_err_with(|| {
                            format!("could not checkout latest commit: {commit_rev}")
                        })?;
                }

                let path = if let Some(then_path) = then_path {
                    group_path.join(then_path)
                } else {
                    group_path
                };

                let git_hash = std::process::Command::new("git")
                    .arg("rev-parse")
                    .arg("HEAD")
                    .current_dir(&path)
                    .output()
                    .wrap_err("could not get git hash for group")?
                    .stdout;
                let git_hash = std::str::from_utf8(&git_hash).unwrap().trim().to_string();

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
        let group_git = self.group_repo(&config.name, analysis, deadline).await?;
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

        let work_queue = Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::new()));

        for g in &self.groups_config.groups {
            for i in &inputs {
                tracing::debug!(name=?g.name, analysis=?i.analysis(), "creating run for");
                let run = db::Run::new(g.name.clone(), i.clone())?;
                // self.db.create_run(run.clone())?;
                work_queue.lock().await.push_back(run);
            }
        }

        loop {
            let mut join_set = tokio::task::JoinSet::new();

            static JOB_SEMAPHORE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(10);

            // for run in self.db.unfinished_runs(100)? {
            for run in work_queue.lock().await.drain(..) {
                let Some(input) = run.input() else {
                    tracing::error!(name=?run.group_name, "could not get input for run");
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
                    .group_repo(&run.group_name, input.analysis(), deadline)
                    .await?;

                if let Some(git_hash) = &group_repo.git_hash {
                    if let Some(data) = self.db.get_cached_run(
                        &db::CacheKeyInput {
                            group_name: &run.group_name,
                            git_hash,
                            input: &input,
                        }
                        .key(),
                    )? {
                        let job = self.hub.add_finished_job(data);
                        self.events_tx
                            .send(CheckoEvent::JobAssigned {
                                group: run.group_name.clone(),
                                kind: JobKind::Analysis(input),
                                job_id: job.id(),
                            })
                            .unwrap();
                        continue;
                    }
                }

                let group_state = self
                    .group_state(&run.group_name, input.analysis(), deadline)
                    .await?;
                let db = self.db.clone();
                let events_tx = self.events_tx.clone();
                join_set.spawn(
                    async move {
                        let _permit = JOB_SEMAPHORE.acquire().await;

                        tracing::debug!(name=?run.group_name, "getting driver from group state");
                        let group_state = group_state.await;
                        let GroupDriver::Driver(driver) = &group_state.driver else {
                            tracing::error!(name=?run.group_name, "group driver missing");

                            return Ok::<_, color_eyre::Report>(());
                        };
                        tracing::debug!(name=?run.group_name, "starting run");
                        // db.start_run(run.id)?;
                        let job = driver.exec_job(
                            &input,
                            InspectifyJobMeta {
                                group_name: Some(run.group_name.clone()),
                            },
                        );
                        group_state.active_jobs.lock().unwrap().push(job.clone());
                        events_tx
                            .send(CheckoEvent::JobAssigned {
                                group: run.group_name.clone(),
                                kind: JobKind::Analysis(input.clone()),
                                job_id: job.id(),
                            })
                            .unwrap();
                        tracing::debug!(name=?run.group_name, "waiting for job");
                        job.wait().await;
                        tracing::debug!(name=?run.group_name, "job finished");
                        group_state
                            .active_jobs
                            .lock()
                            .unwrap()
                            .retain(|j| j.id() != job.id());

                        if let Some(git_hash) = &group_repo.git_hash {
                            db.insert_cached_run(
                                &db::CacheKeyInput {
                                    group_name: &run.group_name,
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

        Ok(())
    }
}
