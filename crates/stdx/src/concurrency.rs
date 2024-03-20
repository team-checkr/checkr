pub type Semaphore = once_cell::sync::Lazy<tokio::sync::Semaphore>;

pub const fn semaphore() -> Semaphore {
    once_cell::sync::Lazy::new(|| {
        tokio::sync::Semaphore::new(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        )
    })
}
