use std::sync::{Arc, Mutex};

pub struct Receiver<T> {
    pub(crate) history: Arc<Mutex<Vec<T>>>,
    pub(crate) queued: Vec<T>,
    pub(crate) core_rx: tokio::sync::broadcast::Receiver<T>,
}

#[derive(Clone)]
pub struct Sender<T> {
    pub(crate) tx: tokio::sync::broadcast::Sender<T>,
}

pub fn channel<T: Clone + std::fmt::Debug + Send + 'static>(
    capacity: usize,
) -> (Sender<T>, Receiver<T>) {
    let (core_tx, core_rx) = tokio::sync::broadcast::channel::<T>(capacity);
    let history = Arc::new(Mutex::new(Vec::new()));
    tokio::spawn({
        let history = history.clone();
        let mut core_rx = core_rx.resubscribe();
        async move {
            while let Ok(item) = core_rx.recv().await {
                history.lock().unwrap().push(item.clone());
            }
        }
    });

    (
        Sender { tx: core_tx },
        Receiver {
            core_rx,
            queued: Vec::new(),
            history,
        },
    )
}

impl<T: Clone + std::fmt::Debug + Send + 'static> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        if let Some(item) = self.queued.pop() {
            return Some(item);
        }

        if let Ok(item) = self.core_rx.recv().await {
            return Some(item);
        }

        None
    }

    pub fn resubscribe(&self) -> Self {
        let queued = self.history.lock().unwrap().clone();
        Self {
            history: self.history.clone(),
            core_rx: self.core_rx.resubscribe(),
            queued,
        }
    }
}

impl<T: Clone + std::fmt::Debug + Send + 'static> Sender<T> {
    pub fn send(&self, item: T) -> Result<usize, tokio::sync::broadcast::error::SendError<T>> {
        self.tx.send(item)
    }
}
