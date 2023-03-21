use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use checko::RunOption;
use checkr::driver::{Driver, DriverError};
use color_eyre::eyre::Context;
use indicatif::ProgressStyle;
use notify_debouncer_mini::DebounceEventResult;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
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

pub fn initialize_driver(dir: &Path, run: &RunOption) -> color_eyre::Result<Driver> {
    if let Some(compile) = &run.compile {
        clear_terminal()?;

        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
        spinner.set_message("compiling your code...");
        spinner.enable_steady_tick(Duration::from_millis(50));

        let driver = Driver::compile(dir, compile, &run.run)
            .wrap_err_with(|| format!("compiling using config: {run:?}"))?;

        spinner.finish();

        Ok(driver)
    } else {
        Ok(Driver::new(dir, &run.run))
    }
}

pub fn spawn_watcher(
    shared_driver: &Arc<Mutex<Driver>>,
    shared_compilation_status: &Arc<Mutex<CompilationStatus>>,
    dir: PathBuf,
    run: checko::RunOption,
) -> Result<(), color_eyre::Report> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let driver = Arc::clone(shared_driver);
    let compilation_status = Arc::clone(shared_compilation_status);

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
            *compilation_status.lock().await = CompilationStatus::new(CompilerState::Compiling);
            let new_driver = if let Some(compile) = &run.compile {
                let compile_result = Driver::compile(&dir, compile, &run.run);

                spinner.finish_and_clear();

                match compile_result {
                    Ok(driver) => driver,
                    Err(DriverError::CompileFailure(output)) => {
                        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
                        let stderr = String::from_utf8(output.stderr.clone()).unwrap();

                        error!("failed to compile:");
                        eprintln!("{stderr}");
                        eprintln!("{stdout}");
                        *compilation_status.lock().await =
                            CompilationStatus::new(CompilerState::CompileError { stdout, stderr });
                        continue;
                    }
                    Err(DriverError::RunCompile(err)) => {
                        error!("run compile failed:");
                        eprintln!("{err}");
                        *compilation_status.lock().await =
                            CompilationStatus::new(CompilerState::CompileError {
                                stdout: format!("{:?}", err),
                                stderr: String::new(),
                            });
                        continue;
                    }
                }
            } else {
                Driver::new(&dir, &run.run)
            };
            info!("compiled in {:?}", compile_start.elapsed());
            *compilation_status.lock().await = CompilationStatus::new(CompilerState::Compiled);
            *driver.lock().await = new_driver;
        }
        // NOTE: It is important to keep the debouncer alive for as long as the
        // tokio process
        drop(debouncer);
    });
    Ok(())
}
