use std::{sync::Arc, time::Duration};

use axum::{extract::State, Json};
use ce_core::ValidationResult;
use ce_shell::{Analysis, Hash, Input};
use driver::{HubEvent, JobId, JobState};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

use crate::checko::{self, config::GroupName, scoreboard::PublicState};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectifyJobMeta {
    pub group_name: Option<GroupName>,
}

#[derive(Clone)]
pub struct AppState {
    pub hub: driver::Hub<InspectifyJobMeta>,
    pub driver: Option<driver::Driver<InspectifyJobMeta>>,
    pub checko: Option<Arc<checko::Checko>>,
    pub public_state: Arc<std::sync::RwLock<Option<PublicState>>>,
}

pub fn endpoints() -> tapi::endpoints::Endpoints<'static, AppState> {
    type E = &'static dyn tapi::endpoints::Endpoint<AppState>;
    tapi::endpoints::Endpoints::new([
        &generate::endpoint as E,
        &events::endpoint as E,
        &checko_csv::endpoint as E,
        &checko_public::endpoint as E,
        &jobs_cancel::endpoint as E,
        &exec_analysis::endpoint as E,
        &exec_reference::endpoint as E,
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
    group_name: Option<GroupName>,
    stdout: String,
    spans: Vec<Span>,
    analysis_data: Option<AnalysisData>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct AnalysisData {
    meta: ce_shell::Meta,
    output: Option<ce_shell::Output>,
    reference_output: Option<ce_shell::Output>,
    validation: Option<ce_core::ValidationResult>,
}

impl AppState {
    #[tracing::instrument(skip(self))]
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
                let meta = input.meta();
                let reference_output = match input.reference_output() {
                    Ok(reference_output) => Some(reference_output),
                    Err(err) => {
                        tracing::warn!(?err, "failed to get reference output");
                        None
                    }
                };
                let output = input.analysis().output_from_str(&stdout);
                let validation = match (state, &output) {
                    (JobState::Succeeded, Ok(output)) => {
                        Some(match input.validate_output(output) {
                            Ok(output) => output,
                            Err(e) => ValidationResult::Mismatch {
                                reason: format!("failed to validate output: {e:?}"),
                            },
                        })
                    }
                    (JobState::Succeeded, Err(e)) => Some(ValidationResult::Mismatch {
                        reason: format!("failed to parse output: {e:?}"),
                    }),
                    _ => None,
                };
                Some(AnalysisData {
                    meta,
                    output: output.ok(),
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
    fn jobs(&self) -> Vec<driver::Job<InspectifyJobMeta>> {
        self.hub
            // .jobs(Some(25))
            .jobs(None)
    }
}

fn periodic_stream<T: Clone + Send + PartialEq + 'static, S: Send + 'static>(
    interval: Duration,
    mut f: impl FnMut() -> T + Send + 'static,
    mut g: impl FnMut(&T) -> S + Send + 'static,
    tx: tokio::sync::mpsc::Sender<Result<S, axum::BoxError>>,
) {
    tokio::spawn(async move {
        let mut last = None;
        loop {
            let new = f();
            if Some(new.clone()) != last {
                tracing::debug!("sending");
                if tx.send(Ok(g(&new))).await.is_err() {
                    break;
                }
                last = Some(new);
            }
            tokio::time::sleep(interval).await;
        }
    });
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "value")]
enum Event {
    Reset,
    CompilationStatus {
        status: Option<CompilationStatus>,
    },
    JobChanged {
        job: Job,
    },
    JobsChanged {
        jobs: Vec<JobId>,
    },
    GroupsConfig {
        config: checko::config::GroupsConfig,
    },
    ProgramsConfig {
        programs: Vec<Program>,
    },
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
pub struct Program {
    pub hash: Hash,
    pub hash_str: String,
    pub input: Input,
}

async fn start_listening_on_job(
    state: AppState,
    tx: tokio::sync::mpsc::Sender<Result<Event, axum::BoxError>>,
    job: driver::Job<InspectifyJobMeta>,
) -> bool {
    let event = Event::JobsChanged {
        jobs: state.jobs().into_iter().map(|j| j.id()).collect(),
    };
    if tx.send(Ok(event)).await.is_err() {
        return false;
    }
    let event = Event::JobChanged {
        job: state.job(job.id()),
    };
    if tx.send(Ok(event)).await.is_err() {
        return false;
    }

    tokio::spawn({
        let state = state.clone();
        let tx = tx.clone();
        async move {
            let mut events = job.events();
            while let Ok(_event) = events.recv().await {
                let job = state.job(job.id());
                let event = Event::JobChanged { job };
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        }
    });

    true
}

#[tapi::tapi(path = "/events", method = Get)]
async fn events(State(state): State<AppState>) -> tapi::endpoints::Sse<Event> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, axum::BoxError>>(1);

    let _ = tx.send(Ok(Event::Reset)).await;

    tokio::spawn({
        let state = state.clone();
        let tx = tx.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let event = Event::JobsChanged {
                jobs: state.jobs().into_iter().map(|j| j.id()).collect(),
            };
            if tx.send(Ok(event)).await.is_err() {
                return;
            }

            for job in state.jobs() {
                if !start_listening_on_job(state.clone(), tx.clone(), job).await {
                    break;
                }
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
                        let job = state.hub.get_job(id).unwrap();
                        if !start_listening_on_job(state.clone(), tx.clone(), job).await {
                            break;
                        }
                    }
                }
            }
        }
    });

    if let Some(driver) = state.driver.clone() {
        periodic_stream(
            Duration::from_millis(100),
            move || {
                driver
                    .current_compilation()
                    .map(|job| (job.clone(), job.state()))
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
    }

    if let Some(checko) = state.checko.clone() {
        tokio::spawn({
            let tx = tx.clone();
            async move {
                tx.send(Ok(Event::GroupsConfig {
                    config: checko.groups_config().clone(),
                }))
                .await
                .unwrap();
                let programs = checko
                    .programs_config()
                    .envs
                    .iter()
                    .flat_map(|(analysis, ps)| {
                        ps.programs.iter().map(|p| {
                            let input = analysis.input_from_str(&p.input).unwrap();
                            let hash = input.hash();
                            let hash_str = hash.hex();
                            Program {
                                hash,
                                hash_str,
                                input,
                            }
                        })
                    })
                    .collect();
                let event = Event::ProgramsConfig { programs };
                let _ = tx.send(Ok(event)).await;
            }
        });
    }

    tapi::endpoints::Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AnalysisExecution {
    id: JobId,
}

#[tapi::tapi(path = "/analysis", method = Post)]
async fn exec_analysis(
    State(state): State<AppState>,
    Json(input): Json<ce_shell::Input>,
) -> Json<Option<AnalysisExecution>> {
    let Some(driver) = state.driver.as_ref() else {
        return Json(None);
    };
    let output = driver.exec_job(&input, InspectifyJobMeta::default());
    let id = output.id();
    Json(Some(AnalysisExecution { id }))
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReferenceExecution {
    meta: ce_shell::Meta,
    output: Option<ce_shell::Output>,
    error: Option<String>,
}

#[tapi::tapi(path = "/reference", method = Post)]
async fn exec_reference(Json(input): Json<ce_shell::Input>) -> Json<ReferenceExecution> {
    let output = input.reference_output();
    let error = output.as_ref().err().map(|e| e.to_string());
    Json(ReferenceExecution {
        meta: input.meta(),
        output: output.ok(),
        error,
    })
}

#[tapi::tapi(path = "/jobs/cancel", method = Post)]
async fn jobs_cancel(State(state): State<AppState>, Json(id): Json<JobId>) {
    if let Some(job) = state.hub.get_job(id) {
        job.kill();
    }
}

#[derive(tapi::Tapi, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data")]
enum JobOutput {
    AnalysisSuccess {
        output: ce_shell::Output,
        reference_output: ce_shell::Output,
        validation: ce_core::ValidationResult,
    },
    CompilationSuccess,
    Failure {
        error: String,
    },
    JobMissing,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
struct CompilationStatus {
    id: JobId,
    state: JobState,
    error_output: Option<Vec<Span>>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", content = "value")]
pub enum PublicEvent {
    Reset,
    StateChanged(PublicState),
}

#[tapi::tapi(path = "/checko-public", method = Get)]
async fn checko_public(State(state): State<AppState>) -> tapi::endpoints::Sse<PublicEvent> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<PublicEvent, axum::BoxError>>(16);

    if state.checko.is_some() {
        let _ = tx.send(Ok(PublicEvent::Reset)).await;

        periodic_stream(
            std::time::Duration::from_millis(500),
            {
                let state = state.clone();
                move || {
                    if let Some(public_state) = &*state.public_state.read().unwrap() {
                        PublicEvent::StateChanged(public_state.clone())
                    } else {
                        PublicEvent::Reset
                    }
                }
            },
            |x| x.clone(),
            tx.clone(),
        );
    }

    tapi::endpoints::Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
}

#[tapi::tapi(path = "/checko-csv", method = Get)]
async fn checko_csv(State(state): State<AppState>) -> String {
    let public_state = state.public_state.read().unwrap();
    if let Some(public_state) = &*public_state {
        public_state.to_csv()
    } else {
        String::new()
    }
}
