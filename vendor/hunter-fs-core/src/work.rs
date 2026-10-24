//! Generic recursive-work coordination. Product selection and result accounting are host-owned.
use std::{
    sync::{Condvar, Mutex},
    thread,
};

/// LIFO tasks with optional nonblocking admission limits. Worker count is capped separately. Closing wakes all
/// waiters, discards pending tasks and rejects new tasks. In-flight work is not
/// interrupted: workers must use their host's cancellation checks for blocking I/O.
pub struct WorkQueue<T> {
    state: Mutex<State<T>>,
    ready: Condvar,
}
/// Weight units are defined by the caller (e.g. estimated owned payload bytes),
/// not allocator/RSS accounting. Admission never waits on worker consumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueLimits {
    pub max_tasks: usize,
    pub max_weight: usize,
}
impl QueueLimits {
    pub const UNBOUNDED: Self = Self {
        max_tasks: usize::MAX,
        max_weight: usize::MAX,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueRejectionReason {
    Closed,
    TaskLimit,
    WeightLimit,
}
#[derive(Debug)]
pub struct QueueRejection<T> {
    pub task: T,
    pub reason: QueueRejectionReason,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueueSnapshot {
    pub pending_tasks: usize,
    pub pending_weight: usize,
    pub peak_tasks: usize,
    pub peak_weight: usize,
}
struct State<T> {
    tasks: Vec<(T, usize)>,
    closed: bool,
    limits: QueueLimits,
    snapshot: QueueSnapshot,
}
impl<T> Default for WorkQueue<T> {
    fn default() -> Self {
        Self::with_limits(QueueLimits::UNBOUNDED)
    }
}
impl<T> WorkQueue<T> {
    pub fn with_limits(limits: QueueLimits) -> Self {
        assert!(
            limits.max_tasks > 0 && limits.max_weight > 0,
            "queue limits must be positive"
        );
        Self {
            state: Mutex::new(State {
                tasks: Vec::new(),
                closed: false,
                limits,
                snapshot: QueueSnapshot::default(),
            }),
            ready: Condvar::new(),
        }
    }
    /// Returns ownership to the caller when shutdown has already started.
    /// For a limited queue this also rejects exhausted admission; use try_push_weighted
    /// to retain the exact reason. No reservation or payload is silently discarded.
    pub fn push(&self, task: T) -> Result<(), T> {
        self.try_push_weighted(task, 1)
            .map_err(|rejected| rejected.task)
    }
    pub fn try_push_weighted(&self, task: T, weight: usize) -> Result<(), QueueRejection<T>> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let reason = if state.closed {
            Some(QueueRejectionReason::Closed)
        } else if state.tasks.len() >= state.limits.max_tasks {
            Some(QueueRejectionReason::TaskLimit)
        } else if weight
            > state
                .limits
                .max_weight
                .saturating_sub(state.snapshot.pending_weight)
        {
            Some(QueueRejectionReason::WeightLimit)
        } else {
            None
        };
        if let Some(reason) = reason {
            drop(state);
            return Err(QueueRejection { task, reason });
        }
        state.tasks.push((task, weight));
        state.snapshot.pending_tasks = state.tasks.len();
        state.snapshot.pending_weight += weight;
        state.snapshot.peak_tasks = state.snapshot.peak_tasks.max(state.snapshot.pending_tasks);
        state.snapshot.peak_weight = state
            .snapshot
            .peak_weight
            .max(state.snapshot.pending_weight);
        drop(state);
        self.ready.notify_one();
        Ok(())
    }
    pub fn snapshot(&self) -> QueueSnapshot {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot
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
            if let Some((task, weight)) = state.tasks.pop() {
                state.snapshot.pending_tasks = state.tasks.len();
                state.snapshot.pending_weight -= weight;
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
            state.snapshot.pending_tasks = 0;
            state.snapshot.pending_weight = 0;
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
    fn admission_is_nonblocking_and_preserves_rejected_payload_and_weight() {
        let queue = WorkQueue::with_limits(QueueLimits {
            max_tasks: 2,
            max_weight: 5,
        });
        queue.try_push_weighted("first", 2).unwrap();
        let rejected = queue.try_push_weighted("too-heavy", 4).unwrap_err();
        assert_eq!(
            (rejected.task, rejected.reason),
            ("too-heavy", QueueRejectionReason::WeightLimit)
        );
        queue.try_push_weighted("second", 3).unwrap();
        let rejected = queue.try_push_weighted("too-many", 0).unwrap_err();
        assert_eq!(
            (rejected.task, rejected.reason),
            ("too-many", QueueRejectionReason::TaskLimit)
        );
        assert_eq!(
            queue.snapshot(),
            QueueSnapshot {
                pending_tasks: 2,
                pending_weight: 5,
                peak_tasks: 2,
                peak_weight: 5
            }
        );
        assert_eq!(queue.pop(), Some("second"));
        queue.try_push_weighted("replacement", 3).unwrap();
        queue.close();
        assert_eq!(queue.snapshot().pending_weight, 0);
        assert_eq!(queue.snapshot().pending_tasks, 0);
        assert_eq!(queue.snapshot().peak_tasks, 2);
        assert_eq!(
            queue.try_push_weighted("closed", 1).unwrap_err().reason,
            QueueRejectionReason::Closed
        );
    }

    #[test]
    fn weight_cannot_overflow_or_be_freed_twice() {
        let queue = WorkQueue::with_limits(QueueLimits::UNBOUNDED);
        queue.try_push_weighted(1, usize::MAX).unwrap();
        assert_eq!(
            queue.try_push_weighted(2, 1).unwrap_err().reason,
            QueueRejectionReason::WeightLimit
        );
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.snapshot().pending_weight, 0);
        queue.close();
        queue.close();
        assert_eq!(queue.snapshot().pending_weight, 0);
    }
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
