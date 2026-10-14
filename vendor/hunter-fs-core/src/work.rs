//! Generic recursive-work coordination. Product selection and result accounting are host-owned.
use std::{
    sync::{Condvar, Mutex},
    thread,
};

/// Unbounded LIFO tasks; worker count is capped separately. Closing wakes all
/// waiters, discards pending tasks and rejects new tasks. In-flight work is not
/// interrupted: workers must use their host's cancellation checks for blocking I/O.
pub struct WorkQueue<T> {
    state: Mutex<State<T>>,
    ready: Condvar,
}
struct State<T> {
    tasks: Vec<T>,
    closed: bool,
}
impl<T> Default for WorkQueue<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                tasks: Vec::new(),
                closed: false,
            }),
            ready: Condvar::new(),
        }
    }
}
impl<T> WorkQueue<T> {
    /// Returns ownership to the caller when shutdown has already started.
    pub fn push(&self, task: T) -> Result<(), T> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            return Err(task);
        }
        state.tasks.push(task);
        drop(state);
        self.ready.notify_one();
        Ok(())
    }
    pub fn pop(&self) -> Option<T> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if state.closed {
                return None;
            }
            if let Some(task) = state.tasks.pop() {
                return Some(task);
            }
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
    pub fn close(&self) {
        let pending = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            std::mem::take(&mut state.tasks)
        };
        self.ready.notify_all();
        // Generic payload destructors must run outside the queue mutex.
        drop(pending);
    }
    /// Keep this guard in the coordinator so every exit closes the queue before
    /// std::thread::scope joins. Dropping the guard does not itself join workers.
    pub fn close_on_drop(&self) -> ShutdownGuard<'_, T> {
        ShutdownGuard {
            queue: self,
            panic_only: false,
        }
    }
    fn close_on_panic(&self) -> ShutdownGuard<'_, T> {
        ShutdownGuard {
            queue: self,
            panic_only: true,
        }
    }
}
#[must_use = "keep the guard alive until coordinator exit"]
pub struct ShutdownGuard<'a, T> {
    queue: &'a WorkQueue<T>,
    panic_only: bool,
}
impl<T> Drop for ShutdownGuard<'_, T> {
    fn drop(&mut self) {
        if !self.panic_only || thread::panicking() {
            self.queue.close();
        }
    }
}

/// Spawn bounded scoped workers; each clone gets its own captured sender/setup.
/// Panics close the queue before the worker unwinds, and thread::scope propagates
/// them after joining. Host owns setup/QoS, channels, stop condition and coordinator.
/// The coordinator MUST retain close_on_drop inside the scope, including on unwind.
pub fn spawn_workers<'scope, 'env, T, F>(
    scope: &'scope thread::Scope<'scope, 'env>,
    queue: &'scope WorkQueue<T>,
    count: usize,
    worker: F,
) where
    T: Send + 'scope,
    F: Fn(&WorkQueue<T>) + Clone + Send + 'scope,
{
    assert!(count > 0, "at least one worker is required");
    for _ in 0..count {
        let worker = worker.clone();
        scope.spawn(move || {
            let _close = queue.close_on_panic();
            worker(queue);
        });
    }
}

pub fn worker_count(maximum: usize) -> usize {
    assert!(maximum > 0, "worker cap must be positive");
    thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };
    #[test]
    fn lifo_and_closed_queue_returns_task_ownership() {
        let queue = WorkQueue::default();
        queue.push(1).unwrap();
        queue.push(2).unwrap();
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(1));
        queue.close();
        assert_eq!(queue.push(3), Err(3));
        assert_eq!(queue.pop(), None);
    }
    #[test]
    fn close_releases_queued_payloads_without_dropping_in_flight_payload() {
        let payload = Arc::new(());
        let queue = WorkQueue::default();
        queue.push(payload.clone()).unwrap();
        queue.push(payload.clone()).unwrap();
        let in_flight = queue.pop().unwrap();
        assert_eq!(Arc::strong_count(&payload), 3);
        queue.close();
        assert_eq!(Arc::strong_count(&payload), 2);
        drop(in_flight);
        assert_eq!(Arc::strong_count(&payload), 1);
    }
    #[test]
    fn closing_releases_all_idle_workers() {
        let queue = WorkQueue::<()>::default();
        let (ready, started) = mpsc::channel();
        let exits = AtomicUsize::new(0);
        thread::scope(|scope| {
            let _close = queue.close_on_drop();
            spawn_workers(scope, &queue, 4, {
                let exits = &exits;
                move |q| {
                    ready.send(()).unwrap();
                    assert_eq!(q.pop(), None);
                    exits.fetch_add(1, Ordering::SeqCst);
                }
            });
            for _ in 0..4 {
                started.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            queue.close();
        });
        assert_eq!(exits.load(Ordering::SeqCst), 4);
    }
    #[test]
    fn workers_can_expand_recursive_tasks_and_finish_before_return() {
        let queue = WorkQueue::default();
        queue.push(0usize).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut seen = Vec::new();
        thread::scope(|scope| {
            let _close = queue.close_on_drop();
            spawn_workers(scope, &queue, 3, move |q| {
                while let Some(n) = q.pop() {
                    if n < 3 {
                        q.push(n * 2 + 1).unwrap();
                        q.push(n * 2 + 2).unwrap();
                    }
                    tx.send(n).unwrap();
                }
            });
            for _ in 0..7 {
                seen.push(rx.recv_timeout(Duration::from_secs(5)).unwrap());
            }
        });
        seen.sort();
        assert_eq!(seen, (0..7).collect::<Vec<_>>());
        assert_eq!(queue.pop(), None);
    }
    #[test]
    fn worker_panic_closes_peers_before_coordinator_exits() {
        let queue = WorkQueue::default();
        queue.push(()).unwrap();
        let active = AtomicUsize::new(0);
        let completed = std::cell::Cell::new(0);
        let (exited, exits) = mpsc::channel();
        struct Exit<'a> {
            active: &'a AtomicUsize,
            exited: mpsc::Sender<()>,
        }
        impl Drop for Exit<'_> {
            fn drop(&mut self) {
                self.active.fetch_sub(1, Ordering::SeqCst);
                self.exited.send(()).unwrap();
            }
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            thread::scope(|scope| {
                let _close = queue.close_on_drop();
                spawn_workers(scope, &queue, 3, {
                    let active = &active;
                    move |q| {
                        active.fetch_add(1, Ordering::SeqCst);
                        let _exit = Exit {
                            active,
                            exited: exited.clone(),
                        };
                        if q.pop().is_some() {
                            panic!("injected worker panic");
                        }
                    }
                });
                // Peers must exit while the coordinator guard is still alive. Merely
                // observing a scope panic would also pass for a coordinator timeout.
                for _ in 0..3 {
                    exits.recv_timeout(Duration::from_secs(5)).unwrap();
                    completed.set(completed.get() + 1);
                }
            })
        }));
        assert_eq!(completed.get(), 3);
        assert!(result.is_err());
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(queue.pop(), None);
    }
    #[test]
    fn coordinator_unwind_closes_queue_before_scope_join() {
        let queue = WorkQueue::<()>::default();
        let (tx, rx) = mpsc::channel();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            thread::scope(|scope| {
                let _close = queue.close_on_drop();
                spawn_workers(scope, &queue, 2, move |q| {
                    tx.send(()).unwrap();
                    assert!(q.pop().is_none());
                });
                rx.recv_timeout(Duration::from_secs(5)).unwrap();
                panic!("injected coordinator panic");
            })
        }));
        assert!(result.is_err());
        assert_eq!(queue.pop(), None);
    }
    #[test]
    fn worker_cap_is_positive_and_respected() {
        assert_eq!(worker_count(1), 1);
        assert!((1..=8).contains(&worker_count(8)));
        assert!(std::panic::catch_unwind(|| worker_count(0)).is_err());
    }
}
