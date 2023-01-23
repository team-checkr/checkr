use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use itertools::Itertools;
use notify::Watcher;
use notify_debouncer_mini::DebounceEventResult;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use verification_lawyer::{
    driver::{Driver, DriverError},
    env::{
        graph::{GraphEnv, GraphEnvInput},
        pv::ProgramVerificationEnv,
        Environment, SecurityEnv, SignEnv, StepWiseEnv, ToMarkdown,
    },
    pg::Determinism,
};

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Analysis {
    Graph,
    Sign,
    StepWise,
    Security,
    ProgramVerification,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub analysis: Analysis,
    pub src: String,
    pub input: String,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub stdout: String,
    pub stderr: String,
    pub parsed_markdown: Option<String>,
    pub took: Duration,
    pub validation_result: Option<ValidationResult>,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum ValidationResult {
    CorrectTerminated,
    CorrectNonTerminated { iterations: u32 },
    Mismatch { reason: String },
    TimeOut,
}

impl From<verification_lawyer::env::ValidationResult> for ValidationResult {
    fn from(r: verification_lawyer::env::ValidationResult) -> Self {
        use verification_lawyer::env::ValidationResult as VR;

        match r {
            VR::CorrectTerminated => ValidationResult::CorrectTerminated,
            VR::CorrectNonTerminated { iterations } => ValidationResult::CorrectNonTerminated {
                iterations: iterations as _,
            },
            VR::Mismatch { reason } => ValidationResult::Mismatch { reason },
            VR::TimeOut => ValidationResult::TimeOut,
        }
    }
}

fn ayo<E: Environment>(driver: &Mutex<Driver>, env: E, cmds: &str, input: &str) -> AnalysisResponse
where
    E: std::fmt::Debug,
    E::Output: ToMarkdown,
{
    info!(env = format!("{env:?}"), input = input);
    let input: E::Input = serde_json::from_str(input).expect("failed to parse input");
    match driver.lock().unwrap().exec_raw_cmds::<E>(&cmds, &input) {
        Ok(exec_output) => {
            let cmds = verification_lawyer::parse::parse_commands(&cmds).unwrap();
            let validation_res = env.validate(&cmds, &input, &exec_output.parsed);
            AnalysisResponse {
                stdout: String::from_utf8(exec_output.output.stdout).unwrap(),
                stderr: String::from_utf8(exec_output.output.stderr).unwrap(),
                parsed_markdown: Some(exec_output.parsed.to_markdown()),
                took: exec_output.took,
                validation_result: Some(validation_res.into()),
            }
        }
        Err(e) => match e {
            verification_lawyer::driver::ExecError::Serialize(_) => todo!(),
            verification_lawyer::driver::ExecError::RunExec(_) => todo!(),
            verification_lawyer::driver::ExecError::CommandFailed(output, took) => {
                AnalysisResponse {
                    stdout: String::from_utf8(output.stdout).unwrap(),
                    stderr: String::from_utf8(output.stderr).unwrap(),
                    parsed_markdown: None,
                    took,
                    validation_result: None,
                }
            }
            verification_lawyer::driver::ExecError::Parse {
                inner,
                run_output,
                time,
            } => AnalysisResponse {
                stdout: String::from_utf8(run_output.stdout).unwrap(),
                stderr: String::from_utf8(run_output.stderr).unwrap(),
                parsed_markdown: None,
                took: time,
                validation_result: None,
            },
        },
    }
}

async fn analyze(
    State(state): State<ApplicationState>,
    Json(body): Json<AnalysisRequest>,
) -> Json<AnalysisResponse> {
    let driver = &*state.driver;
    let cmds = body.src;
    let output = match body.analysis {
        Analysis::Graph => ayo(driver, GraphEnv, &cmds, &body.input),
        Analysis::Sign => ayo(driver, SignEnv, &cmds, &body.input),
        Analysis::StepWise => ayo(driver, StepWiseEnv, &cmds, &body.input),
        Analysis::Security => ayo(driver, SecurityEnv, &cmds, &body.input),
        Analysis::ProgramVerification => ayo(driver, ProgramVerificationEnv, &cmds, &body.input),
    };
    Json(output)
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRequest {
    src: String,
    deterministic: bool,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResponse {
    dot: Option<String>,
}

async fn graph(
    State(state): State<ApplicationState>,
    Json(body): Json<GraphRequest>,
) -> Json<GraphResponse> {
    match state.driver.lock().unwrap().exec_raw_cmds::<GraphEnv>(
        &body.src,
        &GraphEnvInput {
            determinism: match body.deterministic {
                true => Determinism::Deterministic,
                false => Determinism::NonDeterministic,
            },
        },
    ) {
        Ok(output) => Json(GraphResponse {
            dot: Some(output.parsed.dot),
        }),
        Err(_) => Json(GraphResponse { dot: None }),
    }
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
enum CompilerState {
    Compiling,
    Compiled,
    CompileError,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompilationStatus {
    compiled_at: u32,
    state: CompilerState,
}

async fn compilation_status(State(state): State<ApplicationState>) -> Json<CompilationStatus> {
    Json(state.compilation_status.lock().unwrap().clone())
}

#[axum::debug_handler]
async fn static_dir(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    static UI_DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../ui/dist/");

    if uri.path() == "/" {
        return Html(
            UI_DIR
                .get_file("index.html")
                .unwrap()
                .contents_utf8()
                .unwrap(),
        )
        .into_response();
    }

    match UI_DIR.get_file(&uri.path()[1..]) {
        Some(file) => {
            let mime_type = mime_guess::from_path(uri.path())
                .first_raw()
                .map(HeaderValue::from_static)
                .unwrap_or_else(|| {
                    HeaderValue::from_str(mime::APPLICATION_OCTET_STREAM.as_ref()).unwrap()
                });
            (
                [(axum::http::header::CONTENT_TYPE, mime_type)],
                file.contents(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Debug, Parser)]
enum Cli {
    WupWup {
        #[clap(short, long, default_value_t = false)]
        open: bool,
        dir: PathBuf,
    },
}

#[derive(Clone)]
struct ApplicationState {
    driver: Arc<Mutex<Driver>>,
    compilation_status: Arc<Mutex<CompilationStatus>>,
}

async fn run() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::WupWup { open, dir } => {
            let run = infra::RunOption::from_file(dir.join("run.toml"))
                .with_context(|| format!("could not to read {:?}", dir.join("run.toml")))?;

            let driver = if let Some(compile) = &run.compile {
                Driver::compile(&dir, compile, &run.run)
                    .with_context(|| format!("compiling using config: {run:?}"))?
            } else {
                Driver::new(&dir, &run.run)
            };
            let shared_driver = Arc::new(Mutex::new(driver));
            let shared_compilation_status = Arc::new(Mutex::new(CompilationStatus {
                compiled_at: std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as _,
                state: CompilerState::Compiled,
            }));

            let matches = run
                .watch
                .iter()
                .map(|p| glob::Pattern::new(p).unwrap())
                .collect_vec();

            let watcher_driver = Arc::clone(&shared_driver);
            let watcher_compilation_status = Arc::clone(&shared_compilation_status);
            let watcher_dir = dir.clone();
            let watcher_run = run.clone();
            let mut watcher = notify_debouncer_mini::new_debouncer(
                Duration::from_millis(200),
                None,
                move |res: DebounceEventResult| match res {
                    Ok(events) => {
                        if !events
                            .iter()
                            .any(|e| matches.iter().any(|p| p.matches_path(&e.path)))
                        {
                            return;
                        }

                        let (run, dir) = (watcher_run.clone(), watcher_dir.clone());

                        info!("Recompile!");
                        *watcher_compilation_status.lock().unwrap() = CompilationStatus {
                            compiled_at: std::time::SystemTime::now()
                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as _,
                            state: CompilerState::Compiling,
                        };
                        let driver = if let Some(compile) = &run.compile {
                            match Driver::compile(&dir, compile, &run.run) {
                                Ok(driver) => driver,
                                Err(DriverError::CompileFailure(_)) => {
                                    *watcher_compilation_status.lock().unwrap() =
                                        CompilationStatus {
                                            compiled_at: std::time::SystemTime::now()
                                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis()
                                                as _,
                                            state: CompilerState::CompileError,
                                        };
                                    return;
                                }
                                Err(DriverError::RunCompile(_)) => {
                                    *watcher_compilation_status.lock().unwrap() =
                                        CompilationStatus {
                                            compiled_at: std::time::SystemTime::now()
                                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis()
                                                as _,
                                            state: CompilerState::CompileError,
                                        };
                                    return;
                                }
                            }
                        } else {
                            Driver::new(&dir, &run.run)
                        };
                        *watcher_compilation_status.lock().unwrap() = CompilationStatus {
                            compiled_at: std::time::SystemTime::now()
                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as _,
                            state: CompilerState::Compiled,
                        };
                        *watcher_driver.lock().unwrap() = driver;
                    }
                    Err(errors) => errors.iter().for_each(|e| println!("Error {:?}", e)),
                },
            )?;
            watcher
                .watcher()
                .watch(&dir, notify::RecursiveMode::Recursive)?;

            let app = Router::new()
                .route("/analyze", post(analyze))
                .route("/graph", post(graph))
                .route("/compilation-status", get(compilation_status))
                .with_state(ApplicationState {
                    driver: shared_driver,
                    compilation_status: shared_compilation_status,
                });
            let app = app.fallback(static_dir);
            let app = app.layer(
                CorsLayer::new()
                    .allow_origin("*".parse::<HeaderValue>().unwrap())
                    .allow_headers([CONTENT_TYPE])
                    .allow_methods([Method::GET, Method::POST]),
            );
            // NOTE: Enable for HTTP logging
            // .layer(TraceLayer::new_for_http());

            if open {
                tokio::task::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    open::that("http://localhost:3000").unwrap();
                });
            }

            let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap();

            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    run().await
}
