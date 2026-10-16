use std::{sync::Arc, time::Duration};

use dev_storage_core::workspaces::{WorkspaceErrorCode, WorkspaceFailure};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) fn failure(code: WorkspaceErrorCode) -> WorkspaceFailure {
    WorkspaceFailure {
        code,
        existing_id: None,
    }
}

pub struct SelectionGuard {
    _permit: OwnedSemaphorePermit,
}

impl SelectionGuard {
    pub async fn commit<T: Send + 'static>(
        self,
        work: impl FnOnce() -> Result<T, WorkspaceFailure> + Send + 'static,
    ) -> Result<T, WorkspaceFailure> {
        database_job(move || {
            // The blocking commit owns the gate even if the awaiting invocation is dropped.
            let _selection = self;
            work()
        })
        .await
    }
}

pub struct WorkspaceRuntime {
    selection: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    deadline: Duration,
}

impl Default for WorkspaceRuntime {
    fn default() -> Self {
        Self::new(Duration::from_secs(3))
    }
}

impl WorkspaceRuntime {
    pub(crate) fn new(deadline: Duration) -> Self {
        Self {
            selection: Arc::new(Semaphore::new(1)),
            workers: Arc::new(Semaphore::new(2)),
            deadline,
        }
    }
    pub fn try_begin_selection(&self) -> Result<SelectionGuard, WorkspaceFailure> {
        self.selection
            .clone()
            .try_acquire_owned()
            .map(|permit| SelectionGuard { _permit: permit })
            .map_err(|_| failure(WorkspaceErrorCode::OperationInProgress))
    }

    pub async fn validate<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, WorkspaceFailure> + Send + 'static,
    ) -> Result<T, WorkspaceFailure> {
        let permit = self
            .workers
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure(WorkspaceErrorCode::OperationInProgress))?;
        let job = tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            work()
        });
        tokio::time::timeout(self.deadline, job)
            .await
            .map_err(|_| failure(WorkspaceErrorCode::ValidationTimeout))?
            .map_err(|_| failure(WorkspaceErrorCode::ApplicationUnavailable))?
    }
}

pub(crate) async fn database_job<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, WorkspaceFailure> + Send + 'static,
) -> Result<T, WorkspaceFailure> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| failure(WorkspaceErrorCode::ApplicationUnavailable))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::sync::{Condvar, Mutex};

    #[derive(Default)]
    pub(super) struct Barrier {
        released: Mutex<bool>,
        changed: Condvar,
    }
    impl Barrier {
        fn wait(&self) {
            drop(
                self.changed
                    .wait_while(self.released.lock().unwrap(), |done| !*done)
                    .unwrap(),
            );
        }
        fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.changed.notify_all();
        }
    }

    // Catches releasing the selector before its current owner is dropped.
    #[test]
    fn workspace_runtime_selection_gate() {
        let runtime = WorkspaceRuntime::default();
        let selection = runtime.try_begin_selection().unwrap();
        assert!(matches!(
            runtime.try_begin_selection(),
            Err(WorkspaceFailure {
                code: WorkspaceErrorCode::OperationInProgress,
                ..
            })
        ));
        drop(selection);
        assert!(runtime.try_begin_selection().is_ok());
    }

    // Catches releasing the selection while an uncancellable DB commit is still running.
    #[test]
    fn workspace_runtime_commit_retains_selection_after_receiver_drop() {
        tauri::async_runtime::block_on(async {
            let runtime = WorkspaceRuntime::default();
            let barrier = Arc::new(Barrier::default());
            let worker_barrier = barrier.clone();
            let (entered, started) = std::sync::mpsc::channel();
            let mut pending = Box::pin(runtime.try_begin_selection().unwrap().commit(move || {
                entered.send(()).unwrap();
                worker_barrier.wait();
                Ok(())
            }));
            assert!(
                std::future::poll_fn(|cx| std::task::Poll::Ready(
                    pending.as_mut().poll(cx).is_pending()
                ))
                .await
            );
            started.recv_timeout(Duration::from_secs(2)).unwrap();
            drop(pending);
            let still_held = runtime.try_begin_selection().is_err();
            barrier.release();
            assert!(still_held);
            let available = tokio::time::timeout(
                Duration::from_secs(2),
                runtime.selection.clone().acquire_owned(),
            )
            .await
            .unwrap()
            .unwrap();
            drop(available);
            assert!(runtime.try_begin_selection().is_ok());
        });
    }

    // Catches queuing excess metadata, early permit release, and missing deadlines.
    #[test]
    fn workspace_runtime_retains_timed_out_workers() {
        tauri::async_runtime::block_on(async {
            let runtime = WorkspaceRuntime {
                deadline: Duration::from_millis(10),
                ..Default::default()
            };
            let barrier = Arc::new(Barrier::default());
            for _ in 0..2 {
                let worker_barrier = barrier.clone();
                let outcome = runtime
                    .validate(move || {
                        worker_barrier.wait();
                        Ok(())
                    })
                    .await;
                if outcome.as_ref().err().map(|error| error.code)
                    != Some(WorkspaceErrorCode::ValidationTimeout)
                {
                    barrier_release_on_failure(&barrier);
                }
                assert_eq!(
                    outcome.unwrap_err().code,
                    WorkspaceErrorCode::ValidationTimeout
                );
            }
            assert_eq!(
                runtime.validate(|| Ok(())).await.unwrap_err().code,
                WorkspaceErrorCode::OperationInProgress
            );
            barrier.release();
            let permits = runtime.workers.clone().acquire_many_owned(2).await.unwrap();
            drop(permits);
            assert_eq!(runtime.validate(|| Ok(42)).await.unwrap(), 42);
        });
    }

    fn barrier_release_on_failure(barrier: &Arc<Barrier>) {
        barrier.release();
    }
}
