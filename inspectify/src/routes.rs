use std::time::Duration;

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post, IntoMakeService},
    Json, Router,
};
use checkr::{
    env::{graph::GraphEnvInput, Analysis, GraphEnv, Markdown},
    pg::Determinism,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tracing::error;

use crate::{core, ApplicationState, CompilationStatus, ValidationResult};

pub fn router(state: ApplicationState) -> IntoMakeService<Router<()>> {
    Router::new()
        .route("/analyze", post(analyze))
        .route("/graph", post(graph))
        .route("/compilation-status", get(get_compilation_status))
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

pub async fn get_compilation_status(
    State(state): State<ApplicationState>,
) -> Json<CompilationStatus> {
    Json(state.compilation_status.lock().await.clone())
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
pub async fn graph(
    State(state): State<ApplicationState>,
    Json(body): Json<GraphRequest>,
) -> Json<GraphResponse> {
    match state
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
        Ok(output) => Json(GraphResponse {
            dot: Some(output.parsed.dot),
        }),
        Err(_) => Json(GraphResponse { dot: None }),
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
    let cmds = body.src;
    let driver = state.driver.lock().await;
    let output = match driver
        .exec_dyn_raw_cmds(&body.analysis, &cmds, &body.input)
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
            let validation_res = body
                .analysis
                .validate(&cmds, &body.input, &exec_output.parsed.to_string())
                .expect("serialization error");
            AnalysisResponse {
                stdout: String::from_utf8(exec_output.output.stdout).unwrap(),
                stderr: String::from_utf8(exec_output.output.stderr).unwrap(),
                parsed_markdown: Some(
                    body.analysis
                        .output_markdown(&exec_output.parsed.to_string())
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
                inner: _,
                run_output,
                time,
            } => AnalysisResponse {
                stdout: String::from_utf8(run_output.stdout.clone()).unwrap(),
                stderr: String::from_utf8(run_output.stderr.clone()).unwrap(),
                parsed_markdown: None,
                took: *time,
                validation_result: None,
            },
        },
    };

    Json(output)
}
