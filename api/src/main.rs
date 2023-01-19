use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use verification_lawyer::{
    driver::{Driver, ExecOutput},
    env::{
        graph::GraphEnv, pv::ProgramVerificationEnv, Environment, SecurityEnv, SignEnv,
        StepWiseEnv, ToMarkdown,
    },
    generation::Generate,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub analysis: String,
    pub src: String,
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecOutputAny {
    pub stdout: String,
    pub stderr: String,
    pub parsed_markdown: String,
    pub took: Duration,
}

impl<E: Environment> From<ExecOutput<E>> for ExecOutputAny
where
    E::Output: ToMarkdown,
{
    fn from(value: ExecOutput<E>) -> Self {
        Self {
            stdout: String::from_utf8(value.output.stdout).unwrap(),
            stderr: String::from_utf8(value.output.stderr).unwrap(),
            parsed_markdown: value.parsed.to_markdown(),
            took: value.took,
        }
    }
}

#[axum::debug_handler]
async fn analyze(
    shared_driver: State<Arc<Driver>>,
    Json(body): Json<AnalysisRequest>,
) -> Json<ExecOutputAny> {
    use verification_lawyer::env::Environment;

    let cmds = verification_lawyer::parse::parse_commands(&body.src).unwrap();
    info!(input = body.input);
    let output = match body.analysis {
        name if name == SignEnv::command() => {
            type E = SignEnv;
            let input = serde_json::from_str(&body.input).unwrap();
            ExecOutputAny::from(shared_driver.exec::<E>(&cmds, &input).unwrap())
        }
        name if name == StepWiseEnv::command() => {
            type E = StepWiseEnv;
            let input = serde_json::from_str(&body.input).unwrap();
            ExecOutputAny::from(shared_driver.exec::<E>(&cmds, &input).unwrap())
        }
        name if name == SecurityEnv::command() => {
            type E = SecurityEnv;
            let input = serde_json::from_str(&body.input).unwrap();
            ExecOutputAny::from(shared_driver.exec::<E>(&cmds, &input).unwrap())
        }
        name if name == ProgramVerificationEnv::command() => {
            type E = ProgramVerificationEnv;
            let input = serde_json::from_str(&body.input).unwrap();
            ExecOutputAny::from(shared_driver.exec::<E>(&cmds, &input).unwrap())
        }
        name if name == GraphEnv::command() => {
            let input = serde_json::from_str(&body.input).unwrap();
            ExecOutputAny::from(shared_driver.exec::<GraphEnv>(&cmds, &input).unwrap())
        }
        _ => todo!(),
    };

    Json(output)
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

async fn run() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::WupWup { open, dir } => {
            let run = infra::RunOption::from_file(dir.join("run.toml"))?;

            info!(run = format!("{run:?}"));

            let driver = if let Some(compile) = run.compile {
                Driver::compile(dir, compile, run.run)?
            } else {
                Driver::new(dir, run.run)
            };

            // info!(driver = format!("{driver:?}"));

            use verification_lawyer::env::Environment;

            let mut cmds = verification_lawyer::parse::parse_commands("if true -> skip fi")?;
            let mut rng = rand::rngs::SmallRng::from_entropy();
            let input = verification_lawyer::env::sign::SignAnalysisInput::gen(&mut cmds, &mut rng);

            let output = driver.exec::<SignEnv>(&cmds, &input)?;

            info!(output = format!("{output:?}"));

            let shared_driver = Arc::new(driver);

            let app = Router::new()
                .route("/analyze", post(analyze))
                .with_state(shared_driver);
            let app = app.fallback(static_dir);
            let app = app
                .layer(
                    CorsLayer::new()
                        .allow_origin("*".parse::<HeaderValue>().unwrap())
                        .allow_headers([CONTENT_TYPE])
                        .allow_methods([Method::GET, Method::POST]),
                )
                .layer(TraceLayer::new_for_http());

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
