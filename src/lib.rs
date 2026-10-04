use std::{
    borrow::Cow,
    ffi::{OsStr, c_char, c_void},
    fs,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

mod policy_table;
mod thread_qos;
pub use policy_table::{
    bh_policy_table_cell_count, bh_policy_table_cell_facts, bh_policy_table_version,
    bh_scan_with_policy_table,
};

#[cfg(unix)]
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};

pub const MARKER_CARGO_TOML: u32 = 1 << 0;
pub const MARKER_PYPROJECT_TOML: u32 = 1 << 1;
pub const MARKER_SETUP_PY: u32 = 1 << 2;
pub const MARKER_SETUP_CFG: u32 = 1 << 3;
pub const MARKER_PYVENV_CFG: u32 = 1 << 4;

/// Stable metadata describing one user-selectable search exclusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchFilterDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub group: &'static str,
    pub node_names: &'static [&'static str],
    pub suffixes: &'static [&'static str],
    pub requires_venv_marker: bool,
}

/// Canonical catalog used by Rust callers, the CLI, and the C bridge.
pub const SEARCH_FILTER_CATALOG: &[SearchFilterDescriptor] = &[
    SearchFilterDescriptor {
        id: "swift.build",
        title: "Swift build output",
        group: "Swift",
        node_names: &[".build"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "rust.target",
        title: "Rust target directory",
        group: "Rust",
        node_names: &["target"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.bytecode",
        title: "Python bytecode",
        group: "Python",
        node_names: &["__pycache__"],
        suffixes: &[".pyc", ".pyo"],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.pytest-cache",
        title: "pytest cache",
        group: "Python",
        node_names: &[".pytest_cache"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.mypy-cache",
        title: "mypy cache",
        group: "Python",
        node_names: &[".mypy_cache"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.ruff-cache",
        title: "Ruff cache",
        group: "Python",
        node_names: &[".ruff_cache"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.pytype-cache",
        title: "pytype cache",
        group: "Python",
        node_names: &[".pytype"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.build",
        title: "Python build directory",
        group: "Python",
        node_names: &["build"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.dist",
        title: "Python distribution directory",
        group: "Python",
        node_names: &["dist"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.egg-info",
        title: "Python package metadata",
        group: "Python",
        node_names: &[],
        suffixes: &[".egg-info"],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.environment",
        title: "Python virtual environment",
        group: "Python",
        node_names: &[".venv", "venv"],
        suffixes: &[],
        requires_venv_marker: true,
    },
    SearchFilterDescriptor {
        id: "python.tox",
        title: "tox environment",
        group: "Python",
        node_names: &[".tox"],
        suffixes: &[],
        requires_venv_marker: false,
    },
    SearchFilterDescriptor {
        id: "python.nox",
        title: "nox environment",
        group: "Python",
        node_names: &[".nox"],
        suffixes: &[],
        requires_venv_marker: false,
    },
];

static FILTER_CATALOG_JSON: OnceLock<Box<[u8]>> = OnceLock::new();

fn filter_catalog_bytes() -> &'static [u8] {
    FILTER_CATALOG_JSON.get_or_init(|| {
        let mut json = String::from("[");
        for (index, descriptor) in SEARCH_FILTER_CATALOG.iter().enumerate() {
            if index != 0 {
                json.push(',');
            }
            json.push_str("{\"id\":");
            json.push_str(&json_string(descriptor.id));
            json.push_str(",\"title\":");
            json.push_str(&json_string(descriptor.title));
            json.push_str(",\"group\":");
            json.push_str(&json_string(descriptor.group));
            json.push_str(",\"nodeNames\":");
            json.push_str(&json_string_array(descriptor.node_names));
            json.push_str(",\"suffixes\":");
            json.push_str(&json_string_array(descriptor.suffixes));
            json.push_str(",\"requiresVenvMarker\":");
            json.push_str(if descriptor.requires_venv_marker {
                "true"
            } else {
                "false"
            });
            json.push('}');
        }
        json.push(']');
        let mut bytes = json.into_bytes();
        bytes.push(0);
        bytes.into_boxed_slice()
    })
}

/// Return the canonical filter catalog as JSON. Its backing storage lives for the process.
pub fn search_filter_catalog_json() -> &'static str {
    let bytes = filter_catalog_bytes();
    let text = &bytes[..bytes.len() - 1];
    // Generated JSON is UTF-8 because all catalog strings are Rust string literals.
    std::str::from_utf8(text).expect("filter catalog JSON is UTF-8")
}

/// C ABI view of [`search_filter_catalog_json`]. The returned pointer is permanent and NUL-terminated.
#[unsafe(no_mangle)]
pub extern "C" fn bh_search_filter_catalog_json() -> *const c_char {
    filter_catalog_bytes().as_ptr().cast()
}

/// Identify the stable catalog ID for a candidate that the existing CLI policy classifies.
pub fn filter_id_for_candidate(
    facts: &CandidateFacts,
    include_environments: bool,
) -> Option<&'static str> {
    let CandidateAction::Classify(classification) = rust_cli_policy(facts, include_environments)
    else {
        return None;
    };
    SEARCH_FILTER_CATALOG
        .iter()
        .find(|descriptor| {
            let group_code = language_code(descriptor.group);
            if classification.language != group_code {
                return false;
            }
            let Some(name) = facts.name() else {
                return false;
            };
            descriptor.node_names.contains(&name)
                || descriptor
                    .suffixes
                    .iter()
                    .any(|suffix| name.ends_with(suffix))
        })
        .map(|descriptor| descriptor.id)
}

fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            ch if ch < ' ' => result.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => result.push(ch),
        }
    }
    result.push('"');
    result
}

fn json_string_array(values: &[&str]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScanOptions {
    pub apparent_size: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateAction {
    Traverse,
    Prune,
    Classify(ArtifactClassificationCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactClassificationCode {
    pub language: u32,
    pub kind: u32,
}

#[derive(Clone, Debug)]
pub struct CandidateFacts {
    pub node_name: Vec<u8>,
    pub is_directory: bool,
    pub is_symbolic_link: bool,
    pub has_artifact_ancestor: bool,
    pub own_marker_files: u32,
    pub parent_marker_files: u32,
}

impl CandidateFacts {
    pub fn name(&self) -> Option<&str> {
        std::str::from_utf8(&self.node_name).ok()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub id: u64,
    pub relative_path: PathBuf,
    pub language: &'static str,
    pub kind: &'static str,
    pub bytes: u64,
    pub nested: bool,
    pub partial: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanWarning {
    pub relative_path: PathBuf,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanStatus {
    Completed,
    Cancelled,
    Incomplete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanEvent {
    ArtifactDiscovered(Artifact),
    ArtifactCompleted { id: u64, bytes: u64, partial: bool },
    Warning(ScanWarning),
    Finished(ScanStatus),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanReport {
    pub artifacts: Vec<Artifact>,
    pub warnings: Vec<ScanWarning>,
    pub status: ScanStatus,
    #[cfg(test)]
    pub probes: ProbeCounts,
}

impl ScanReport {
    pub fn total_bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .filter(|artifact| !artifact.nested)
            .map(|artifact| artifact.bytes)
            .sum()
    }
}

pub struct ScanControl {
    cancelled: AtomicBool,
    failed: AtomicBool,
}

impl ScanControl {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            failed: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn fail(&self) {
        self.failed.store(true, Ordering::Release);
    }

    fn is_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    fn should_stop(&self) -> bool {
        self.is_cancelled() || self.is_failed()
    }
}

impl Default for ScanControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Upper bound on listing threads; directory listing stops scaling past a few cores.
const MAX_SCAN_WORKERS: usize = 8;

/// Scans `root`. Worker threads list directories and read the metadata of measured files;
/// `policy` and `emit` run only on the calling thread, one call at a time.
pub fn scan_with_policy(
    root: &Path,
    options: ScanOptions,
    control: &ScanControl,
    mut policy: impl FnMut(&CandidateFacts) -> CandidateAction,
    mut emit: impl FnMut(&ScanEvent),
) -> ScanReport {
    let probes = Probes::default();
    let queue = WorkQueue::default();
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(MAX_SCAN_WORKERS);
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker_qos = thread_qos::capture();
    let (artifacts, warnings) = std::thread::scope(|scope| {
        // Closing on every exit, including a panicking callback, lets the workers finish.
        let _close = CloseOnDrop(&queue);
        for _ in 0..workers {
            let sender = sender.clone();
            let (queue, probes) = (&queue, &probes);
            scope.spawn(move || {
                thread_qos::apply(worker_qos);
                run_worker(queue, &sender, control, probes)
            });
        }
        drop(sender);
        let mut scanner = Scanner {
            root,
            options,
            control,
            policy: &mut policy,
            emit: &mut emit,
            probes: &probes,
            queue: &queue,
            receiver,
            artifacts: Vec::new(),
            progress: Vec::new(),
            warnings: Vec::new(),
            nodes: Vec::new(),
            awaiting: std::collections::HashMap::new(),
            outstanding: 0,
        };
        scanner.run();
        (scanner.artifacts, scanner.warnings)
    });
    let status = if control.is_failed() {
        ScanStatus::Failed
    } else if control.is_cancelled() {
        ScanStatus::Cancelled
    } else if warnings.is_empty() {
        ScanStatus::Completed
    } else {
        ScanStatus::Incomplete
    };
    emit(&ScanEvent::Finished(status));
    ScanReport {
        artifacts,
        warnings,
        status,
        #[cfg(test)]
        probes: probes.counts(),
    }
}

pub fn rust_cli_policy(facts: &CandidateFacts, include_environments: bool) -> CandidateAction {
    if facts.is_symbolic_link {
        return CandidateAction::Prune;
    }
    let Some(name) = facts.name() else {
        return CandidateAction::Traverse;
    };
    if facts.is_directory && name == ".git" && !facts.has_artifact_ancestor {
        return CandidateAction::Prune;
    }
    if facts.own_marker_files & MARKER_PYVENV_CFG != 0
        && !include_environments
        && !facts.has_artifact_ancestor
    {
        return CandidateAction::Prune;
    }
    let classification = match name {
        ".build" if facts.is_directory => Some(("swift", "build")),
        "target" if facts.is_directory && facts.parent_marker_files & MARKER_CARGO_TOML != 0 => {
            Some(("rust", "build"))
        }
        "__pycache__" if facts.is_directory => Some(("python", "bytecode")),
        ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".pytype" | ".tox" | ".nox"
            if facts.is_directory =>
        {
            Some(("python", "cache"))
        }
        "build" | "dist"
            if facts.is_directory
                && facts.parent_marker_files
                    & (MARKER_PYPROJECT_TOML | MARKER_SETUP_PY | MARKER_SETUP_CFG)
                    != 0 =>
        {
            Some(("python", "build"))
        }
        ".venv" | "venv"
            if facts.is_directory
                && include_environments
                && facts.own_marker_files & MARKER_PYVENV_CFG != 0 =>
        {
            Some(("python", "environment"))
        }
        _ if name.ends_with(".egg-info") => Some(("python", "metadata")),
        _ if !facts.is_directory && (name.ends_with(".pyc") || name.ends_with(".pyo")) => {
            Some(("python", "bytecode"))
        }
        _ => None,
    };
    classification.map_or(CandidateAction::Traverse, |(language, kind)| {
        CandidateAction::Classify(ArtifactClassificationCode {
            language: language_code(language),
            kind: kind_code(kind),
        })
    })
}

fn language_code(language: &str) -> u32 {
    match language {
        "Swift" | "swift" => 1,
        "Rust" | "rust" => 2,
        "Python" | "python" => 3,
        _ => 0,
    }
}

fn kind_code(kind: &str) -> u32 {
    match kind {
        "Build output" | "build" => 1,
        "Cache" | "cache" => 2,
        "Environment" | "environment" => 3,
        "Test environment" | "test-environment" => 4,
        "bytecode" => 5,
        "metadata" => 6,
        _ => 0,
    }
}

fn language_name(code: u32) -> &'static str {
    match code {
        1 => "swift",
        2 => "rust",
        3 => "python",
        _ => "unknown",
    }
}

fn kind_name(code: u32) -> &'static str {
    match code {
        1 => "build",
        2 => "cache",
        3 => "environment",
        4 => "test-environment",
        5 => "bytecode",
        6 => "metadata",
        _ => "unknown",
    }
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    let relative = path.strip_prefix(root).unwrap_or(path);
    if relative.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        relative.to_owned()
    }
}

struct ListTask {
    node: usize,
    path: PathBuf,
    parent_markers: u32,
    /// The directory lies inside an artifact, so its own and its files' sizes are counted.
    measure: bool,
}

enum WorkerMessage {
    /// Sent before any metadata is read, so discovery does not wait for measurement.
    Listed {
        node: usize,
        listing: Listing,
        measuring: bool,
    },
    /// Follows `Listed` when `measuring`; `files` is aligned with the listing's children.
    Measured {
        node: usize,
        directory: std::io::Result<fs::Metadata>,
        files: Vec<Option<std::io::Result<fs::Metadata>>>,
    },
}

#[derive(Default)]
struct WorkQueue {
    state: std::sync::Mutex<QueueState>,
    ready: std::sync::Condvar,
}

#[derive(Default)]
struct QueueState {
    tasks: Vec<ListTask>,
    closed: bool,
}

impl WorkQueue {
    fn push(&self, task: ListTask) {
        self.state.lock().unwrap().tasks.push(task);
        self.ready.notify_one();
    }

    /// Last in, first out: depth-first order completes artifacts early and bounds the queue.
    fn pop(&self) -> Option<ListTask> {
        let mut state = self.state.lock().unwrap();
        loop {
            if state.closed {
                return None;
            }
            if let Some(task) = state.tasks.pop() {
                return Some(task);
            }
            state = self.ready.wait(state).unwrap();
        }
    }

    fn close(&self) {
        self.state.lock().unwrap().closed = true;
        self.ready.notify_all();
    }
}

struct CloseOnDrop<'a>(&'a WorkQueue);

struct CloseOnPanic<'a>(&'a WorkQueue);

impl Drop for CloseOnPanic<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.close();
        }
    }
}

impl Drop for CloseOnDrop<'_> {
    fn drop(&mut self) {
        self.0.close();
    }
}

fn run_worker(
    queue: &WorkQueue,
    results: &std::sync::mpsc::Sender<WorkerMessage>,
    control: &ScanControl,
    probes: &Probes,
) {
    // A panicking worker would never send its pending messages; closing the queue lets the
    // other workers exit, which disconnects the channel and releases the caller.
    let _close = CloseOnPanic(queue);
    while let Some(task) = queue.pop() {
        let listing = list_directory(&task.path, control, probes);
        let name = task.path.file_name().map(os_str_bytes).unwrap_or_default();
        // Speculatively measure likely artifact roots; the coordinator reads anything missed.
        let measuring =
            task.measure || may_classify_directory(&name, listing.markers, task.parent_markers);
        let files: Vec<_> = if measuring {
            listing
                .children
                .iter()
                .map(|(child, file_type)| {
                    (!file_type.is_dir() && !file_type.is_symlink()).then(|| child.clone())
                })
                .collect()
        } else {
            Vec::new()
        };
        let node = task.node;
        if results
            .send(WorkerMessage::Listed {
                node,
                listing,
                measuring,
            })
            .is_err()
        {
            return;
        }
        if measuring {
            let directory = probes.metadata(&task.path);
            let mut measured = Vec::with_capacity(files.len());
            for file in files {
                // A stopped scan needs no more sizes, but the caller may be waiting for this
                // message, so the partial result is still sent.
                if control.should_stop() {
                    break;
                }
                measured.push(file.map(|file| probes.metadata(&file)));
            }
            let files = measured;
            let message = WorkerMessage::Measured {
                node,
                directory,
                files,
            };
            if results.send(message).is_err() {
                return;
            }
        }
    }
}

/// Mirrors the built-in policies closely enough to prefetch sizes; results never depend on it.
fn may_classify_directory(name: &[u8], markers: u32, parent_markers: u32) -> bool {
    if markers & MARKER_PYVENV_CFG != 0 {
        return true;
    }
    match name {
        b".git" => false,
        b"target" => parent_markers & MARKER_CARGO_TOML != 0,
        b"build" | b"dist" => {
            parent_markers & (MARKER_PYPROJECT_TOML | MARKER_SETUP_PY | MARKER_SETUP_CFG) != 0
        }
        _ => is_policy_candidate(name, true, 0),
    }
}

fn list_directory(path: &Path, control: &ScanControl, probes: &Probes) -> Listing {
    let mut listing = Listing::default();
    let entries = match probes.read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            listing.errors.push((path.to_owned(), error));
            return listing;
        }
    };
    for entry in entries {
        if control.should_stop() {
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                listing.errors.push((path.to_owned(), error));
                continue;
            }
        };
        let child = entry.path();
        // Without d_type the type needs a lookup that can fail for the child.
        match entry.file_type() {
            Ok(file_type) => {
                let marker = child.file_name().map_or(0, marker_flag);
                // A marker must be a file; only a symlinked marker needs resolving.
                if marker != 0
                    && (file_type.is_file()
                        || (file_type.is_symlink() && probes.marker_is_file(&child)))
                {
                    listing.markers |= marker;
                }
                listing.children.push((child, file_type));
            }
            Err(error) => listing.errors.push((child, error)),
        }
    }
    listing
}

/// A directory waiting for its listing. `chain` holds the artifacts strictly above it.
struct DirNode {
    path: PathBuf,
    /// `None` only for the selected root, which has no parent listing.
    parent_markers: Option<u32>,
    chain: Vec<u64>,
    root_metadata: Option<fs::Metadata>,
}

/// A listed directory whose sizes are still being read. `chain` includes the directory
/// itself when it is an artifact; `parent_len` is the length of the chain above it.
struct PendingMeasure {
    path: PathBuf,
    chain: Vec<u64>,
    parent_len: usize,
    root_metadata: Option<fs::Metadata>,
    files: Vec<FileSlot>,
}

struct FileSlot {
    index: usize,
    path: PathBuf,
    artifact: Option<u64>,
}

#[derive(Default)]
struct ArtifactProgress {
    bytes: u64,
    /// Directories below the artifact root that are queued but not yet finished.
    pending: usize,
    partial: bool,
    root_finished: bool,
    completed: bool,
}

struct Scanner<'a, P, E> {
    root: &'a Path,
    options: ScanOptions,
    control: &'a ScanControl,
    policy: &'a mut P,
    emit: &'a mut E,
    probes: &'a Probes,
    queue: &'a WorkQueue,
    receiver: std::sync::mpsc::Receiver<WorkerMessage>,
    artifacts: Vec<Artifact>,
    progress: Vec<ArtifactProgress>,
    warnings: Vec<ScanWarning>,
    nodes: Vec<Option<DirNode>>,
    /// `None` marks a speculative measurement nobody needs.
    awaiting: std::collections::HashMap<usize, Option<PendingMeasure>>,
    outstanding: usize,
}

impl<P, E> Scanner<'_, P, E>
where
    P: FnMut(&CandidateFacts) -> CandidateAction,
    E: FnMut(&ScanEvent),
{
    fn emit(&mut self, event: &ScanEvent) {
        (self.emit)(event);
    }

    fn warn(&mut self, path: &Path, error: impl std::fmt::Display, chain: &[u64]) {
        let warning = ScanWarning {
            relative_path: relative_path(self.root, path),
            message: error.to_string(),
        };
        self.warnings.push(warning.clone());
        self.emit(&ScanEvent::Warning(warning));
        for id in chain {
            self.progress[Self::index(*id)].partial = true;
        }
    }

    fn index(id: u64) -> usize {
        usize::try_from(id - 1).expect("artifact IDs fit in memory")
    }

    fn run(&mut self) {
        let root = self.root;
        match self.probes.metadata(root) {
            Err(error) => self.warn(root, error, &[]),
            Ok(metadata) if metadata.is_dir() => {
                self.enqueue(
                    DirNode {
                        path: root.to_owned(),
                        parent_markers: None,
                        chain: Vec::new(),
                        root_metadata: Some(metadata),
                    },
                    0,
                );
            }
            Ok(metadata) => {
                // A selected file or symlink is its own candidate, measured if classified.
                let file_type = metadata.file_type();
                if let Some(slot) = self.visit_file(root.to_owned(), file_type, 0, &[]) {
                    self.measure_file(&slot, Some(Ok(metadata)), &[]);
                }
            }
        }
        while self.outstanding > 0 && !self.control.should_stop() {
            let Ok(message) = self.receiver.recv() else {
                break;
            };
            self.outstanding -= 1;
            // Messages after a stop may be partial; open artifacts are completed below.
            if self.control.should_stop() {
                break;
            }
            match message {
                WorkerMessage::Listed {
                    node,
                    listing,
                    measuring,
                } => self.process_listing(node, listing, measuring),
                WorkerMessage::Measured {
                    node,
                    directory,
                    files,
                } => {
                    if let Some(Some(pending)) = self.awaiting.remove(&node) {
                        self.finish_measure(pending, Some(directory), files);
                    }
                }
            }
        }
        // Stopping leaves open artifacts; report them innermost first, as partial.
        for index in (0..self.progress.len()).rev() {
            if !self.progress[index].completed {
                self.progress[index].partial = true;
                self.complete(index as u64 + 1);
            }
        }
    }

    fn enqueue(&mut self, node: DirNode, parent_markers: u32) {
        let task = ListTask {
            node: self.nodes.len(),
            path: node.path.clone(),
            parent_markers,
            measure: !node.chain.is_empty(),
        };
        self.nodes.push(Some(node));
        self.outstanding += 1;
        self.queue.push(task);
    }

    fn discover(&mut self, path: &Path, nested: bool, language: u32, kind: u32) -> u64 {
        let id = self.artifacts.len() as u64 + 1;
        let artifact = Artifact {
            id,
            relative_path: relative_path(self.root, path),
            language: language_name(language),
            kind: kind_name(kind),
            bytes: 0,
            nested,
            partial: false,
        };
        self.emit(&ScanEvent::ArtifactDiscovered(artifact.clone()));
        self.artifacts.push(artifact);
        self.progress.push(ArtifactProgress::default());
        id
    }

    fn complete(&mut self, id: u64) {
        let index = Self::index(id);
        let progress = &mut self.progress[index];
        progress.completed = true;
        let bytes = progress.bytes;
        let partial = progress.partial || self.control.should_stop();
        self.artifacts[index].bytes = bytes;
        self.artifacts[index].partial = partial;
        self.emit(&ScanEvent::ArtifactCompleted { id, bytes, partial });
    }

    /// Runs the policy for a non-directory entry. Returns a slot when its size is needed.
    fn visit_file(
        &mut self,
        path: PathBuf,
        file_type: fs::FileType,
        index: usize,
        chain: &[u64],
    ) -> Option<FileSlot> {
        let is_symlink = file_type.is_symlink();
        let name = path.file_name().map(os_str_bytes).unwrap_or_default();
        let action = if is_symlink || is_policy_candidate(&name, false, 0) {
            (self.policy)(&CandidateFacts {
                node_name: name.into_owned(),
                is_directory: false,
                is_symbolic_link: is_symlink,
                has_artifact_ancestor: !chain.is_empty(),
                own_marker_files: 0,
                parent_marker_files: 0,
            })
        } else {
            CandidateAction::Traverse
        };
        if is_symlink || action == CandidateAction::Prune || self.control.should_stop() {
            return None;
        }
        let artifact = match action {
            CandidateAction::Classify(code) => {
                Some(self.discover(&path, !chain.is_empty(), code.language, code.kind))
            }
            CandidateAction::Traverse | CandidateAction::Prune => None,
        };
        (!chain.is_empty() || artifact.is_some()).then_some(FileSlot {
            index,
            path,
            artifact,
        })
    }

    /// Adds a file's size to `chain` and completes the file's own artifact, if any.
    fn measure_file(
        &mut self,
        slot: &FileSlot,
        metadata: Option<std::io::Result<fs::Metadata>>,
        chain: &[u64],
    ) {
        let metadata = metadata.unwrap_or_else(|| self.probes.metadata(&slot.path));
        let bytes = match metadata {
            Ok(metadata) => metadata_bytes(&metadata, self.options.apparent_size),
            Err(error) => {
                let mut affected = chain.to_vec();
                affected.extend(slot.artifact);
                self.warn(&slot.path, error, &affected);
                0
            }
        };
        for id in chain {
            let progress = &mut self.progress[Self::index(*id)];
            progress.bytes = progress.bytes.saturating_add(bytes);
        }
        if let Some(id) = slot.artifact {
            self.progress[Self::index(id)].bytes = bytes;
            self.complete(id);
        }
    }

    fn process_listing(&mut self, node: usize, listing: Listing, measuring: bool) {
        if measuring {
            self.outstanding += 1;
        }
        let DirNode {
            path,
            parent_markers,
            mut chain,
            root_metadata,
        } = self.nodes[node]
            .take()
            .expect("each directory is listed once");
        let parent_len = chain.len();
        let name = path.file_name().map(os_str_bytes).unwrap_or_default();
        let own_markers = listing.markers & MARKER_PYVENV_CFG;
        let parent_markers = if matches!(name.as_ref(), b"target" | b"build" | b"dist") {
            parent_markers.unwrap_or_else(|| {
                path.parent()
                    .map(|parent| marker_flags(parent, self.probes))
                    .unwrap_or_default()
            }) & !MARKER_PYVENV_CFG
        } else {
            0
        };
        let action = if is_policy_candidate(&name, true, own_markers) {
            (self.policy)(&CandidateFacts {
                node_name: name.into_owned(),
                is_directory: true,
                is_symbolic_link: false,
                has_artifact_ancestor: parent_len > 0,
                own_marker_files: own_markers,
                parent_marker_files: parent_markers,
            })
        } else {
            CandidateAction::Traverse
        };
        if self.control.should_stop() {
            return;
        }
        if action == CandidateAction::Prune {
            if measuring {
                self.awaiting.insert(node, None);
            }
            self.finish_directory(&chain, parent_len);
            return;
        }
        if let CandidateAction::Classify(code) = action {
            let id = self.discover(&path, parent_len > 0, code.language, code.kind);
            chain.push(id);
        }
        for (error_path, error) in listing.errors {
            self.warn(&error_path, error, &chain);
        }
        let mut files = Vec::new();
        for (index, (child, file_type)) in listing.children.into_iter().enumerate() {
            if self.control.should_stop() {
                return;
            }
            if file_type.is_dir() {
                for id in &chain {
                    self.progress[Self::index(*id)].pending += 1;
                }
                let child_node = DirNode {
                    path: child,
                    parent_markers: Some(listing.markers),
                    chain: chain.clone(),
                    root_metadata: None,
                };
                self.enqueue(child_node, listing.markers);
            } else if let Some(slot) = self.visit_file(child, file_type, index, &chain) {
                files.push(slot);
            }
        }
        let pending = PendingMeasure {
            path,
            chain,
            parent_len,
            root_metadata,
            files,
        };
        if pending.chain.is_empty() && pending.files.is_empty() {
            if measuring {
                self.awaiting.insert(node, None);
            }
            self.finish_directory(&pending.chain, parent_len);
        } else if measuring {
            self.awaiting.insert(node, Some(pending));
        } else {
            self.finish_measure(pending, None, Vec::new());
        }
    }

    /// Sizes from a worker are used when present; anything missing is read here.
    fn finish_measure(
        &mut self,
        pending: PendingMeasure,
        directory: Option<std::io::Result<fs::Metadata>>,
        mut files: Vec<Option<std::io::Result<fs::Metadata>>>,
    ) {
        let PendingMeasure {
            path,
            chain,
            parent_len,
            root_metadata,
            files: slots,
        } = pending;
        if !chain.is_empty() {
            let metadata = match root_metadata {
                Some(metadata) => Ok(metadata),
                None => directory.unwrap_or_else(|| self.probes.metadata(&path)),
            };
            match metadata {
                Ok(metadata) => {
                    let bytes = metadata_bytes(&metadata, self.options.apparent_size);
                    for id in &chain {
                        let progress = &mut self.progress[Self::index(*id)];
                        progress.bytes = progress.bytes.saturating_add(bytes);
                    }
                }
                Err(error) => self.warn(&path, error, &chain),
            }
        }
        for slot in &slots {
            let metadata = files.get_mut(slot.index).and_then(Option::take);
            self.measure_file(slot, metadata, &chain);
        }
        self.finish_directory(&chain, parent_len);
    }

    /// Marks a directory finished in every artifact above it and completes any artifact
    /// whose subtree has no queued directories left, innermost first.
    fn finish_directory(&mut self, chain: &[u64], parent_len: usize) {
        for id in &chain[..parent_len] {
            self.progress[Self::index(*id)].pending -= 1;
        }
        if let Some(id) = chain.get(parent_len) {
            self.progress[Self::index(*id)].root_finished = true;
        }
        for id in chain.iter().rev() {
            let progress = &self.progress[Self::index(*id)];
            if progress.root_finished && progress.pending == 0 && !progress.completed {
                self.complete(*id);
            }
        }
    }
}

#[derive(Default)]
struct Listing {
    children: Vec<(PathBuf, fs::FileType)>,
    markers: u32,
    /// Each error names the path it concerns: the listed directory or one child.
    errors: Vec<(PathBuf, std::io::Error)>,
}

fn marker_flag(name: &OsStr) -> u32 {
    match os_str_bytes(name).as_ref() {
        b"Cargo.toml" => MARKER_CARGO_TOML,
        b"pyproject.toml" => MARKER_PYPROJECT_TOML,
        b"setup.py" => MARKER_SETUP_PY,
        b"setup.cfg" => MARKER_SETUP_CFG,
        b"pyvenv.cfg" => MARKER_PYVENV_CFG,
        _ => 0,
    }
}

/// Filesystem probes. Tests read per-scan counts, which also cover worker threads.
#[derive(Default)]
struct Probes {
    #[cfg(test)]
    metadata: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    marker_probes: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    listing_threads: std::sync::Mutex<Vec<std::thread::ThreadId>>,
}

impl Probes {
    fn metadata(&self, path: &Path) -> std::io::Result<fs::Metadata> {
        #[cfg(test)]
        self.metadata.fetch_add(1, Ordering::Relaxed);
        fs::symlink_metadata(path)
    }

    fn marker_is_file(&self, path: &Path) -> bool {
        #[cfg(test)]
        self.marker_probes.fetch_add(1, Ordering::Relaxed);
        path.is_file()
    }

    fn read_dir(&self, path: &Path) -> std::io::Result<fs::ReadDir> {
        #[cfg(test)]
        {
            let thread = std::thread::current().id();
            let mut threads = self.listing_threads.lock().unwrap();
            if !threads.contains(&thread) {
                threads.push(thread);
            }
        }
        fs::read_dir(path)
    }

    #[cfg(test)]
    fn counts(&self) -> ProbeCounts {
        ProbeCounts {
            metadata: self.metadata.load(Ordering::Relaxed),
            marker_probes: self.marker_probes.load(Ordering::Relaxed),
            listing_threads: self.listing_threads.lock().unwrap().clone(),
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProbeCounts {
    pub metadata: usize,
    pub marker_probes: usize,
    pub listing_threads: Vec<std::thread::ThreadId>,
}

fn os_str_bytes(value: &OsStr) -> Cow<'_, [u8]> {
    #[cfg(unix)]
    {
        Cow::Borrowed(value.as_bytes())
    }
    #[cfg(not(unix))]
    {
        match value.to_string_lossy() {
            Cow::Borrowed(text) => Cow::Borrowed(text.as_bytes()),
            Cow::Owned(text) => Cow::Owned(text.into_bytes()),
        }
    }
}

fn marker_flags(path: &Path, probes: &Probes) -> u32 {
    let mut flags = 0;
    if probes.marker_is_file(&path.join("Cargo.toml")) {
        flags |= MARKER_CARGO_TOML;
    }
    if probes.marker_is_file(&path.join("pyproject.toml")) {
        flags |= MARKER_PYPROJECT_TOML;
    }
    if probes.marker_is_file(&path.join("setup.py")) {
        flags |= MARKER_SETUP_PY;
    }
    if probes.marker_is_file(&path.join("setup.cfg")) {
        flags |= MARKER_SETUP_CFG;
    }
    flags
}

fn is_policy_candidate(name: &[u8], is_directory: bool, own_markers: u32) -> bool {
    if own_markers & MARKER_PYVENV_CFG != 0 {
        return true;
    }
    if !is_directory {
        return name.ends_with(b".pyc") || name.ends_with(b".pyo") || name.ends_with(b".egg-info");
    }
    matches!(
        name,
        b".git"
            | b".build"
            | b"target"
            | b"build"
            | b"dist"
            | b"__pycache__"
            | b".pytest_cache"
            | b".mypy_cache"
            | b".ruff_cache"
            | b".pytype"
            | b".tox"
            | b".nox"
            | b".venv"
            | b"venv"
    ) || name.ends_with(b".egg-info")
}

fn metadata_bytes(metadata: &fs::Metadata, apparent_size: bool) -> u64 {
    #[cfg(unix)]
    {
        if apparent_size {
            metadata.len()
        } else {
            metadata.blocks().saturating_mul(512)
        }
    }
    #[cfg(not(unix))]
    {
        let _ = apparent_size;
        metadata.len()
    }
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[repr(C)]
pub struct BHCandidateFacts {
    pub node_name: *const u8,
    pub node_name_len: usize,
    pub is_directory: u8,
    pub is_symbolic_link: u8,
    pub has_artifact_ancestor: u8,
    pub own_marker_files: u32,
    pub parent_marker_files: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BHCandidateDecision {
    pub action: u32,
    pub language: u32,
    pub kind: u32,
}

#[repr(C)]
pub struct BHScanEvent {
    pub event_type: u32,
    pub artifact_id: u64,
    pub path: *const u8,
    pub path_len: usize,
    pub language: u32,
    pub kind: u32,
    pub bytes: u64,
    pub partial: u8,
    pub status: u32,
    pub message: *const u8,
    pub message_len: usize,
}

pub type BHPolicyCallback = unsafe extern "C" fn(
    context: *mut c_void,
    facts: *const BHCandidateFacts,
    decision: *mut BHCandidateDecision,
) -> i32;
pub type BHEventCallback = unsafe extern "C" fn(context: *mut c_void, event: *const BHScanEvent);

#[unsafe(no_mangle)]
pub extern "C" fn bh_scan_control_create() -> *mut ScanControl {
    Box::into_raw(Box::new(ScanControl::new()))
}

/// # Safety
/// `control` must be a live pointer returned by `bh_scan_control_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_scan_control_cancel(control: *mut ScanControl) {
    if let Some(control) = unsafe { control.as_ref() } {
        control.cancel();
    }
}

/// # Safety
/// `control` must be a live pointer returned by `bh_scan_control_create`, and no scan may use it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_scan_control_destroy(control: *mut ScanControl) {
    if !control.is_null() {
        drop(unsafe { Box::from_raw(control) });
    }
}

/// Run a synchronous scan. The callbacks run on the calling thread and all pointed-to payloads
/// are valid only until the callback returns. Destroy `control` only after this function returns.
///
/// # Safety
/// All pointers must remain valid for the duration of the call; callbacks must obey their C ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_scan(
    control: *mut ScanControl,
    root: *const u8,
    root_len: usize,
    apparent_size: u8,
    policy_callback: Option<BHPolicyCallback>,
    event_callback: Option<BHEventCallback>,
    context: *mut c_void,
) -> i32 {
    if control.is_null() || root.is_null() || policy_callback.is_none() || event_callback.is_none()
    {
        return 3;
    }
    let policy_callback = policy_callback.unwrap();
    let mut policy_failed = false;
    unsafe {
        run_ffi_scan(
            control,
            root,
            root_len,
            apparent_size,
            event_callback.unwrap(),
            context,
            |facts| {
                let mut decision = BHCandidateDecision::default();
                let c_facts = BHCandidateFacts {
                    node_name: facts.node_name.as_ptr(),
                    node_name_len: facts.node_name.len(),
                    is_directory: u8::from(facts.is_directory),
                    is_symbolic_link: u8::from(facts.is_symbolic_link),
                    has_artifact_ancestor: u8::from(facts.has_artifact_ancestor),
                    own_marker_files: facts.own_marker_files,
                    parent_marker_files: facts.parent_marker_files,
                };
                let succeeded = policy_callback(context, &c_facts, &mut decision);
                if succeeded == 0 {
                    policy_failed = true;
                    return None;
                }
                policy_table::action_from_decision(decision).or_else(|| {
                    policy_failed = true;
                    None
                })
            },
        )
    }
    .map_or(3, |status| if policy_failed { 3 } else { status })
}

/// Run the common scan and preserve the callback's historical failure and panic handling.
unsafe fn run_ffi_scan(
    control: *mut ScanControl,
    root: *const u8,
    root_len: usize,
    apparent_size: u8,
    event_callback: BHEventCallback,
    context: *mut c_void,
    mut policy: impl FnMut(&CandidateFacts) -> Option<CandidateAction>,
) -> Option<i32> {
    if control.is_null() || root.is_null() {
        return None;
    }
    let root_bytes = unsafe { std::slice::from_raw_parts(root, root_len) };
    let root_path = path_from_bytes(root_bytes);
    let control = unsafe { &*control };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let report = scan_with_policy(
            &root_path,
            ScanOptions {
                apparent_size: apparent_size != 0,
            },
            control,
            |facts| match policy(facts) {
                Some(action) => action,
                None => {
                    control.fail();
                    CandidateAction::Prune
                }
            },
            |event| unsafe { send_ffi_event(event_callback, context, event) },
        );
        status_code(report.status)
    }));
    match result {
        Ok(status) => Some(status),
        Err(_) => {
            let event = BHScanEvent {
                event_type: 4,
                artifact_id: 0,
                path: std::ptr::null(),
                path_len: 0,
                language: 0,
                kind: 0,
                bytes: 0,
                partial: 0,
                status: 3,
                message: std::ptr::null(),
                message_len: 0,
            };
            unsafe { event_callback(context, &event) };
            Some(3)
        }
    }
}

unsafe fn send_ffi_event(callback: BHEventCallback, context: *mut c_void, event: &ScanEvent) {
    let empty: [u8; 0] = [];
    let (ffi, _path, _message) = match event {
        ScanEvent::ArtifactDiscovered(artifact) => {
            let path = path_bytes(&artifact.relative_path);
            let ffi = BHScanEvent {
                event_type: 1,
                artifact_id: artifact.id,
                path: path.as_ptr(),
                path_len: path.len(),
                language: language_code(artifact.language),
                kind: kind_code(artifact.kind),
                bytes: 0,
                partial: 0,
                status: 0,
                message: empty.as_ptr(),
                message_len: 0,
            };
            (ffi, path, Vec::new())
        }
        ScanEvent::ArtifactCompleted { id, bytes, partial } => (
            BHScanEvent {
                event_type: 2,
                artifact_id: *id,
                path: empty.as_ptr(),
                path_len: 0,
                language: 0,
                kind: 0,
                bytes: *bytes,
                partial: u8::from(*partial),
                status: 0,
                message: empty.as_ptr(),
                message_len: 0,
            },
            Vec::new(),
            Vec::new(),
        ),
        ScanEvent::Warning(warning) => {
            let path = path_bytes(&warning.relative_path);
            let message = warning.message.as_bytes().to_vec();
            let ffi = BHScanEvent {
                event_type: 3,
                artifact_id: 0,
                path: path.as_ptr(),
                path_len: path.len(),
                language: 0,
                kind: 0,
                bytes: 0,
                partial: 0,
                status: 0,
                message: message.as_ptr(),
                message_len: message.len(),
            };
            (ffi, path, message)
        }
        ScanEvent::Finished(status) => (
            BHScanEvent {
                event_type: 4,
                artifact_id: 0,
                path: empty.as_ptr(),
                path_len: 0,
                language: 0,
                kind: 0,
                bytes: 0,
                partial: 0,
                status: status_code(*status) as u32,
                message: empty.as_ptr(),
                message_len: 0,
            },
            Vec::new(),
            Vec::new(),
        ),
    };
    // C callbacks borrow these payloads only until they return, so keep both buffers alive.
    unsafe { callback(context, &ffi) };
}

fn path_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        path.as_os_str().to_string_lossy().as_bytes().to_vec()
    }
}

fn status_code(status: ScanStatus) -> i32 {
    match status {
        ScanStatus::Completed => 0,
        ScanStatus::Cancelled => 1,
        ScanStatus::Incomplete => 2,
        ScanStatus::Failed => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicBool, AtomicU64, Ordering},
    };

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            loop {
                let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("build-hunter-core-{}-{id}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create test tree {}: {error}", path.display()),
                }
            }
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn temporary_trees_have_independent_paths_and_cleanup() {
        let barrier = std::sync::Barrier::new(32);
        let mut trees: Vec<TempTree> = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..32)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        (0..16).map(|_| TempTree::new()).collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().unwrap())
                .collect()
        });
        let paths: std::collections::HashSet<_> = trees.iter().map(|tree| &tree.0).collect();
        assert_eq!(
            paths.len(),
            trees.len(),
            "live fixtures must never share a path"
        );

        let removed = trees.pop().unwrap();
        let removed_path = removed.0.clone();
        drop(removed);
        assert!(!removed_path.exists());
        assert!(trees.iter().all(|tree| tree.0.is_dir()));
    }

    #[test]
    fn swift_policy_events_measure_outer_roots_and_keep_path_bytes() {
        let tree = TempTree::new();
        fs::create_dir_all(tree.0.join("Package/.build/__pycache__")).unwrap();
        fs::write(
            tree.0.join("Package/.build/__pycache__/module.pyc"),
            b"123456",
        )
        .unwrap();
        fs::create_dir_all(tree.0.join("Nested/target")).unwrap();
        fs::write(tree.0.join("Nested/Cargo.toml"), "").unwrap();
        let control = ScanControl::new();
        let mut candidates = Vec::new();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &control,
            |facts| {
                candidates.push((
                    facts.name().unwrap_or_default().to_owned(),
                    facts.has_artifact_ancestor,
                ));
                if facts.has_artifact_ancestor {
                    return CandidateAction::Traverse;
                }
                match facts.name() {
                    Some(".build") => CandidateAction::Classify(ArtifactClassificationCode {
                        language: 1,
                        kind: 1,
                    }),
                    Some("target") if facts.parent_marker_files & MARKER_CARGO_TOML != 0 => {
                        CandidateAction::Classify(ArtifactClassificationCode {
                            language: 2,
                            kind: 1,
                        })
                    }
                    Some(".git") => CandidateAction::Prune,
                    _ => CandidateAction::Traverse,
                }
            },
            |_| {},
        );
        assert_eq!(report.artifacts.len(), 2);
        assert!(report.artifacts.iter().all(|artifact| !artifact.nested));
        assert_eq!(
            report.total_bytes(),
            report.artifacts.iter().map(|artifact| artifact.bytes).sum()
        );
        assert!(
            candidates
                .iter()
                .any(|(name, nested)| name == "__pycache__" && *nested)
        );
        assert!(
            report
                .artifacts
                .iter()
                .any(|artifact| artifact.relative_path == Path::new("Package/.build"))
        );
        assert_eq!(report.status, ScanStatus::Completed);
    }

    #[test]
    fn selected_artifact_root_uses_dot_as_its_relative_path() {
        let tree = TempTree::new();
        let selected_root = tree.0.join(".build");
        fs::create_dir_all(&selected_root).unwrap();
        let report = scan_with_policy(
            &selected_root,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| match facts.name() {
                Some(".build") => CandidateAction::Classify(ArtifactClassificationCode {
                    language: 1,
                    kind: 1,
                }),
                _ => CandidateAction::Traverse,
            },
            |_| {},
        );

        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(report.artifacts[0].relative_path, Path::new("."));
    }

    #[test]
    fn cancellation_emits_a_partial_completion_and_terminal_state() {
        let tree = TempTree::new();
        fs::create_dir_all(tree.0.join(".build")).unwrap();
        for index in 0..16 {
            fs::write(tree.0.join(format!(".build/{index}")), vec![0; 1024]).unwrap();
        }
        let control = ScanControl::new();
        let cancelled = AtomicBool::new(false);
        let mut completion_was_partial = false;
        let report = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &control,
            |facts| match facts.name() {
                Some(".build") if !facts.has_artifact_ancestor => {
                    CandidateAction::Classify(ArtifactClassificationCode {
                        language: 1,
                        kind: 1,
                    })
                }
                _ => CandidateAction::Traverse,
            },
            |event| match event {
                ScanEvent::ArtifactDiscovered(_) if !cancelled.swap(true, Ordering::SeqCst) => {
                    control.cancel()
                }
                ScanEvent::ArtifactCompleted { partial, .. } => completion_was_partial = *partial,
                _ => {}
            },
        );
        assert!(completion_was_partial);
        assert_eq!(report.status, ScanStatus::Cancelled);
    }

    #[derive(Default)]
    struct FfiObservations {
        policy_calls: usize,
        completion_partial: Option<bool>,
        terminal_status: Option<u32>,
    }

    unsafe extern "C" fn reject_policy(
        context: *mut c_void,
        _facts: *const BHCandidateFacts,
        _decision: *mut BHCandidateDecision,
    ) -> i32 {
        // SAFETY: run_ffi keeps this exclusively borrowed context alive until bh_scan returns.
        let observations = unsafe { &mut *context.cast::<FfiObservations>() };
        observations.policy_calls += 1;
        0
    }

    unsafe extern "C" fn reject_nested_policy(
        context: *mut c_void,
        facts: *const BHCandidateFacts,
        decision: *mut BHCandidateDecision,
    ) -> i32 {
        // SAFETY: bh_scan supplies borrowed facts/decision and run_ffi owns the context.
        let observations = unsafe { &mut *context.cast::<FfiObservations>() };
        observations.policy_calls += 1;
        if unsafe { (*facts).has_artifact_ancestor } != 0 {
            return 0;
        }
        unsafe {
            *decision = BHCandidateDecision {
                action: 2,
                language: 1,
                kind: 1,
            };
        }
        1
    }

    unsafe extern "C" fn observe_ffi_event(context: *mut c_void, event: *const BHScanEvent) {
        // SAFETY: these pointers are valid for this synchronous callback; no payload escapes it.
        let observations = unsafe { &mut *context.cast::<FfiObservations>() };
        let event = unsafe { &*event };
        match event.event_type {
            2 => observations.completion_partial = Some(event.partial != 0),
            4 => observations.terminal_status = Some(event.status),
            _ => {}
        }
    }

    fn run_ffi(root: &Path, policy: BHPolicyCallback) -> (i32, FfiObservations) {
        let control = bh_scan_control_create();
        let root_bytes = path_bytes(root);
        let mut observations = FfiObservations::default();
        // SAFETY: control, path bytes and context outlive the synchronous scan; callbacks borrow
        // them only on this thread. The handle is destroyed after the scan and callbacks finish.
        let status = unsafe {
            let status = bh_scan(
                control,
                root_bytes.as_ptr(),
                root_bytes.len(),
                1,
                Some(policy),
                Some(observe_ffi_event),
                (&mut observations as *mut FfiObservations).cast(),
            );
            bh_scan_control_destroy(control);
            status
        };
        (status, observations)
    }

    #[test]
    fn ffi_policy_failure_stops_before_visiting_other_candidates() {
        let tree = TempTree::new();
        for index in 0..8 {
            fs::create_dir_all(tree.0.join(format!("Package{index}/.build"))).unwrap();
        }

        let (status, observations) = run_ffi(&tree.0, reject_policy);

        assert_eq!(status, 3);
        assert_eq!(observations.terminal_status, Some(3));
        assert_eq!(observations.policy_calls, 1);
    }

    #[test]
    fn ffi_policy_failure_marks_an_open_artifact_measurement_partial() {
        let tree = TempTree::new();
        let root = tree.0.join(".build");
        fs::create_dir_all(root.join("__pycache__")).unwrap();

        let (status, observations) = run_ffi(&root, reject_nested_policy);

        assert_eq!(status, 3);
        assert_eq!(observations.terminal_status, Some(3));
        assert_eq!(observations.completion_partial, Some(true));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn artifact_relative_paths_keep_non_utf8_bytes_losslessly() {
        let tree = TempTree::new();
        let invalid_parent = tree.0.join(OsStr::from_bytes(b"invalid-\xff"));
        let build_root = invalid_parent.join(".build");
        fs::create_dir_all(&build_root).unwrap();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| match facts.name() {
                Some(".build") => CandidateAction::Classify(ArtifactClassificationCode {
                    language: 1,
                    kind: 1,
                }),
                _ => CandidateAction::Traverse,
            },
            |_| {},
        );
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(
            report.artifacts[0].relative_path.as_os_str().as_bytes(),
            b"invalid-\xff/.build"
        );
    }

    #[test]
    fn rust_cli_profile_preserves_environment_opt_in_and_marker_rules() {
        let mut facts = CandidateFacts {
            node_name: b".venv".to_vec(),
            is_directory: true,
            is_symbolic_link: false,
            has_artifact_ancestor: false,
            own_marker_files: MARKER_PYVENV_CFG,
            parent_marker_files: 0,
        };
        assert_eq!(rust_cli_policy(&facts, false), CandidateAction::Prune);
        assert!(matches!(
            rust_cli_policy(&facts, true),
            CandidateAction::Classify(_)
        ));
        facts.node_name = b"target".to_vec();
        facts.own_marker_files = 0;
        assert_eq!(rust_cli_policy(&facts, false), CandidateAction::Traverse);
        facts.parent_marker_files = MARKER_CARGO_TOML;
        assert!(matches!(
            rust_cli_policy(&facts, false),
            CandidateAction::Classify(_)
        ));
    }

    fn facts(name: &str, is_directory: bool, parent_marker_files: u32) -> CandidateFacts {
        CandidateFacts {
            node_name: name.as_bytes().to_vec(),
            is_directory,
            is_symbolic_link: false,
            has_artifact_ancestor: false,
            own_marker_files: 0,
            parent_marker_files,
        }
    }

    #[test]
    fn filter_catalog_is_generated_from_public_descriptors_and_has_stable_ffi_storage() {
        let json = search_filter_catalog_json();
        assert!(json.starts_with("[{\"id\":\"swift.build\""));
        assert!(json.contains("\"id\":\"python.bytecode\",\"title\":\"Python bytecode\",\"group\":\"Python\",\"nodeNames\":[\"__pycache__\"],\"suffixes\":[\".pyc\",\".pyo\"],\"requiresVenvMarker\":false"));
        assert!(json.contains("\"id\":\"python.environment\""));
        let pointer = bh_search_filter_catalog_json();
        assert_eq!(pointer, bh_search_filter_catalog_json());
        // SAFETY: the exported catalog pointer is documented to remain valid for this process.
        let ffi_json = unsafe { std::ffi::CStr::from_ptr(pointer) }
            .to_str()
            .unwrap();
        assert_eq!(ffi_json, json);
        assert_eq!(SEARCH_FILTER_CATALOG.len(), 13);
    }

    #[test]
    fn filter_ids_are_individual_and_keep_existing_classification_eligibility() {
        assert_eq!(
            filter_id_for_candidate(&facts(".build", true, 0), false),
            Some("swift.build")
        );
        assert_eq!(
            filter_id_for_candidate(&facts("target", true, MARKER_CARGO_TOML), false),
            Some("rust.target")
        );
        assert_eq!(
            filter_id_for_candidate(&facts("target", true, 0), false),
            None
        );
        assert_eq!(
            filter_id_for_candidate(&facts(".pytest_cache", true, 0), false),
            Some("python.pytest-cache")
        );
        assert_eq!(
            filter_id_for_candidate(&facts(".mypy_cache", true, 0), false),
            Some("python.mypy-cache")
        );
        assert_eq!(
            filter_id_for_candidate(&facts(".ruff_cache", true, 0), false),
            Some("python.ruff-cache")
        );
        assert_eq!(
            filter_id_for_candidate(&facts(".pytype", true, 0), false),
            Some("python.pytype-cache")
        );
        assert_eq!(
            filter_id_for_candidate(&facts(".tox", true, 0), false),
            Some("python.tox")
        );
        assert_eq!(
            filter_id_for_candidate(&facts(".nox", true, 0), false),
            Some("python.nox")
        );
        assert_eq!(
            filter_id_for_candidate(&facts("wheel.egg-info", true, 0), false),
            Some("python.egg-info")
        );
        assert_eq!(
            filter_id_for_candidate(&facts("module.pyc", false, 0), false),
            Some("python.bytecode")
        );
        assert_eq!(
            filter_id_for_candidate(&facts("build", true, MARKER_PYPROJECT_TOML), false),
            Some("python.build")
        );
        assert_eq!(
            filter_id_for_candidate(&facts("dist", true, MARKER_PYPROJECT_TOML), false),
            Some("python.dist")
        );

        let mut environment = facts(".venv", true, 0);
        environment.own_marker_files = MARKER_PYVENV_CFG;
        assert_eq!(filter_id_for_candidate(&environment, false), None);
        assert_eq!(
            filter_id_for_candidate(&environment, true),
            Some("python.environment")
        );
    }

    #[test]
    fn directories_are_listed_off_the_calling_thread_and_callbacks_stay_on_it() {
        let tree = TempTree::new();
        for project in 0..32 {
            fs::create_dir_all(tree.0.join(format!("project-{project}/.build/debug"))).unwrap();
            fs::write(
                tree.0.join(format!("project-{project}/.build/debug/o")),
                b"o",
            )
            .unwrap();
            fs::create_dir_all(tree.0.join(format!("project-{project}/Sources/App"))).unwrap();
        }
        let caller = std::thread::current().id();
        let mut callback_threads = std::collections::HashSet::new();
        let mut events = Vec::new();

        let report = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| {
                callback_threads.insert(std::thread::current().id());
                rust_cli_policy(facts, false)
            },
            |event| {
                assert_eq!(std::thread::current().id(), caller);
                events.push(event.clone());
            },
        );

        assert_eq!(callback_threads, std::collections::HashSet::from([caller]));
        assert!(!report.probes.listing_threads.is_empty());
        assert!(
            !report.probes.listing_threads.contains(&caller),
            "directory listings run on scanner workers"
        );
        assert_eq!(report.artifacts.len(), 32);
        assert_eq!(report.status, ScanStatus::Completed);
        for artifact in &report.artifacts {
            let discovered = events
                .iter()
                .position(|event| matches!(event, ScanEvent::ArtifactDiscovered(found) if found.id == artifact.id))
                .unwrap();
            let completed = events
                .iter()
                .position(|event| matches!(event, ScanEvent::ArtifactCompleted { id, .. } if *id == artifact.id))
                .unwrap();
            assert!(discovered < completed);
            let build = tree.0.join(&artifact.relative_path);
            assert_eq!(
                artifact.bytes,
                fs::metadata(&build).unwrap().len()
                    + fs::metadata(build.join("debug")).unwrap().len()
                    + fs::metadata(build.join("debug/o")).unwrap().len()
            );
        }
        assert!(matches!(
            events.last(),
            Some(ScanEvent::Finished(ScanStatus::Completed))
        ));
    }

    #[test]
    fn external_cancel_while_a_worker_measures_still_returns() {
        let tree = TempTree::new();
        let build = tree.0.join(".build");
        fs::create_dir_all(&build).unwrap();
        for index in 0..8_000 {
            fs::write(build.join(format!("object-{index}.o")), b"").unwrap();
        }
        let root = tree.0.clone();
        let control = std::sync::Arc::new(ScanControl::new());
        let (discovered, discovery) = std::sync::mpsc::channel();
        let (finished, result) = std::sync::mpsc::channel();
        let canceller = control.clone();
        std::thread::spawn(move || {
            if discovery.recv().is_ok() {
                // Cancel while the worker reads sizes and the caller waits for them.
                std::thread::sleep(std::time::Duration::from_millis(1));
                canceller.cancel();
            }
        });
        let scanner = control.clone();
        std::thread::spawn(move || {
            let report = scan_with_policy(
                &root,
                ScanOptions {
                    apparent_size: true,
                },
                &scanner,
                |facts| rust_cli_policy(facts, false),
                |event| {
                    if matches!(event, ScanEvent::ArtifactDiscovered(_)) {
                        let _ = discovered.send(());
                    }
                },
            );
            let _ = finished.send(report);
        });

        let report = result
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("a cancelled scan returns instead of waiting for unsent sizes");
        assert_eq!(report.status, ScanStatus::Cancelled);
        assert!(report.artifacts.iter().all(|artifact| artifact.partial));
    }

    #[test]
    fn nodes_outside_artifacts_are_typed_without_metadata_calls() {
        let tree = TempTree::new();
        for module in 0..4 {
            let sources = tree.0.join(format!("Sources/module-{module}"));
            fs::create_dir_all(&sources).unwrap();
            for file in 0..16 {
                fs::write(sources.join(format!("file-{file}.swift")), b"source").unwrap();
            }
        }
        fs::create_dir_all(tree.0.join(".build/debug")).unwrap();
        fs::write(tree.0.join(".build/debug/object.o"), b"object").unwrap();
        fs::write(tree.0.join(".build/manifest"), b"manifest").unwrap();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |_| {},
        );

        // The selected root, then `.build`, `debug`, `object.o` and `manifest` are measured.
        assert_eq!(report.probes.metadata, 5);
        assert_eq!(report.artifacts.len(), 1);
        let build = tree.0.join(".build");
        assert_eq!(
            report.artifacts[0].bytes,
            fs::metadata(&build).unwrap().len()
                + fs::metadata(build.join("debug")).unwrap().len()
                + fs::metadata(build.join("debug/object.o")).unwrap().len()
                + fs::metadata(build.join("manifest")).unwrap().len()
        );
    }

    fn artifact_paths(root: &Path, include_environments: bool) -> (Vec<PathBuf>, usize) {
        let report = scan_with_policy(
            root,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, include_environments),
            |_| {},
        );
        let probes = report.probes.marker_probes;
        let mut paths: Vec<_> = report
            .artifacts
            .into_iter()
            .map(|artifact| artifact.relative_path)
            .collect();
        paths.sort();
        (paths, probes)
    }

    #[test]
    fn markers_come_from_directory_listings_without_extra_probes() {
        let tree = TempTree::new();
        for module in 0..16 {
            fs::create_dir_all(tree.0.join(format!("app/Sources/module-{module}"))).unwrap();
        }
        fs::create_dir_all(tree.0.join("service/target/debug")).unwrap();
        fs::write(tree.0.join("service/Cargo.toml"), b"[package]").unwrap();
        fs::create_dir_all(tree.0.join("tool/dist")).unwrap();
        fs::write(tree.0.join("tool/setup.cfg"), b"[metadata]").unwrap();
        fs::create_dir_all(tree.0.join("plain/build")).unwrap();
        fs::create_dir_all(tree.0.join(".venv/lib")).unwrap();
        fs::write(tree.0.join(".venv/pyvenv.cfg"), b"home = /usr").unwrap();
        fs::create_dir_all(tree.0.join("venv/pyvenv.cfg")).unwrap();

        let (excluded, probes) = artifact_paths(&tree.0, false);
        assert_eq!(probes, 0);
        assert_eq!(
            excluded,
            [PathBuf::from("service/target"), PathBuf::from("tool/dist")]
        );

        let (included, probes) = artifact_paths(&tree.0, true);
        assert_eq!(probes, 0);
        assert_eq!(
            included,
            [
                PathBuf::from(".venv"),
                PathBuf::from("service/target"),
                PathBuf::from("tool/dist")
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_markers_count_when_they_resolve_to_files() {
        let tree = TempTree::new();
        fs::create_dir_all(tree.0.join("shared")).unwrap();
        fs::write(tree.0.join("shared/Cargo.toml"), b"[package]").unwrap();
        fs::create_dir_all(tree.0.join("linked/target")).unwrap();
        std::os::unix::fs::symlink("../shared/Cargo.toml", tree.0.join("linked/Cargo.toml"))
            .unwrap();
        fs::create_dir_all(tree.0.join("dangling/target")).unwrap();
        std::os::unix::fs::symlink("missing.toml", tree.0.join("dangling/Cargo.toml")).unwrap();

        let (paths, probes) = artifact_paths(&tree.0, false);

        assert_eq!(paths, [PathBuf::from("linked/target")]);
        assert_eq!(probes, 2, "only symlinked markers are resolved");
    }

    #[test]
    fn excluding_roots_traverses_them_and_keeps_subtree_measurement_whole() {
        let tree = TempTree::new();
        fs::create_dir_all(tree.0.join(".pytest_cache/inner/.build")).unwrap();
        fs::write(tree.0.join(".pytest_cache/inner/.build/object"), b"swift").unwrap();
        fs::create_dir_all(tree.0.join("app/.build/__pycache__")).unwrap();
        fs::write(
            tree.0.join("app/.build/__pycache__/module.pyc"),
            b"bytecode",
        )
        .unwrap();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |candidate| {
                let action = rust_cli_policy(candidate, false);
                match action {
                    CandidateAction::Classify(_)
                        if matches!(
                            filter_id_for_candidate(candidate, false),
                            Some("python.pytest-cache" | "python.bytecode")
                        ) =>
                    {
                        CandidateAction::Traverse
                    }
                    other => other,
                }
            },
            |_| {},
        );

        assert!(
            report
                .artifacts
                .iter()
                .any(|artifact| artifact.relative_path == Path::new(".pytest_cache/inner/.build"))
        );
        assert!(
            !report
                .artifacts
                .iter()
                .any(|artifact| artifact.kind == "cache" || artifact.kind == "bytecode")
        );
        let accepted_build = report
            .artifacts
            .iter()
            .find(|artifact| artifact.relative_path == Path::new("app/.build"))
            .unwrap();
        assert_eq!(
            accepted_build.bytes,
            fs::metadata(tree.0.join("app/.build")).unwrap().len()
                + fs::metadata(tree.0.join("app/.build/__pycache__"))
                    .unwrap()
                    .len()
                + fs::metadata(tree.0.join("app/.build/__pycache__/module.pyc"))
                    .unwrap()
                    .len()
        );
        assert!(report.total_bytes() >= b"bytecode".len() as u64);
    }
}
