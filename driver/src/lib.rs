pub mod ansi;
pub mod config;
mod hub;
mod job;

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use ce_shell::Input;
use color_eyre::eyre::Context;
use config::RunOption;

pub use hub::{Hub, HubEvent};
use itertools::Itertools;
pub use job::{Job, JobData, JobEvent, JobId, JobKind, JobState};
use tracing::Instrument;

#[derive(Debug, Clone)]
pub struct Driver<M: 'static> {
    hub: Hub<M>,
    cwd: PathBuf,
    config: RunOption,
    current_compilation: Arc<RwLock<Option<Job<M>>>>,
    latest_successfull_compile: Arc<RwLock<Option<Job<M>>>>,
}

impl<M> PartialEq for Driver<M> {
    fn eq(&self, other: &Self) -> bool {
        self.hub == other.hub
            && self.config == other.config
            && Arc::ptr_eq(
                &self.latest_successfull_compile,
                &other.latest_successfull_compile,
            )
    }
}

impl<M: Debug + Send + Sync + 'static> Driver<M> {
    #[tracing::instrument(skip(hub))]
    pub fn new_from_path(
        hub: Hub<M>,
        cwd: impl AsRef<Path> + Debug,
        path: impl AsRef<Path> + Debug,
    ) -> color_eyre::Result<Self> {
        let cwd = dunce::canonicalize(cwd.as_ref())
            .wrap_err_with(|| format!("could not canonicalize cwd: {cwd:?}"))?;
        let path = path.as_ref().to_path_buf();
        let src = std::fs::read_to_string(&path)
            .wrap_err_with(|| format!("could not read run options at '{}'", path.display()))?;
        let config: RunOption = toml::from_str(&src).wrap_err("error parsing run options")?;

        Ok(Self {
            config,
            cwd,
            hub,
            current_compilation: Default::default(),
            latest_successfull_compile: Default::default(),
        })
    }
    #[tracing::instrument(skip_all, fields(analysis=%input.analysis()))]
    pub fn exec_job(&self, input: &Input, meta: M) -> color_eyre::Result<Job<M>> {
        let mut args = self
            .config
            .run()
            .split(' ')
            .map(|s| s.to_string())
            .collect_vec();
        args.push(input.analysis().code().to_string());
        args.push(input.to_string());
        self.hub.exec_command(
            JobKind::Analysis(input.clone()),
            &self.cwd,
            meta,
            &args[0],
            &args[1..],
        )
    }

    #[tracing::instrument(skip_all)]
    pub fn start_recompile(&self, meta: M) -> Option<color_eyre::Result<Job<M>>>
    where
        M: Clone,
    {
        self.config.compile.as_ref().map(|compile| {
            if let Some(job) = self.current_compilation.write().unwrap().take() {
                job.kill();
            }

            let args = compile.split(' ').collect_vec();
            let job = self.hub.exec_command(
                JobKind::Compilation,
                &self.cwd,
                meta,
                args[0],
                &args[1..],
            )?;
            self.current_compilation
                .write()
                .unwrap()
                .replace(job.clone());
            tokio::spawn({
                let driver = self.clone();
                let job = job.clone();
                async move {
                    tracing::debug!("waiting for it to compile...");
                    let state = job.wait().await;
                    tracing::debug!(?state, "finished!");
                    if let JobState::Succeeded = state {
                        *driver.latest_successfull_compile.write().unwrap() = Some(job)
                    }
                }
            });
            Ok(job)
        })
    }
    pub fn config(&self) -> &RunOption {
        &self.config
    }
    #[tracing::instrument(skip_all)]
    pub fn spawn_watcher(&self, meta: M) -> color_eyre::Result<tokio::task::JoinHandle<()>>
    where
        M: Clone,
    {
        let driver = self.clone();

        let config = driver.config();
        let dir = driver.cwd.clone();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let matches = config
            .watch
            .iter()
            .map(|p| glob::Pattern::new(p).wrap_err_with(|| format!("{p:?} was not a valid glob")))
            .collect::<color_eyre::Result<Vec<glob::Pattern>>>()?;
        let not_matches = config
            .ignore
            .iter()
            .map(|p| glob::Pattern::new(p).wrap_err_with(|| format!("{p:?} was not a valid glob")))
            .collect::<color_eyre::Result<Vec<glob::Pattern>>>()?;
        let debouncer_dir = dunce::canonicalize(&dir)?;
        let mut debouncer = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(200),
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(events) => {
                    if !events.iter().any(|e| {
                        let p = match e.path.strip_prefix(&debouncer_dir) {
                            Ok(p) => p,
                            Err(_) => &e.path,
                        };

                        let matches_positive = matches.iter().any(|pat| pat.matches_path(p));
                        let matches_negative = not_matches.iter().any(|pat| pat.matches_path(p));

                        matches_positive && !matches_negative
                    }) {
                        return;
                    }
                    tracing::debug!("a file was saved: {events:?}");

                    tx.send(()).expect("sending to file watcher failed");
                }
                Err(err) => tracing::error!(?err, "Error"),
            },
        )?;
        debouncer
            .watcher()
            .watch(&dir, notify::RecursiveMode::Recursive)?;

        Ok(tokio::spawn(
            async move {
                let mut last_job: Option<color_eyre::Result<Job<M>>> = None;
                while let Some(()) = rx.recv().await {
                    if let Some(Ok(last_job)) = last_job {
                        last_job.kill();
                    }
                    last_job = driver.start_recompile(meta.clone());
                }
                // NOTE: It is important to keep the debouncer alive for as long as the
                // tokio process
                drop(debouncer);
            }
            .in_current_span(),
        ))
    }

    pub fn latest_successfull_compile(&self) -> Option<Job<M>> {
        self.latest_successfull_compile.read().unwrap().clone()
    }

    pub fn current_compilation(&self) -> Option<Job<M>> {
        self.current_compilation.read().unwrap().clone()
    }

    #[tracing::instrument(skip_all)]
    pub fn ensure_compile(&self, meta: M) -> color_eyre::Result<Option<Job<M>>>
    where
        M: Clone,
    {
        let current_compilation_job = self.current_compilation.read().unwrap().clone();
        if let Some(job) = current_compilation_job {
            Ok(Some(job))
        } else {
            self.start_recompile(meta).transpose()
        }
    }
}
