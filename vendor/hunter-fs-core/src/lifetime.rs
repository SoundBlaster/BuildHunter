//! Synchronous same-thread guard and backend lifetime assembly.

/// Original setup error, without product status or error decoration.
#[derive(Debug)]
pub enum GuardedSetupError<GuardError, BackendError> {
    Guard(GuardError),
    Backend(BackendError),
}

/// Enter the host guard, construct the backend, then run the body on this thread.
/// The owned backend drops before the guard, including ordinary unwinding and a
/// single backend-destructor panic. Guard setup failure skips both callbacks;
/// backend setup failure/panic skips the body and releases the guard.
///
/// Borrowed non-Send guards, backends, errors and results are supported. A body
/// must finish any scoped work before returning and must not extract guard-dependent
/// resources into its result or elsewhere. Externally borrowed backend state is
/// not owned or destroyed here. Unused callback captures are not covered by the
/// backend cleanup ordering. Guard destructors must not panic; panic=abort and
/// double panics cannot promise restoration. This does not interrupt active I/O.
pub fn with_guarded_backend<G, B, GE, BE, R>(
    enter: impl FnOnce() -> Result<G, GE>,
    prepare: impl FnOnce() -> Result<B, BE>,
    run: impl FnOnce(&mut B) -> R,
) -> Result<R, GuardedSetupError<GE, BE>> {
    let _guard = enter().map_err(GuardedSetupError::Guard)?;
    let mut backend = prepare().map_err(GuardedSetupError::Backend)?;
    let result = run(&mut backend);
    drop(backend);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{GuardedSetupError, WorkQueue, run_scoped_workers, with_guarded_backend};
    use std::{cell::RefCell, rc::Rc, thread::ThreadId};

    struct Resource<'a> {
        name: &'static str,
        events: &'a RefCell<Vec<&'static str>>,
        owner: ThreadId,
        panic_on_drop: bool,
    }
    impl Drop for Resource<'_> {
        fn drop(&mut self) {
            assert_eq!(std::thread::current().id(), self.owner);
            self.events.borrow_mut().push(self.name);
            assert!(!self.panic_on_drop, "backend destructor panic");
        }
    }
    fn resource<'a>(name: &'static str, events: &'a RefCell<Vec<&'static str>>) -> Resource<'a> {
        Resource {
            name,
            events,
            owner: std::thread::current().id(),
            panic_on_drop: false,
        }
    }
    #[test]
    fn guard_then_backend_then_body_with_reverse_cleanup_and_non_send_result() {
        let events = RefCell::new(Vec::new());
        let result = with_guarded_backend(
            || {
                events.borrow_mut().push("enter");
                Ok::<_, ()>(resource("guard", &events))
            },
            || {
                events.borrow_mut().push("prepare");
                Ok::<_, ()>(resource("backend", &events))
            },
            |backend| {
                assert_eq!(backend.name, "backend");
                events.borrow_mut().push("body");
                Rc::new(RefCell::new(7))
            },
        )
        .unwrap();
        assert_eq!(*result.borrow(), 7);
        assert_eq!(
            *events.borrow(),
            ["enter", "prepare", "body", "backend", "guard"]
        );
    }
    #[test]
    fn guard_failure_keeps_original_non_clone_reason_and_skips_backend_and_body() {
        struct Reason;
        let reason = Reason;
        let result = with_guarded_backend(
            || Err::<(), _>(&reason),
            || -> Result<(), ()> { panic!("must not prepare") },
            |_| panic!("must not run"),
        );
        let Err(GuardedSetupError::Guard(error)) = result else {
            panic!("guard error")
        };
        assert!(std::ptr::eq(error, &reason));
    }
    #[test]
    fn backend_failure_and_factory_panic_release_guard_without_body() {
        let events = RefCell::new(Vec::new());
        let reason = Rc::new(RefCell::new(9));
        let result = with_guarded_backend(
            || Ok::<_, ()>(resource("guard", &events)),
            || Err::<(), _>(&reason),
            |_| panic!("must not run"),
        );
        let Err(GuardedSetupError::Backend(error)) = result else {
            panic!("backend error")
        };
        assert!(std::ptr::eq(error, &reason));
        assert_eq!(*events.borrow(), ["guard"]);
        events.borrow_mut().clear();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_guarded_backend(
                || Ok::<_, ()>(resource("guard", &events)),
                || -> Result<(), ()> { panic!("factory") },
                |_| panic!("must not run"),
            )
        }));
        assert!(panic.is_err());
        assert_eq!(*events.borrow(), ["guard"]);
    }
    #[test]
    fn body_panic_or_backend_drop_panic_still_release_guard_on_same_thread() {
        for panic_in_body in [true, false] {
            let events = RefCell::new(Vec::new());
            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_guarded_backend(
                    || Ok::<_, ()>(resource("guard", &events)),
                    || {
                        let mut backend = resource("backend", &events);
                        backend.panic_on_drop = !panic_in_body;
                        Ok::<_, ()>(backend)
                    },
                    |_| {
                        assert!(!panic_in_body, "body");
                    },
                )
            }));
            assert!(panic.is_err());
            assert_eq!(*events.borrow(), ["backend", "guard"]);
        }
    }
    #[test]
    fn worker_join_precedes_backend_and_outer_guard_release() {
        let events = std::sync::Mutex::new(Vec::new());
        struct Backend<'a> {
            queue: WorkQueue<()>,
            events: &'a std::sync::Mutex<Vec<&'static str>>,
        }
        impl Drop for Backend<'_> {
            fn drop(&mut self) {
                self.events.lock().unwrap().push("backend");
            }
        }
        struct Guard<'a>(&'a std::sync::Mutex<Vec<&'static str>>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.lock().unwrap().push("guard");
            }
        }
        with_guarded_backend(
            || Ok::<_, ()>(Guard(&events)),
            || {
                Ok::<_, ()>(Backend {
                    queue: WorkQueue::default(),
                    events: &events,
                })
            },
            |backend| {
                run_scoped_workers(
                    &backend.queue,
                    1,
                    |queue| {
                        assert!(queue.pop().is_none());
                        events.lock().unwrap().push("worker-exit");
                    },
                    |_| (),
                    || events.lock().unwrap().push("coordinate"),
                );
                events.lock().unwrap().push("terminal");
            },
        )
        .unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            ["coordinate", "worker-exit", "terminal", "backend", "guard"]
        );
    }
}
