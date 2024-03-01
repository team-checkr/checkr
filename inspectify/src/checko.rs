pub mod config;
mod db;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ce_shell::Analysis;
use color_eyre::eyre::{Context, ContextCompat};
use driver::{Driver, Hub, Job, JobData, JobId, JobKind};
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
    group_states: Mutex<IndexMap<(String, Analysis), Arc<GroupState>>>,
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
            group_states: Default::default(),
            events_tx,
            events_rx,
        })
    }

    pub fn events(&self) -> crate::history_broadcaster::Receiver<CheckoEvent> {
        self.events_rx.resubscribe()
    }

    pub fn repopulate_hub(&self) -> color_eyre::Result<()> {
        for run in self.db.all_runs()? {
            tracing::debug!(run_id=?run.id, "repopulating hub");
            let data = run.data.decompress();
            let kind = data.kind.clone();
            let job = self.hub.add_finished_job(data);
            self.events_tx
                .send(CheckoEvent::JobAssigned {
                    group: run.group_name.clone(),
                    kind,
                    job_id: job.id(),
                })
                .unwrap();
        }

        Ok(())
    }

    pub fn groups_config(&self) -> &config::GroupsConfig {
        &self.groups_config
    }

    pub fn programs_config(&self) -> config::CanonicalProgramsConfig {
        self.programs_config.canonicalize().unwrap()
    }

    #[tracing::instrument(skip(self))]
    pub fn group_state(
        &self,
        group_name: &str,
        analysis: Analysis,
    ) -> color_eyre::Result<impl Future<Output = Arc<GroupState>> + 'static> {
        tracing::debug!(?group_name, ?analysis, "getting group state");
        let mut group_states = self.group_states.lock().unwrap();

        let key = (group_name.to_string(), analysis);

        let state = if group_states.contains_key(&key) {
            Arc::clone(&group_states[&key])
        } else {
            let state = self.build_group_state(group_name, analysis)?;
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
    fn build_group_state(
        &self,
        group_name: &str,
        analysis: Analysis,
    ) -> color_eyre::Result<Arc<GroupState>> {
        tracing::debug!(?group_name, "building group state");
        let config = self
            .groups_config
            .groups
            .iter()
            .find(|g| g.name == group_name)
            .wrap_err_with(|| format!("group '{}' not found", group_name))?;
        match self.build_group_driver(config, analysis) {
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
    fn build_group_driver(
        &self,
        config: &config::GroupConfig,
        analysis: Analysis,
    ) -> color_eyre::Result<Driver<InspectifyJobMeta>> {
        let group_path = match (&config.git, &config.path) {
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

                // TODO: we should retry git operations a couple of times, and
                // not rely on sleeping for a bit
                std::thread::sleep(std::time::Duration::from_secs(2));

                std::process::Command::new("git")
                    .arg("clone")
                    .arg(git)
                    .args(["."])
                    .current_dir(&group_path)
                    .stderr(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .output()
                    .wrap_err_with(|| format!("could not clone group git repository: '{git}'"))?;
                std::process::Command::new("git")
                    .arg("pull")
                    .current_dir(&group_path)
                    .stderr(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .output()
                    .wrap_err_with(|| format!("could not pull group git repository: '{git}'"))?;

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

                if let Some(then_path) = then_path {
                    group_path.join(then_path)
                } else {
                    group_path
                }
            }
            (_, Some(path)) => dunce::canonicalize(PathBuf::from(path))
                .wrap_err_with(|| format!("could not canonicalize group path: '{}'", path))?,
            _ => todo!(),
        };
        let driver = Driver::new_from_path(
            self.hub.clone(),
            &group_path,
            group_path.join(config.run.as_deref().unwrap_or("run.toml")),
        )?;
        Ok(driver)
    }

    #[tracing::instrument(skip(self))]
    pub async fn work(&self) -> color_eyre::Result<()> {
        let inputs = self.programs_config.inputs().collect_vec();

        for g in &self.groups_config.groups {
            for i in &inputs {
                let run_id = self.db.run_by_group_and_input(&g.name, i)?;
                if run_id.is_none() {
                    tracing::debug!(name=?g.name, analysis=?i.analysis(), "creating run for");
                    let run = db::Run::new(g.name.clone(), i.clone())?;
                    self.db.create_run(run)?;
                }
            }
        }

        loop {
            let mut join_set = tokio::task::JoinSet::new();

            for run in self.db.unfinished_runs(10)? {
                tracing::debug!(name=?run.group_name, "found unfinished run");
                let Some(input) = run.input() else {
                    tracing::error!(name=?run.group_name, "could not get input for run");
                    continue;
                };
                let group_state = self.group_state(&run.group_name, input.analysis())?;
                let db = self.db.clone();
                let events_tx = self.events_tx.clone();
                join_set.spawn(
                    async move {
                        tracing::debug!(name=?run.group_name, "getting driver from group state");
                        let group_state = group_state.await;
                        let GroupDriver::Driver(driver) = &group_state.driver else {
                            tracing::error!(name=?run.group_name, "group driver missing");
                            db.finish_run(
                                run.id,
                                &JobData {
                                    stderr: Default::default(),
                                    stdout: Default::default(),
                                    combined: Default::default(),
                                    kind: JobKind::Analysis(input),
                                    state: driver::JobState::Warning,
                                    meta: InspectifyJobMeta {
                                        group_name: Some(run.group_name.clone()),
                                    },
                                },
                            )?;
                            return Ok::<_, color_eyre::Report>(());
                        };
                        tracing::debug!(name=?run.group_name, "starting run");
                        db.start_run(run.id)?;
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
                                kind: JobKind::Analysis(input),
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

                        db.finish_run(run.id, &job.data())?;
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
