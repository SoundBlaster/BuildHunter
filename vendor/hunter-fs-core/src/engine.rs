//! Serial scan orchestration over the existing coordinator and entry processors.
use crate::{
    ChildDecision, ChildHandler, ChildTraversal, CoordinatorExit, DirectoryListing,
    DirectoryRequest, DirectoryResult, FilesystemBackend, MeasuredSize, MeasurementHandler,
    RootAdmission, SelectedEntry, SelectedMeasurementPlan, TraversalCoordinator, TraversalHandler,
    TraversalOutcome, TraversalTasks,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

/// Host-selected directory work and opaque branch interpretation state.
pub struct BranchPlan<Branch> {
    pub path: PathBuf,
    pub include_directory: bool,
    pub root_metadata: Option<fs::Metadata>,
    pub context: Branch,
}
/// Prepare may abandon without completion (e.g. cancellation), complete without
/// routing (e.g. pruning), or route children and reconcile selected measurements.
// Metadata layout differs by platform; this short-lived control value stays inline
// to avoid one heap allocation for every traversed directory.
#[allow(
    clippy::large_enum_variant,
    reason = "Inline per-directory control avoids a hot-path allocation"
)]
pub enum ListingAction<Branch> {
    Abandon,
    Complete(Branch),
    Traverse(BranchPlan<Branch>),
}
/// Engine-owned deferred selection; payload types remain host-defined.
/// This is not a bounded cache or a filesystem snapshot.
pub struct PendingSelection<Branch, Selection> {
    plan: SelectedMeasurementPlan<Selection>,
    branch: Branch,
}
pub type ScanTasks<'a, H> = TraversalTasks<
    'a,
    <H as ScanHandler>::State,
    PendingSelection<<H as ScanHandler>::Branch, <H as ScanHandler>::Selection>,
    <H as ScanHandler>::Context,
>;

/// Domain interpretation around one serial filesystem engine. Callbacks execute
/// on the caller and may borrow non-Send state. The host owns selection/access,
/// scheduling meaning, guards, units, reporting and failure/cancellation policy.
/// Responses must follow the ledger protocol: one valid Listed per reserved ID,
/// Listed before its promised Measured, and one receipt per reservation. Control
/// replies retire a reservation too. Invalid listing IDs may panic; ignored or
/// unknown measured payloads are dropped. This API does not validate wire identity.
/// prepare_listing owns interpretation of listing errors before they are released.
/// Abandon does not itself set stop: the host must choose stop/partial semantics.
pub trait ScanHandler: Sized {
    type State;
    type Branch;
    type Selection;
    type Context: Clone;
    type RootReason;
    type Message;
    fn initialize(
        &mut self,
        admission: RootAdmission<Self::RootReason>,
        tasks: &mut ScanTasks<'_, Self>,
    ) -> bool;
    fn should_stop(&self) -> bool;
    fn checkpoint(&mut self);
    fn response_received(&mut self, outstanding: usize);
    /// Interpret control messages, or return the typed directory response unchanged.
    /// Runs only after receipt accounting and the coordinator's stop check.
    fn response(&mut self, message: Self::Message)
    -> Option<DirectoryResult<usize, Self::Context>>;
    /// Optional telemetry before taking the directory's listing state.
    fn listing_received(&mut self) {}
    /// Speculative measurement reservation is visible before preparation.
    fn measurement_pending(&mut self, _outstanding: usize) {}
    fn prepare_listing(
        &mut self,
        state: Self::State,
        listing: &mut DirectoryListing<Self::Context>,
    ) -> ListingAction<Self::Branch>;
    fn select(
        &mut self,
        branch: &Self::Branch,
        index: usize,
        path: &Path,
        kind: crate::EntryType,
    ) -> ChildDecision;
    fn directory(
        &mut self,
        branch: &Self::Branch,
        index: usize,
        request: DirectoryRequest<Self::Context>,
        tasks: &mut ScanTasks<'_, Self>,
    );
    fn entry(
        &mut self,
        branch: &Self::Branch,
        index: usize,
        path: PathBuf,
        kind: crate::EntryType,
    ) -> Option<SelectedEntry<Self::Selection>>;
    fn directory_measured(
        &mut self,
        branch: &Self::Branch,
        path: &Path,
        metadata: io::Result<MeasuredSize>,
    );
    fn measurement_checkpoint(&mut self, index: usize);
    fn entry_measured(
        &mut self,
        branch: &Self::Branch,
        entry: &SelectedEntry<Self::Selection>,
        metadata: io::Result<MeasuredSize>,
    );
    fn finish_directory(&mut self, branch: &Self::Branch);
    /// Partial finalization before ledger drop; not worker join or terminal status.
    fn finish(&mut self, exit: CoordinatorExit);
}

/// Single-use caller-thread entrypoint combining root admission, receipt/ledger
/// coordination, child routing and selected metadata reconciliation. Worker launch,
/// unread receiver ownership and final publication remain in the enclosing scope.
///
/// ```compile_fail
/// use hunter_fs_core::{ScanEngine, ScanHandler, FilesystemBackend};
/// fn twice<H: ScanHandler, B: FilesystemBackend>(engine: ScanEngine<'_, '_, H>,
///     backend: &B, receiver: &std::sync::mpsc::Receiver<H::Message>) {
///     engine.drive(backend, receiver, None);
///     engine.drive(backend, receiver, None); // moved
/// }
/// ```
pub struct ScanEngine<'env, 'host, H: ScanHandler> {
    host: &'host mut H,
    tasks: ScanTasks<'env, H>,
}
impl<'env, 'host, H: ScanHandler> ScanEngine<'env, 'host, H> {
    pub fn new(host: &'host mut H, tasks: ScanTasks<'env, H>) -> Self {
        Self { host, tasks }
    }
    /// Call after workers start, inside their guarded scope. No extra stop probe,
    /// enumeration, retry, thread or join is introduced. Initialization false skips
    /// pump/finish. Drop unread receiver before the enclosing scope joins workers.
    pub fn run<B: FilesystemBackend + ?Sized>(
        self,
        backend: &B,
        root: &Path,
        preflight: impl FnOnce(&Path) -> Option<H::RootReason>,
        receiver: &mpsc::Receiver<H::Message>,
        wake_interval: Option<Duration>,
    ) -> TraversalOutcome {
        let mut adapter = Adapter {
            host: self.host,
            backend,
        };
        TraversalCoordinator::new(&mut adapter, self.tasks).run(
            backend,
            root,
            preflight,
            receiver,
            wake_interval,
        )
    }
    /// Drive previously initialized tasks with the same engine dispatch protocol.
    pub fn drive<B: FilesystemBackend + ?Sized>(
        self,
        backend: &B,
        receiver: &mpsc::Receiver<H::Message>,
        wake_interval: Option<Duration>,
    ) -> TraversalOutcome {
        let mut adapter = Adapter {
            host: self.host,
            backend,
        };
        TraversalCoordinator::new(&mut adapter, self.tasks).drive(receiver, wake_interval)
    }
}
struct Adapter<'a, H, B: ?Sized> {
    host: &'a mut H,
    backend: &'a B,
}
impl<H: ScanHandler, B: FilesystemBackend + ?Sized> TraversalHandler for Adapter<'_, H, B> {
    type State = H::State;
    type Pending = PendingSelection<H::Branch, H::Selection>;
    type Context = H::Context;
    type RootReason = H::RootReason;
    type Message = H::Message;
    fn initialize(
        &mut self,
        admission: RootAdmission<H::RootReason>,
        tasks: &mut ScanTasks<'_, H>,
    ) -> bool {
        self.host.initialize(admission, tasks)
    }
    fn should_stop(&self) -> bool {
        self.host.should_stop()
    }
    fn checkpoint(&mut self) {
        self.host.checkpoint();
    }
    fn response_received(&mut self, outstanding: usize) {
        self.host.response_received(outstanding);
    }
    fn dispatch(&mut self, message: H::Message, tasks: &mut ScanTasks<'_, H>) {
        let Some(response) = self.host.response(message) else {
            return;
        };
        match response {
            DirectoryResult::Listed {
                id,
                mut listing,
                measuring,
            } => {
                self.host.listing_received();
                let state = tasks.take_listing(id, measuring);
                if measuring {
                    self.host.measurement_pending(tasks.outstanding());
                }
                match self.host.prepare_listing(state, &mut listing) {
                    ListingAction::Abandon => {}
                    ListingAction::Complete(branch) => {
                        if measuring {
                            tasks.defer_measurement(id, None);
                        }
                        self.host.finish_directory(&branch);
                    }
                    ListingAction::Traverse(plan) => {
                        // Release the error buffer before routing, matching preparation's
                        // ownership transfer. Hosts must interpret needed errors in prepare.
                        drop(listing.errors);
                        let routed = ChildTraversal::new(
                            listing.children,
                            listing.context,
                            plan.include_directory,
                        )
                        .route(&mut Children {
                            host: self.host,
                            tasks,
                            branch: &plan.context,
                        });
                        if routed.stopped {
                            return;
                        }
                        let pending = PendingSelection {
                            plan: SelectedMeasurementPlan {
                                path: plan.path,
                                include_directory: plan.include_directory,
                                root_metadata: plan.root_metadata,
                                entries: routed.selected,
                            },
                            branch: plan.context,
                        };
                        if !pending.plan.include_directory && pending.plan.entries.is_empty() {
                            if measuring {
                                tasks.defer_measurement(id, None);
                            }
                            self.host.finish_directory(&pending.branch);
                        } else if measuring {
                            tasks.defer_measurement(id, Some(pending));
                        } else {
                            self.resolve(pending, None, Vec::new());
                        }
                    }
                }
            }
            DirectoryResult::Measured(result) => {
                if let Some(pending) = tasks.take_measurement(result.id) {
                    self.resolve(pending, Some(result.directory), result.files);
                }
            }
        }
    }
    fn finish(&mut self, exit: CoordinatorExit) {
        self.host.finish(exit);
    }
}
impl<H: ScanHandler, B: FilesystemBackend + ?Sized> Adapter<'_, H, B> {
    fn resolve(
        &mut self,
        pending: PendingSelection<H::Branch, H::Selection>,
        directory: Option<io::Result<fs::Metadata>>,
        files: crate::FileMeasurements,
    ) {
        pending.plan.resolve(
            self.backend,
            directory,
            files,
            &mut Measurements {
                host: self.host,
                branch: &pending.branch,
            },
        );
    }
}
struct Children<'a, 'queue, H: ScanHandler> {
    host: &'a mut H,
    tasks: &'a mut ScanTasks<'queue, H>,
    branch: &'a H::Branch,
}
impl<H: ScanHandler> ChildHandler<H::Context> for Children<'_, '_, H> {
    type Selected = SelectedEntry<H::Selection>;
    fn select(&mut self, index: usize, path: &Path, kind: crate::EntryType) -> ChildDecision {
        self.host.select(self.branch, index, path, kind)
    }
    fn directory(&mut self, index: usize, request: DirectoryRequest<H::Context>) {
        self.host.directory(self.branch, index, request, self.tasks);
    }
    fn entry(
        &mut self,
        index: usize,
        path: PathBuf,
        kind: crate::EntryType,
    ) -> Option<Self::Selected> {
        self.host.entry(self.branch, index, path, kind)
    }
}
struct Measurements<'a, H: ScanHandler> {
    host: &'a mut H,
    branch: &'a H::Branch,
}
impl<H: ScanHandler> MeasurementHandler<H::Selection> for Measurements<'_, H> {
    fn directory(&mut self, path: &Path, metadata: io::Result<MeasuredSize>) {
        self.host.directory_measured(self.branch, path, metadata);
    }
    fn checkpoint(&mut self, index: usize) {
        self.host.measurement_checkpoint(index);
    }
    fn entry(&mut self, entry: &SelectedEntry<H::Selection>, metadata: io::Result<MeasuredSize>) {
        self.host.entry_measured(self.branch, entry, metadata);
    }
    fn finish(&mut self) {
        self.host.finish_directory(self.branch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DirectoryMeasurement, WorkQueue, with_scan_session};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct Backend {
        calls: RefCell<Vec<PathBuf>>,
    }
    impl FilesystemBackend for Backend {
        type Entries = std::iter::Empty<io::Result<fs::DirEntry>>;
        fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            self.calls.borrow_mut().push(path.to_owned());
            let source = Path::new(env!("CARGO_MANIFEST_DIR"));
            fs::symlink_metadata(if path == Path::new("literal/../root") {
                source.to_owned()
            } else {
                source.join("Cargo.toml")
            })
        }
        fn read_dir(&self, _: &Path) -> io::Result<Self::Entries> {
            panic!("no enumeration by engine")
        }
    }
    fn backend() -> Backend {
        Backend {
            calls: RefCell::new(Vec::new()),
        }
    }
    #[derive(Clone, Copy)]
    enum Mode {
        Traverse,
        Complete,
        Abandon,
    }
    struct Node {
        path: PathBuf,
        root: Option<fs::Metadata>,
        mode: Mode,
    }
    struct Branch<'a> {
        label: &'a str,
        drops: Rc<Cell<usize>>,
    }
    impl Drop for Branch<'_> {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    struct Value(Rc<Cell<usize>>);
    impl Drop for Value {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    #[allow(
        clippy::large_enum_variant,
        reason = "Fixture preserves the host inline directory/control envelope"
    )]
    enum Message {
        Directory(DirectoryResult<usize, u8>),
        Control,
    }
    struct Host<'a> {
        label: &'a str,
        trace: Vec<String>,
        finish_values: Vec<usize>,
        mode: Mode,
        include: bool,
        stopped: bool,
        stop_select: bool,
        stop_checkpoint: bool,
        panic_entry: bool,
        branch_drops: Rc<Cell<usize>>,
        value_drops: Rc<Cell<usize>>,
        owner: std::thread::ThreadId,
    }
    fn host(label: &str) -> Host<'_> {
        Host {
            label,
            trace: vec![],
            finish_values: vec![],
            mode: Mode::Traverse,
            include: true,
            stopped: false,
            stop_select: false,
            stop_checkpoint: false,
            panic_entry: false,
            branch_drops: Rc::new(Cell::new(0)),
            value_drops: Rc::new(Cell::new(0)),
            owner: std::thread::current().id(),
        }
    }
    fn request(path: &Path) -> DirectoryRequest<u8> {
        DirectoryRequest {
            path: path.to_owned(),
            context: 7,
            measure: false,
        }
    }
    impl<'a> ScanHandler for Host<'a> {
        type State = Node;
        type Branch = Branch<'a>;
        type Selection = Value;
        type Context = u8;
        type RootReason = &'static str;
        type Message = Message;
        fn initialize(
            &mut self,
            admission: RootAdmission<&'static str>,
            tasks: &mut ScanTasks<'_, Self>,
        ) -> bool {
            assert_eq!(std::thread::current().id(), self.owner);
            self.trace.push("initialize".into());
            match admission {
                RootAdmission::Observed(crate::RootObservation::Directory(root)) => {
                    let path = PathBuf::from("literal/../root");
                    tasks
                        .enqueue(
                            request(&path),
                            Node {
                                path,
                                root: Some(root),
                                mode: self.mode,
                            },
                            |_| (),
                        )
                        .unwrap();
                    true
                }
                RootAdmission::Skipped(_) => false,
                _ => panic!("root observation"),
            }
        }
        fn should_stop(&self) -> bool {
            self.stopped
        }
        fn checkpoint(&mut self) {
            self.trace.push("checkpoint".into());
            self.stopped |= self.stop_checkpoint;
        }
        fn response_received(&mut self, n: usize) {
            self.trace.push(format!("received:{n}"));
        }
        fn response(&mut self, message: Message) -> Option<DirectoryResult<usize, u8>> {
            self.trace.push("response".into());
            match message {
                Message::Directory(r) => Some(r),
                Message::Control => {
                    self.stopped = true;
                    None
                }
            }
        }
        fn listing_received(&mut self) {
            self.trace.push("listing".into());
        }
        fn measurement_pending(&mut self, n: usize) {
            self.trace.push(format!("pending:{n}"));
        }
        fn prepare_listing(
            &mut self,
            state: Node,
            listing: &mut DirectoryListing<u8>,
        ) -> ListingAction<Self::Branch> {
            self.trace.push(format!("prepare:{}", state.path.display()));
            // This sample host interprets errors before engine releases the buffer.
            for (_, e) in std::mem::take(&mut listing.errors) {
                self.trace.push(format!("error:{:?}", e.kind()));
            }
            let branch = Branch {
                label: self.label,
                drops: self.branch_drops.clone(),
            };
            match state.mode {
                Mode::Abandon => {
                    self.stopped = true;
                    ListingAction::Abandon
                }
                Mode::Complete => ListingAction::Complete(branch),
                Mode::Traverse => ListingAction::Traverse(BranchPlan {
                    path: state.path,
                    include_directory: self.include,
                    root_metadata: state.root,
                    context: branch,
                }),
            }
        }
        fn select(
            &mut self,
            branch: &Self::Branch,
            index: usize,
            _: &Path,
            _: crate::EntryType,
        ) -> ChildDecision {
            assert_eq!(branch.label, self.label);
            self.trace.push(format!("select:{index}"));
            if self.stop_select && index == 1 {
                self.stopped = true;
                ChildDecision::Stop
            } else if index == 1 {
                ChildDecision::Skip
            } else {
                ChildDecision::Visit
            }
        }
        fn directory(
            &mut self,
            _: &Self::Branch,
            index: usize,
            req: DirectoryRequest<u8>,
            tasks: &mut ScanTasks<'_, Self>,
        ) {
            self.trace
                .push(format!("directory:{index}:{}", req.context));
            let path = req.path.clone();
            tasks
                .enqueue(
                    req,
                    Node {
                        path,
                        root: None,
                        mode: Mode::Complete,
                    },
                    |_| (),
                )
                .unwrap();
        }
        fn entry(
            &mut self,
            _: &Self::Branch,
            index: usize,
            path: PathBuf,
            _: crate::EntryType,
        ) -> Option<SelectedEntry<Value>> {
            self.trace.push(format!("entry:{index}"));
            Some(SelectedEntry {
                index,
                path,
                context: Value(self.value_drops.clone()),
            })
        }
        fn directory_measured(
            &mut self,
            _: &Self::Branch,
            path: &Path,
            m: io::Result<MeasuredSize>,
        ) {
            assert_eq!(path, Path::new("literal/../root"));
            assert!(m.is_ok());
            self.trace.push("size:directory".into());
        }
        fn measurement_checkpoint(&mut self, i: usize) {
            self.trace.push(format!("measure:{i}"));
        }
        fn entry_measured(
            &mut self,
            _: &Self::Branch,
            e: &SelectedEntry<Value>,
            m: io::Result<MeasuredSize>,
        ) {
            assert!(!self.panic_entry, "measurement callback");
            self.trace.push(format!(
                "size:{}:{}",
                e.index,
                if m.is_ok() { "ok" } else { "error" }
            ));
        }
        fn finish_directory(&mut self, _: &Self::Branch) {
            self.finish_values.push(self.value_drops.get());
            self.trace.push("directory-finish".into());
        }
        fn finish(&mut self, _: CoordinatorExit) {
            self.trace.push(format!(
                "finish:{}:{}",
                self.branch_drops.get(),
                self.value_drops.get()
            ));
        }
    }
    fn kind(directory: bool) -> crate::EntryType {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"));
        fs::symlink_metadata(if directory {
            p.to_owned()
        } else {
            p.join("Cargo.toml")
        })
        .unwrap()
        .file_type()
        .into()
    }
    fn listed(id: usize, measuring: bool, children: Vec<(PathBuf, crate::EntryType)>) -> Message {
        Message::Directory(DirectoryResult::Listed {
            id,
            measuring,
            listing: DirectoryListing {
                children,
                context: 9,
                errors: vec![],
            },
        })
    }
    fn measured(id: usize, files: crate::FileMeasurements) -> Message {
        Message::Directory(DirectoryResult::Measured(DirectoryMeasurement {
            id,
            directory: Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            files,
        }))
    }
    fn run(h: &mut Host<'_>, b: &Backend, messages: Vec<Message>) -> TraversalOutcome {
        let q = WorkQueue::default();
        let (tx, rx) = mpsc::channel();
        for m in messages {
            tx.send(m).unwrap();
        }
        drop(tx);
        let out = ScanEngine::new(h, ScanTasks::<Host<'_>>::new(&q)).run(
            b,
            Path::new("literal/../root"),
            |_| None,
            &rx,
            None,
        );
        q.close();
        out
    }
    #[test]
    fn deferred_root_and_pruned_child_share_ledger_without_retrying_cached_errors() {
        let label = String::from("borrowed");
        let mut h = host(&label);
        let b = backend();
        let out = run(
            &mut h,
            &b,
            vec![
                listed(
                    0,
                    true,
                    vec![
                        ("child".into(), kind(true)),
                        ("skip".into(), kind(false)),
                        ("literal/../file".into(), kind(false)),
                    ],
                ),
                measured(
                    0,
                    vec![
                        None,
                        None,
                        Some(Err(io::Error::from(io::ErrorKind::PermissionDenied))),
                    ],
                ),
                listed(1, true, vec![]),
                measured(1, vec![]),
            ],
        );
        assert_eq!(out.outstanding, 0);
        assert_eq!(out.exit, Some(CoordinatorExit::Drained));
        assert_eq!(*b.calls.borrow(), [PathBuf::from("literal/../root")]);
        assert!(h.trace.iter().any(|s| s == "directory:0:9"));
        assert!(h.trace.iter().any(|s| s == "size:2:error"));
        assert_eq!(
            h.trace
                .iter()
                .filter(|s| s.as_str() == "directory-finish")
                .count(),
            2
        );
        assert_eq!(h.branch_drops.get(), 2);
        assert_eq!(h.value_drops.get(), 1);
        assert_eq!(
            &h.trace[..6],
            [
                "initialize",
                "checkpoint",
                "received:0",
                "response",
                "listing",
                "pending:1"
            ]
        );
        assert_eq!(h.trace.last().unwrap(), "finish:2:1");
    }
    #[test]
    fn immediate_measurement_keeps_original_path_and_finishes_before_payload_release() {
        let mut h = host("host");
        h.include = false;
        let b = backend();
        run(
            &mut h,
            &b,
            vec![listed(
                0,
                false,
                vec![("literal/../file".into(), kind(false))],
            )],
        );
        assert_eq!(
            *b.calls.borrow(),
            [
                PathBuf::from("literal/../root"),
                PathBuf::from("literal/../file")
            ]
        );
        assert!(
            h.trace
                .windows(4)
                .any(|x| x == ["entry:0", "measure:0", "size:0:ok", "directory-finish"])
        );
        assert_eq!(
            h.finish_values,
            [0],
            "selected payload survives the directory completion callback"
        );
        assert_eq!(h.trace.last().unwrap(), "finish:1:1");
    }
    #[test]
    fn empty_unselected_directory_completes_and_consumes_ignored_measurement() {
        let mut h = host("host");
        h.include = false;
        let b = backend();
        let out = run(
            &mut h,
            &b,
            vec![listed(0, true, vec![]), measured(0, vec![])],
        );
        assert_eq!(out.outstanding, 0);
        assert_eq!(b.calls.borrow().len(), 1);
        assert!(
            !h.trace
                .iter()
                .any(|s| s.starts_with("measure:") || s.starts_with("size:"))
        );
        assert_eq!(
            h.trace
                .iter()
                .filter(|s| s.as_str() == "directory-finish")
                .count(),
            1
        );
    }
    #[test]
    fn abandon_and_child_stop_release_prefix_without_completing_or_defer_repair() {
        for abandon in [true, false] {
            let mut h = host("host");
            h.mode = if abandon {
                Mode::Abandon
            } else {
                Mode::Traverse
            };
            h.stop_select = !abandon;
            let b = backend();
            let out = run(
                &mut h,
                &b,
                vec![listed(
                    0,
                    true,
                    vec![
                        ("a".into(), kind(false)),
                        ("b".into(), kind(false)),
                        ("c".into(), kind(false)),
                    ],
                )],
            );
            assert_eq!(out.exit, Some(CoordinatorExit::Stopped));
            assert_eq!(out.outstanding, 1);
            assert!(
                !h.trace
                    .iter()
                    .any(|s| s == "directory-finish" || s == "select:2" || s.starts_with("size:"))
            );
            assert_eq!(h.value_drops.get(), usize::from(!abandon));
            assert_eq!(h.branch_drops.get(), 1);
        }
    }
    #[test]
    fn preflight_skip_and_checkpoint_stop_preserve_admission_and_receipt_boundaries() {
        let q = WorkQueue::default();
        let (tx, rx) = mpsc::channel();
        let b = backend();
        let mut h = host("host");
        let out = ScanEngine::new(&mut h, ScanTasks::<Host<'_>>::new(&q)).run(
            &b,
            Path::new("literal/../root"),
            |_| Some("skip"),
            &rx,
            None,
        );
        assert_eq!(out.exit, None);
        assert!(b.calls.borrow().is_empty());
        assert_eq!(h.trace, ["initialize"]);
        h.trace.clear();
        h.stop_checkpoint = true;
        let mut tasks = ScanTasks::<Host<'_>>::new(&q);
        tasks
            .enqueue(
                request(Path::new("root")),
                Node {
                    path: "root".into(),
                    root: None,
                    mode: Mode::Traverse,
                },
                |_| (),
            )
            .unwrap();
        tx.send(Message::Control).unwrap();
        let out = ScanEngine::new(&mut h, tasks).drive(&b, &rx, None);
        assert_eq!(out.outstanding, 0);
        assert_eq!(out.exit, Some(CoordinatorExit::Stopped));
        assert_eq!(h.trace, ["checkpoint", "received:0", "finish:0:0"]);
        q.close();
    }
    #[test]
    fn control_response_is_host_interpreted_and_disconnection_retains_pending_work() {
        let b = backend();
        let mut h = host("host");
        let out = run(&mut h, &b, vec![Message::Control]);
        // Drained wins at the next loop boundary, even after host failure.
        assert_eq!(out.exit, Some(CoordinatorExit::Drained));
        assert!(h.stopped);
        assert_eq!(out.outstanding, 0);
        assert!(h.trace.iter().any(|s| s == "response"));
        let mut h = host("host");
        let out = run(&mut h, &b, vec![]);
        assert_eq!(out.exit, Some(CoordinatorExit::Disconnected));
        assert_eq!(out.outstanding, 1);
    }
    #[test]
    fn measurement_callback_panic_unwinds_selection_and_skips_both_finishes() {
        let b = backend();
        let mut h = host("host");
        h.panic_entry = true;
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(
                &mut h,
                &b,
                vec![
                    listed(0, true, vec![("a".into(), kind(false))]),
                    measured(
                        0,
                        vec![Some(Ok(MeasuredSize {
                            apparent: 7,
                            allocated: 8,
                        }))],
                    ),
                ],
            )
        }));
        assert!(panic.is_err());
        assert_eq!(h.branch_drops.get(), 1);
        assert_eq!(h.value_drops.get(), 1);
        assert!(
            !h.trace
                .iter()
                .any(|s| s == "directory-finish" || s.starts_with("finish:"))
        );
        assert_eq!(b.calls.borrow().len(), 1);
    }
    #[test]
    fn session_runs_engine_on_caller_and_joins_before_outer_finalization() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let b = backend();
        let mut h = host("host");
        h.include = false;
        let joined = AtomicBool::new(false);
        let out =
            with_scan_session(
                || Ok::<_, ()>(()),
                || Ok::<_, ()>(()),
                1,
                |_, session: crate::ScanSession<'_, crate::DirectoryTask<usize, u8>>| {
                    let (tx, rx) = mpsc::channel();
                    let out =
                        session.run(
                            |q| {
                                while let Some(task) = q.pop() {
                                    tx.send(listed(task.id, false, vec![])).unwrap();
                                }
                                joined.store(true, Ordering::SeqCst);
                            },
                            |_| (),
                            |q| {
                                let out = ScanEngine::new(&mut h, ScanTasks::<Host<'_>>::new(q))
                                    .run(&b, Path::new("literal/../root"), |_| None, &rx, None);
                                drop(rx);
                                out
                            },
                        );
                    assert!(joined.load(Ordering::SeqCst));
                    assert_eq!(h.trace.last().unwrap(), "finish:1:0");
                    out
                },
            )
            .unwrap();
        assert_eq!(out.exit, Some(CoordinatorExit::Drained));
        assert_eq!(out.outstanding, 0);
    }

    #[test]
    fn launch_collects_host_and_drops_receiver_before_helpers_stop_and_join() {
        use crate::{EngineLaunch, ScanChannel};
        use std::sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        };
        let b = backend();
        let label = String::from("borrowed launch");
        let mut h = host(&label);
        h.include = false;
        let alive = h.branch_drops.clone();
        let joined = AtomicBool::new(false);
        let stopped = AtomicBool::new(false);
        let events = Mutex::new(Vec::new());
        let owner = std::thread::current().id();
        struct Stop<'a> {
            events: &'a Mutex<Vec<&'static str>>,
            alive: &'a Rc<Cell<usize>>,
            stopped: &'a AtomicBool,
        }
        impl Drop for Stop<'_> {
            fn drop(&mut self) {
                assert_eq!(
                    Rc::strong_count(self.alive),
                    1,
                    "host-only state released before helpers stop"
                );
                self.events.lock().unwrap().push("stop");
                self.stopped.store(true, Ordering::SeqCst);
            }
        }
        let result = with_scan_session(
            || Ok::<_, ()>(()),
            || Ok::<_, ()>(()),
            1,
            |_, session: crate::ScanSession<'_, crate::DirectoryTask<usize, u8>>| {
                let channel = ScanChannel::new();
                let sender = channel.sender();
                let probe = channel.sender();
                let result = session.run_engine(
                    EngineLaunch {
                        backend: &b,
                        root: Path::new("literal/../root"),
                        channel,
                        preflight: |_: &Path| {
                            events.lock().unwrap().push("preflight");
                            None
                        },
                        wake_interval: None,
                    },
                    |q| {
                        while let Some(task) = q.pop() {
                            sender.send(listed(task.id, false, vec![])).unwrap();
                        }
                        assert!(stopped.load(Ordering::SeqCst));
                        events.lock().unwrap().push("joined");
                        joined.store(true, Ordering::SeqCst);
                    },
                    |_| Stop {
                        events: &events,
                        alive: &alive,
                        stopped: &stopped,
                    },
                    || {
                        assert_eq!(std::thread::current().id(), owner);
                        events.lock().unwrap().push("make");
                        h
                    },
                    |host, outcome| {
                        assert_eq!(std::thread::current().id(), owner);
                        assert_eq!(outcome.exit, Some(CoordinatorExit::Drained));
                        assert!(!joined.load(Ordering::SeqCst));
                        assert!(
                            probe.send(Message::Control).is_err(),
                            "receiver released before collect"
                        );
                        assert_eq!(host.trace.last().unwrap(), "finish:1:0");
                        drop(host);
                        events.lock().unwrap().push("collect");
                        Rc::new(Cell::new(7))
                    },
                );
                assert!(joined.load(Ordering::SeqCst));
                events.lock().unwrap().push("finalize");
                result
            },
        )
        .unwrap();
        assert_eq!(result.get(), 7);
        assert_eq!(
            *events.lock().unwrap(),
            ["make", "preflight", "collect", "stop", "joined", "finalize"]
        );
    }
    #[test]
    fn launch_factory_panic_releases_receiver_before_queue_closure_and_join() {
        use crate::{EngineLaunch, ScanChannel};
        use std::sync::atomic::{AtomicBool, Ordering};
        let b = backend();
        let joined = AtomicBool::new(false);
        let (ready, started) = mpsc::channel();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_scan_session(
                || Ok::<_, ()>(()),
                || Ok::<_, ()>(()),
                1,
                |_, session: crate::ScanSession<'_, crate::DirectoryTask<usize, u8>>| {
                    let channel = ScanChannel::new();
                    let sender = channel.sender();
                    session.run_engine(
                        EngineLaunch {
                            backend: &b,
                            root: Path::new("literal/../root"),
                            channel,
                            preflight: |_: &Path| None,
                            wake_interval: None,
                        },
                        |q| {
                            ready.send(()).unwrap();
                            while q.pop().is_some() {}
                            assert!(sender.send(Message::Control).is_err());
                            joined.store(true, Ordering::SeqCst);
                        },
                        |_| (),
                        || -> Host<'_> {
                            started.recv_timeout(Duration::from_secs(5)).unwrap();
                            panic!("factory")
                        },
                        |_, _| panic!("collect must not run"),
                    )
                },
            )
        }));
        assert!(panic.is_err());
        assert!(joined.load(Ordering::SeqCst));
        assert!(b.calls.borrow().is_empty());
    }
    #[test]
    fn bounded_launch_factory_panic_wakes_a_producer_before_join() {
        use crate::{EngineLaunch, ScanChannel};
        use std::sync::atomic::{AtomicBool, Ordering};
        let b = backend();
        let joined = AtomicBool::new(false);
        let (ready, started) = mpsc::channel();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_scan_session(
                || Ok::<_, ()>(()),
                || Ok::<_, ()>(()),
                1,
                |_, session: crate::ScanSession<'_, crate::DirectoryTask<usize, u8>>| {
                    let channel = ScanChannel::bounded(0);
                    let sender = channel.sender();
                    session.run_engine(
                        EngineLaunch {
                            backend: &b,
                            root: Path::new("literal/../root"),
                            channel,
                            preflight: |_: &Path| None,
                            wake_interval: None,
                        },
                        |_| {
                            ready.send(()).unwrap();
                            // No receiver consumes this rendezvous send. Only dropping
                            // the engine-owned receiver on panic can release the worker.
                            assert!(sender.send(Message::Control).is_err());
                            joined.store(true, Ordering::SeqCst);
                        },
                        |_| (),
                        || -> Host<'_> {
                            started.recv_timeout(Duration::from_secs(5)).unwrap();
                            panic!("factory");
                        },
                        |_, _| panic!("collect must not run"),
                    )
                },
            )
        }));
        assert!(panic.is_err());
        assert!(joined.load(Ordering::SeqCst));
        assert!(b.calls.borrow().is_empty());
    }
    #[test]
    fn launch_collect_panic_joins_before_outer_backend_and_guard_drop() {
        use crate::{EngineLaunch, ScanChannel};
        use std::sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        };
        let b = backend();
        let joined = AtomicBool::new(false);
        let events = Mutex::new(Vec::new());
        struct DropAfterJoin<'a>(&'a AtomicBool, &'a Mutex<Vec<&'static str>>, &'static str);
        impl Drop for DropAfterJoin<'_> {
            fn drop(&mut self) {
                assert!(self.0.load(Ordering::SeqCst));
                self.1.lock().unwrap().push(self.2);
            }
        }
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_scan_session(
                || Ok::<_, ()>(DropAfterJoin(&joined, &events, "guard")),
                || Ok::<_, ()>(DropAfterJoin(&joined, &events, "backend")),
                1,
                |_, session: crate::ScanSession<'_, crate::DirectoryTask<usize, u8>>| {
                    let channel = ScanChannel::new();
                    let sender = channel.sender();
                    let probe = channel.sender();
                    session.run_engine(
                        EngineLaunch {
                            backend: &b,
                            root: Path::new("literal/../root"),
                            channel,
                            preflight: |_: &Path| Some("skip"),
                            wake_interval: None,
                        },
                        |q| {
                            while q.pop().is_some() {}
                            assert!(sender.send(Message::Control).is_err());
                            joined.store(true, Ordering::SeqCst);
                        },
                        |_| (),
                        || host("host"),
                        |_, outcome| -> () {
                            assert_eq!(outcome.exit, None);
                            assert!(probe.send(Message::Control).is_err());
                            panic!("collect")
                        },
                    )
                },
            )
        }));
        assert!(panic.is_err());
        assert_eq!(*events.lock().unwrap(), ["backend", "guard"]);
        assert!(b.calls.borrow().is_empty());
    }
    #[test]
    fn launch_declined_root_still_collects_and_joins_without_metadata_or_pump() {
        use crate::{EngineLaunch, ScanChannel};
        use std::sync::atomic::{AtomicBool, Ordering};
        let b = backend();
        let joined = AtomicBool::new(false);
        let result = with_scan_session(
            || Ok::<_, ()>(()),
            || Ok::<_, ()>(()),
            1,
            |_, session: crate::ScanSession<'_, crate::DirectoryTask<usize, u8>>| {
                let channel = ScanChannel::new();
                let sender = channel.sender();
                let result = session.run_engine(
                    EngineLaunch {
                        backend: &b,
                        root: Path::new("literal/../root"),
                        channel,
                        preflight: |_: &Path| Some("skip"),
                        wake_interval: None,
                    },
                    |q| {
                        while q.pop().is_some() {}
                        assert!(sender.send(Message::Control).is_err());
                        joined.store(true, Ordering::SeqCst);
                    },
                    |_| (),
                    || host("host"),
                    |host, outcome| {
                        assert_eq!(host.trace, ["initialize"]);
                        assert_eq!(outcome.exit, None);
                        outcome
                    },
                );
                assert!(joined.load(Ordering::SeqCst));
                result
            },
        )
        .unwrap();
        assert_eq!(result.outstanding, 0);
        assert!(b.calls.borrow().is_empty());
    }
}
