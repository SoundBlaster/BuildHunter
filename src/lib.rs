use hunter_fs_core::{
    BranchPlan, ChildDecision, DirectoryListing, DirectoryMeasurement, DirectoryMeasurementPlan,
    DirectoryRequest, DirectoryResult, DirectoryTask, DirectoryVisit, DirectoryWorker,
    FilesystemBackend, GuardedSetupError, ListingAction, MeasuredSize, PendingSelection,
    RootAdmission, RootObservation, ScanHandler, SelectedEntry, StdFilesystem, SubtreeProgress,
    TraversalTasks, collect_directory, process_resources, run_directory_worker, thread_qos,
    with_guarded_backend,
};
#[cfg(test)]
use hunter_fs_core::{ScanEngine, spawn_workers};
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
mod profile;
pub use profile::{PROFILE_INTERVAL, ScanProfileSample};
mod resources;
pub use resources::{ResourceHistory, ResourceSummary};
mod materialization;
mod measurement;
mod scan_session;
/// Thread-scoped protection for filesystem preflight before starting a scan.
/// On macOS this refuses materialization; other platforms use a no-op guard.
pub use materialization::Guard as NonMaterializingIoGuard;
pub use policy_table::{
    bh_policy_table_cell_count, bh_policy_table_cell_facts, bh_policy_table_version,
    bh_scan_with_policy_table,
};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

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

/// Type of the directory entry itself, without following symbolic links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateNodeKind {
    Directory,
    RegularFile,
    Symlink,
    Other,
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
    /// Traversal finished, but observed entries disappeared before measurement.
    CompletedWithChanges,
    Cancelled,
    Incomplete,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanEvent {
    Profile(ScanProfileSample),
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
    profiling: AtomicBool,
    cancelled: AtomicBool,
    failed: AtomicBool,
    listing_ns: std::sync::atomic::AtomicU64,
    measurement_ns: std::sync::atomic::AtomicU64,
    bulk_directories: std::sync::atomic::AtomicU64,
    bulk_fallbacks: std::sync::atomic::AtomicU64,
}

/// Aggregate worker wall time, including overlapping work on different threads.
/// Populated only when profiling is enabled; not a partition of total scan time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanWorkTimings {
    pub listing_ns: u64,
    pub measurement_ns: u64,
    pub bulk_directories: u64,
    pub bulk_fallbacks: u64,
}

impl ScanControl {
    pub fn new() -> Self {
        Self {
            profiling: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            listing_ns: std::sync::atomic::AtomicU64::new(0),
            measurement_ns: std::sync::atomic::AtomicU64::new(0),
            bulk_directories: std::sync::atomic::AtomicU64::new(0),
            bulk_fallbacks: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Enable before starting a scan. Disabled scans emit no profile events.
    pub fn set_profiling(&self, enabled: bool) {
        self.profiling.store(enabled, Ordering::Release);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn work_timings(&self) -> ScanWorkTimings {
        ScanWorkTimings {
            listing_ns: self.listing_ns.load(Ordering::Relaxed),
            measurement_ns: self.measurement_ns.load(Ordering::Relaxed),
            bulk_directories: self.bulk_directories.load(Ordering::Relaxed),
            bulk_fallbacks: self.bulk_fallbacks.load(Ordering::Relaxed),
        }
    }

    fn timer(&self) -> Option<std::time::Instant> {
        self.profiling
            .load(Ordering::Relaxed)
            .then(std::time::Instant::now)
    }

    fn record_time(counter: &std::sync::atomic::AtomicU64, start: Option<std::time::Instant>) {
        if let Some(start) = start {
            counter.fetch_add(
                u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
        }
    }

    /// Whether the caller has requested cancellation. Does not imply workers returned.
    pub fn is_cancelled(&self) -> bool {
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

/// Recognize macOS managed cloud roots lexically, before any directory enumeration.
/// HOME is deliberately not used: a sandboxed app's HOME points at its container.
/// Legacy sync folders outside these locations cannot be identified by path alone.
#[cfg(any(target_os = "macos", all(test, unix)))]
fn managed_cloud_location(path: &Path) -> Option<&'static str> {
    use std::path::Component;
    if !path.is_absolute() {
        return None;
    }
    let normalized;
    let path = if path.components().any(|part| part == Component::ParentDir) {
        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    result.pop();
                }
                _ => result.push(component.as_os_str()),
            }
        }
        normalized = result;
        normalized.as_path()
    } else {
        path
    };
    let user_path = path
        .strip_prefix("/System/Volumes/Data/Users")
        .or_else(|_| path.strip_prefix("/Users"))
        .ok()?;
    let mut components = user_path.components();
    let account = components.next()?.as_os_str();
    if account == "Shared" {
        return None;
    }
    if components.next()?.as_os_str() != "Library" {
        return None;
    }
    match components.next()?.as_os_str().to_str()? {
        "Mobile Documents" => Some("iCloud"),
        "CloudStorage" => Some("File Provider"),
        _ => None,
    }
}

fn cloud_skip_reason(path: &Path) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let absolute;
        let path = if path.is_absolute() {
            path
        } else {
            absolute = std::path::absolute(path).ok()?;
            absolute.as_path()
        };
        managed_cloud_location(path).map(|provider| {
            format!("Skipped cloud-managed location ({provider}); contents were not scanned")
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
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
    policy: impl FnMut(&CandidateFacts) -> CandidateAction,
    emit: impl FnMut(&ScanEvent),
) -> ScanReport {
    scan_with_probes(root, options, control, policy, emit, &Probes::default())
}

/// Observe every visited candidate, including arbitrary names. This entrypoint
/// disables the legacy name gate, marker resolution and speculative root sizing.
/// Callbacks run serially on the caller thread under the non-materializing guard.
/// The path identifies the observed entry; it is not a no-follow access handle.
/// Callers acquiring facts must enforce scope/no-follow themselves. The existing
/// numeric classification/report ABI is unchanged; product adapters own metadata.
pub fn scan_with_candidate_observer(
    root: &Path,
    options: ScanOptions,
    control: &ScanControl,
    observer: impl FnMut(&Path, &CandidateFacts, CandidateNodeKind) -> CandidateAction,
    emit: impl FnMut(&ScanEvent),
) -> ScanReport {
    scan_session::run(
        root,
        options,
        control,
        observer,
        emit,
        &Probes::default(),
        false,
    )
}

fn scan_with_probes(
    root: &Path,
    options: ScanOptions,
    control: &ScanControl,
    mut policy: impl FnMut(&CandidateFacts) -> CandidateAction,
    emit: impl FnMut(&ScanEvent),
    probes: &Probes,
) -> ScanReport {
    scan_session::run(
        root,
        options,
        control,
        |_, facts, _| policy(facts),
        emit,
        probes,
        true,
    )
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

type ListTask = DirectoryTask<usize, u32>;
type Listing = DirectoryListing<u32>;
enum WorkerMessage {
    SetupFailed(std::io::Error),
    Directory(DirectoryResult<usize, u32>),
}

type WorkQueue = hunter_fs_core::WorkQueue<ListTask>;

fn run_worker(
    queue: &WorkQueue,
    results: &std::sync::mpsc::Sender<WorkerMessage>,
    control: &ScanControl,
    probes: &Probes,
    legacy_policy: bool,
) {
    run_worker_with_guard(
        queue,
        results,
        control,
        probes,
        legacy_policy,
        materialization::Guard::enter,
    );
}

fn run_worker_with_guard<G>(
    queue: &WorkQueue,
    results: &std::sync::mpsc::Sender<WorkerMessage>,
    control: &ScanControl,
    probes: &Probes,
    legacy_policy: bool,
    enter: impl FnOnce() -> std::io::Result<G>,
) {
    let result = with_guarded_backend(
        enter,
        || {
            Ok::<_, std::convert::Infallible>(WorkerAdapter {
                results,
                control,
                probes,
                legacy_policy,
            })
        },
        |backend| run_directory_worker(queue, backend),
    );
    match result {
        Ok(()) => {}
        Err(GuardedSetupError::Guard(error)) => {
            let _ = results.send(WorkerMessage::SetupFailed(error));
        }
        Err(GuardedSetupError::Backend(never)) => match never {},
    }
}

struct WorkerAdapter<'a> {
    results: &'a std::sync::mpsc::Sender<WorkerMessage>,
    control: &'a ScanControl,
    probes: &'a Probes,
    legacy_policy: bool,
}
impl DirectoryWorker for WorkerAdapter<'_> {
    type Id = usize;
    type Context = u32;

    fn list(&mut self, task: &ListTask) -> Listing {
        let listing_started = self.control.timer();
        let listing = list_directory(&task.path, self.control, self.probes, self.legacy_policy);
        ScanControl::record_time(&self.control.listing_ns, listing_started);
        listing
    }
    fn prefetch(&mut self, task: &ListTask, listing: &Listing) -> bool {
        let name = task.path.file_name().map(os_str_bytes).unwrap_or_default();
        // Speculatively measure likely artifact roots; the coordinator reads anything missed.
        !task.measure
            && self.legacy_policy
            && may_classify_directory(&name, listing.context, task.context)
    }
    fn measure(
        &mut self,
        plan: DirectoryMeasurementPlan<'_, usize>,
    ) -> DirectoryMeasurement<usize> {
        let control = self.control;
        let measurement_started = control.timer();
        let measured = measurement::measure_directory(plan, control, self.probes);
        ScanControl::record_time(&control.measurement_ns, measurement_started);
        measured
    }
    fn publish(&mut self, result: DirectoryResult<usize, u32>) -> bool {
        self.results.send(WorkerMessage::Directory(result)).is_ok()
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

fn list_directory(
    path: &Path,
    control: &ScanControl,
    probes: &Probes,
    legacy_policy: bool,
) -> Listing {
    let (listing, outcome) = collect_directory(
        path,
        probes.read_dir(path),
        0,
        || control.should_stop(),
        |child, file_type, markers| {
            observe_legacy_marker(child, file_type, markers, probes, legacy_policy)
        },
    );
    if outcome == DirectoryVisit::OpenFailed {
        return listing;
    }
    #[cfg(test)]
    let mut listing = listing;
    #[cfg(test)]
    if let Err(error) = probes.hook(ProbePoint::Listed, path) {
        listing.errors.push((path.to_owned(), error));
    }
    listing
}

/// Context interpretation and intentional marker-link following stay product-owned.
fn observe_legacy_marker(
    child: &Path,
    file_type: fs::FileType,
    markers: &mut u32,
    probes: &Probes,
    legacy_policy: bool,
) -> std::io::Result<()> {
    let marker = child.file_name().map_or(0, marker_flag);
    if legacy_policy
        && marker != 0
        && (file_type.is_file() || (file_type.is_symlink() && probes.marker_is_file(child)?))
    {
        *markers |= marker;
    }
    Ok(())
}

/// A directory waiting for its listing. `chain` holds the artifacts strictly above it.
struct DirNode {
    path: PathBuf,
    /// `None` only for the selected root, which has no parent listing.
    parent_markers: Option<u32>,
    chain: Vec<u64>,
    root_metadata: Option<fs::Metadata>,
}

/// Product branch interpretation retained across routing and deferred measurement.
/// `chain` includes a selected directory; `parent_len` is the ancestry above it.
struct ArtifactBranch {
    chain: Vec<u64>,
    parent_len: usize,
}

type FileSlot = SelectedEntry<Option<u64>>;

type ProductTasks<'a> =
    TraversalTasks<'a, DirNode, PendingSelection<ArtifactBranch, Option<u64>>, u32>;

struct ArtifactHandler<'a, P, E> {
    root: &'a Path,
    options: ScanOptions,
    control: &'a ScanControl,
    policy: &'a mut P,
    legacy_policy: bool,
    emit: &'a mut E,
    probes: &'a Probes,
    artifacts: Vec<Artifact>,
    progress: Vec<SubtreeProgress>,
    warnings: Vec<ScanWarning>,
    had_errors: bool,
    changed: bool,
    profile: Option<&'a profile::ProfileState>,
}

impl<P, E> ScanHandler for ArtifactHandler<'_, P, E>
where
    P: FnMut(&Path, &CandidateFacts, CandidateNodeKind) -> CandidateAction,
    E: FnMut(&ScanEvent),
{
    type State = DirNode;
    type Branch = ArtifactBranch;
    type Selection = Option<u64>;
    type Context = u32;
    type RootReason = String;
    type Message = WorkerMessage;
    fn initialize(
        &mut self,
        admission: RootAdmission<String>,
        tasks: &mut ProductTasks<'_>,
    ) -> bool {
        let root = self.root;
        match admission {
            RootAdmission::Skipped(reason) => {
                self.warn(root, reason, &[]);
                return false;
            }
            RootAdmission::Unavailable(error) => self.warn(root, error, &[]),
            RootAdmission::Observed(RootObservation::Directory(metadata)) => {
                self.enqueue(
                    DirNode {
                        path: root.to_owned(),
                        parent_markers: None,
                        chain: Vec::new(),
                        root_metadata: Some(metadata),
                    },
                    DirectoryRequest {
                        path: root.to_owned(),
                        context: 0,
                        measure: false,
                    },
                    tasks,
                );
            }
            RootAdmission::Observed(RootObservation::Entry(metadata)) => {
                let file_type = metadata.file_type();
                if let Some(slot) = self.visit_file(root.to_owned(), file_type, 0, &[]) {
                    self.measure_file(&slot, Some(Ok(metadata.into())), &[]);
                }
            }
        }
        true
    }
    fn should_stop(&self) -> bool {
        self.control.should_stop()
    }
    fn checkpoint(&mut self) {
        self.emit_profile();
    }
    fn response_received(&mut self, outstanding: usize) {
        self.sync_outstanding(outstanding);
    }
    fn response(&mut self, message: WorkerMessage) -> Option<DirectoryResult<usize, u32>> {
        match message {
            WorkerMessage::SetupFailed(error) => {
                self.warn(
                    self.root,
                    format!("Cannot prevent cloud downloads on scanner worker: {error}"),
                    &[],
                );
                self.control.fail();
                None
            }
            WorkerMessage::Directory(result) => Some(result),
        }
    }
    fn listing_received(&mut self) {
        if let Some(profile) = self.profile {
            profile::bump(&profile.counters.entries, 1);
            profile::bump(&profile.counters.directories, 1);
        }
    }
    fn measurement_pending(&mut self, outstanding: usize) {
        self.sync_outstanding(outstanding);
    }
    fn prepare_listing(
        &mut self,
        state: DirNode,
        listing: &mut Listing,
    ) -> ListingAction<ArtifactBranch> {
        self.prepare_directory(state, listing)
    }
    fn select(
        &mut self,
        branch: &ArtifactBranch,
        index: usize,
        path: &Path,
        _: fs::FileType,
    ) -> ChildDecision {
        if index.is_multiple_of(32) {
            self.emit_profile();
        }
        if self.control.should_stop() {
            return ChildDecision::Stop;
        }
        if let Some(reason) = cloud_skip_reason(path) {
            self.warn(path, reason, &branch.chain);
            return ChildDecision::Skip;
        }
        ChildDecision::Visit
    }
    fn directory(
        &mut self,
        branch: &ArtifactBranch,
        _: usize,
        request: DirectoryRequest<u32>,
        tasks: &mut ProductTasks<'_>,
    ) {
        for id in &branch.chain {
            self.progress[Self::index(*id)].add_pending_directory();
        }
        let node = DirNode {
            path: request.path.clone(),
            parent_markers: Some(request.context),
            chain: branch.chain.clone(),
            root_metadata: None,
        };
        self.enqueue(node, request, tasks);
    }
    fn entry(
        &mut self,
        branch: &ArtifactBranch,
        index: usize,
        path: PathBuf,
        file_type: fs::FileType,
    ) -> Option<FileSlot> {
        self.visit_file(path, file_type, index, &branch.chain)
    }
    fn directory_measured(
        &mut self,
        branch: &ArtifactBranch,
        path: &Path,
        metadata: std::io::Result<MeasuredSize>,
    ) {
        match metadata {
            Ok(metadata) => {
                let bytes = metadata.bytes(self.options.apparent_size);
                if let Some(profile) = self.profile {
                    profile::bump(&profile.counters.measured_bytes, bytes);
                }
                for id in &branch.chain {
                    self.progress[Self::index(*id)].add_bytes(bytes);
                }
            }
            Err(error) => self.io_error(path, error, &branch.chain),
        }
    }
    fn measurement_checkpoint(&mut self, index: usize) {
        if index.is_multiple_of(32) {
            self.emit_profile();
        }
    }
    fn entry_measured(
        &mut self,
        branch: &ArtifactBranch,
        selected: &FileSlot,
        metadata: std::io::Result<MeasuredSize>,
    ) {
        self.measure_file(selected, Some(metadata), &branch.chain);
    }
    fn finish_directory(&mut self, branch: &ArtifactBranch) {
        self.finish_directory(&branch.chain, branch.parent_len);
    }
    fn finish(&mut self, _exit: hunter_fs_core::CoordinatorExit) {
        for index in (0..self.progress.len()).rev() {
            if !self.progress[index].is_completed() {
                self.progress[index].mark_partial();
                self.complete(index as u64 + 1);
            }
        }
    }
}

impl<P, E> ArtifactHandler<'_, P, E>
where
    P: FnMut(&Path, &CandidateFacts, CandidateNodeKind) -> CandidateAction,
    E: FnMut(&ScanEvent),
{
    /// Delivers samples the sampler thread queued; their timestamps are when they were taken.
    fn emit_profile(&mut self) {
        let Some(profile) = self.profile else { return };
        for sample in profile.drain() {
            self.emit(&ScanEvent::Profile(sample));
        }
    }

    fn sync_outstanding(&self, outstanding: usize) {
        if let Some(profile) = self.profile {
            profile
                .counters
                .pending_tasks
                .store(outstanding as u64, Ordering::Relaxed);
        }
    }

    fn emit(&mut self, event: &ScanEvent) {
        (self.emit)(event);
    }

    fn warn(&mut self, path: &Path, error: impl std::fmt::Display, chain: &[u64]) {
        self.had_errors = true;
        self.notice(path, error, chain);
    }

    fn io_error(&mut self, path: &Path, error: std::io::Error, chain: &[u64]) {
        if error.kind() == std::io::ErrorKind::NotFound
            && path != self.root
            && !materialization::is_dataless_error(&error)
        {
            // A vanished child is expected on a live tree, but losing the
            // selected root invalidates the scan rather than just that entry.
            match self.probes.symlink_metadata(self.root) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => {
                    self.warn(self.root, "Selected root is no longer a directory", chain);
                    return;
                }
                Err(root_error) => {
                    self.warn(self.root, root_error, chain);
                    return;
                }
            }
            self.changed = true;
            self.notice(
                path,
                "Changed during scan: entry disappeared before it could be measured",
                chain,
            );
        } else {
            self.warn(path, error, chain);
        }
    }

    fn notice(&mut self, path: &Path, message: impl std::fmt::Display, chain: &[u64]) {
        let warning = ScanWarning {
            relative_path: relative_path(self.root, path),
            message: message.to_string(),
        };
        if let Some(profile) = self.profile {
            profile::bump(&profile.counters.warnings, 1);
        }
        self.warnings.push(warning.clone());
        self.emit(&ScanEvent::Warning(warning));
        for id in chain {
            self.progress[Self::index(*id)].mark_partial();
        }
    }

    fn index(id: u64) -> usize {
        usize::try_from(id - 1).expect("artifact IDs fit in memory")
    }

    fn begin_profile(&mut self) {
        if let Some(profile) = self.profile {
            profile.record();
        }
        self.emit_profile();
    }

    #[cfg(test)]
    fn run(
        &mut self,
        queue: &WorkQueue,
        receiver: &std::sync::mpsc::Receiver<WorkerMessage>,
    ) -> hunter_fs_core::TraversalOutcome {
        self.begin_profile();
        let wake_interval = self.profile.map(|_| profile::PROFILE_INTERVAL);
        let root = self.root;
        let probes = self.probes;
        ScanEngine::new(self, ProductTasks::new(queue)).run(
            probes,
            root,
            cloud_skip_reason,
            receiver,
            wake_interval,
        )
    }

    fn enqueue(
        &mut self,
        node: DirNode,
        request: DirectoryRequest<u32>,
        tasks: &mut ProductTasks<'_>,
    ) {
        if tasks
            .enqueue(request, node, |outstanding| {
                self.sync_outstanding(outstanding)
            })
            .is_err()
        {
            self.control.fail();
        }
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
        if let Some(profile) = self.profile {
            profile::bump(&profile.counters.artifacts, 1);
        }
        self.emit(&ScanEvent::ArtifactDiscovered(artifact.clone()));
        self.artifacts.push(artifact);
        self.progress.push(SubtreeProgress::default());
        id
    }

    fn complete(&mut self, id: u64) {
        let index = Self::index(id);
        let completion = self.progress[index]
            .complete(self.control.should_stop())
            .expect("each artifact completes once");
        let bytes = completion.bytes;
        let partial = completion.partial;
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
        if let Some(profile) = self.profile {
            profile::bump(&profile.counters.entries, 1);
        }
        let is_symlink = file_type.is_symlink();
        let name = path.file_name().map(os_str_bytes).unwrap_or_default();
        let action = if !self.legacy_policy || is_symlink || is_policy_candidate(&name, false, 0) {
            (self.policy)(
                &path,
                &CandidateFacts {
                    node_name: name.into_owned(),
                    is_directory: false,
                    is_symbolic_link: is_symlink,
                    has_artifact_ancestor: !chain.is_empty(),
                    own_marker_files: 0,
                    parent_marker_files: 0,
                },
                if is_symlink {
                    CandidateNodeKind::Symlink
                } else if file_type.is_file() {
                    CandidateNodeKind::RegularFile
                } else {
                    CandidateNodeKind::Other
                },
            )
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
            context: artifact,
        })
    }

    /// Adds a file's size to `chain` and completes the file's own artifact, if any.
    fn measure_file(
        &mut self,
        slot: &FileSlot,
        metadata: Option<std::io::Result<MeasuredSize>>,
        chain: &[u64],
    ) {
        let metadata = slot.resolve_metadata(self.probes, metadata);
        let bytes = match metadata {
            Ok(metadata) => metadata.bytes(self.options.apparent_size),
            Err(error) => {
                let mut affected = chain.to_vec();
                affected.extend(slot.context);
                self.io_error(&slot.path, error, &affected);
                0
            }
        };
        if let Some(profile) = self.profile {
            profile::bump(&profile.counters.measured_bytes, bytes);
        }
        for id in chain {
            let progress = &mut self.progress[Self::index(*id)];
            progress.add_bytes(bytes);
        }
        if let Some(id) = slot.context {
            self.progress[Self::index(id)].set_bytes(bytes);
            self.complete(id);
        }
    }

    fn prepare_directory(
        &mut self,
        state: DirNode,
        listing: &mut Listing,
    ) -> ListingAction<ArtifactBranch> {
        let DirNode {
            path,
            parent_markers,
            mut chain,
            root_metadata,
        } = state;
        let parent_len = chain.len();
        let name = path.file_name().map(os_str_bytes).unwrap_or_default();
        let own_markers = listing.context & MARKER_PYVENV_CFG;
        let parent_markers =
            if self.legacy_policy && matches!(name.as_ref(), b"target" | b"build" | b"dist") {
                parent_markers.unwrap_or_else(|| {
                    path.parent().map_or(0, |parent| {
                        let (flags, errors) = marker_flags(parent, self.probes);
                        listing.errors.extend(errors);
                        flags
                    })
                }) & !MARKER_PYVENV_CFG
            } else {
                0
            };
        let action = if !self.legacy_policy || is_policy_candidate(&name, true, own_markers) {
            (self.policy)(
                &path,
                &CandidateFacts {
                    node_name: name.into_owned(),
                    is_directory: true,
                    is_symbolic_link: false,
                    has_artifact_ancestor: parent_len > 0,
                    own_marker_files: own_markers,
                    parent_marker_files: parent_markers,
                },
                CandidateNodeKind::Directory,
            )
        } else {
            CandidateAction::Traverse
        };
        if self.control.should_stop() {
            return ListingAction::Abandon;
        }
        if action == CandidateAction::Prune {
            return ListingAction::Complete(ArtifactBranch { chain, parent_len });
        }
        if let CandidateAction::Classify(code) = action {
            let id = self.discover(&path, parent_len > 0, code.language, code.kind);
            chain.push(id);
        }
        for (index, (error_path, error)) in
            std::mem::take(&mut listing.errors).into_iter().enumerate()
        {
            if index % 32 == 0 {
                self.emit_profile();
            }
            self.io_error(&error_path, error, &chain);
        }
        ListingAction::Traverse(BranchPlan {
            path,
            include_directory: !chain.is_empty(),
            root_metadata,
            context: ArtifactBranch { chain, parent_len },
        })
    }

    /// Marks a directory finished in every artifact above it and completes any artifact
    /// whose subtree has no queued directories left, innermost first.
    fn finish_directory(&mut self, chain: &[u64], parent_len: usize) {
        for id in &chain[..parent_len] {
            self.progress[Self::index(*id)].finish_pending_directory();
        }
        if let Some(id) = chain.get(parent_len) {
            self.progress[Self::index(*id)].finish_root();
        }
        for id in chain.iter().rev() {
            let progress = &self.progress[Self::index(*id)];
            if progress.is_ready() {
                self.complete(*id);
            }
        }
    }
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
    hook: Option<Box<ProbeHook>>,
    #[cfg(all(test, target_os = "macos"))]
    materialization_policies: std::sync::Mutex<Vec<i32>>,
    #[cfg(test)]
    metadata: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    marker_probes: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    listing_threads: std::sync::Mutex<Vec<std::thread::ThreadId>>,
}

impl FilesystemBackend for Probes {
    type Entries = fs::ReadDir;
    fn symlink_metadata(&self, path: &Path) -> std::io::Result<fs::Metadata> {
        #[cfg(test)]
        {
            self.record_policy();
            self.metadata.fetch_add(1, Ordering::Relaxed);
            self.hook(ProbePoint::Metadata, path)?;
        }
        StdFilesystem
            .symlink_metadata(path)
            .map_err(|error| materialization::describe_error(path, error))
    }

    fn read_dir(&self, path: &Path) -> std::io::Result<fs::ReadDir> {
        #[cfg(test)]
        {
            self.record_policy();
            self.hook(ProbePoint::ReadDir, path)?;
            let thread = std::thread::current().id();
            let mut threads = self.listing_threads.lock().unwrap();
            if !threads.contains(&thread) {
                threads.push(thread);
            }
        }
        StdFilesystem
            .read_dir(path)
            .map_err(|error| materialization::describe_error(path, error))
    }
}

impl Probes {
    #[cfg(test)]
    fn hook(&self, point: ProbePoint, path: &Path) -> std::io::Result<()> {
        self.hook.as_ref().map_or(Ok(()), |hook| hook(point, path))
    }
    #[cfg(test)]
    fn record_policy(&self) {
        #[cfg(target_os = "macos")]
        self.materialization_policies
            .lock()
            .unwrap()
            .push(materialization::current_policy());
    }

    fn marker_is_file(&self, path: &Path) -> std::io::Result<bool> {
        #[cfg(test)]
        {
            self.record_policy();
            self.marker_probes.fetch_add(1, Ordering::Relaxed);
        }
        fs::metadata(path)
            .map(|metadata| metadata.is_file())
            .map_err(|error| materialization::describe_error(path, error))
    }

    #[cfg(test)]
    fn counts(&self) -> ProbeCounts {
        ProbeCounts {
            #[cfg(target_os = "macos")]
            materialization_policies: self.materialization_policies.lock().unwrap().clone(),
            metadata: self.metadata.load(Ordering::Relaxed),
            marker_probes: self.marker_probes.load(Ordering::Relaxed),
            listing_threads: self.listing_threads.lock().unwrap().clone(),
        }
    }
}

#[cfg(test)]
type ProbeHook = dyn Fn(ProbePoint, &Path) -> std::io::Result<()> + Send + Sync;

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum ProbePoint {
    Metadata,
    ReadDir,
    Listed,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProbeCounts {
    #[cfg(target_os = "macos")]
    pub materialization_policies: Vec<i32>,
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

fn marker_flags(path: &Path, probes: &Probes) -> (u32, Vec<(PathBuf, std::io::Error)>) {
    let mut flags = 0;
    let mut errors = Vec::new();
    for (name, flag) in [
        ("Cargo.toml", MARKER_CARGO_TOML),
        ("pyproject.toml", MARKER_PYPROJECT_TOML),
        ("setup.py", MARKER_SETUP_PY),
        ("setup.cfg", MARKER_SETUP_CFG),
    ] {
        let marker = path.join(name);
        match probes.marker_is_file(&marker) {
            Ok(true) => flags |= flag,
            Ok(false) => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && !materialization::is_dataless_error(&error) => {}
            Err(error) => errors.push((marker, error)),
        }
    }
    (flags, errors)
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
    pub profile: *const ScanProfileSample,
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

/// Enable profile events before invoking a scan. Existing callers remain opt-out.
/// # Safety
/// `control` must be a live pointer returned by `bh_scan_control_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_scan_control_set_profiling(control: *mut ScanControl, enabled: u8) {
    if let Some(control) = unsafe { control.as_ref() } {
        control.set_profiling(enabled != 0);
    }
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
                profile: std::ptr::null(),
            };
            unsafe { event_callback(context, &event) };
            Some(3)
        }
    }
}

unsafe fn send_ffi_event(callback: BHEventCallback, context: *mut c_void, event: &ScanEvent) {
    let empty: [u8; 0] = [];
    if let ScanEvent::Profile(sample) = event {
        let ffi = BHScanEvent {
            event_type: 5,
            artifact_id: 0,
            path: std::ptr::null(),
            path_len: 0,
            language: 0,
            kind: 0,
            bytes: 0,
            partial: 0,
            status: 0,
            message: std::ptr::null(),
            message_len: 0,
            profile: sample,
        };
        unsafe { callback(context, &ffi) };
        return;
    }
    let (ffi, _path, _message) = match event {
        ScanEvent::Profile(_) => unreachable!(),
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
                profile: std::ptr::null(),
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
                profile: std::ptr::null(),
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
                profile: std::ptr::null(),
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
                profile: std::ptr::null(),
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
        ScanStatus::CompletedWithChanges => 4,
        ScanStatus::Cancelled => 1,
        ScanStatus::Incomplete => 2,
        ScanStatus::Failed => 3,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn candidate_observer_bypasses_name_gate_and_legacy_marker_probes() {
        let tree = TempTree::new();
        fs::create_dir_all(tree.0.join("unusual-dir")).unwrap();
        fs::write(tree.0.join("unusual-dir/note.bin"), b"12345").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(tree.0.join("missing"), tree.0.join("Cargo.toml")).unwrap();
        let caller = std::thread::current().id();
        let probes = Probes::default();
        let mut observed = Vec::new();
        let report = scan_session::run(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |path, facts, kind| {
                assert_eq!(std::thread::current().id(), caller);
                assert_eq!(facts.own_marker_files, 0);
                assert_eq!(facts.parent_marker_files, 0);
                observed.push((relative_path(&tree.0, path), kind));
                if facts.name() == Some("note.bin") {
                    CandidateAction::Classify(ArtifactClassificationCode {
                        language: 3,
                        kind: 5,
                    })
                } else {
                    CandidateAction::Traverse
                }
            },
            |_| {},
            &probes,
            false,
        );
        assert_eq!(report.status, ScanStatus::Completed);
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(report.artifacts[0].bytes, 5);
        assert_eq!(report.probes.marker_probes, 0);
        assert!(observed.contains(&(PathBuf::from("unusual-dir"), CandidateNodeKind::Directory)));
        assert!(observed.contains(&(
            PathBuf::from("unusual-dir/note.bin"),
            CandidateNodeKind::RegularFile
        )));
    }

    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicBool, AtomicU64, Ordering},
    };

    pub(crate) struct TempTree(pub(crate) PathBuf);

    impl TempTree {
        pub(crate) fn new() -> Self {
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
    fn vanished_file_marks_its_artifact_partial_without_failing_the_scan() {
        let tree = TempTree::new();
        let file = tree.0.join(".build/object.o");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"temporary object").unwrap();
        let removed = AtomicBool::new(false);
        let probes = Probes {
            hook: Some(Box::new(move |point, path| {
                if point == ProbePoint::Metadata
                    && path == file
                    && !removed.swap(true, Ordering::SeqCst)
                {
                    fs::remove_file(path)?;
                }
                Ok(())
            })),
            ..Probes::default()
        };
        let report = scan_with_probes(
            &tree.0,
            ScanOptions::default(),
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |_| {},
            &probes,
        );
        assert_eq!(report.status, ScanStatus::CompletedWithChanges);
        assert!(report.artifacts[0].partial);
        assert!(report.warnings[0].message.contains("Changed during scan"));
    }

    #[test]
    fn nested_subtrees_propagate_partial_sizes_and_complete_innermost_once() {
        let tree = TempTree::new();
        let build = tree.0.join(".build");
        let cache = build.join("__pycache__");
        fs::create_dir_all(&cache).unwrap();
        fs::write(build.join("object.o"), b"12345").unwrap();
        fs::write(cache.join("good.pyc"), b"1234567").unwrap();
        let denied = cache.join("denied.pyc");
        fs::write(&denied, b"unmeasured").unwrap();
        let failed_path = denied.clone();
        let probes = Probes {
            hook: Some(Box::new(move |point, path| {
                if point == ProbePoint::Metadata && path == failed_path {
                    Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
                } else {
                    Ok(())
                }
            })),
            ..Probes::default()
        };
        let mut completed = Vec::new();
        let report = scan_with_probes(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |event| {
                if let ScanEvent::ArtifactCompleted { id, .. } = event {
                    completed.push(*id);
                }
            },
            &probes,
        );
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(
            report.warnings[0].relative_path,
            Path::new(".build/__pycache__/denied.pyc")
        );
        let find = |path: &str| {
            report
                .artifacts
                .iter()
                .find(|a| a.relative_path == Path::new(path))
                .unwrap()
        };
        let outer = find(".build");
        let inner = find(".build/__pycache__");
        let good = find(".build/__pycache__/good.pyc");
        let failed = find(".build/__pycache__/denied.pyc");
        assert_eq!(report.artifacts.len(), 4);
        let cache_bytes = fs::symlink_metadata(&cache).unwrap().len();
        assert_eq!(inner.bytes, cache_bytes + 7);
        assert_eq!(
            outer.bytes,
            fs::symlink_metadata(&build).unwrap().len() + 5 + cache_bytes + 7
        );
        assert_eq!(report.total_bytes(), outer.bytes);
        assert!(outer.partial && inner.partial && failed.partial);
        assert_eq!(failed.bytes, 0);
        assert_eq!(good.bytes, 7);
        assert!(!good.partial);
        let position = |id| completed.iter().position(|seen| *seen == id).unwrap();
        assert!(position(good.id) < position(inner.id));
        assert!(position(failed.id) < position(inner.id));
        assert!(position(inner.id) < position(outer.id));
        for artifact in &report.artifacts {
            assert_eq!(completed.iter().filter(|id| **id == artifact.id).count(), 1);
        }
    }

    #[test]
    fn worker_phase_timings_are_opt_in_and_preserve_the_report() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join(".build")).unwrap();
        fs::write(tree.0.join(".build/object.o"), b"fixture").unwrap();
        let plain = ScanControl::new();
        let measured = ScanControl::new();
        measured.set_profiling(true);
        let scan = |control: &ScanControl| {
            scan_with_policy(
                &tree.0,
                ScanOptions::default(),
                control,
                |facts| rust_cli_policy(facts, false),
                |_| {},
            )
        };
        let old = scan(&plain);
        let new = scan(&measured);
        assert_eq!(plain.work_timings(), ScanWorkTimings::default());
        assert!(measured.work_timings().listing_ns > 0);
        assert!(measured.work_timings().measurement_ns > 0);
        assert_eq!(old.artifacts, new.artifacts);
        assert_eq!(old.status, new.status);
        assert!(old.warnings.is_empty() && new.warnings.is_empty());
        #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
        assert!(measured.work_timings().bulk_directories > 0);
    }

    fn scan_hooked(
        tree: &TempTree,
        hook: impl Fn(ProbePoint, &Path) -> std::io::Result<()> + Send + Sync + 'static,
    ) -> ScanReport {
        scan_with_probes(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |_| {},
            &Probes {
                hook: Some(Box::new(hook)),
                ..Probes::default()
            },
        )
    }

    #[test]
    fn worker_guard_failure_publishes_setup_error_without_backend_work() {
        let queue = WorkQueue::default();
        queue.close();
        let probes = Probes::default();
        let (sender, receiver) = std::sync::mpsc::channel();
        run_worker_with_guard(&queue, &sender, &ScanControl::new(), &probes, true, || {
            Err::<(), _>(std::io::Error::from_raw_os_error(13))
        });
        let WorkerMessage::SetupFailed(error) = receiver.try_recv().unwrap() else {
            panic!("setup failure")
        };
        assert_eq!(error.raw_os_error(), Some(13));
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        assert_eq!(probes.counts().metadata, 0);
        assert!(probes.counts().listing_threads.is_empty());
    }

    #[test]
    fn worker_backend_panic_releases_same_thread_guard() {
        use std::cell::Cell;
        let tree = TempTree::new();
        let queue = WorkQueue::default();
        queue
            .push(ListTask {
                id: 0,
                path: tree.0.clone(),
                context: 0,
                measure: false,
            })
            .unwrap();
        let dropped = Cell::new(false);
        struct Guard<'a>(&'a Cell<bool>, std::thread::ThreadId);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                assert_eq!(std::thread::current().id(), self.1);
                self.0.set(true);
            }
        }
        let probes = Probes {
            hook: Some(Box::new(|_, _| panic!("injected backend panic"))),
            ..Probes::default()
        };
        let (sender, _receiver) = std::sync::mpsc::channel();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_worker_with_guard(&queue, &sender, &ScanControl::new(), &probes, true, || {
                Ok(Guard(&dropped, std::thread::current().id()))
            });
        }));
        assert!(result.is_err());
        assert!(dropped.get());
    }

    #[test]
    fn selected_file_reuses_root_metadata_even_with_precancelled_startup() {
        let tree = TempTree::new();
        let root = tree.0.join("selected.pyc");
        fs::write(&root, b"1234567").unwrap();
        for cancelled in [false, true] {
            let control = ScanControl::new();
            if cancelled {
                control.cancel();
            }
            let mut terminal = Vec::new();
            let report = scan_with_policy(
                &root,
                ScanOptions {
                    apparent_size: true,
                },
                &control,
                |facts| rust_cli_policy(facts, false),
                |event| {
                    if let ScanEvent::Finished(status) = event {
                        terminal.push(*status);
                    }
                },
            );
            let expected = if cancelled {
                ScanStatus::Cancelled
            } else {
                ScanStatus::Completed
            };
            assert_eq!(report.status, expected);
            assert_eq!(terminal, [expected]);
            assert_eq!(report.probes.metadata, 1, "reuse the initial observation");
            assert!(report.probes.listing_threads.is_empty());
            assert!(report.warnings.is_empty());
            if cancelled {
                assert!(report.artifacts.is_empty());
            } else {
                assert_eq!(report.artifacts.len(), 1);
                assert_eq!(report.artifacts[0].relative_path, Path::new("."));
                assert_eq!(report.artifacts[0].bytes, 7);
            }
        }
    }

    #[test]
    fn missing_selected_root_remains_incomplete_instead_of_child_churn() {
        let tree = TempTree::new();
        let report = scan_with_policy(
            &tree.0.join("missing"),
            ScanOptions::default(),
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |_| {},
        );
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].relative_path, Path::new("."));
        assert_eq!(report.probes.metadata, 1);
        assert!(report.probes.listing_threads.is_empty());
        assert!(report.artifacts.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn selected_symlink_roots_never_follow_file_directory_or_dangling_targets() {
        let tree = TempTree::new();
        fs::write(tree.0.join("file.pyc"), b"payload").unwrap();
        fs::create_dir(tree.0.join("directory")).unwrap();
        for target in ["file.pyc", "directory", "missing"] {
            let root = tree.0.join(format!("link-{target}.pyc"));
            std::os::unix::fs::symlink(target, &root).unwrap();
            let report = scan_with_policy(
                &root,
                ScanOptions::default(),
                &ScanControl::new(),
                |facts| rust_cli_policy(facts, false),
                |_| {},
            );
            assert_eq!(report.status, ScanStatus::Completed);
            assert!(report.warnings.is_empty() && report.artifacts.is_empty());
            assert_eq!(report.probes.metadata, 1);
            assert!(report.probes.listing_threads.is_empty());
        }
    }

    #[test]
    fn coordinator_consumes_ignored_speculative_measurement_before_finishing() {
        let tree = TempTree::new();
        let root = tree.0.join(".build");
        fs::create_dir(&root).unwrap();
        let control = ScanControl::new();
        let probes = Probes::default();
        let queue = WorkQueue::default();
        let (sender, receiver) = std::sync::mpsc::channel();
        // Script the worker protocol to exercise result consumption deterministically.
        sender
            .send(WorkerMessage::Directory(DirectoryResult::Listed {
                id: 0,
                listing: Listing::default(),
                measuring: true,
            }))
            .unwrap();
        sender
            .send(WorkerMessage::Directory(DirectoryResult::Measured(
                DirectoryMeasurement {
                    id: 0,
                    directory: Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
                    files: Vec::new(),
                },
            )))
            .unwrap();
        let mut policy =
            |_: &Path, _: &CandidateFacts, _: CandidateNodeKind| CandidateAction::Prune;
        let mut emit = |_: &ScanEvent| {};
        let mut scanner = ArtifactHandler {
            root: &root,
            options: ScanOptions::default(),
            control: &control,
            policy: &mut policy,
            legacy_policy: true,
            emit: &mut emit,
            probes: &probes,
            artifacts: Vec::new(),
            progress: Vec::new(),
            warnings: Vec::new(),
            had_errors: false,
            changed: false,
            profile: None,
        };
        let outcome = scanner.run(&queue, &receiver);
        assert_eq!(outcome.outstanding, 0);
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        assert!(
            scanner.warnings.is_empty(),
            "ignored measurements cannot change status"
        );
        assert!(scanner.artifacts.is_empty());
        assert!(!scanner.had_errors);
        assert!(!control.should_stop());
        queue.close();
    }

    #[test]
    fn coordinator_profile_cancellation_preserves_receipt_and_dispatch_boundaries() {
        for queued_response in [false, true] {
            let tree = TempTree::new();
            let control = ScanControl::new();
            let probes = Probes::default();
            let queue = WorkQueue::default();
            let (sender, receiver) = std::sync::mpsc::channel();
            if queued_response {
                sender
                    .send(WorkerMessage::SetupFailed(std::io::Error::from(
                        std::io::ErrorKind::PermissionDenied,
                    )))
                    .unwrap();
            }
            let profile = profile::ProfileState::new();
            profile.record();
            let mut policy = |_: &Path, _: &CandidateFacts, _: CandidateNodeKind| {
                panic!("cancelled result must not reach classification")
            };
            let mut samples = 0;
            let mut emit = |event: &ScanEvent| {
                if matches!(event, ScanEvent::Profile(_)) {
                    samples += 1;
                    control.cancel();
                } else {
                    panic!("cancelled setup error must not be dispatched");
                }
            };
            let mut scanner = ArtifactHandler {
                root: &tree.0,
                options: ScanOptions::default(),
                control: &control,
                policy: &mut policy,
                legacy_policy: true,
                emit: &mut emit,
                probes: &probes,
                artifacts: Vec::new(),
                progress: Vec::new(),
                warnings: Vec::new(),
                had_errors: false,
                changed: false,
                profile: Some(&profile),
            };
            let mut tasks = ProductTasks::new(&queue);
            scanner.enqueue(
                DirNode {
                    path: tree.0.clone(),
                    parent_markers: None,
                    chain: Vec::new(),
                    root_metadata: None,
                },
                DirectoryRequest {
                    path: tree.0.clone(),
                    context: 0,
                    measure: false,
                },
                &mut tasks,
            );
            let outcome = ScanEngine::new(&mut scanner, tasks).drive(
                &probes,
                &receiver,
                Some(std::time::Duration::from_millis(1)),
            );
            assert_eq!(outcome.exit, Some(hunter_fs_core::CoordinatorExit::Stopped));

            let outstanding = usize::from(!queued_response);
            assert_eq!(outcome.outstanding, outstanding);
            assert_eq!(
                profile.counters.pending_tasks.load(Ordering::Relaxed),
                outstanding as u64
            );
            assert!(scanner.warnings.is_empty());
            assert!(!scanner.had_errors && !control.is_failed());
            assert!(control.is_cancelled());
            assert_eq!(samples, 1);
            drop(sender);
            queue.close();
        }
    }

    #[test]
    #[cfg(unix)]
    fn listing_marker_observer_preserves_links_errors_and_policy_disabled_mode() {
        let tree = TempTree::new();
        fs::write(tree.0.join("Cargo.toml"), b"regular").unwrap();
        fs::write(tree.0.join("data"), b"target").unwrap();
        std::os::unix::fs::symlink("data", tree.0.join("pyproject.toml")).unwrap();
        std::os::unix::fs::symlink("missing", tree.0.join("setup.py")).unwrap();
        fs::create_dir(tree.0.join("setup.cfg")).unwrap();
        let _guard = materialization::Guard::enter().unwrap();
        for legacy_policy in [true, false] {
            let probes = Probes::default();
            let listing = list_directory(&tree.0, &ScanControl::new(), &probes, legacy_policy);
            assert_eq!(
                listing.children.len(),
                5,
                "observer failure must retain its child"
            );
            let link = listing
                .children
                .iter()
                .find(|(p, _)| p.ends_with("setup.py"))
                .unwrap();
            assert!(link.1.is_symlink());
            if legacy_policy {
                assert_eq!(listing.context, MARKER_CARGO_TOML | MARKER_PYPROJECT_TOML);
                assert_eq!(probes.counts().marker_probes, 2);
                assert_eq!(listing.errors.len(), 1);
                assert_eq!(listing.errors[0].0, tree.0.join("setup.py"));
                assert_eq!(listing.errors[0].1.kind(), std::io::ErrorKind::NotFound);
            } else {
                assert_eq!(listing.context, 0);
                assert_eq!(probes.counts().marker_probes, 0);
                assert!(listing.errors.is_empty());
            }
        }
    }

    #[test]
    fn worker_publishes_listing_before_metadata_and_cancelled_measurement_response() {
        let tree = TempTree::new();
        fs::write(tree.0.join("data"), b"payload").unwrap();
        let queue = WorkQueue::default();
        queue
            .push(ListTask {
                id: 17,
                path: tree.0.clone(),
                measure: true,
                context: 0,
            })
            .unwrap_or_else(|_| panic!("open queue"));
        let control = std::sync::Arc::new(ScanControl::new());
        let (sender, receiver) = std::sync::mpsc::channel();
        let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
        let (ready, signal) = std::sync::mpsc::channel();
        let probes = Probes {
            hook: Some(Box::new({
                let receiver = receiver.clone();
                let control = control.clone();
                let root = tree.0.clone();
                move |point, path| {
                    if point == ProbePoint::Metadata && path == root {
                        assert!(matches!(
                            receiver.lock().unwrap().try_recv(),
                            Ok(WorkerMessage::Directory(DirectoryResult::Listed {
                                id: 17,
                                measuring: true,
                                ..
                            }))
                        ));
                        control.cancel();
                        ready.send(()).unwrap();
                    }
                    Ok(())
                }
            })),
            ..Probes::default()
        };
        std::thread::scope(|scope| {
            let _close = queue.close_on_drop();
            spawn_workers(scope, &queue, 1, {
                let control = &control;
                let probes = &probes;
                move |q| run_worker(q, &sender, control, probes, false)
            });
            signal
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            let message = receiver
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            match message {
                WorkerMessage::Directory(DirectoryResult::Measured(measurement)) => {
                    assert_eq!(measurement.id, 17);
                    assert!(measurement.directory.is_ok());
                    assert!(measurement.files.is_empty());
                }
                _ => panic!("measurement must follow listing"),
            }
        });
        assert!(control.is_cancelled());
    }

    #[test]
    fn disconnected_worker_recipient_skips_measurement_and_leaves_next_task_queued() {
        let tree = TempTree::new();
        fs::write(tree.0.join("data"), b"payload").unwrap();
        let queue = WorkQueue::default();
        for id in [18, 17] {
            queue
                .push(ListTask {
                    id,
                    path: tree.0.clone(),
                    measure: true,
                    context: 0,
                })
                .unwrap_or_else(|_| panic!("open queue"));
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        #[cfg(target_os = "macos")]
        let (previous_policy, guarded_policy) = {
            let previous = materialization::current_policy();
            let _guard = materialization::Guard::enter().unwrap();
            (previous, materialization::current_policy())
        };
        let probes = Probes::default();
        run_worker(&queue, &sender, &ScanControl::new(), &probes, false);
        assert_eq!(queue.pop().unwrap().id, 18);
        queue.close();
        let counts = probes.counts();
        assert_eq!(
            counts.metadata, 0,
            "rejected listing must suppress metadata and bulk preparation"
        );
        assert_eq!(
            counts.marker_probes, 0,
            "observer mode keeps legacy policy disabled"
        );
        #[cfg(target_os = "macos")]
        {
            assert!(!counts.materialization_policies.is_empty());
            assert!(
                counts
                    .materialization_policies
                    .iter()
                    .all(|policy| *policy == guarded_policy)
            );
            assert_eq!(materialization::current_policy(), previous_policy);
        }
    }

    #[test]
    fn callback_unwind_closes_scoped_workers_and_allows_a_followup_scan() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join(".build")).unwrap();
        fs::write(tree.0.join(".build/object.o"), b"data").unwrap();
        for panic_policy in [true, false] {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                scan_with_policy(
                    &tree.0,
                    ScanOptions::default(),
                    &ScanControl::new(),
                    |facts| {
                        if panic_policy && facts.name() == Some(".build") {
                            panic!("injected policy panic");
                        }
                        rust_cli_policy(facts, false)
                    },
                    |event| {
                        if !panic_policy && matches!(event, ScanEvent::ArtifactDiscovered(_)) {
                            panic!("injected observer panic");
                        }
                    },
                )
            }));
            assert!(result.is_err());
            let report = scan_with_policy(
                &tree.0,
                ScanOptions::default(),
                &ScanControl::new(),
                |facts| rust_cli_policy(facts, false),
                |_| {},
            );
            assert_eq!(report.status, ScanStatus::Completed);
            assert_eq!(report.artifacts.len(), 1);
        }
    }
    #[test]
    fn worker_probe_panic_propagates_without_stranding_scanner() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join(".build")).unwrap();
        let triggered = std::sync::Arc::new(AtomicBool::new(false));
        let flag = triggered.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scan_hooked(&tree, move |point, path| {
                if point == ProbePoint::ReadDir && path.file_name() == Some(OsStr::new(".build")) {
                    flag.store(true, Ordering::SeqCst);
                    panic!("injected worker probe panic");
                }
                Ok(())
            })
        }));
        assert!(triggered.load(Ordering::SeqCst));
        assert!(result.is_err());
        let report = scan_with_policy(
            &tree.0,
            ScanOptions::default(),
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |_| {},
        );
        assert_eq!(report.status, ScanStatus::Completed);
    }

    #[test]
    fn vanished_directory_marks_ancestors_but_not_other_artifacts() {
        let tree = TempTree::new();
        let removed = tree.0.join(".build/__pycache__");
        fs::create_dir_all(&removed).unwrap();
        fs::write(removed.join("module.pyc"), b"temporary").unwrap();
        fs::create_dir_all(tree.0.join(".pytest_cache")).unwrap();
        let once = AtomicBool::new(false);
        let report = scan_hooked(&tree, move |point, path| {
            if point == ProbePoint::ReadDir && path == removed && !once.swap(true, Ordering::SeqCst)
            {
                fs::remove_dir_all(path)?;
            }
            Ok(())
        });
        assert_eq!(report.status, ScanStatus::CompletedWithChanges);
        assert_eq!(report.artifacts.len(), 3);
        for artifact in &report.artifacts {
            assert_eq!(
                artifact.partial,
                artifact.relative_path.starts_with(".build")
            );
        }
        assert_eq!(status_code(report.status), 4);
    }

    #[test]
    fn renamed_file_is_reported_as_changed_without_counting_its_new_name_twice() {
        let tree = TempTree::new();
        let old = tree.0.join(".build/old.o");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, b"temporary").unwrap();
        let once = AtomicBool::new(false);
        let report = scan_hooked(&tree, move |point, path| {
            if point == ProbePoint::Metadata && path == old && !once.swap(true, Ordering::SeqCst) {
                fs::rename(path, path.with_file_name("new.o"))?;
            }
            Ok(())
        });
        assert_eq!(report.status, ScanStatus::CompletedWithChanges);
        assert!(report.artifacts[0].partial);
        assert_eq!(
            report.total_bytes(),
            fs::symlink_metadata(tree.0.join(".build")).unwrap().len()
        );
    }

    #[test]
    fn new_files_before_listing_are_included_and_after_listing_wait_for_rescan() {
        for point in [ProbePoint::ReadDir, ProbePoint::Listed] {
            let tree = TempTree::new();
            let build = tree.0.join(".build");
            fs::create_dir(&build).unwrap();
            fs::write(build.join("old.o"), b"old").unwrap();
            let once = AtomicBool::new(false);
            let report = scan_hooked(&tree, move |observed, path| {
                if observed == point && path == build && !once.swap(true, Ordering::SeqCst) {
                    fs::write(path.join("new.o"), b"new object")?;
                }
                Ok(())
            });
            let directory_bytes = fs::symlink_metadata(tree.0.join(".build")).unwrap().len();
            let new_bytes = if point == ProbePoint::ReadDir { 10 } else { 0 };
            assert_eq!(report.total_bytes(), directory_bytes + 3 + new_bytes);
            assert_eq!(report.status, ScanStatus::Completed);
            assert!(report.warnings.is_empty());
            assert!(!report.artifacts[0].partial);
        }
    }

    #[test]
    fn disappearance_of_selected_root_remains_an_error() {
        let tree = TempTree::new();
        let root = tree.0.clone();
        let report = scan_hooked(&tree, move |point, path| {
            if point == ProbePoint::ReadDir && path == root {
                fs::remove_dir(path)?;
            }
            Ok(())
        });
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert!(!report.warnings.is_empty());
        assert!(!report.warnings[0].message.contains("Changed during scan"));
    }

    #[test]
    fn selected_root_vanishing_after_listing_is_not_ordinary_child_churn() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join(".build")).unwrap();
        fs::write(tree.0.join(".build/object.o"), b"temporary").unwrap();
        let root = tree.0.clone();
        let report = scan_hooked(&tree, move |point, path| {
            if point == ProbePoint::Listed && path == root {
                fs::remove_dir_all(path)?;
            }
            Ok(())
        });
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.relative_path == Path::new("."))
        );
    }

    #[test]
    fn real_errors_still_make_a_changing_scan_incomplete() {
        let tree = TempTree::new();
        let build = tree.0.join(".build");
        fs::create_dir(&build).unwrap();
        fs::write(build.join("vanish.o"), b"gone").unwrap();
        fs::write(build.join("denied.o"), b"denied").unwrap();
        let report = scan_hooked(&tree, move |point, path| {
            if point == ProbePoint::Metadata {
                if path.file_name() == Some(OsStr::new("vanish.o")) {
                    return Err(std::io::Error::from(std::io::ErrorKind::NotFound));
                }
                if path.file_name() == Some(OsStr::new("denied.o")) {
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
                }
            }
            Ok(())
        });
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(report.warnings.len(), 2);
        assert!(report.artifacts[0].partial);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn identified_cloud_skip_is_not_mistaken_for_temporary_file_churn() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join(".build")).unwrap();
        fs::write(tree.0.join(".build/cloud.o"), b"fixture").unwrap();
        let report = scan_hooked(&tree, |point, path| {
            if point == ProbePoint::Metadata && path.file_name() == Some(OsStr::new("cloud.o")) {
                return Err(materialization::dataless_error(std::io::Error::from(
                    std::io::ErrorKind::NotFound,
                )));
            }
            Ok(())
        });
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert!(
            report.warnings[0]
                .message
                .contains("contents were not downloaded")
        );
    }

    #[test]
    fn profiling_preserves_results_and_counts_nested_measurements_once() {
        let tree = TempTree::new();
        fs::create_dir_all(tree.0.join(".build/__pycache__")).unwrap();
        fs::write(tree.0.join(".build/__pycache__/module.pyc"), b"fixture").unwrap();
        let plain_control = ScanControl::default();
        let mut plain_profiles = 0;
        let plain = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &plain_control,
            |facts| rust_cli_policy(facts, false),
            |event| {
                if matches!(event, ScanEvent::Profile(_)) {
                    plain_profiles += 1
                }
            },
        );
        assert_eq!(plain_profiles, 0);
        let control = ScanControl::default();
        control.set_profiling(true);
        let mut samples = Vec::new();
        let profiled = scan_with_policy(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &control,
            |facts| rust_cli_policy(facts, false),
            |event| {
                if let ScanEvent::Profile(sample) = event {
                    samples.push(*sample)
                }
            },
        );
        assert_eq!(profiled.status, plain.status);
        assert_eq!(profiled.total_bytes(), plain.total_bytes());
        assert_eq!(profiled.artifacts.len(), plain.artifacts.len());
        assert!(samples.len() >= 2);
        let last = samples.last().unwrap();
        assert_eq!(last.measured_bytes, plain.total_bytes());
        assert_eq!(last.entries, 4); // root, .build, __pycache__, module.pyc
        assert_eq!(last.directories, 3);
        assert_eq!(last.artifacts as usize, plain.artifacts.len());
        assert_eq!(last.pending_tasks, 0);
        assert!(last.elapsed_us > 0);
    }

    #[test]
    fn profile_updates_inside_a_large_directory_response() {
        let tree = TempTree::new();
        for index in 0..128 {
            fs::write(tree.0.join(format!("module-{index}.pyc")), b"fixture").unwrap();
        }
        let control = ScanControl::new();
        control.set_profiling(true);
        let mut samples = Vec::new();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions::default(),
            &control,
            |facts| rust_cli_policy(facts, false),
            |event| match event {
                ScanEvent::ArtifactDiscovered(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(4))
                }
                ScanEvent::Profile(sample) => samples.push(*sample),
                _ => {}
            },
        );
        assert_eq!(report.artifacts.len(), 128);
        assert!(
            samples
                .iter()
                .any(|sample| sample.artifacts > 0 && sample.artifacts < 128),
            "A single large listing must not hold back progress until every child is classified"
        );
    }

    #[test]
    fn blocked_event_consumer_does_not_thin_the_profile_timeline() {
        // A consumer that cannot keep up blocks the coordinator inside `emit`. Samples are
        // taken on their own thread, so the timeline keeps its interval through the stall.
        // Before the sampler thread each stall became a single gap as long as the stall.
        const STALL_MS: u64 = 500;
        let tree = TempTree::new();
        for index in 0..4 {
            let build = tree.0.join(format!("package-{index}/.build"));
            fs::create_dir_all(&build).unwrap();
            fs::write(build.join("object.o"), b"fixture").unwrap();
        }
        let control = ScanControl::new();
        control.set_profiling(true);
        let mut samples = Vec::new();
        scan_with_policy(
            &tree.0,
            ScanOptions::default(),
            &control,
            |facts| rust_cli_policy(facts, false),
            |event| match event {
                ScanEvent::ArtifactDiscovered(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(STALL_MS))
                }
                ScanEvent::Profile(sample) => samples.push(*sample),
                _ => {}
            },
        );
        let timeline: Vec<_> = samples
            .iter()
            .map(|sample| sample.elapsed_us / 1000)
            .collect();
        let elapsed_ms = *timeline.last().unwrap();
        assert!(
            elapsed_ms >= 4 * STALL_MS,
            "four stalls, timeline {timeline:?}"
        );
        // A busy runner can wake the sampler late; allow that, but not a stall-long gap.
        assert!(
            samples.len() as u64 >= elapsed_ms / 100 * 7 / 10,
            "{} samples over {elapsed_ms} ms: {timeline:?}",
            samples.len()
        );
        let largest_gap = timeline
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .max()
            .unwrap();
        assert!(
            largest_gap < STALL_MS * 7 / 10,
            "samples must not wait for the consumer: largest gap {largest_gap} ms in {timeline:?}"
        );
        assert!(
            samples
                .windows(2)
                .all(|pair| pair[0].elapsed_us < pair[1].elapsed_us
                    && pair[0].entries <= pair[1].entries)
        );
    }

    #[test]
    fn cancelled_profile_still_emits_a_terminal_snapshot() {
        let tree = TempTree::new();
        let control = ScanControl::default();
        control.set_profiling(true);
        control.cancel();
        let mut events = Vec::new();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions::default(),
            &control,
            |_| CandidateAction::Traverse,
            |event| events.push(event.clone()),
        );
        assert_eq!(report.status, ScanStatus::Cancelled);
        assert!(matches!(events.first(), Some(ScanEvent::Profile(_))));
        assert!(matches!(events[events.len() - 2], ScanEvent::Profile(_)));
        assert!(matches!(
            events.last(),
            Some(ScanEvent::Finished(ScanStatus::Cancelled))
        ));
    }

    #[test]
    fn ffi_profile_payload_can_be_copied_during_callback() {
        unsafe extern "C" fn capture(context: *mut c_void, event: *const BHScanEvent) {
            let event = unsafe { &*event };
            assert_eq!(event.event_type, 5);
            unsafe {
                *(context as *mut ScanProfileSample) = *event.profile;
            }
        }
        let sample = ScanProfileSample {
            elapsed_us: 1_000_000,
            entries: 42,
            measured_bytes: 99,
            cpu_user_ns: 2_000_000,
            cpu_system_ns: 1_000_000,
            resident_bytes: 4096,
            resource_flags: 3,
            ..ScanProfileSample::default()
        };
        assert_eq!(size_of::<ScanProfileSample>(), 88);
        assert_eq!(std::mem::offset_of!(ScanProfileSample, cpu_user_ns), 56);
        assert_eq!(std::mem::offset_of!(ScanProfileSample, resource_flags), 80);
        let mut copied = ScanProfileSample::default();
        unsafe {
            send_ffi_event(
                capture,
                (&mut copied as *mut ScanProfileSample).cast(),
                &ScanEvent::Profile(sample),
            );
        }
        assert_eq!(copied, sample);
    }

    #[cfg(unix)]
    #[test]
    fn managed_cloud_paths_match_only_standard_user_locations() {
        for root in [
            "/Users/developer/Library/Mobile Documents",
            "/Users/developer/Library/CloudStorage",
            "/System/Volumes/Data/Users/developer/Library/CloudStorage",
        ] {
            assert!(managed_cloud_location(Path::new(root)).is_some());
            assert!(
                managed_cloud_location(&Path::new(root).join("provider/project/target")).is_some()
            );
        }
        for local in [
            "/Users/Shared/Library/Mobile Documents/project/target",
            "/Users/Shared/Library/CloudStorage/project/target",
            "/System/Volumes/Data/Users/Shared/Library/CloudStorage/project/target",
            "/Users/developer/Library/Caches",
            "/Users/developer/Library/CloudStorage-backup",
            "/Users/developer/project/Library/CloudStorage",
            "/Users/developer/project/iCloud/target",
            "/Users/developer/Library/Mobile Documents/../Caches",
            "Library/CloudStorage",
        ] {
            assert!(
                managed_cloud_location(Path::new(local)).is_none(),
                "{local}"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn selected_cloud_location_is_marked_without_filesystem_probes() {
        for root in [
            "/Users/buildhunter-cloud-test/Library/Mobile Documents",
            "/Users/buildhunter-cloud-test/Library/CloudStorage/provider/target",
        ] {
            let mut events = Vec::new();
            let report = scan_with_policy(
                Path::new(root),
                ScanOptions::default(),
                &ScanControl::new(),
                |_| panic!("Cloud locations must not reach artifact policy"),
                |event| events.push(event.clone()),
            );
            assert_eq!(report.status, ScanStatus::Incomplete);
            assert!(report.artifacts.is_empty());
            assert_eq!(report.warnings.len(), 1);
            assert!(
                report.warnings[0]
                    .message
                    .contains("Skipped cloud-managed location")
            );
            assert_eq!(report.probes.metadata, 0);
            assert!(report.probes.listing_threads.is_empty());
            assert!(matches!(events.first(), Some(ScanEvent::Warning(_))));
            assert_eq!(
                events.last(),
                Some(&ScanEvent::Finished(ScanStatus::Incomplete))
            );
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
    fn planned_children_preserve_nested_sizes_and_host_pruning() {
        let tree = TempTree::new();
        let bucket = tree.0.join("bucket");
        fs::create_dir_all(bucket.join("keep/deep")).unwrap();
        fs::create_dir_all(bucket.join("skip")).unwrap();
        for (name, size) in [
            ("one", 1),
            ("keep/two", 17),
            ("keep/deep/three", 113),
            ("skip/hidden", 8192),
        ] {
            fs::write(bucket.join(name), vec![0; size]).unwrap();
        }
        let mut visited = Vec::new();
        let mut completions = Vec::new();
        let report = scan_with_candidate_observer(
            &tree.0,
            ScanOptions {
                apparent_size: true,
            },
            &ScanControl::new(),
            |path, facts, kind| {
                visited.push((path.to_owned(), facts.has_artifact_ancestor));
                if facts.name() == Some("skip") {
                    CandidateAction::Prune
                } else if facts.name() == Some("bucket") || kind == CandidateNodeKind::RegularFile {
                    CandidateAction::Classify(ArtifactClassificationCode {
                        language: 1,
                        kind: 1,
                    })
                } else {
                    CandidateAction::Traverse
                }
            },
            |event| {
                if let ScanEvent::ArtifactCompleted { id, .. } = event {
                    completions.push(*id);
                }
            },
        );
        assert_eq!(report.status, ScanStatus::Completed);
        assert!(report.warnings.is_empty());
        assert_eq!(report.artifacts.len(), 4);
        let expected = ["", "keep", "keep/deep"]
            .iter()
            .map(|name| fs::symlink_metadata(bucket.join(name)).unwrap().len())
            .sum::<u64>()
            + 131;
        assert_eq!(report.total_bytes(), expected);
        for (name, bytes) in [("one", 1), ("keep/two", 17), ("keep/deep/three", 113)] {
            let artifact = report
                .artifacts
                .iter()
                .find(|a| a.relative_path == Path::new("bucket").join(name))
                .unwrap();
            assert_eq!(artifact.bytes, bytes);
            assert!(artifact.nested);
        }
        assert!(visited.contains(&(bucket.join("skip"), true)));
        assert!(
            !visited
                .iter()
                .any(|(path, _)| path == &bucket.join("skip/hidden"))
        );
        assert!(visited.contains(&(bucket.join("keep/deep/three"), true)));
        for artifact in &report.artifacts {
            assert!(!artifact.partial);
            assert_eq!(
                completions.iter().filter(|id| **id == artifact.id).count(),
                1
            );
        }
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
        let expected = if cfg!(all(target_os = "macos", feature = "macos-bulk-metadata")) {
            3
        } else {
            5
        };
        assert_eq!(report.probes.metadata, expected);
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

    #[cfg(unix)]
    #[test]
    fn unreadable_symlinked_marker_is_visible_for_parent_and_selected_artifact() {
        let tree = TempTree::new();
        fs::create_dir(tree.0.join("target")).unwrap();
        std::os::unix::fs::symlink("Cargo.toml", tree.0.join("Cargo.toml")).unwrap();
        for root in [&tree.0, &tree.0.join("target")] {
            let report = scan_with_policy(
                root,
                ScanOptions::default(),
                &ScanControl::new(),
                |facts| rust_cli_policy(facts, false),
                |_| {},
            );
            assert_eq!(report.status, ScanStatus::Incomplete);
            assert_eq!(report.warnings.len(), 1);
            assert!(report.warnings[0].relative_path.ends_with("Cargo.toml"));
            assert!(report.artifacts.is_empty());
        }
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

#[cfg(test)]
mod migration_parity;
