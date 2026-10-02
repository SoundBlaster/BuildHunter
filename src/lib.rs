use std::{
    ffi::{OsStr, c_void},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(unix)]
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};

pub const MARKER_CARGO_TOML: u32 = 1 << 0;
pub const MARKER_PYPROJECT_TOML: u32 = 1 << 1;
pub const MARKER_SETUP_PY: u32 = 1 << 2;
pub const MARKER_SETUP_CFG: u32 = 1 << 3;
pub const MARKER_PYVENV_CFG: u32 = 1 << 4;

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

pub fn scan_with_policy(
    root: &Path,
    options: ScanOptions,
    control: &ScanControl,
    mut policy: impl FnMut(&CandidateFacts) -> CandidateAction,
    mut emit: impl FnMut(&ScanEvent),
) -> ScanReport {
    let mut scanner = Scanner {
        root,
        options,
        control,
        policy: &mut policy,
        emit: &mut emit,
        artifacts: Vec::new(),
        warnings: Vec::new(),
        next_id: 1,
    };
    scanner.walk(root, false);
    let status = if control.is_failed() {
        ScanStatus::Failed
    } else if control.is_cancelled() {
        ScanStatus::Cancelled
    } else if scanner.warnings.is_empty() {
        ScanStatus::Completed
    } else {
        ScanStatus::Incomplete
    };
    scanner.emit(&ScanEvent::Finished(status));
    ScanReport {
        artifacts: scanner.artifacts,
        warnings: scanner.warnings,
        status,
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

struct Scanner<'a, P, E> {
    root: &'a Path,
    options: ScanOptions,
    control: &'a ScanControl,
    policy: &'a mut P,
    emit: &'a mut E,
    artifacts: Vec<Artifact>,
    warnings: Vec<ScanWarning>,
    next_id: u64,
}

impl<P, E> Scanner<'_, P, E>
where
    P: FnMut(&CandidateFacts) -> CandidateAction,
    E: FnMut(&ScanEvent),
{
    fn emit(&mut self, event: &ScanEvent) {
        (self.emit)(event);
    }

    fn warn(&mut self, path: &Path, error: impl std::fmt::Display) {
        let warning = ScanWarning {
            relative_path: relative_path(self.root, path),
            message: error.to_string(),
        };
        self.warnings.push(warning.clone());
        self.emit(&ScanEvent::Warning(warning));
    }

    fn walk(&mut self, path: &Path, inside_artifact: bool) -> u64 {
        if self.control.should_stop() {
            return 0;
        }
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.warn(path, error);
                return 0;
            }
        };
        let is_symlink = metadata.file_type().is_symlink();
        let is_directory = metadata.is_dir();
        let node_name = path.file_name().map(os_str_bytes).unwrap_or_default();
        // Marker probes are restricted to candidates; checking siblings for every path would
        // dominate traversal cost on large source trees.
        let own_markers = if is_directory && path.join("pyvenv.cfg").is_file() {
            MARKER_PYVENV_CFG
        } else {
            0
        };
        let parent_markers =
            if is_directory && matches!(node_name.as_slice(), b"target" | b"build" | b"dist") {
                path.parent().map(marker_flags).unwrap_or_default()
            } else {
                0
            };
        let query_policy = is_symlink || is_policy_candidate(&node_name, is_directory, own_markers);
        let action = if query_policy {
            (self.policy)(&CandidateFacts {
                node_name: node_name.clone(),
                is_directory,
                is_symbolic_link: is_symlink,
                has_artifact_ancestor: inside_artifact,
                own_marker_files: own_markers,
                parent_marker_files: parent_markers,
            })
        } else {
            CandidateAction::Traverse
        };
        if is_symlink || action == CandidateAction::Prune {
            return 0;
        }

        let classification = match action {
            CandidateAction::Classify(code) => {
                Some((language_name(code.language), kind_name(code.kind)))
            }
            CandidateAction::Traverse | CandidateAction::Prune => None,
        };
        let id = classification.map(|(language, kind)| {
            let id = self.next_id;
            self.next_id = self.next_id.saturating_add(1);
            let artifact = Artifact {
                id,
                relative_path: relative_path(self.root, path),
                language,
                kind,
                bytes: 0,
                nested: inside_artifact,
                partial: false,
            };
            self.emit(&ScanEvent::ArtifactDiscovered(artifact.clone()));
            self.artifacts.push(artifact);
            id
        });
        let artifact_ancestor = inside_artifact || id.is_some();
        let warning_count = self.warnings.len();
        let mut bytes = metadata_bytes(&metadata, self.options.apparent_size);
        if is_directory {
            match fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries {
                        if self.control.should_stop() {
                            break;
                        }
                        match entry {
                            Ok(entry) => {
                                bytes = bytes
                                    .saturating_add(self.walk(&entry.path(), artifact_ancestor));
                            }
                            Err(error) => self.warn(path, error),
                        }
                    }
                }
                Err(error) => self.warn(path, error),
            }
        }
        if let Some(id) = id {
            let partial = self.control.should_stop() || self.warnings.len() > warning_count;
            let index = usize::try_from(id - 1).ok();
            if let Some(artifact) = index.and_then(|index| self.artifacts.get_mut(index)) {
                artifact.bytes = bytes;
                artifact.partial = partial;
            }
            self.emit(&ScanEvent::ArtifactCompleted { id, bytes, partial });
        }
        bytes
    }
}

fn os_str_bytes(value: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        value.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        value.to_string_lossy().as_bytes().to_vec()
    }
}

fn marker_flags(path: &Path) -> u32 {
    let mut flags = 0;
    if path.join("Cargo.toml").is_file() {
        flags |= MARKER_CARGO_TOML;
    }
    if path.join("pyproject.toml").is_file() {
        flags |= MARKER_PYPROJECT_TOML;
    }
    if path.join("setup.py").is_file() {
        flags |= MARKER_SETUP_PY;
    }
    if path.join("setup.cfg").is_file() {
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
    let root_bytes = unsafe { std::slice::from_raw_parts(root, root_len) };
    let root_path = path_from_bytes(root_bytes);
    let control = unsafe { &*control };
    let policy_callback = policy_callback.unwrap();
    let event_callback = event_callback.unwrap();
    let mut policy_failed = false;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let report = scan_with_policy(
            &root_path,
            ScanOptions {
                apparent_size: apparent_size != 0,
            },
            control,
            |facts| {
                let decision = BHCandidateDecision::default();
                let mut decision = decision;
                let c_facts = BHCandidateFacts {
                    node_name: facts.node_name.as_ptr(),
                    node_name_len: facts.node_name.len(),
                    is_directory: u8::from(facts.is_directory),
                    is_symbolic_link: u8::from(facts.is_symbolic_link),
                    has_artifact_ancestor: u8::from(facts.has_artifact_ancestor),
                    own_marker_files: facts.own_marker_files,
                    parent_marker_files: facts.parent_marker_files,
                };
                let succeeded = unsafe { policy_callback(context, &c_facts, &mut decision) };
                if succeeded == 0 {
                    policy_failed = true;
                    control.fail();
                    return CandidateAction::Prune;
                }
                match decision.action {
                    0 => CandidateAction::Traverse,
                    1 => CandidateAction::Prune,
                    2 => CandidateAction::Classify(ArtifactClassificationCode {
                        language: decision.language,
                        kind: decision.kind,
                    }),
                    _ => {
                        policy_failed = true;
                        control.fail();
                        CandidateAction::Prune
                    }
                }
            },
            |event| unsafe { send_ffi_event(event_callback, context, event) },
        );
        if policy_failed {
            3
        } else {
            status_code(report.status)
        }
    }));
    match result {
        Ok(status) => status,
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
            3
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
        sync::atomic::{AtomicBool, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("build-hunter-core-{suffix}"));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
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
}
