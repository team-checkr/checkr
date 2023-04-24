use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use checkr::{
    config::RunOption,
    driver::{Driver, DriverError},
};
use color_eyre::eyre::Context;
use indicatif::ProgressStyle;
use notify_debouncer_mini::DebounceEventResult;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info};

use crate::clear_terminal;

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum CompilerState {
    Compiling,
    Compiled,
    CompileError { stdout: String, stderr: String },
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationStatus {
    compiled_at: u32,
    state: CompilerState,
}

impl CompilationStatus {
    pub fn compiled() -> Self {
        Self::new(CompilerState::Compiled)
    }
    pub fn new(state: CompilerState) -> Self {
        Self {
            compiled_at: std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as _,
            state,
        }
    }
}

#[derive(Clone)]
pub struct Compilation {
    pub driver: Arc<Mutex<Driver>>,
    pub status: Arc<Mutex<CompilationStatus>>,
    pub stream: Arc<broadcast::Sender<CompilationStatus>>,
}

impl Compilation {
    pub async fn initialize(dir: PathBuf, run: RunOption) -> color_eyre::Result<Self> {
        let driver = initialize_driver(&dir, &run).await?;

        let driver = Arc::new(Mutex::new(driver));
        let status = Arc::new(Mutex::new(CompilationStatus::compiled()));
        let (stream, _rx) = tokio::sync::broadcast::channel(100);
        let stream = Arc::new(stream);

        let compilation = Self {
            driver,
            status,
            stream,
        };

        compilation.clone().spawn_watcher(dir, run)?;

        Ok(compilation)
    }

    fn spawn_watcher(self, dir: PathBuf, run: RunOption) -> Result<(), color_eyre::Report> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let matches = run
            .watch
            .iter()
            .map(|p| glob::Pattern::new(p).wrap_err_with(|| format!("{p:?} was not a valid glob")))
            .collect::<Result<Vec<glob::Pattern>, color_eyre::Report>>()?;
        let not_matches = run
            .ignore
            .iter()
            .map(|p| glob::Pattern::new(p).wrap_err_with(|| format!("{p:?} was not a valid glob")))
            .collect::<Result<Vec<glob::Pattern>, color_eyre::Report>>()?;
        let debouncer_dir = dir.canonicalize()?;
        let mut debouncer = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(200),
            None,
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    // debug!("a file was saved: {events:?}");
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

                    tx.send(()).expect("sending to file watcher failed");
                }
                Err(errors) => errors.iter().for_each(|e| eprintln!("Error {e:?}")),
            },
        )?;
        debouncer
            .watcher()
            .watch(&dir, notify::RecursiveMode::Recursive)?;

        tokio::spawn(async move {
            while let Some(()) = rx.recv().await {
                let _ = clear_terminal();

                let spinner = indicatif::ProgressBar::new_spinner();
                spinner.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
                spinner.set_message("recompiling your code due to changes...");
                spinner.enable_steady_tick(Duration::from_millis(50));

                // info!("recompiling due to changes!");
                let compile_start = std::time::Instant::now();
                let status = CompilationStatus::new(CompilerState::Compiling);
                let _ = self.stream.send(status.clone());
                *self.status.lock().await = status;
                let new_driver = if let Some(compile) = run.compile.clone() {
                    let compile_result = {
                        let dir = dir.clone();
                        let run = run.run.clone();
                        Driver::compile(dir, &compile, &run).await
                    };

                    spinner.finish_and_clear();

                    match compile_result {
                        Ok(driver) => driver,
                        Err(DriverError::CompileFailure(output)) => {
                            let stdout = String::from_utf8(output.stdout.clone()).unwrap();
                            let stderr = String::from_utf8(output.stderr.clone()).unwrap();

                            error!("failed to compile:");
                            eprintln!("{stderr}");
                            eprintln!("{stdout}");
                            let status = CompilationStatus::new(CompilerState::CompileError {
                                stdout,
                                stderr,
                            });
                            let _ = self.stream.send(status.clone());
                            *self.status.lock().await = status;
                            continue;
                        }
                        Err(DriverError::RunCompile(err)) => {
                            error!("run compile failed:");
                            eprintln!("{err}");
                            let status = CompilationStatus::new(CompilerState::CompileError {
                                stdout: format!("{:?}", err),
                                stderr: String::new(),
                            });
                            let _ = self.stream.send(status.clone());
                            *self.status.lock().await = status;
                            continue;
                        }
                    }
                } else {
                    Driver::new(&dir, &run.run)
                };
                info!("compiled in {:?}", compile_start.elapsed());
                let status = CompilationStatus::new(CompilerState::Compiled);
                let _ = self.stream.send(status.clone());
                *self.status.lock().await = status;
                *self.driver.lock().await = new_driver;
            }
            // NOTE: It is important to keep the debouncer alive for as long as the
            // tokio process
            drop(debouncer);
        });
        Ok(())
    }
}

async fn initialize_driver(dir: &Path, run: &RunOption) -> color_eyre::Result<Driver> {
    if let Some(compile) = &run.compile {
        clear_terminal()?;

        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
        spinner.set_message("compiling your code...");
        spinner.enable_steady_tick(Duration::from_millis(50));

        let driver = Driver::compile(dir, compile, &run.run)
            .await
            .wrap_err_with(|| format!("compiling using config: {run:?}"))?;

        spinner.finish();

        Ok(driver)
    } else {
        Ok(Driver::new(dir, &run.run))
    }
}
