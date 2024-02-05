use std::{
    ffi::OsStr,
    fmt::Debug,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{atomic::AtomicUsize, Arc, RwLock},
};

use color_eyre::eyre::Context;
use notify::event;
use tokio::{io::AsyncReadExt, sync::Mutex, task::JoinSet};
use tracing::Instrument;

use crate::{
    job::{Job, JobEvent, JobEventSource, JobInner, JobKind},
    JobId, JobState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HubEvent {
    JobAdded(JobId),
}

#[derive(Debug, Clone)]
pub struct Hub<T> {
    next_job_id: Arc<AtomicUsize>,
    jobs: Arc<RwLock<Vec<Job<T>>>>,
    events_tx: Arc<tokio::sync::broadcast::Sender<HubEvent>>,
    events_rx: Arc<tokio::sync::broadcast::Receiver<HubEvent>>,
}

impl<T> PartialEq for Hub<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.jobs, &other.jobs)
    }
}

impl<T: Send + Sync + 'static> Hub<T> {
    pub fn new() -> color_eyre::Result<Self> {
        let next_job_id = Arc::new(AtomicUsize::new(0));
        let jobs = Arc::new(RwLock::new(Vec::new()));

        let (events_tx, events_rx) = tokio::sync::broadcast::channel(128);

        Ok(Self {
            next_job_id,
            jobs,
            events_tx: Arc::new(events_tx),
            events_rx: Arc::new(events_rx),
        })
    }

    pub fn events(&self) -> tokio::sync::broadcast::Receiver<HubEvent> {
        self.events_rx.resubscribe()
    }

    fn next_job_id(&self) -> JobId {
        JobId {
            value: self
                .next_job_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }

    #[tracing::instrument(skip_all, fields(?kind))]
    pub fn exec_command(
        &self,
        kind: JobKind,
        cwd: impl AsRef<Path> + Debug,
        data: T,
        program: impl AsRef<OsStr> + Debug,
        args: impl IntoIterator<Item = impl AsRef<OsStr>> + Debug,
    ) -> color_eyre::Result<Job<T>>
    where
        T: Debug,
    {
        let id = self.next_job_id();

        let mut cmd = tokio::process::Command::new(program);

        cmd.current_dir(cwd);

        cmd.args(args)
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        cmd.kill_on_drop(true);

        cmd.env("CARGO_TERM_COLOR", "always");

        tracing::debug!(?cmd, "spawning");

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn {:?}", cmd))?;

        let stdin = child.stdin.take().expect("we piped stdin");
        let stderr = child.stderr.take().expect("we piped stderr");
        let stdout = child.stdout.take().expect("we piped stdout");

        let (events_tx, events_rx) = tokio::sync::broadcast::channel(128);

        let mut join_set = tokio::task::JoinSet::new();
        let stderr = spawn_reader(
            JobEventSource::Stderr,
            &mut join_set,
            stderr,
            events_tx.clone(),
        );
        let stdout = spawn_reader(
            JobEventSource::Stdout,
            &mut join_set,
            stdout,
            events_tx.clone(),
        );

        let job = Job::new(
            id,
            JobInner {
                id,
                child: tokio::sync::RwLock::new(Some(child)),
                stdin: Some(stdin),
                events_tx: Arc::new(events_tx),
                events_rx: Arc::new(events_rx),
                join_set: Mutex::new(join_set),
                stderr,
                stdout,
                combined: Default::default(),
                kind,
                state: Default::default(),
                data,
            },
        );

        self.jobs.write().unwrap().push(job.clone());
        self.events_tx.send(HubEvent::JobAdded(id)).unwrap();

        Ok(job)
    }
    pub fn jobs(&self, count: Option<usize>) -> Vec<Job<T>> {
        if let Some(count) = count {
            self.jobs.read().unwrap()[self.jobs.read().unwrap().len().saturating_sub(count)..]
                .to_vec()
        } else {
            self.jobs.read().unwrap().clone()
        }
    }

    pub fn get_job(&self, id: JobId) -> Option<Job<T>> {
        self.jobs(None).iter().find(|j| j.id() == id).cloned()
    }

    pub fn add_finished_job(&self, j: FinishedJobParams<T>) -> Job<T> {
        let id = self.next_job_id();

        let (events_tx, events_rx) = tokio::sync::broadcast::channel(128);
        let inner = JobInner {
            id,
            child: Default::default(),
            stdin: Default::default(),
            events_tx: Arc::new(events_tx),
            events_rx: Arc::new(events_rx),
            join_set: Default::default(),
            stderr: Arc::new(RwLock::new(j.stderr)),
            stdout: Arc::new(RwLock::new(j.stdout)),
            combined: Arc::new(RwLock::new(j.combined)),
            kind: j.kind,
            state: RwLock::new(j.state),
            data: j.data,
        };
        let job = Job::new(id, inner);
        self.jobs.write().unwrap().push(job.clone());
        self.events_tx.send(HubEvent::JobAdded(id)).unwrap();

        job
    }
}

pub struct FinishedJobParams<T> {
    pub kind: JobKind,
    pub data: T,
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
    pub combined: Vec<u8>,
    pub state: JobState,
}

#[tracing::instrument(skip_all, fields(spawn_reader=%src))]
fn spawn_reader(
    src: JobEventSource,
    join_set: &mut JoinSet<()>,
    mut reader: impl AsyncReadExt + Sized + Unpin + Send + 'static,
    event_tx: tokio::sync::broadcast::Sender<JobEvent>,
) -> Arc<RwLock<Vec<u8>>> {
    let output = Arc::<RwLock<Vec<u8>>>::default();
    join_set.spawn({
        let output = Arc::clone(&output);
        async move {
            let mut buf = Vec::with_capacity(1024);
            loop {
                buf.clear();
                let read_n = reader.read_buf(&mut buf).await.expect("read failed");
                if read_n == 0 {
                    tracing::debug!("closed");
                    event_tx.send(JobEvent::Closed { src }).unwrap();
                    break;
                }
                let (from, to) = {
                    let mut output = output.write().unwrap();
                    let from = output.len();
                    output.extend_from_slice(&buf);
                    let to = output.len();
                    (from, to)
                };
                event_tx.send(JobEvent::Wrote { src, from, to }).unwrap();
            }
        }
        .in_current_span()
    });
    output
}
