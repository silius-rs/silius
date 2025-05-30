//! https://github.com/ReamLabs/ream/tree/master/crates/common/executor

use std::future::Future;

use tokio::{runtime::Runtime, sync::broadcast, task::JoinHandle};

pub struct SiliusExecutor {
    runtime: Runtime,
    shutdown: broadcast::Sender<()>,
}

impl SiliusExecutor {
    pub fn new() -> std::io::Result<Self> {
        let runtime = Runtime::new()?;
        let (shutdown, _) = broadcast::channel(1);
        Ok(Self { runtime, shutdown })
    }

    /// Creates a new TaskExecutor with an existing runtime
    pub fn with_runtime(runtime: Runtime) -> Self {
        let (shutdown, _) = broadcast::channel(1);
        Self { runtime, shutdown }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let mut shutdown = self.shutdown.subscribe();
        self.runtime.spawn(async move {
            tokio::select! {
                result = future => result,
                _ = shutdown.recv() => panic!("Task cancelled due to shutdown"),
            }
        })
    }

    pub fn spawn_cancellable<F, Fut, T>(&self, future_fn: F) -> JoinHandle<Option<T>>
    where
        F: FnOnce(broadcast::Receiver<()>) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send,
        T: Send + 'static,
    {
        let shutdown = self.shutdown.subscribe();
        self.runtime.spawn(async move {
            let future = future_fn(shutdown);
            tokio::select! {
                result = future => Some(result),
                _ = tokio::signal::ctrl_c() => None,
            }
        })
    }

    /// Spawns a blocking task in a dedicated thread pool
    pub fn spawn_blocking<F, R>(&self, task: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.runtime.spawn_blocking(task)
    }

    /// Spawns multiple tasks and returns a handle that resolves when all tasks complete
    pub fn spawn_many<F, Fut, T>(&self, futures: impl IntoIterator<Item = F>) -> JoinHandle<Vec<T>>
    where
        F: FnOnce(broadcast::Receiver<()>) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send,
        T: Send + 'static,
    {
        let futures: Vec<_> = futures
            .into_iter()
            .map(|f| self.spawn_cancellable(f))
            .collect();

        self.runtime.spawn(async move {
            let results = futures::future::join_all(futures).await;
            results
                .into_iter()
                .filter_map(|r| r.ok().flatten())
                .collect()
        })
    }

    /// Triggers a shutdown signal to all spawned tasks
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }

    /// Get a reference to the underlying runtime
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    #[test]
    fn test_basic_task() {
        let executor = SiliusExecutor::new().unwrap();

        let handle = executor.spawn(async {
            sleep(Duration::from_millis(100)).await;
            42
        });

        assert_eq!(executor.runtime.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn test_cancellable_task() {
        let executor = SiliusExecutor::new().unwrap();

        let handle = executor.spawn_cancellable(|mut shutdown| async move {
            tokio::select! {
                _ = sleep(Duration::from_secs(1)) => "completed",
                _ = shutdown.recv() => "cancelled",
            }
        });

        executor.shutdown();
        assert_eq!(
            executor.runtime.block_on(handle).unwrap(),
            Some("cancelled")
        );
    }

    #[test]
    fn test_spawn_many() {
        let executor = SiliusExecutor::new().unwrap();

        let tasks = (0..3).map(|i| {
            move |_shutdown| async move {
                sleep(Duration::from_millis(50 * (i + 1) as u64)).await;
                i
            }
        });

        let handle = executor.spawn_many(tasks);
        let results = executor.runtime.block_on(handle).unwrap();
        assert_eq!(results, vec![0, 1, 2]);
    }
}
