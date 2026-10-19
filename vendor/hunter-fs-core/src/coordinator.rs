//! Root admission and serial task/response coordination with opaque host state.
use crate::{
    CoordinatorExit, DirectoryRequest, DirectoryTask, DirectoryTaskLedger, FilesystemBackend,
    RootAdmission, ScanCoordinator, WorkQueue, admit_root, drive_scan,
};
use std::{path::Path, sync::mpsc, time::Duration};

/// Owned per-scan accounting with a borrowed queue. State/pending/context are opaque.
/// IDs, speculative measurement slots and outstanding responses follow the ledger
/// protocol. This is unbounded storage, not a filesystem snapshot or a join fence.
pub struct TraversalTasks<'env, State, Pending, Context> {
    queue: &'env WorkQueue<DirectoryTask<usize, Context>>,
    ledger: DirectoryTaskLedger<State, Pending>,
}
impl<'env, State, Pending, Context> TraversalTasks<'env, State, Pending, Context> {
    pub fn new(queue: &'env WorkQueue<DirectoryTask<usize, Context>>) -> Self {
        Self {
            queue,
            ledger: DirectoryTaskLedger::default(),
        }
    }
    pub fn outstanding(&self) -> usize {
        self.ledger.outstanding()
    }

    /// Reserve ID/state/response, notify caller telemetry, then publish the task.
    /// Rejection returns the unchanged task and retains its reservation; host must
    /// stop/interpret failure. Callback panic unwinds before publication. The queue
    /// lock is not held across callback or payload destruction.
    pub fn enqueue(
        &mut self,
        request: DirectoryRequest<Context>,
        state: State,
        before_publish: impl FnOnce(usize),
    ) -> Result<usize, DirectoryTask<usize, Context>> {
        let task = self.ledger.register(request, state);
        let id = task.id;
        before_publish(self.outstanding());
        self.queue.push(task).map(|()| id)
    }
    pub fn take_listing(&mut self, id: usize, measuring: bool) -> State {
        self.ledger.take_listing(id, measuring)
    }
    pub fn defer_measurement(&mut self, id: usize, pending: Option<Pending>) {
        self.ledger.defer_measurement(id, pending);
    }
    pub fn take_measurement(&mut self, id: usize) -> Option<Pending> {
        self.ledger.take_measurement(id)
    }
}

/// Caller-thread interpretation of admission, opaque responses and product state.
/// No Send/static bound is required. Root initialization returns false to skip the
/// pump and finish callback. Hosts preserve valid listing/measurement ordering,
/// interpret setup failure, and must stop after rejected enqueue/disconnection.
pub trait TraversalHandler {
    type State;
    type Pending;
    type Context;
    type RootReason;
    type Message;
    fn initialize(
        &mut self,
        admission: RootAdmission<Self::RootReason>,
        tasks: &mut TraversalTasks<'_, Self::State, Self::Pending, Self::Context>,
    ) -> bool;
    fn should_stop(&self) -> bool;
    fn checkpoint(&mut self);
    /// Called after accounting a receipt, before checking stop or dispatching it.
    fn response_received(&mut self, outstanding: usize);
    fn dispatch(
        &mut self,
        message: Self::Message,
        tasks: &mut TraversalTasks<'_, Self::State, Self::Pending, Self::Context>,
    );
    /// Partial/product finalization after the pump, before ledger drop. Not join.
    fn finish(&mut self, exit: CoordinatorExit);
}

/// Result-loop outcome, distinct from worker join and host scan/report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraversalOutcome {
    /// None means the host declined initialization; no pump/finish callback ran.
    pub exit: Option<CoordinatorExit>,
    /// Unanswered reservations are retained on stop/disconnection, not erased.
    pub outstanding: usize,
}

/// Single-use coordinator owning accounting, borrowing the host and scoped queue.
/// Host callbacks may schedule/defer tasks through the provided task access. The
/// coordinator has no knowledge of selection vocabulary or report representation.
pub struct TraversalCoordinator<'env, 'host, H: TraversalHandler> {
    host: &'host mut H,
    tasks: TraversalTasks<'env, H::State, H::Pending, H::Context>,
}
impl<'env, 'host, H: TraversalHandler> TraversalCoordinator<'env, 'host, H> {
    pub fn new(
        host: &'host mut H,
        tasks: TraversalTasks<'env, H::State, H::Pending, H::Context>,
    ) -> Self {
        Self { host, tasks }
    }

    /// Observe the original root after host preflight, initialize through host,
    /// then drive. Call inside the guarded worker scope to preserve launch timing.
    /// No extra cancellation check, enumeration, normalization or retry is added.
    pub fn run<B: FilesystemBackend + ?Sized>(
        mut self,
        backend: &B,
        root: &Path,
        preflight: impl FnOnce(&Path) -> Option<H::RootReason>,
        receiver: &mpsc::Receiver<H::Message>,
        wake_interval: Option<Duration>,
    ) -> TraversalOutcome {
        let admission = admit_root(backend, root, preflight);
        if !self.host.initialize(admission, &mut self.tasks) {
            return TraversalOutcome {
                exit: None,
                outstanding: self.tasks.outstanding(),
            };
        }
        self.drive(receiver, wake_interval)
    }

    /// Drive already initialized tasks using the same receipt/stop/checkpoint
    /// boundaries as drive_scan, then call host finish once. Consumes the state.
    /// Disconnection leaves unanswered counts visible. Panics propagate without
    /// finish; resources drop normally. No workers spawn or join here. Host must
    /// own shutdown scope and release unread receivers before its actual join.
    pub fn drive(
        mut self,
        receiver: &mpsc::Receiver<H::Message>,
        wake_interval: Option<Duration>,
    ) -> TraversalOutcome {
        let exit = drive_scan(&mut self, receiver, wake_interval);
        self.host.finish(exit);
        TraversalOutcome {
            exit: Some(exit),
            outstanding: self.tasks.outstanding(),
        }
    }
}
impl<H: TraversalHandler> ScanCoordinator for TraversalCoordinator<'_, '_, H> {
    type Message = H::Message;
    fn outstanding(&self) -> usize {
        self.tasks.outstanding()
    }
    fn should_stop(&self) -> bool {
        self.host.should_stop()
    }
    fn checkpoint(&mut self) {
        self.host.checkpoint();
    }
    fn response_received(&mut self) {
        self.tasks.ledger.response_received();
        self.host.response_received(self.tasks.outstanding());
    }
    fn dispatch(&mut self, message: H::Message) {
        self.host.dispatch(message, &mut self.tasks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RootObservation;
    use std::{
        cell::{Cell, RefCell},
        fs, io,
        rc::Rc,
    };

    struct Backend {
        calls: Cell<usize>,
        denied: bool,
    }
    impl FilesystemBackend for Backend {
        type Entries = std::iter::Empty<io::Result<fs::DirEntry>>;
        fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            assert_eq!(path, Path::new("literal/../root"));
            self.calls.set(self.calls.get() + 1);
            if self.denied {
                Err(io::Error::from_raw_os_error(13))
            } else {
                fs::symlink_metadata(env!("CARGO_MANIFEST_DIR"))
            }
        }
        fn read_dir(&self, _: &Path) -> io::Result<Self::Entries> {
            panic!("no enumeration in coordinator")
        }
    }
    #[derive(Debug)]
    enum Message {
        Listed(usize, bool),
        Measured(usize),
        SetupFailed,
    }
    type Tasks<'a> = TraversalTasks<'a, &'static str, Rc<Cell<usize>>, ()>;
    #[derive(Default)]
    struct Host<'a> {
        trace: RefCell<Vec<String>>,
        owner: Option<std::thread::ThreadId>,
        skipped: Option<&'a str>,
        error: Option<i32>,
        stopped: bool,
        stop_checkpoint: bool,
        stop_receipt: bool,
        recurse: bool,
        pending: Rc<Cell<usize>>,
        finished: usize,
        panic_dispatch: bool,
    }
    fn request(name: &str) -> DirectoryRequest<()> {
        DirectoryRequest {
            path: name.into(),
            context: (),
            measure: false,
        }
    }
    impl<'a> TraversalHandler for Host<'a> {
        type State = &'static str;
        type Pending = Rc<Cell<usize>>;
        type Context = ();
        type RootReason = &'a str;
        type Message = Message;
        fn initialize(&mut self, admission: RootAdmission<&'a str>, tasks: &mut Tasks<'_>) -> bool {
            assert_eq!(Some(std::thread::current().id()), self.owner);
            self.trace.borrow_mut().push("initialize".into());
            match admission {
                RootAdmission::Skipped(reason) => {
                    self.skipped = Some(reason);
                    return false;
                }
                RootAdmission::Unavailable(e) => {
                    self.error = e.raw_os_error();
                }
                RootAdmission::Observed(RootObservation::Directory(_)) => {
                    tasks
                        .enqueue(request("root"), "root", |n| {
                            self.trace.borrow_mut().push(format!("queued:{n}"))
                        })
                        .unwrap();
                }
                RootAdmission::Observed(RootObservation::Entry(_)) => panic!("expected directory"),
            }
            true
        }
        fn should_stop(&self) -> bool {
            self.stopped
        }
        fn checkpoint(&mut self) {
            self.trace.borrow_mut().push("checkpoint".into());
            self.stopped |= self.stop_checkpoint;
        }
        fn response_received(&mut self, n: usize) {
            self.trace.borrow_mut().push(format!("received:{n}"));
            self.stopped |= self.stop_receipt;
        }
        fn dispatch(&mut self, message: Message, tasks: &mut Tasks<'_>) {
            assert!(!self.panic_dispatch, "host dispatch");
            self.trace.borrow_mut().push("dispatch".into());
            match message {
                Message::Listed(id, measuring) => {
                    let state = tasks.take_listing(id, measuring);
                    if measuring {
                        tasks.defer_measurement(id, Some(self.pending.clone()));
                    }
                    if state == "root" && self.recurse {
                        tasks
                            .enqueue(request("child"), "child", |n| {
                                self.trace.borrow_mut().push(format!("queued:{n}"))
                            })
                            .unwrap();
                    }
                }
                Message::Measured(id) => {
                    if let Some(p) = tasks.take_measurement(id) {
                        p.set(p.get() + 1);
                    }
                }
                Message::SetupFailed => self.stopped = true,
            }
        }
        fn finish(&mut self, _: CoordinatorExit) {
            self.finished += 1;
            self.trace.borrow_mut().push("finish".into());
        }
    }
    fn host<'a>() -> Host<'a> {
        Host {
            owner: Some(std::thread::current().id()),
            ..Host::default()
        }
    }
    fn backend() -> Backend {
        Backend {
            calls: Cell::new(0),
            denied: false,
        }
    }

    #[test]
    fn root_recursive_tasks_and_measurement_receipts_share_one_owned_ledger() {
        let queue = WorkQueue::default();
        let (tx, rx) = mpsc::channel();
        tx.send(Message::Listed(0, true)).unwrap();
        tx.send(Message::Measured(0)).unwrap();
        tx.send(Message::Listed(1, false)).unwrap();
        let mut host = host();
        host.recurse = true;
        let backend = backend();
        let out = TraversalCoordinator::new(&mut host, Tasks::new(&queue)).run(
            &backend,
            Path::new("literal/../root"),
            |_| None,
            &rx,
            None,
        );
        assert_eq!(
            out,
            TraversalOutcome {
                exit: Some(CoordinatorExit::Drained),
                outstanding: 0
            }
        );
        assert_eq!(backend.calls.get(), 1);
        assert_eq!(host.pending.get(), 1);
        assert_eq!(
            Rc::strong_count(&host.pending),
            1,
            "deferred payload released"
        );
        assert_eq!(host.finished, 1);
        assert_eq!(
            *host.trace.borrow(),
            [
                "initialize",
                "queued:1",
                "checkpoint",
                "received:0",
                "dispatch",
                "queued:2",
                "checkpoint",
                "received:1",
                "dispatch",
                "checkpoint",
                "received:0",
                "dispatch",
                "finish"
            ]
        );
        queue.close();
    }
    #[test]
    fn skipped_root_preserves_borrowed_reason_without_observation_or_finish() {
        let queue = WorkQueue::default();
        let (_tx, rx) = mpsc::channel();
        let mut host = host();
        let backend = backend();
        let reason = String::from("host decision");
        let out = TraversalCoordinator::new(&mut host, Tasks::new(&queue)).run(
            &backend,
            Path::new("literal/../root"),
            |_| Some(reason.as_str()),
            &rx,
            None,
        );
        assert_eq!(
            out,
            TraversalOutcome {
                exit: None,
                outstanding: 0
            }
        );
        assert_eq!(backend.calls.get(), 0);
        assert_eq!(host.finished, 0);
        assert!(std::ptr::eq(host.skipped.unwrap(), reason.as_str()));
        assert_eq!(*host.trace.borrow(), ["initialize"]);
    }
    #[test]
    fn unavailable_root_retains_os_error_and_host_decides_whether_to_drive() {
        let queue = WorkQueue::default();
        let (_tx, rx) = mpsc::channel();
        let mut host = host();
        let backend = Backend {
            calls: Cell::new(0),
            denied: true,
        };
        let out = TraversalCoordinator::new(&mut host, Tasks::new(&queue)).run(
            &backend,
            Path::new("literal/../root"),
            |_| None,
            &rx,
            None,
        );
        assert_eq!(out.exit, Some(CoordinatorExit::Drained));
        assert_eq!(host.error, Some(13));
        assert_eq!(host.finished, 1);
        assert_eq!(*host.trace.borrow(), ["initialize", "finish"]);
    }
    #[test]
    fn checkpoint_cancellation_preserves_receive_retire_then_stop_boundary() {
        for queued in [false, true] {
            let queue = WorkQueue::default();
            let (tx, rx) = mpsc::channel();
            if queued {
                tx.send(Message::SetupFailed).unwrap();
            }
            let mut host = host();
            host.stop_checkpoint = true;
            let mut tasks = Tasks::new(&queue);
            tasks.enqueue(request("root"), "root", |_| ()).unwrap();
            let out = TraversalCoordinator::new(&mut host, tasks)
                .drive(&rx, Some(Duration::from_millis(1)));
            assert_eq!(out.exit, Some(CoordinatorExit::Stopped));
            assert_eq!(out.outstanding, usize::from(!queued));
            assert_eq!(host.finished, 1);
            assert!(!host.trace.borrow().iter().any(|s| s == "dispatch"));
            assert_eq!(
                host.trace
                    .borrow()
                    .iter()
                    .filter(|s| s.starts_with("received:"))
                    .count(),
                usize::from(queued)
            );
            queue.close();
        }
    }
    #[test]
    fn disconnection_retains_unanswered_work_and_still_finalizes_on_caller() {
        let queue = WorkQueue::default();
        let (tx, rx) = mpsc::channel();
        drop(tx);
        let mut host = host();
        let mut tasks = Tasks::new(&queue);
        tasks.enqueue(request("root"), "root", |_| ()).unwrap();
        let out = TraversalCoordinator::new(&mut host, tasks).drive(&rx, None);
        assert_eq!(
            out,
            TraversalOutcome {
                exit: Some(CoordinatorExit::Disconnected),
                outstanding: 1
            }
        );
        assert_eq!(host.finished, 1);
        queue.close();
    }
    #[test]
    fn rejected_publication_notifies_before_return_and_retains_reservation() {
        let queue = WorkQueue::default();
        queue.close();
        let mut tasks = Tasks::new(&queue);
        let notified = Cell::new(0);
        let rejected = tasks
            .enqueue(request("literal/../root"), "state", |n| notified.set(n))
            .unwrap_err();
        assert_eq!(notified.get(), 1);
        assert_eq!(tasks.outstanding(), 1);
        assert_eq!(rejected.id, 0);
        assert_eq!(rejected.path, Path::new("literal/../root"));
    }
    #[test]
    fn callback_panic_skips_finish_and_releases_deferred_state() {
        let queue = WorkQueue::default();
        let (tx, rx) = mpsc::channel();
        tx.send(Message::Measured(0)).unwrap();
        let mut host = host();
        host.panic_dispatch = true;
        let mut tasks = Tasks::new(&queue);
        tasks.enqueue(request("root"), "root", |_| ()).unwrap();
        // Model an already consumed listing whose measurement is still expected.
        tasks.ledger.response_received();
        tasks.take_listing(0, true);
        tasks.defer_measurement(0, Some(host.pending.clone()));
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            TraversalCoordinator::new(&mut host, tasks).drive(&rx, None)
        }));
        assert!(panic.is_err());
        assert_eq!(host.finished, 0);
        assert_eq!(Rc::strong_count(&host.pending), 1);
        queue.close();
    }
}
