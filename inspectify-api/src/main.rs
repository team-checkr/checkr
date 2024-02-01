use axum::{extract::State, Json, Router};
use ce_shell::{Analysis, EnvExt};
use driver::JobId;
use rand::SeedableRng;
use tapi::RouterExt;

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GenerateParams {
    analysis: Analysis,
}

#[tapi::tapi(path = "/generate", method = Post)]
async fn generate(Json(params): Json<GenerateParams>) -> Json<ce_shell::Input> {
    let input = params
        .analysis
        .gen_input(&mut rand::rngs::SmallRng::from_entropy());
    Json(input)
}

#[derive(Clone)]
struct AppState {
    pub hub: driver::Hub<()>,
    pub driver: driver::Driver,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct Span {
    text: String,
    fg: Option<driver::ansi::Color>,
    bg: Option<driver::ansi::Color>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct Job {
    id: JobId,
    state: driver::JobState,
    kind: driver::JobKind,
    stdout: String,
    spans: Vec<Span>,
}

impl AppState {
    fn jobs(&self) -> Vec<Job> {
        self.hub
            .jobs(Some(25))
            .into_iter()
            .map(|job| {
                let id = job.id();
                let state = job.state();
                let kind = job.kind();
                let stdout = job.stdout();
                let combined = job.stdout_and_stderr();
                let spans = driver::ansi::parse_ansi(&combined)
                    .into_iter()
                    .map(|s| Span {
                        text: s.text,
                        fg: s.fg,
                        bg: s.bg,
                    })
                    .collect();

                Job {
                    id,
                    state,
                    kind,
                    stdout,
                    spans,
                }
            })
            .collect()
    }
}

#[tapi::tapi(path = "/jobs", method = Get)]
async fn jobs(State(state): State<AppState>) -> tapi::Sse<Vec<Job>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<Job>, axum::BoxError>>(1);

    tokio::spawn(async move {
        let mut last: Vec<Job> = Vec::new();
        loop {
            let jobs = state.jobs();
            if jobs != last {
                tx.send(Ok(jobs.clone())).await.unwrap();
                last = jobs;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    });

    tapi::Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
}

#[tapi::tapi(path = "/analysis", method = Post)]
async fn exec_analysis(
    State(state): State<AppState>,
    Json(input): Json<ce_shell::Input>,
) -> Json<JobId> {
    let output = state.driver.exec_job(&input);
    Json(output.id())
}

#[tapi::tapi(path = "/cancel-job", method = Post)]
async fn cancel_job(State(state): State<AppState>, Json(id): Json<JobId>) {
    if let Some(job) = state.hub.get_job(id) {
        job.kill();
    }
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JobOutput {
    output: ce_shell::Output,
    validation: ce_core::ValidationResult,
}

#[tapi::tapi(path = "/wait-for-job", method = Post)]
async fn wait_for_job(
    State(state): State<AppState>,
    Json(id): Json<JobId>,
) -> Json<Option<JobOutput>> {
    if let Some(job) = state.hub.get_job(id) {
        job.wait().await;
        match job.kind() {
            driver::JobKind::Analysis(a, input) => {
                let output = a.parse_output(&job.stdout()).unwrap();
                let validation = input.validate_output(&output).unwrap();
                Json(Some(JobOutput { output, validation }))
            }
            driver::JobKind::Compilation => todo!(),
        }
    } else {
        Json(None)
    }
}

#[derive(tapi::Tapi, Debug, Clone, serde::Deserialize)]
struct GclDotInput {
    determinism: gcl::pg::Determinism,
    commands: gcl::ast::Commands,
}

#[tapi::tapi(path = "/gcl-dot", method = Post)]
async fn gcl_dot(
    State(state): State<AppState>,
    Json(GclDotInput {
        determinism,
        commands,
    }): Json<GclDotInput>,
) -> Json<ce_graph::GraphOutput> {
    let job = state.driver.exec_job(&ce_graph::GraphEnv::generalize_input(
        &ce_graph::GraphInput {
            commands: commands.clone(),
            determinism,
        },
    ));

    match job.wait().await {
        driver::JobState::Succeeded => {
            let output: ce_graph::GraphOutput = serde_json::from_str(&job.stdout()).unwrap();
            Json(output)
        }
        driver::JobState::Failed => {
            let pg = gcl::pg::ProgramGraph::new(determinism, &commands);
            Json(ce_graph::GraphOutput { dot: pg.dot() })
        }

        driver::JobState::Queued => todo!(),
        driver::JobState::Running => todo!(),
        driver::JobState::Canceled => todo!(),
        driver::JobState::Warning => todo!(),
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let endpoints = tapi::Endpoints::new([
        &generate::endpoint as &dyn tapi::Endpoint<_>,
        &jobs::endpoint,
        &exec_analysis::endpoint,
        &cancel_job::endpoint,
        &wait_for_job::endpoint,
        &gcl_dot::endpoint,
    ])
    .with_ty::<ce_shell::Envs>();

    let hub = driver::Hub::default();
    let driver = driver::Driver::new_from_path(hub.clone(), "./run.toml")?;
    driver.start_recompile();

    driver.spawn_watcher()?;

    let api: Router = Router::new()
        .tapis(&endpoints)
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(AppState { hub, driver });
    let app: Router = Router::new().nest("/api", api);

    let ts_client_path = std::path::PathBuf::from("./inspectify-app/src/lib/api.ts");
    // write TypeScript client if and only if the path already exists
    if ts_client_path.exists() {
        // only write if the contents are different
        let ts_client = endpoints.ts_client();
        let prev = std::fs::read_to_string(&ts_client_path).unwrap_or_default();
        if prev != ts_client {
            std::fs::write(&ts_client_path, ts_client).unwrap();
        }
    } else {
        println!("{}", endpoints.ts_client());
    }

    println!("{}", std::any::type_name::<ce_shell::Analysis>());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
