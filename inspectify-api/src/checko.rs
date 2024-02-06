mod config;
mod db;

use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use color_eyre::eyre::{Context, ContextCompat};
use driver::{Driver, Hub, Job, JobData, JobKind};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::endpoints::InspectifyJobMeta;

use self::db::RunData;

pub struct Checko {
    hub: Hub<InspectifyJobMeta>,
    path: PathBuf,
    db: db::CheckoDb,
    groups_config: config::GroupsConfig,
    programs_config: config::ProgramsConfig,
    group_states: IndexMap<String, Arc<GroupState>>,
}

pub struct GroupState {
    config: config::GroupConfig,
    driver: Driver<InspectifyJobMeta>,
    active_jobs: Mutex<Vec<Job<InspectifyJobMeta>>>,
}

impl Checko {
    pub fn open(hub: Hub<InspectifyJobMeta>, path: &Path) -> color_eyre::Result<Self> {
        let path = path
            .canonicalize()
            .wrap_err_with(|| format!("could not canonicalize path: '{}'", path.display()))?;

        let runs_db_path = path.join("runs.db3");
        let groups_path = path
            .join("groups.toml")
            .canonicalize()
            .wrap_err_with(|| format!("missing groups.toml at '{}'", path.display()))?;
        let programs_path = path
            .join("programs.toml")
            .canonicalize()
            .wrap_err_with(|| format!("missing programs.toml at '{}'", path.display()))?;

        let db = db::CheckoDb::open(&runs_db_path).wrap_err("could not open db")?;
        let groups = config::read_groups(groups_path)?;
        let programs = config::read_programs(programs_path)?;

        Ok(Self {
            hub,
            path,
            db,
            groups_config: groups,
            programs_config: programs,
            group_states: IndexMap::new(),
        })
    }

    pub fn repopulate_hub(&self) -> color_eyre::Result<()> {
        for run in self.db.all_runs()? {
            let data = run.data.decompress();

            let (stderr, stdout, combined) = match (data.stderr, data.stdout, data.combined) {
                (Some(stderr), Some(stdout), Some(combined)) => (stderr, stdout, combined),
                _ => continue,
            };

            self.hub.add_finished_job(JobData {
                kind: JobKind::Analysis(data.input),
                stderr: stderr.into_bytes(),
                stdout: stdout.into_bytes(),
                combined: combined.into_bytes(),
                state: driver::JobState::Warning,
                meta: InspectifyJobMeta {
                    group_name: Some(run.group_name.clone()),
                },
            });
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn group_state(
        &mut self,
        group_name: &str,
    ) -> color_eyre::Result<impl Future<Output = Arc<GroupState>> + 'static> {
        let state = if self.group_states.contains_key(group_name) {
            let group_name = group_name.to_string();
            Arc::clone(&self.group_states[&group_name])
        } else {
            let config = self
                .groups_config
                .groups
                .iter()
                .find(|g| g.name == group_name)
                .wrap_err_with(|| format!("group '{}' not found", group_name))?;

            let group_path = self.path.join("groups").join(&config.name);

            std::fs::create_dir_all(&group_path).wrap_err_with(|| {
                format!(
                    "could not create group directory: '{}'",
                    group_path.display()
                )
            })?;

            std::process::Command::new("git")
                .arg("clone")
                .arg(&config.git)
                .args(["."])
                .current_dir(&group_path)
                .stderr(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .output()
                .wrap_err_with(|| {
                    format!("could not clone group git repository: '{}'", config.git)
                })?;
            std::process::Command::new("git")
                .arg("pull")
                .current_dir(&group_path)
                .stderr(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .output()
                .wrap_err_with(|| {
                    format!("could not pull group git repository: '{}'", config.git)
                })?;

            let date: Option<&str> = None;
            // checkout latest commit before a set date
            if let Some(date) = date {
                let commit_rev_bytes = std::process::Command::new("git")
                    .arg("rev-list")
                    .arg("-n")
                    .arg("1")
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
                    .wrap_err_with(|| format!("could not checkout latest commit: {commit_rev}"))?;
            }

            let driver =
                Driver::new_from_path(self.hub.clone(), &group_path, group_path.join("run.toml"))?;

            let state = Arc::new(GroupState {
                config: config.clone(),
                driver,
                active_jobs: Mutex::new(Vec::new()),
            });

            self.group_states
                .insert(group_name.to_string(), Arc::clone(&state));

            state
        };

        let compile_job = state.driver.ensure_compile(InspectifyJobMeta {
            group_name: Some(group_name.to_string()),
        })?;

        Ok(async move {
            if let Some(job) = compile_job {
                job.wait().await;
            }
            state
        })
    }

    pub async fn work(&mut self) -> color_eyre::Result<()> {
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

            for run in self.db.unfinished_runs(3)? {
                let group_state = self.group_state(&run.group_name)?;
                let db = self.db.clone();
                join_set.spawn(async move {
                    let group_state = group_state.await;
                    db.start_run(run.id)?;
                    let input = run.input();
                    let job = group_state.driver.exec_job(
                        &input,
                        InspectifyJobMeta {
                            group_name: Some(run.group_name.clone()),
                        },
                    )?;
                    group_state.active_jobs.lock().unwrap().push(job.clone());
                    job.wait().await;
                    group_state
                        .active_jobs
                        .lock()
                        .unwrap()
                        .retain(|j| j.id() != job.id());
                    db.finish_run(
                        run.id,
                        &RunData {
                            input,
                            stdout: Some(job.stdout()),
                            stderr: Some(job.stderr()),
                            combined: Some(job.stdout_and_stderr()),
                        },
                    )?;
                    Ok::<_, color_eyre::Report>(())
                });
            }

            if join_set.is_empty() {
                break;
            }

            while join_set.join_next().await.is_some() {}
        }

        Ok(())
    }
}
