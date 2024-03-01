use std::{
    ffi::OsStr,
    fmt::Debug,
    path::Path,
    process::Stdio,
    sync::{atomic::AtomicUsize, Arc, RwLock},
    time::Duration,
};

use tokio::{io::AsyncReadExt, sync::Mutex};

use crate::{
    job::{Job, JobData, JobEvent, JobInner, JobKind},
    JobId, JobState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HubEvent {
    JobAdded(JobId),
}

#[derive(Debug, Clone)]
pub struct Hub<M> {
    next_job_id: Arc<AtomicUsize>,
    jobs: Arc<RwLock<Vec<Job<M>>>>,
    events_tx: Arc<tokio::sync::broadcast::Sender<HubEvent>>,
    events_rx: Arc<tokio::sync::broadcast::Receiver<HubEvent>>,
}

impl<M> PartialEq for Hub<M> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.jobs, &other.jobs)
    }
}

impl<M: Send + Sync + 'static> Hub<M> {
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
        meta: M,
        program: impl AsRef<OsStr> + Debug,
        args: impl IntoIterator<Item = impl AsRef<OsStr>> + Debug,
    ) -> Job<M>
    where
        M: Debug,
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

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let mut data = JobData::new(kind, meta);
                data.state = JobState::Failed;
                data.stderr = format!("{:?}", e).into();
                data.combined = data.stderr.clone();
                return self.add_finished_job(data);
            }
        };

        let mut stderr = child.stderr.take().expect("we piped stderr");
        let mut stdout = child.stdout.take().expect("we piped stdout");

        let (events_tx, events_rx) = tokio::sync::broadcast::channel(128);

        // Terminate the job if it has been running for longer than the timeout.
        // We give a generous timeout for compilation jobs, and a more strict one for analysis jobs.
        let timeout = match &kind {
            JobKind::Analysis(_) => Duration::from_secs(10),
            JobKind::Compilation => Duration::from_secs(60),
        };
        let max_output = 2usize.pow(14);
        let data = Arc::new(RwLock::new(JobData::new(kind, meta)));

        enum Exit {
            ExitStatus(std::process::ExitStatus),
            Terminated,
        }

        let mut stderr_buf = Vec::with_capacity(1024);
        let mut stdout_buf = Vec::with_capacity(1024);
        let task = tokio::spawn({
            let data1 = Arc::clone(&data);
            let events_tx1 = events_tx.clone();
            let main_task = async move {
                let mut bytes_left = max_output;
                let mut exit_status = None;
                let mut stderr_empty = false;
                let mut stdout_empty = false;
                loop {
                    stderr_buf.clear();
                    stdout_buf.clear();
                    tokio::select! {
                        Ok(n) = stderr.read_buf(&mut stderr_buf), if !stderr_empty => {
                            stderr_empty = n == 0;
                            bytes_left = bytes_left.saturating_sub(n);
                            let mut data = data1.write().unwrap();
                            data.stderr.extend_from_slice(&stderr_buf[..n]);
                            data.combined.extend_from_slice(&stderr_buf[..n]);
                        }
                        Ok(n) = stdout.read_buf(&mut stdout_buf), if !stdout_empty => {
                            stdout_empty = n == 0;
                            bytes_left = bytes_left.saturating_sub(n);
                            let mut data = data1.write().unwrap();
                            data.stdout.extend_from_slice(&stdout_buf[..n]);
                            data.combined.extend_from_slice(&stdout_buf[..n]);
                        }
                        Ok(es) = child.wait(), if exit_status.is_none() => {
                            exit_status = Some(es);
                        },
                        else => {
                            break if let Some(exit_status) = exit_status {
                                Exit::ExitStatus(exit_status)
                            } else {
                                Exit::Terminated
                            };
                        }
                    }
                    if bytes_left == 0 {
                        data1.write().unwrap().state = JobState::OutputLimitExceeded;
                        let _ = child.kill().await;
                        break Exit::Terminated;
                    }
                    events_tx1.send(JobEvent::Wrote {}).unwrap();
                }
            };
            let data2 = Arc::clone(&data);
            let events_tx2 = events_tx.clone();
            async move {
                match tokio::time::timeout(timeout, main_task).await {
                    Ok(exit) => {
                        let mut data = data2.write().unwrap();
                        data.state = match exit {
                            Exit::ExitStatus(exit_status) => {
                                if exit_status.success() {
                                    JobState::Succeeded
                                } else {
                                    JobState::Failed
                                }
                            }
                            Exit::Terminated => JobState::OutputLimitExceeded,
                        };
                    }
                    Err(_elasped) => {
                        let mut data = data2.write().unwrap();
                        data.state = JobState::Timeout;
                    }
                }
                events_tx2.send(JobEvent::Finished).unwrap();
            }
        });

        let job = Job::new(
            id,
            JobInner {
                task: Arc::new(Mutex::new(Some(task))),
                events_rx: Arc::new(events_rx),
                data,
                wait_lock: Default::default(),
            },
        );

        self.jobs.write().unwrap().push(job.clone());
        self.events_tx.send(HubEvent::JobAdded(id)).unwrap();

        job
    }
    pub fn jobs(&self, count: Option<usize>) -> Vec<Job<M>> {
        if let Some(count) = count {
            self.jobs.read().unwrap()[self.jobs.read().unwrap().len().saturating_sub(count)..]
                .to_vec()
        } else {
            self.jobs.read().unwrap().clone()
        }
    }

    pub fn get_job(&self, id: JobId) -> Option<Job<M>> {
        self.jobs(None).iter().find(|j| j.id() == id).cloned()
    }

    pub fn add_finished_job(&self, j: JobData<M>) -> Job<M> {
        let id = self.next_job_id();

        let (_events_tx, events_rx) = tokio::sync::broadcast::channel(128);
        let inner = JobInner {
            task: Arc::new(Mutex::new(None)),
            events_rx: Arc::new(events_rx),
            data: Arc::new(RwLock::new(j)),
            wait_lock: Default::default(),
        };
        let job = Job::new(id, inner);
        self.jobs.write().unwrap().push(job.clone());
        self.events_tx.send(HubEvent::JobAdded(id)).unwrap();

        job
    }
}
