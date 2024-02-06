use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
    time::Instant,
};

use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinSet};
use tracing::Instrument;

#[derive(tapi::Tapi, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId {
    pub(crate) value: usize,
}
#[derive(Debug)]
pub struct Job<T> {
    id: JobId,
    started: Instant,
    inner: Arc<JobInner<T>>,
}
#[derive(Debug)]
pub(crate) struct JobInner<M> {
    pub(crate) id: JobId,
    pub(crate) child: tokio::sync::RwLock<Option<tokio::process::Child>>,
    pub(crate) stdin: Option<tokio::process::ChildStdin>,
    pub(crate) events_tx: Arc<tokio::sync::broadcast::Sender<JobEvent>>,
    pub(crate) events_rx: Arc<tokio::sync::broadcast::Receiver<JobEvent>>,
    pub(crate) join_set: Mutex<JoinSet<()>>,
    pub(crate) data: Arc<RwLock<JobData<M>>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JobData<M> {
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
    pub combined: Vec<u8>,
    pub kind: JobKind,
    pub state: JobState,
    pub meta: M,
}

impl<M> JobData<M> {
    pub fn new(kind: JobKind, meta: M) -> Self {
        Self {
            kind,
            meta,
            stderr: Default::default(),
            stdout: Default::default(),
            combined: Default::default(),
            state: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobEventSource {
    Stdout,
    Stderr,
}

impl std::fmt::Debug for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JobId").field(&self.value).finish()
    }
}

impl std::fmt::Display for JobEventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobEventSource::Stdout => write!(f, "stdout"),
            JobEventSource::Stderr => write!(f, "stderr"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobEvent {
    Wrote {
        src: JobEventSource,
        from: usize,
        to: usize,
    },
    Closed {
        src: JobEventSource,
    },
    Finished,
}

impl<T> PartialEq for Job<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.started == other.started
            && Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> Clone for Job<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            started: self.started,
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Send + Sync + 'static> Job<T> {
    pub(crate) fn new(id: JobId, inner: JobInner<T>) -> Job<T> {
        let started = Instant::now();

        let job = Job {
            id,
            started,
            inner: Arc::new(inner),
        };

        tokio::spawn({
            let job = job.clone();
            async move {
                job.wait().await;
                job.inner.events_tx.send(JobEvent::Finished).unwrap();
                tracing::debug!("all streams closed");
            }
        });

        tokio::spawn({
            let job = job.clone();
            let mut events_rx = job.inner.events_rx.resubscribe();
            async move {
                while let Ok(ev) = events_rx.recv().await {
                    match ev {
                        JobEvent::Wrote { src, from, to } => {
                            let data = &mut *job.inner.data.write().unwrap();
                            match src {
                                JobEventSource::Stdout => {
                                    data.combined.extend_from_slice(&data.stdout[from..to])
                                }
                                JobEventSource::Stderr => {
                                    data.combined.extend_from_slice(&data.stderr[from..to])
                                }
                            }
                        }
                        JobEvent::Closed { src } => {
                            tracing::debug!(?src, "closed");
                        }
                        JobEvent::Finished => {}
                    }
                }
            }
            .instrument(tracing::info_span!("job", id=?job.id))
        });

        job
    }
    pub fn id(&self) -> JobId {
        self.id
    }
    pub fn started(&self) -> Instant {
        self.started
    }
    pub fn events(&self) -> tokio::sync::broadcast::Receiver<JobEvent> {
        self.inner.events_rx.resubscribe()
    }
    pub fn data(&self) -> impl Deref<Target = JobData<T>> + '_ {
        self.inner.data.read().unwrap()
    }
    fn data_mut(&self) -> impl DerefMut<Target = JobData<T>> + '_ {
        self.inner.data.write().unwrap()
    }
    pub fn raw_stdout_and_stderr(&self) -> Vec<u8> {
        self.data().combined.to_vec()
    }
    pub fn stdout_and_stderr(&self) -> String {
        String::from_utf8(self.data().combined.to_vec()).unwrap_or_default()
    }
    pub fn raw_stdout(&self) -> Vec<u8> {
        self.data().stdout.to_vec()
    }
    pub fn stdout(&self) -> String {
        String::from_utf8(self.data().stdout.to_vec()).unwrap_or_default()
    }
    pub fn raw_stderr(&self) -> Vec<u8> {
        self.data().stderr.to_vec()
    }
    pub fn stderr(&self) -> String {
        String::from_utf8(self.data().stderr.to_vec()).unwrap_or_default()
    }
    pub fn kind(&self) -> JobKind {
        self.data().kind.clone()
    }
    pub fn state(&self) -> JobState {
        self.data().state
    }
    pub async fn wait(&self) -> JobState {
        while self.inner.join_set.lock().await.join_next().await.is_some() {}
        let mut guard = self.inner.child.write().await;
        let child = guard.take();
        if let Some(mut child) = child {
            match child.wait().await {
                Ok(es) => {
                    tracing::debug!(?es, "set state");
                    let mut data = self.data_mut();
                    if data.state != JobState::Canceled {
                        data.state = if es.success() {
                            JobState::Succeeded
                        } else {
                            JobState::Failed
                        }
                    }
                }
                Err(_) => todo!(),
            }
        }
        self.state()
    }
    #[tracing::instrument(skip_all)]
    pub fn kill(&self) {
        let job = self.clone();
        tokio::spawn(async move {
            if let Some(child) = &mut *job.inner.child.write().await {
                let mut data = job.data_mut();
                if let JobState::Queued | JobState::Running = data.state {
                    data.state = JobState::Canceled
                }
                child.start_kill().unwrap();
            }
        });
    }

    pub fn meta(&self) -> T
    where
        T: Clone,
    {
        self.data().meta.clone()
    }
}

#[derive(tapi::Tapi, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum JobKind {
    Compilation,
    Analysis(ce_shell::Input),
}

impl std::fmt::Debug for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compilation => write!(f, "Compilation"),
            Self::Analysis(input) => f
                .debug_tuple("Analysis")
                .field(&input.analysis())
                .field(&"...")
                .finish(),
        }
    }
}

#[derive(tapi::Tapi, Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum JobState {
    Queued,
    #[default]
    Running,
    Succeeded,
    Canceled,
    Failed,
    Warning,
}

impl Display for JobKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobKind::Compilation => write!(f, "Compilation"),
            JobKind::Analysis(input) => write!(f, "{}", input.analysis()),
        }
    }
}

impl Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobState::Queued => write!(f, "Queued"),
            JobState::Running => write!(f, "Running"),
            JobState::Succeeded => write!(f, "Succeeded"),
            JobState::Canceled => write!(f, "Canceled"),
            JobState::Failed => write!(f, "Failed"),
            JobState::Warning => write!(f, "Warning"),
        }
    }
}
