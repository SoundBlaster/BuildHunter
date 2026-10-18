//! Scoped startup/shutdown assembly; effects and terminal interpretation stay in the host.
use crate::{WorkQueue, spawn_workers};
use std::thread;

/// With a positive worker count, start auxiliary scoped threads, launch workers, then run the coordinator on
/// the calling thread. Drop the returned helper-stop guard before closing the
/// queue; the enclosing scope joins every thread before returning the result.
///
/// The coordinator and helper-start closures are owned inside that scope, so
/// their captures (including unread receivers) release before join on unwind.
/// Host must move receivers into the coordinator and release them before it
/// returns. Host callbacks/guards must be well-behaved on drop. Helpers must stop
/// when their guard drops, and workers must react to queue closure/host stop.
/// There is no syscall interruption, timeout, setup-error interpretation or
/// terminal publication here. Panics propagate after scoped threads join.
pub fn run_scoped_workers<'env, T, W, S, G, C, R>(
    queue: &'env WorkQueue<T>,
    count: usize,
    worker: W,
    start_helpers: S,
    coordinate: C,
) -> R
where
    T: Send + 'env,
    W: Fn(&WorkQueue<T>) + Clone + Send + 'env,
    S: for<'scope> FnOnce(&'scope thread::Scope<'scope, 'env>) -> G,
    C: FnOnce() -> R,
{
    thread::scope(move |scope| {
        let _close = queue.close_on_drop();
        let _helpers = start_helpers(scope);
        spawn_workers(scope, queue, count, worker);
        coordinate()
    })
}

#[cfg(test)]
mod tests {
    use crate::{WorkQueue, run_scoped_workers};
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc,
        },
        thread,
        time::Duration,
    };
    const TIMEOUT: Duration = Duration::from_secs(5);
    #[test]
    fn helpers_stop_before_queue_closure_and_all_threads_join_before_return() {
        let queue = WorkQueue::<()>::default();
        let exits = AtomicUsize::new(0);
        let sidecar = AtomicBool::new(false);
        let before_close = Cell::new(false);
        let owner = thread::current().id();
        let result = Rc::new(Cell::new(0));
        struct Stop<'a> {
            queue: &'a WorkQueue<()>,
            before: &'a Cell<bool>,
            _stop: mpsc::Sender<()>,
        }
        impl Drop for Stop<'_> {
            fn drop(&mut self) {
                self.before.set(self.queue.push(()).is_ok());
            }
        }
        let actual = run_scoped_workers(
            &queue,
            2,
            |q| {
                while q.pop().is_some() {}
                exits.fetch_add(1, Ordering::SeqCst);
            },
            |scope| {
                let (stop, signal) = mpsc::channel();
                let sidecar = &sidecar;
                scope.spawn(move || {
                    assert!(signal.recv().is_err());
                    sidecar.store(true, Ordering::SeqCst);
                });
                Stop {
                    queue: &queue,
                    before: &before_close,
                    _stop: stop,
                }
            },
            || {
                assert_eq!(thread::current().id(), owner);
                result.set(7);
                result.clone()
            },
        );
        assert!(Rc::ptr_eq(&actual, &result));
        assert_eq!(actual.get(), 7);
        assert!(before_close.get());
        assert_eq!(exits.load(Ordering::SeqCst), 2);
        assert!(sidecar.load(Ordering::SeqCst));
        assert!(queue.pop().is_none());
    }
    #[test]
    fn setup_failure_is_opaque_and_terminal_publication_can_follow_actual_join() {
        let queue = WorkQueue::<()>::default();
        let exited = AtomicBool::new(false);
        let (sender, receiver) = mpsc::channel();
        let seen = Cell::new(false);
        struct Exit<'a>(&'a AtomicBool);
        impl Drop for Exit<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let status = run_scoped_workers(
            &queue,
            1,
            |_| {
                let _exit = Exit(&exited);
                sender.send("setup failed").unwrap();
            },
            |_| (),
            || {
                let msg = receiver.recv_timeout(TIMEOUT).unwrap();
                seen.set(true);
                msg
            },
        );
        assert!(seen.get());
        assert_eq!(status, "setup failed");
        assert!(
            exited.load(Ordering::SeqCst),
            "terminal publication after this return sees joined workers"
        );
    }
    #[test]
    fn coordinator_unwind_drops_receiver_before_joining_blocked_worker() {
        let queue = WorkQueue::<()>::default();
        let exited = AtomicBool::new(false);
        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let (ready, started) = mpsc::channel();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_scoped_workers(
                &queue,
                1,
                |_| {
                    ready.send(()).unwrap();
                    assert!(sender.send(()).is_err());
                    exited.store(true, Ordering::SeqCst);
                },
                |_| (),
                move || {
                    let _receiver = receiver;
                    started.recv_timeout(TIMEOUT).unwrap();
                    panic!("coordinator panic")
                },
            )
        }));
        assert!(result.is_err());
        assert!(exited.load(Ordering::SeqCst));
        assert!(queue.pop().is_none());
    }
    #[test]
    fn helper_start_unwind_drops_unused_coordinator_captures_before_sidecar_join() {
        let queue = WorkQueue::<()>::default();
        let exited = AtomicBool::new(false);
        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_scoped_workers(
                &queue,
                1,
                |_| panic!("worker must not start"),
                |scope| -> () {
                    let exited = &exited;
                    scope.spawn(move || {
                        assert!(sender.send(()).is_err());
                        exited.store(true, Ordering::SeqCst);
                    });
                    panic!("helper start panic")
                },
                move || {
                    drop(receiver);
                    panic!("coordinator must not run")
                },
            )
        }));
        assert!(result.is_err());
        assert!(exited.load(Ordering::SeqCst));
        assert!(queue.pop().is_none());
    }
    #[test]
    fn worker_panic_closes_peers_and_propagates_after_join() {
        let queue = WorkQueue::default();
        queue.push(()).unwrap();
        let (sender, receiver) = mpsc::channel();
        let exited = Cell::new(0);
        struct Exit(mpsc::Sender<()>);
        impl Drop for Exit {
            fn drop(&mut self) {
                let _ = self.0.send(());
            }
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_scoped_workers(
                &queue,
                3,
                |q| {
                    let _exit = Exit(sender.clone());
                    if q.pop().is_some() {
                        panic!("worker panic");
                    }
                },
                |_| (),
                || {
                    for _ in 0..3 {
                        receiver.recv_timeout(TIMEOUT).unwrap();
                        exited.set(exited.get() + 1);
                    }
                },
            )
        }));
        assert!(result.is_err());
        assert_eq!(exited.get(), 3);
        assert!(queue.pop().is_none());
    }
}
