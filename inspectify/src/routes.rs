use std::time::Duration;

use axum::{
    extract::{
        ws::{self, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post, IntoMakeService},
    Json, Router,
};
use checkr::{
    env::{graph::GraphEnvInput, Analysis, EnvError, GraphEnv, Markdown},
    pg::Determinism,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tracing::error;

use crate::{core, ApplicationState, CompilationStatus, ValidationResult};

pub fn router(state: ApplicationState) -> IntoMakeService<Router<()>> {
    Router::new()
        .route("/analyze", post(analyze))
        .route("/graph", post(graph))
        .route("/compilation-ws", get(compilation_ws))
        .route("/core/generate_program", post(core::generate_program))
        .route("/core/dot", post(core::dot))
        .route(
            "/core/complete_input_from_json",
            post(core::complete_input_from_json),
        )
        .route("/core/generate_input_for", post(core::generate_input_for))
        .route("/core/run_analysis", post(core::run_analysis))
        .with_state(state)
        .fallback(static_dir)
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_headers([CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST]),
        )
        .into_make_service()
}

#[axum::debug_handler]
pub async fn static_dir(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    static UI_DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/ui/dist/");

    if uri.path() == "/" {
        let index = if let Some(index) = UI_DIR.get_file("index.html") {
            index.contents_utf8().unwrap()
        } else {
            "Frontend has not been build for release yet! Visit <a href=\"http://localhost:3001/\">localhost:3001</a> for the development site!"
        };
        return Html(index).into_response();
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

async fn compilation_ws(
    ws: WebSocketUpgrade,
    State(state): State<ApplicationState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: ApplicationState) {
    let (mut sender, _rx) = stream.split();

    let mut rx = state.compilation.stream.subscribe();

    tokio::spawn(async move {
        let prepare = move |status: &CompilationStatus| {
            ws::Message::Text(serde_json::to_string(status).unwrap())
        };

        if sender
            .send(prepare(&*state.compilation.status.lock().await))
            .await
            .is_err()
        {
            return;
        }

        while let Ok(status) = rx.recv().await {
            if sender.send(prepare(&status)).await.is_err() {
                break;
            }
        }
    });
}

#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRequest {
    src: String,
    deterministic: bool,
}
#[typeshare::typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum GraphResponse {
    Graph { dot: String },
    Error { error: String },
}
pub async fn graph(
    State(state): State<ApplicationState>,
    Json(body): Json<GraphRequest>,
) -> Json<GraphResponse> {
    match state
        .compilation
        .driver
        .lock()
        .await
        .exec_raw_cmds::<GraphEnv>(
            &body.src,
            &GraphEnvInput {
                determinism: match body.deterministic {
                    true => Determinism::Deterministic,
                    false => Determinism::NonDeterministic,
                },
            },
        )
        .await
    {
        Ok(output) => Json(GraphResponse::Graph {
            dot: output.parsed.dot,
        }),
        Err(err) => Json(GraphResponse::Error {
            error: format!("{err:#?}"),
        }),
    }
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
    pub parsed_markdown: Option<Markdown>,
    pub took: Duration,
    pub validation_result: Option<ValidationResult>,
}
pub async fn analyze(
    State(state): State<ApplicationState>,
    Json(body): Json<AnalysisRequest>,
) -> Json<AnalysisResponse> {
    let input = body
        .analysis
        .input_from_str(&body.input)
        .expect("failed to parse input");
    let cmds = body.src;
    let driver = state.compilation.driver.lock().await;
    let output = match driver
        .exec_dyn_raw_cmds(body.analysis, &cmds, &body.input)
        .await
    {
        Ok(exec_output) => {
            let cmds = match checkr::parse::parse_commands(&cmds) {
                Ok(cmds) => cmds,
                Err(err) => {
                    error!("Parse error: {:?}", checkr::miette::Error::new(err));
                    return Json(AnalysisResponse {
                        stdout: "".to_string(),
                        stderr: "".to_string(),
                        parsed_markdown: None,
                        took: Duration::ZERO,
                        validation_result: None,
                    });
                }
            };
            let validation_res =
                match body
                    .analysis
                    .validate(&cmds, input.clone(), exec_output.parsed.clone())
                {
                    Ok(res) => res,
                    Err(err) => {
                        let stdout = String::from_utf8(exec_output.output.stdout).unwrap();
                        return Json(AnalysisResponse {
                            stdout: stdout.clone(),
                            stderr: String::from_utf8(exec_output.output.stderr).unwrap(),
                            parsed_markdown: None,
                            took: exec_output.took,
                            validation_result: Some(match &err {
                                EnvError::ParseInput { .. } => ValidationResult::InvalidInput {
                                    input: input.to_string(),
                                    error: err.to_string(),
                                },
                                EnvError::ParseOutput { .. } => ValidationResult::InvalidOutput {
                                    output: stdout,
                                    expected_output_format: body
                                        .analysis
                                        .run(&cmds, input)
                                        .ok()
                                        .map(|v| v.to_string()),
                                    error: err.to_string(),
                                },
                                EnvError::InvalidInputForProgram { input, .. } => {
                                    ValidationResult::InvalidInput {
                                        input: input.to_string(),
                                        error: err.to_string(),
                                    }
                                }
                            }),
                        });
                    }
                };
            AnalysisResponse {
                stdout: String::from_utf8(exec_output.output.stdout).unwrap(),
                stderr: String::from_utf8(exec_output.output.stderr).unwrap(),
                parsed_markdown: Some(
                    exec_output
                        .parsed
                        .to_markdown()
                        .expect("serialization error during markdown generation"),
                ),
                took: exec_output.took,
                validation_result: Some(validation_res.into()),
            }
        }
        Err(e) => match &e {
            checkr::driver::ExecError::Serialize(_) => todo!(),
            checkr::driver::ExecError::RunExec { .. } => AnalysisResponse {
                stdout: String::new(),
                stderr: format!("Failed to run executable: {:?}", color_eyre::Report::new(e)),
                parsed_markdown: None,
                took: Duration::ZERO,
                validation_result: None,
            },
            checkr::driver::ExecError::CommandFailed(output, took) => AnalysisResponse {
                stdout: String::from_utf8(output.stdout.clone()).unwrap(),
                stderr: String::from_utf8(output.stderr.clone()).unwrap(),
                parsed_markdown: None,
                took: *took,
                validation_result: None,
            },
            checkr::driver::ExecError::Parse {
                inner,
                run_output,
                time,
            } => {
                let stdout = String::from_utf8(run_output.stdout.clone()).unwrap();
                let stderr = String::from_utf8(run_output.stderr.clone()).unwrap();
                AnalysisResponse {
                    stdout: stdout.clone(),
                    stderr,
                    parsed_markdown: None,
                    took: *time,
                    validation_result: Some(ValidationResult::InvalidOutput {
                        output: stdout,
                        expected_output_format: None,
                        error: inner.to_string(),
                    }),
                }
            }
        },
    };

    Json(output)
}
