//! Serial child routing and selected-entry metadata reconciliation.
use crate::{
    ChildDecision, ChildTraversal, DirectoryRequest, FilesystemBackend, MeasuredSize, PlannedChild,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Host selection/access effects and interpretation of admitted children.
/// All callbacks are serial on the caller; no Send or static bound is imposed.
pub trait ChildHandler<Context> {
    type Selected;
    fn select(&mut self, index: usize, path: &Path, file_type: fs::FileType) -> ChildDecision;
    fn directory(&mut self, index: usize, request: DirectoryRequest<Context>);
    fn entry(
        &mut self,
        index: usize,
        path: PathBuf,
        file_type: fs::FileType,
    ) -> Option<Self::Selected>;
}
/// Selected prefix in input order. On stop the host decides whether to keep it.
pub struct RoutedChildren<Selected> {
    pub selected: Vec<Selected>,
    pub stopped: bool,
}
impl<Context: Clone> ChildTraversal<Context> {
    /// Consume the listing without another child vector; collect host-selected
    /// entry payloads only. Host select precedes routing at each original index.
    /// Empty/exhausted input never calls select; stop is terminal with no later
    /// callback. No additional stop check, I/O, identity or scheduling is added.
    /// The listing/unconsumed suffix and plan context drop before normal return,
    /// including on stop; panic also drops the collected prefix during unwind.
    pub fn route<H: ChildHandler<Context>>(mut self, host: &mut H) -> RoutedChildren<H::Selected> {
        let mut selected = Vec::new();
        while let Some(child) = self.next_child(|i, p, t| host.select(i, p, t)) {
            match child {
                PlannedChild::Directory { index, request } => host.directory(index, request),
                PlannedChild::Entry {
                    index,
                    path,
                    file_type,
                } => {
                    if let Some(entry) = host.entry(index, path, file_type) {
                        selected.push(entry);
                    }
                }
            }
        }
        RoutedChildren {
            selected,
            stopped: self.is_stopped(),
        }
    }
}

/// A host-selected original listing slot with opaque selection context.
/// No regular-file/no-link assertion is made here; selection policy is the host's.
pub struct SelectedEntry<Context> {
    pub index: usize,
    pub path: PathBuf,
    pub context: Context,
}
impl<Context> SelectedEntry<Context> {
    /// Use a supplied result, including errors, without retry. Only absence causes
    /// one backend symlink_metadata call on the unmodified path. No guard is entered.
    pub fn resolve_metadata<B: FilesystemBackend + ?Sized>(
        &self,
        backend: &B,
        supplied: Option<io::Result<MeasuredSize>>,
    ) -> io::Result<MeasuredSize> {
        supplied.unwrap_or_else(|| backend.symlink_metadata(&self.path).map(MeasuredSize::from))
    }
}

/// Selected coordinator-side measurement work, distinct from a worker's full plan.
/// Original positions match response slots; context and directory inclusion are
/// entirely host-selected. No tree snapshot, type filtering or memory cap is implied.
pub struct SelectedMeasurementPlan<Context> {
    pub path: PathBuf,
    pub include_directory: bool,
    pub root_metadata: Option<fs::Metadata>,
    pub entries: Vec<SelectedEntry<Context>>,
}
/// Interpret measurements/checkpoints and complete host state before buffers drop.
pub trait MeasurementHandler<Context> {
    fn directory(&mut self, path: &Path, metadata: io::Result<MeasuredSize>);
    /// Enumerated selected-entry position, not the original listing index.
    fn checkpoint(&mut self, index: usize);
    fn entry(&mut self, selected: &SelectedEntry<Context>, metadata: io::Result<MeasuredSize>);
    fn finish(&mut self);
}
impl<Context> SelectedMeasurementPlan<Context> {
    /// Resolve selected directory first, then entries in selection order, then finish.
    /// Directory precedence is root observation, supplied worker result, backend.
    /// File slots are taken once by original index; absent/out-of-range/previously
    /// consumed slots fall back once. Cached errors never trigger fallback.
    /// Checkpoint precedes slot lookup/backend access for every selected entry.
    /// No stop check is introduced; host owns cancellation/partial interpretation.
    /// Owned entries/cached suffix remain alive through finish and release afterward.
    /// Callback/backend panic propagates without finish and releases owned buffers.
    pub fn resolve<B: FilesystemBackend + ?Sized, H: MeasurementHandler<Context>>(
        self,
        backend: &B,
        directory: Option<io::Result<fs::Metadata>>,
        mut files: Vec<Option<io::Result<MeasuredSize>>>,
        host: &mut H,
    ) {
        if self.include_directory {
            let metadata = match self.root_metadata {
                Some(metadata) => Ok(metadata),
                None => directory.unwrap_or_else(|| backend.symlink_metadata(&self.path)),
            };
            host.directory(&self.path, metadata.map(MeasuredSize::from));
        }
        for (index, entry) in self.entries.iter().enumerate() {
            host.checkpoint(index);
            let supplied = files.get_mut(entry.index).and_then(Option::take);
            let metadata = entry.resolve_metadata(backend, supplied);
            host.entry(entry, metadata);
        }
        host.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    struct Context {
        clones: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
    }
    impl Clone for Context {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                clones: self.clones.clone(),
                drops: self.drops.clone(),
            }
        }
    }
    impl Drop for Context {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    struct Slot(Rc<Cell<usize>>);
    impl Drop for Slot {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    struct Router {
        calls: Vec<String>,
        skip: Option<usize>,
        stop: Option<usize>,
        panic_entry: Option<usize>,
        selected_drops: Rc<Cell<usize>>,
    }
    impl ChildHandler<Context> for Router {
        type Selected = Slot;
        fn select(&mut self, index: usize, _: &Path, _: fs::FileType) -> ChildDecision {
            self.calls.push(format!("select:{index}"));
            if self.skip == Some(index) {
                ChildDecision::Skip
            } else if self.stop == Some(index) {
                ChildDecision::Stop
            } else {
                ChildDecision::Visit
            }
        }
        fn directory(&mut self, index: usize, request: DirectoryRequest<Context>) {
            self.calls.push(format!("directory:{index}"));
            assert!(request.measure);
        }
        fn entry(&mut self, index: usize, _: PathBuf, _: fs::FileType) -> Option<Slot> {
            self.calls.push(format!("entry:{index}"));
            assert_ne!(self.panic_entry, Some(index), "entry callback");
            Some(Slot(self.selected_drops.clone()))
        }
    }
    fn router() -> Router {
        Router {
            calls: vec![],
            skip: None,
            stop: None,
            panic_entry: None,
            selected_drops: Rc::new(Cell::new(0)),
        }
    }
    fn kind(directory: bool) -> fs::FileType {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"));
        fs::symlink_metadata(if directory {
            p.to_path_buf()
        } else {
            p.join("Cargo.toml")
        })
        .unwrap()
        .file_type()
    }
    fn context() -> Context {
        Context {
            clones: Rc::new(Cell::new(0)),
            drops: Rc::new(Cell::new(0)),
        }
    }
    #[test]
    fn child_routing_keeps_original_slots_prefix_and_releases_consumed_plan() {
        let context = context();
        let clones = context.clones.clone();
        let drops = context.drops.clone();
        let mut host = router();
        host.skip = Some(2);
        host.stop = Some(3);
        let plan = ChildTraversal::new(
            vec![
                ("file".into(), kind(false)),
                ("dir".into(), kind(true)),
                ("skip".into(), kind(false)),
                ("stop".into(), kind(false)),
                ("suffix".into(), kind(true)),
            ],
            context,
            true,
        );
        let result = plan.route(&mut host);
        assert!(result.stopped);
        assert_eq!(result.selected.len(), 1);
        assert_eq!(
            host.calls,
            [
                "select:0",
                "entry:0",
                "select:1",
                "directory:1",
                "select:2",
                "select:3"
            ]
        );
        assert_eq!(clones.get(), 1);
        assert_eq!(drops.get(), 2, "request and consumed plan context released");
        assert_eq!(host.selected_drops.get(), 0);
        drop(result);
        assert_eq!(host.selected_drops.get(), 1);
    }
    #[test]
    fn empty_child_routing_has_no_callbacks_or_cancellation_probe() {
        let mut host = router();
        host.stop = Some(0);
        let result = ChildTraversal::new(vec![], context(), true).route(&mut host);
        assert!(!result.stopped);
        assert!(result.selected.is_empty());
        assert!(host.calls.is_empty());
    }
    #[test]
    fn child_callback_panic_releases_selected_prefix_and_plan() {
        let context = context();
        let drops = context.drops.clone();
        let mut host = router();
        host.panic_entry = Some(1);
        let plan = ChildTraversal::new(
            vec![("first".into(), kind(false)), ("panic".into(), kind(false))],
            context,
            true,
        );
        let panic =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| plan.route(&mut host)));
        assert!(panic.is_err());
        assert_eq!(drops.get(), 1);
        assert_eq!(host.selected_drops.get(), 1);
    }
    struct Backend {
        calls: RefCell<Vec<PathBuf>>,
    }
    impl FilesystemBackend for Backend {
        type Entries = std::iter::Empty<io::Result<fs::DirEntry>>;
        fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            self.calls.borrow_mut().push(path.into());
            fs::symlink_metadata(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        }
        fn read_dir(&self, _: &Path) -> io::Result<Self::Entries> {
            panic!("no enumeration in resolution")
        }
    }
    struct Measurements {
        directory: Option<Result<u64, Option<i32>>>,
        entries: Vec<(usize, Result<u64, Option<i32>>)>,
        checkpoints: Vec<usize>,
        finished: usize,
        panic_checkpoint: bool,
        drops: Rc<Cell<usize>>,
    }
    impl<C> MeasurementHandler<C> for Measurements {
        fn directory(&mut self, _: &Path, value: io::Result<MeasuredSize>) {
            self.directory = Some(value.map(|m| m.apparent).map_err(|e| e.raw_os_error()));
        }
        fn checkpoint(&mut self, index: usize) {
            self.checkpoints.push(index);
            assert!(!self.panic_checkpoint, "checkpoint panic");
        }
        fn entry(&mut self, entry: &SelectedEntry<C>, value: io::Result<MeasuredSize>) {
            assert_eq!(
                self.checkpoints.len(),
                self.entries.len() + 1,
                "checkpoint before measurement delivery"
            );
            self.entries.push((
                entry.index,
                value.map(|m| m.apparent).map_err(|e| e.raw_os_error()),
            ));
        }
        fn finish(&mut self) {
            assert_eq!(self.drops.get(), 0, "selected payloads live through finish");
            self.finished += 1;
        }
    }
    fn measurements() -> Measurements {
        Measurements {
            directory: None,
            entries: vec![],
            checkpoints: vec![],
            finished: 0,
            panic_checkpoint: false,
            drops: Rc::new(Cell::new(0)),
        }
    }
    fn backend() -> Backend {
        Backend {
            calls: RefCell::new(vec![]),
        }
    }
    #[test]
    fn resolution_keeps_root_precedence_cached_errors_original_positions_and_missing_fallbacks() {
        let backend = backend();
        let mut host = measurements();
        let root = fs::symlink_metadata(env!("CARGO_MANIFEST_DIR")).unwrap();
        let root_bytes = root.len();
        let entries = [
            (2, "cached"),
            (0, "error"),
            (1, "missing"),
            (9, "out-of-range"),
            (2, "duplicate"),
        ]
        .into_iter()
        .map(|(index, path)| SelectedEntry {
            index,
            path: path.into(),
            context: Slot(host.drops.clone()),
        })
        .collect();
        let plan = SelectedMeasurementPlan {
            path: "literal/../dir".into(),
            include_directory: true,
            root_metadata: Some(root),
            entries,
        };
        plan.resolve(
            &backend,
            Some(Err(io::Error::from_raw_os_error(5))),
            vec![
                Some(Err(io::Error::from_raw_os_error(13))),
                None,
                Some(Ok(MeasuredSize {
                    apparent: 41,
                    allocated: 512,
                })),
            ],
            &mut host,
        );
        assert_eq!(host.directory, Some(Ok(root_bytes)));
        assert_eq!(host.entries[0], (2, Ok(41)));
        assert_eq!(host.entries[1], (0, Err(Some(13))));
        assert_eq!(
            *backend.calls.borrow(),
            [
                PathBuf::from("missing"),
                PathBuf::from("out-of-range"),
                PathBuf::from("duplicate")
            ]
        );
        assert_eq!(host.checkpoints, [0, 1, 2, 3, 4]);
        assert_eq!(host.finished, 1);
        assert_eq!(host.drops.get(), 5);
    }
    #[test]
    fn directory_uses_worker_result_or_one_raw_path_fallback_without_error_retry() {
        for supplied in [0, 1, 2] {
            let backend = backend();
            let mut host = measurements();
            let directory = match supplied {
                0 => None,
                1 => Some(fs::symlink_metadata(env!("CARGO_MANIFEST_DIR"))),
                _ => Some(Err(io::Error::from_raw_os_error(13))),
            };
            SelectedMeasurementPlan::<()> {
                path: "literal/../dir".into(),
                include_directory: true,
                root_metadata: None,
                entries: vec![],
            }
            .resolve(&backend, directory, vec![], &mut host);
            assert_eq!(backend.calls.borrow().len(), usize::from(supplied == 0));
            if supplied == 0 {
                assert_eq!(backend.calls.borrow()[0], Path::new("literal/../dir"));
            }
            if supplied == 2 {
                assert_eq!(host.directory, Some(Err(Some(13))));
            } else {
                assert!(matches!(host.directory, Some(Ok(_))));
            }
            assert_eq!(host.finished, 1);
        }
    }
    #[test]
    fn unselected_directory_and_empty_entries_finish_without_io_or_checkpoint() {
        let backend = backend();
        let mut host = measurements();
        SelectedMeasurementPlan::<()> {
            path: "unselected".into(),
            include_directory: false,
            root_metadata: None,
            entries: vec![],
        }
        .resolve(
            &backend,
            Some(Err(io::Error::from_raw_os_error(13))),
            vec![],
            &mut host,
        );
        assert!(backend.calls.borrow().is_empty());
        assert!(host.directory.is_none());
        assert!(host.checkpoints.is_empty());
        assert_eq!(host.finished, 1);
    }
    #[test]
    fn measurement_callback_panic_releases_payloads_without_fallback_or_finish() {
        let backend = backend();
        let mut host = measurements();
        host.panic_checkpoint = true;
        let plan = SelectedMeasurementPlan {
            path: "dir".into(),
            include_directory: false,
            root_metadata: None,
            entries: vec![SelectedEntry {
                index: 0,
                path: "file".into(),
                context: Slot(host.drops.clone()),
            }],
        };
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            plan.resolve(&backend, None, vec![], &mut host)
        }));
        assert!(panic.is_err());
        assert!(backend.calls.borrow().is_empty());
        assert_eq!(host.finished, 0);
        assert_eq!(host.drops.get(), 1);
    }
}
