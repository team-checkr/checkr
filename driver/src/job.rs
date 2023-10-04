use std::{
    fmt::Display,
    ops::Deref,
    sync::{Arc, RwLock},
    time::Instant,
};

use ce_shell::Analysis;
use tokio::{sync::Mutex, task::JoinSet};
use tracing::Instrument;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
pub(crate) struct JobInner<T> {
    pub(crate) id: JobId,
    pub(crate) child: tokio::sync::RwLock<Option<tokio::process::Child>>,
    pub(crate) stdin: Option<tokio::process::ChildStdin>,
    pub(crate) events_rx: tokio::sync::broadcast::Receiver<JobEvent>,
    pub(crate) join_set: Mutex<JoinSet<()>>,
    pub(crate) stderr: Arc<RwLock<Vec<u8>>>,
    pub(crate) stdout: Arc<RwLock<Vec<u8>>>,
    pub(crate) combined: Arc<RwLock<Vec<u8>>>,
    pub(crate) kind: JobKind,
    pub(crate) state: RwLock<JobState>,
    pub(crate) data: T,
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
                            let src = match src {
                                JobEventSource::Stdout => job.inner.stdout.read().unwrap(),
                                JobEventSource::Stderr => job.inner.stderr.read().unwrap(),
                            };
                            job.inner
                                .combined
                                .write()
                                .unwrap()
                                .extend_from_slice(&src[from..to]);
                        }
                        JobEvent::Closed { src } => {
                            tracing::debug!(?src, "closed");
                        }
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
    pub fn raw_stdout_and_stderr(&self) -> impl Deref<Target = Vec<u8>> + '_ {
        self.inner.combined.read().unwrap()
    }
    pub fn stdout_and_stderr(&self) -> String {
        String::from_utf8(self.raw_stdout_and_stderr().to_vec()).unwrap_or_default()
    }
    pub fn stdout(&self) -> String {
        String::from_utf8(self.inner.stdout.read().unwrap().to_vec()).unwrap_or_default()
    }
    pub fn stderr(&self) -> String {
        String::from_utf8(self.inner.stderr.read().unwrap().to_vec()).unwrap_or_default()
    }
    pub fn kind(&self) -> JobKind {
        self.inner.kind.clone()
    }
    pub fn state(&self) -> JobState {
        *self.inner.state.read().unwrap()
    }
    pub async fn wait(&self) -> JobState {
        while self.inner.join_set.lock().await.join_next().await.is_some() {}
        let mut guard = self.inner.child.write().await;
        let child = guard.take();
        if let Some(mut child) = child {
            match child.wait().await {
                Ok(es) => {
                    tracing::debug!(?es, "set state");
                    if *self.inner.state.read().unwrap() != JobState::Canceled {
                        *self.inner.state.write().unwrap() = if es.success() {
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
                let state = *job.inner.state.read().unwrap();
                if let JobState::Queued | JobState::Running = state {
                    *job.inner.state.write().unwrap() = JobState::Canceled
                }
                child.start_kill().unwrap();
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JobKind {
    Compilation,
    Analysis(Analysis),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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
            JobKind::Analysis(analysis) => write!(f, "{analysis}"),
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
