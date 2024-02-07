use std::time::Duration;

use axum::{extract::State, Json};
use ce_core::ValidationResult;
use ce_shell::{Analysis, EnvExt};
use driver::{HubEvent, JobId, JobState};
use gcl::ast::TargetKind;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InspectifyJobMeta {
    pub group_name: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub hub: driver::Hub<InspectifyJobMeta>,
    pub driver: driver::Driver<InspectifyJobMeta>,
}

pub fn endpoints() -> tapi::Endpoints<'static, AppState> {
    type E = &'static dyn tapi::Endpoint<AppState>;
    tapi::Endpoints::new([
        &generate::endpoint as E,
        &events::endpoint as E,
        &jobs_cancel::endpoint as E,
        &jobs_wait::endpoint as E,
        &exec_analysis::endpoint as E,
        &gcl_dot::endpoint as E,
        &gcl_free_vars::endpoint as E,
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
    group_name: Option<String>,
    stdout: String,
    spans: Vec<Span>,
    analysis_data: Option<AnalysisData>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct AnalysisData {
    reference_output: ce_shell::Output,
    validation: ce_core::ValidationResult,
}

impl AppState {
    fn job(&self, id: JobId) -> Job {
        let job = self.hub.get_job(id).unwrap();
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

        let analysis_data = match &kind {
            driver::JobKind::Analysis(input) => {
                let reference_output = input.reference_output().unwrap();
                let validation = match input.analysis().output_from_str(&stdout) {
                    Ok(output) => input.validate_output(&output).unwrap(),
                    Err(e) => ValidationResult::Mismatch {
                        reason: format!("failed to parse output: {e:?}"),
                    },
                };
                Some(AnalysisData {
                    reference_output,
                    validation,
                })
            }
            _ => None,
        };

        Job {
            id,
            state,
            kind,
            group_name: job.meta().group_name.clone(),
            stdout,
            spans,
            analysis_data,
        }
    }
    fn jobs(&self) -> Vec<JobId> {
        self.hub
            .jobs(Some(25))
            .into_iter()
            .map(|job| job.id())
            .collect()
    }
}

fn periodic_stream<T: Clone + Send + PartialEq + 'static, S: Send + 'static>(
    interval: Duration,
    mut f: impl FnMut() -> T + Send + 'static,
    mut g: impl FnMut(&T) -> S + Send + 'static,
    tx: tokio::sync::mpsc::Sender<Result<S, axum::BoxError>>,
) {
    // let (tx, rx) = tokio::sync::mpsc::channel::<Result<S, axum::BoxError>>(1);

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

    // tapi::Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
}

// #[tapi::tapi(path = "/jobs/list", method = Get)]
// async fn jobs_list(State(state): State<AppState>) -> tapi::Sse<Vec<JobId>> {
//     periodic_stream(
//         Duration::from_millis(100),
//         move || state.jobs(),
//         |jobs| jobs.clone(),
//     )
// }

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "value")]
enum Event {
    CompilationStatus { status: Option<CompilationStatus> },
    JobChanged { id: JobId, job: Job },
    JobsChanged { jobs: Vec<JobId> },
}

#[tapi::tapi(path = "/events", method = Get)]
async fn events(State(state): State<AppState>) -> tapi::Sse<Event> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, axum::BoxError>>(1);

    tokio::spawn({
        let state = state.clone();
        let tx = tx.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(100)).await;

            tx.send(Ok(Event::JobsChanged { jobs: state.jobs() }))
                .await
                .unwrap();

            for id in state.jobs() {
                let job = state.job(id);
                tx.send(Ok(Event::JobChanged { id, job })).await.unwrap();
            }
        }
    });

    tokio::spawn({
        let state = state.clone();
        let tx = tx.clone();
        async move {
            let mut events = state.hub.events();
            while let Ok(event) = events.recv().await {
                match event {
                    HubEvent::JobAdded(id) => {
                        tx.send(Ok(Event::JobsChanged { jobs: state.jobs() }))
                            .await
                            .unwrap();

                        tokio::spawn({
                            let state = state.clone();
                            let tx = tx.clone();
                            async move {
                                let mut events = state.hub.get_job(id).unwrap().events();
                                while let Ok(_event) = events.recv().await {
                                    let job = state.job(id);
                                    tx.send(Ok(Event::JobChanged { id, job })).await.unwrap();
                                }
                            }
                        });
                    }
                }
            }
        }
    });

    periodic_stream(
        Duration::from_millis(100),
        {
            let state = state.clone();
            move || {
                state
                    .driver
                    .current_compilation()
                    .map(|job| (job.clone(), job.state()))
            }
        },
        |cached| Event::CompilationStatus {
            status: cached.clone().map(|(job, state)| CompilationStatus {
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
            }),
        },
        tx.clone(),
    );

    // periodic_stream(
    //     Duration::from_millis(100),
    //     move || state.job(id),
    //     |job| JobEvent::JobChanged {
    //         id: job.id,
    //         job: job.clone(),
    //     },
    //     tx.clone(),
    // );

    // tokio::spawn(async move {
    //     let mut last = None;
    //     loop {
    //         let new = f();
    //         if Some(new.clone()) != last {
    //             tx.send(Ok(g(&new))).await.unwrap();
    //             last = Some(new);
    //         }
    //         tokio::time::sleep(interval).await;
    //     }
    // });

    tapi::Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
}

#[tapi::tapi(path = "/analysis", method = Post)]
async fn exec_analysis(
    State(state): State<AppState>,
    Json(input): Json<ce_shell::Input>,
) -> Json<JobId> {
    let output = state
        .driver
        .exec_job(&input, InspectifyJobMeta::default())
        .unwrap();
    Json(output.id())
}

#[tapi::tapi(path = "/jobs/cancel", method = Post)]
async fn jobs_cancel(State(state): State<AppState>, Json(id): Json<JobId>) {
    if let Some(job) = state.hub.get_job(id) {
        job.kill();
    }
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JobOutput {
    output: ce_shell::Output,
    validation: ce_core::ValidationResult,
}

#[tapi::tapi(path = "/jobs/wait", method = Post)]
async fn jobs_wait(
    State(state): State<AppState>,
    Json(id): Json<JobId>,
) -> Json<Option<JobOutput>> {
    if let Some(job) = state.hub.get_job(id) {
        match job.wait().await {
            driver::JobState::Succeeded => match job.kind() {
                driver::JobKind::Analysis(input) => {
                    let output = input.analysis().output_from_str(&job.stdout()).unwrap();
                    let validation = input.validate_output(&output).unwrap();
                    Json(Some(JobOutput { output, validation }))
                }
                driver::JobKind::Compilation => todo!(),
            },
            state => {
                tracing::error!(stdout=?job.stdout(), stderr=?job.stderr(), ?state, "job failed");
                Json(None)
            }
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

#[tapi::tapi(path = "/gcl/dot", method = Post)]
async fn gcl_dot(
    State(state): State<AppState>,
    Json(GclDotInput {
        determinism,
        commands,
    }): Json<GclDotInput>,
) -> Json<ce_graph::GraphOutput> {
    let job = state
        .driver
        .exec_job(
            &ce_graph::GraphEnv::generalize_input(&ce_graph::GraphInput {
                commands: commands.clone(),
                determinism,
            }),
            InspectifyJobMeta::default(),
        )
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
struct Target {
    name: gcl::ast::Target,
    kind: TargetKind,
}

#[tapi::tapi(path = "/gcl/free-vars", method = Post)]
async fn gcl_free_vars(Json(commands): Json<gcl::ast::Commands>) -> Json<Vec<Target>> {
    Json(
        commands
            .fv()
            .into_iter()
            .map(|target| Target {
                name: target.clone(),
                kind: match target {
                    gcl::ast::Target::Variable(_) => TargetKind::Variable,
                    gcl::ast::Target::Array(_, _) => TargetKind::Array,
                },
            })
            .collect(),
    )
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct CompilationStatus {
    id: JobId,
    state: JobState,
    error_output: Option<Vec<Span>>,
}
