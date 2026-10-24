//! One guarded session, one queue and one scoped worker launch.
use crate::{
    GuardedSetupError, QueueLimits, WorkQueue, run_scoped_workers, with_guarded_backend,
    worker_count,
};
use std::thread;

/// A single-use launch token borrowing the guarded session's queue.
/// Hosts select task meaning, root admission, worker effects and final publication.
/// The token cannot be constructed or cloned by a consumer.
///
/// ```compile_fail
/// use hunter_fs_core::{ScanSession, WorkQueue};
/// fn launch_twice(session: ScanSession<'_, ()>) {
///     session.run(|_: &WorkQueue<()>| {}, |_| (), |_| ());
///     session.run(|_: &WorkQueue<()>| {}, |_| (), |_| ()); // moved
/// }
/// ```
pub struct ScanSession<'env, T> {
    queue: &'env WorkQueue<T>,
    workers: usize,
}

impl<'env, T: Send + 'env> ScanSession<'env, T> {
    /// Start helpers/workers and coordinate on the caller, returning only after join.
    /// Consuming the token prevents relaunching its already-closed queue.
    /// Move unread receivers into `coordinate` and release them before it returns;
    /// its captures also drop before join on unwind. Helper-stop destructors must
    /// stop helpers, workers must react to queue closure, and drops must not panic.
    pub fn run<W, S, G, C, R>(self, worker: W, start_helpers: S, coordinate: C) -> R
    where
        W: Fn(&WorkQueue<T>) + Clone + Send + 'env,
        S: for<'scope> FnOnce(&'scope thread::Scope<'scope, 'env>) -> G,
        C: FnOnce(&WorkQueue<T>) -> R,
    {
        run_scoped_workers(self.queue, self.workers, worker, start_helpers, move || {
            coordinate(self.queue)
        })
    }
}

/// Enter the host guard, prepare its backend, then allocate a queue and select
/// available parallelism capped by `maximum_workers`. A zero cap panics after
/// successful preparation; setup failures skip queue selection and the body.
/// The caller-thread body may consume the launch token, then finalize after join
/// while backend and guard are still active. Backend drops before guard.
///
/// Root admission remains a coordinator operation (e.g. `admit_root`), preserving
/// host admission/publication timing relative to worker startup. No task is queued
/// here. An unused token starts no workers. All `with_guarded_backend` obligations
/// apply: no guard-dependent handles may escape, external borrows stay external,
/// guard drops must not panic; no timeout, I/O interruption or memory cap is added.
pub fn with_scan_session<T, G, B, GE, BE, R>(
    enter: impl FnOnce() -> Result<G, GE>,
    prepare: impl FnOnce() -> Result<B, BE>,
    maximum_workers: usize,
    run: impl FnOnce(&mut B, ScanSession<'_, T>) -> R,
) -> Result<R, GuardedSetupError<GE, BE>> {
    with_scan_session_limits(enter, prepare, maximum_workers, QueueLimits::UNBOUNDED, run)
}

/// Explicit host-selected task/weight admission limits. Queue construction remains
/// inside the successful guarded backend scope. Full admission never blocks the
/// coordinator; callers must handle rejection and tear down the response receiver
/// before joining blocked producers. Limits do not account for arbitrary payload bytes.
pub fn with_scan_session_limits<T, G, B, GE, BE, R>(
    enter: impl FnOnce() -> Result<G, GE>,
    prepare: impl FnOnce() -> Result<B, BE>,
    maximum_workers: usize,
    queue_limits: QueueLimits,
    run: impl FnOnce(&mut B, ScanSession<'_, T>) -> R,
) -> Result<R, GuardedSetupError<GE, BE>> {
    with_guarded_backend(enter, prepare, |backend| {
        let queue = WorkQueue::with_limits(queue_limits);
        let workers = worker_count(maximum_workers);
        run(
            backend,
            ScanSession {
                queue: &queue,
                workers,
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
    };

    struct DropEvent<'a>(&'a Mutex<Vec<&'static str>>, &'static str);
    impl Drop for DropEvent<'_> {
        fn drop(&mut self) {
            self.0.lock().unwrap().push(self.1);
        }
    }

    #[test]
    fn joins_before_host_finalization_and_releases_backend_before_guard() {
        let events = Mutex::new(Vec::new());
        let owner = thread::current().id();
        let result = with_scan_session(
            || Ok::<_, ()>(DropEvent(&events, "guard")),
            || Ok::<_, ()>(DropEvent(&events, "backend")),
            1,
            |_, session: ScanSession<'_, ()>| {
                let value = session.run(
                    |q| {
                        while q.pop().is_some() {}
                        events.lock().unwrap().push("joined");
                    },
                    |_| DropEvent(&events, "stop-helpers"),
                    |q| {
                        assert_eq!(thread::current().id(), owner);
                        q.push(()).unwrap();
                        Rc::new(RefCell::new(7))
                    },
                );
                assert_eq!(*events.lock().unwrap(), ["stop-helpers", "joined"]);
                events.lock().unwrap().push("finalize");
                value
            },
        )
        .unwrap();
        assert_eq!(*result.borrow(), 7);
        assert_eq!(
            *events.lock().unwrap(),
            ["stop-helpers", "joined", "finalize", "backend", "guard"]
        );
    }

    #[test]
    fn setup_failures_skip_launch_and_preserve_original_reason() {
        let reason = Rc::new(());
        let result = with_scan_session::<(), _, _, _, (), ()>(
            || Err::<(), _>(&reason),
            || -> Result<(), ()> { panic!("prepare must not run") },
            0,
            |_, _| panic!("body must not run"),
        );
        assert!(matches!(result, Err(GuardedSetupError::Guard(r)) if std::ptr::eq(r, &reason)));
        let events = Mutex::new(Vec::new());
        let result = with_scan_session::<(), _, _, (), _, ()>(
            || Ok(DropEvent(&events, "guard")),
            || Err::<(), _>(&reason),
            0,
            |_, _| panic!("body must not run"),
        );
        assert!(matches!(result, Err(GuardedSetupError::Backend(r)) if std::ptr::eq(r, &reason)));
        assert_eq!(*events.lock().unwrap(), ["guard"]);
    }

    #[test]
    fn coordinator_unwind_releases_receiver_then_joins_before_outer_cleanup() {
        let events = Mutex::new(Vec::new());
        let exited = AtomicBool::new(false);
        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let (ready, started) = mpsc::channel();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_scan_session(
                || Ok::<_, ()>(DropEvent(&events, "guard")),
                || Ok::<_, ()>(DropEvent(&events, "backend")),
                1,
                |_, session: ScanSession<'_, ()>| {
                    session.run(
                        |_| {
                            ready.send(()).unwrap();
                            assert!(sender.send(()).is_err());
                            exited.store(true, Ordering::SeqCst);
                            events.lock().unwrap().push("joined");
                        },
                        |_| (),
                        move |_| {
                            let _receiver = receiver;
                            started
                                .recv_timeout(std::time::Duration::from_secs(5))
                                .unwrap();
                            panic!("coordinator");
                        },
                    )
                },
            )
        }));
        assert!(panic.is_err());
        assert!(exited.load(Ordering::SeqCst));
        assert_eq!(*events.lock().unwrap(), ["joined", "backend", "guard"]);
    }

    #[test]
    fn unused_token_does_not_launch_and_zero_cap_unwinds_owned_resources() {
        let events = Mutex::new(Vec::new());
        with_scan_session::<(), _, _, (), (), _>(
            || Ok(DropEvent(&events, "guard")),
            || Ok(DropEvent(&events, "backend")),
            1,
            |_, _| (),
        )
        .unwrap();
        assert_eq!(*events.lock().unwrap(), ["backend", "guard"]);
        events.lock().unwrap().clear();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_scan_session::<(), _, _, (), (), _>(
                || Ok(DropEvent(&events, "guard")),
                || Ok(DropEvent(&events, "backend")),
                0,
                |_, _| panic!("invalid cap must skip body"),
            )
        }));
        assert!(panic.is_err());
        assert_eq!(*events.lock().unwrap(), ["backend", "guard"]);
    }
}
