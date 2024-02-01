use std::time::Duration;

use axum::{extract::State, Json};
use ce_shell::{Analysis, EnvExt};
use driver::{JobId, JobState};
use rand::SeedableRng;

#[derive(Clone)]
pub struct AppState {
    pub hub: driver::Hub<()>,
    pub driver: driver::Driver,
}

pub fn endpoints() -> tapi::Endpoints<'static, AppState> {
    type E = &'static dyn tapi::Endpoint<AppState>;
    tapi::Endpoints::new([
        &generate::endpoint as E,
        &jobs::endpoint as E,
        &exec_analysis::endpoint as E,
        &cancel_job::endpoint as E,
        &wait_for_job::endpoint as E,
        &gcl_dot::endpoint as E,
        &compilation_status::endpoint as E,
    ])
}

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

fn periodic_stream<T: Clone + Send + PartialEq + 'static, S: Send + 'static>(
    interval: Duration,
    mut f: impl FnMut() -> T + Send + 'static,
    mut g: impl FnMut(&T) -> S + Send + 'static,
) -> tapi::Sse<S> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<S, axum::BoxError>>(1);

    tokio::spawn(async move {
        let mut last = None;
        loop {
            let new = f();
            if Some(new.clone()) != last {
                tx.send(Ok(g(&new))).await.unwrap();
                last = Some(new);
            }
            tokio::time::sleep(interval).await;
        }
    });

    tapi::Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
}

#[tapi::tapi(path = "/jobs", method = Get)]
async fn jobs(State(state): State<AppState>) -> tapi::Sse<Vec<Job>> {
    periodic_stream(
        Duration::from_millis(100),
        move || state.jobs(),
        |jobs| jobs.clone(),
    )
}

#[tapi::tapi(path = "/analysis", method = Post)]
async fn exec_analysis(
    State(state): State<AppState>,
    Json(input): Json<ce_shell::Input>,
) -> Json<JobId> {
    let output = state.driver.exec_job(&input).unwrap();
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
    let job = state
        .driver
        .exec_job(&ce_graph::GraphEnv::generalize_input(
            &ce_graph::GraphInput {
                commands: commands.clone(),
                determinism,
            },
        ))
        .unwrap();

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

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct CompilationStatus {
    id: JobId,
    state: JobState,
    error_output: Option<Vec<Span>>,
}

#[tapi::tapi(path = "/compilation-status", method = Get)]
async fn compilation_status(State(state): State<AppState>) -> tapi::Sse<Option<CompilationStatus>> {
    periodic_stream(
        Duration::from_millis(100),
        move || {
            state
                .driver
                .current_compilation()
                .map(|job| (job.clone(), job.state()))
        },
        |cached| {
            cached.clone().map(|(job, state)| CompilationStatus {
                id: job.id(),
                state,
                error_output: if job.state() == JobState::Failed {
                    let combined = job.stdout_and_stderr();
                    let spans = driver::ansi::parse_ansi(&combined)
                        .into_iter()
                        .map(|s| Span {
                            text: s.text,
                            fg: s.fg,
                            bg: s.bg,
                        })
                        .collect();
                    Some(spans)
                } else {
                    None
                },
            })
        },
    )
}
