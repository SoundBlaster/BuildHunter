//! Versioned projection of scanner facts onto the finite Swift-policy vocabulary.

use crate::{
    BHCandidateDecision, BHCandidateFacts, BHEventCallback, CandidateAction, CandidateFacts,
    ScanControl,
};
use std::ffi::c_void;

pub const VERSION: u32 = 1;
pub const CLASS_COUNT: usize = 18;
pub const CELL_COUNT: usize = CLASS_COUNT * 256 + 1;
const INVALID_UTF8_CELL: usize = CELL_COUNT - 1;

const REPRESENTATIVES: [&[u8]; CLASS_COUNT] = [
    b"other-node",
    b".git",
    b".build",
    b"target",
    b"__pycache__",
    b".pytest_cache",
    b".mypy_cache",
    b".ruff_cache",
    b".pytype",
    b".tox",
    b".nox",
    b"build",
    b"dist",
    b".venv",
    b"venv",
    b"sample.egg-info",
    b"sample.pyc",
    b"sample.pyo",
];
static INVALID_UTF8_REPRESENTATIVE: [u8; 1] = [0xff];

#[unsafe(no_mangle)]
pub extern "C" fn bh_policy_table_version() -> u32 {
    VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn bh_policy_table_cell_count() -> usize {
    CELL_COUNT
}

#[unsafe(no_mangle)]
/// Write immutable representative facts for one cell in the version 1 domain.
///
/// # Safety
/// If non-null, `facts` must point to writable storage for one `BHCandidateFacts`.
pub unsafe extern "C" fn bh_policy_table_cell_facts(
    index: usize,
    facts: *mut BHCandidateFacts,
) -> i32 {
    if facts.is_null() || index >= CELL_COUNT {
        return 0;
    }
    let (name, flags) = if index == INVALID_UTF8_CELL {
        (&INVALID_UTF8_REPRESENTATIVE[..], 0)
    } else {
        let class = index / 256;
        let flags = (index % 256) as u8;
        (REPRESENTATIVES[class], flags)
    };
    let parent_bits = u32::from(flags >> 4) & 0x0f;
    unsafe {
        *facts = BHCandidateFacts {
            node_name: name.as_ptr(),
            node_name_len: name.len(),
            is_directory: flags & 1,
            is_symbolic_link: (flags >> 1) & 1,
            has_artifact_ancestor: (flags >> 2) & 1,
            own_marker_files: if flags & 8 != 0 { 1 << 4 } else { 0 },
            parent_marker_files: parent_bits,
        };
    }
    1
}

/// Project UTF-8 exactly once; invalid names occupy the dedicated final cell.
pub fn project_cell(facts: &CandidateFacts) -> usize {
    let Some(name) = facts.name() else {
        return INVALID_UTF8_CELL;
    };
    let class = match name {
        ".git" => 1,
        ".build" => 2,
        "target" => 3,
        "__pycache__" => 4,
        ".pytest_cache" => 5,
        ".mypy_cache" => 6,
        ".ruff_cache" => 7,
        ".pytype" => 8,
        ".tox" => 9,
        ".nox" => 10,
        "build" => 11,
        "dist" => 12,
        ".venv" => 13,
        "venv" => 14,
        _ if name.ends_with(".egg-info") => 15,
        _ if name.ends_with(".pyc") => 16,
        _ if name.ends_with(".pyo") => 17,
        _ => 0,
    };
    let mut flags = u8::from(facts.is_directory)
        | (u8::from(facts.is_symbolic_link) << 1)
        | (u8::from(facts.has_artifact_ancestor) << 2);
    if facts.own_marker_files & (1 << 4) != 0 {
        flags |= 1 << 3;
    }
    for (source, target) in [(1, 4), (2, 5), (4, 6), (8, 7)] {
        if facts.parent_marker_files & source != 0 {
            flags |= 1 << target;
        }
    }
    class * 256 + usize::from(flags)
}

pub fn action_from_decision(decision: BHCandidateDecision) -> Option<CandidateAction> {
    match decision.action {
        0 => Some(CandidateAction::Traverse),
        1 => Some(CandidateAction::Prune),
        2 => Some(CandidateAction::Classify(
            crate::ArtifactClassificationCode {
                language: decision.language,
                kind: decision.kind,
            },
        )),
        _ => None,
    }
}

/// Scan using an owned, immutable copy of the caller's finite decision table.
///
/// # Safety
/// `control` must be a live scan-control pointer, `root` must address `root_len` readable bytes,
/// and `decisions` must address `decision_count` readable entries. The event callback and context
/// must obey the same lifetime and C ABI requirements as [`crate::bh_scan`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_scan_with_policy_table(
    control: *mut ScanControl,
    root: *const u8,
    root_len: usize,
    apparent_size: u8,
    table_version: u32,
    decisions: *const BHCandidateDecision,
    decision_count: usize,
    event_callback: Option<BHEventCallback>,
    context: *mut c_void,
) -> i32 {
    let Some(event_callback) = event_callback else {
        return 3;
    };
    if control.is_null()
        || root.is_null()
        || decisions.is_null()
        || table_version != VERSION
        || decision_count != CELL_COUNT
    {
        return 3;
    }
    let caller_decisions = unsafe { std::slice::from_raw_parts(decisions, decision_count) };
    if caller_decisions
        .iter()
        .any(|item| !(0..=2).contains(&item.action))
    {
        return 3;
    }
    let owned = caller_decisions.to_vec().into_boxed_slice();
    unsafe {
        crate::run_ffi_scan(
            control,
            root,
            root_len,
            apparent_size,
            event_callback,
            context,
            |facts| action_from_decision(owned[project_cell(facts)]),
        )
    }
    .unwrap_or(3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MARKER_CARGO_TOML, MARKER_PYPROJECT_TOML, MARKER_PYVENV_CFG, MARKER_SETUP_CFG,
        MARKER_SETUP_PY,
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bh-policy-table-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[derive(Default)]
    struct Events {
        discovered: Vec<Vec<u8>>,
        completed: usize,
        partials: Vec<bool>,
        finished: Option<u32>,
        calls: usize,
    }
    unsafe extern "C" fn collect_events(context: *mut c_void, event: *const crate::BHScanEvent) {
        let state = unsafe { &mut *context.cast::<Events>() };
        state.calls += 1;
        let event = unsafe { &*event };
        match event.event_type {
            1 => state
                .discovered
                .push(unsafe { std::slice::from_raw_parts(event.path, event.path_len) }.to_vec()),
            2 => {
                state.completed += 1;
                state.partials.push(event.partial != 0);
            }
            4 => state.finished = Some(event.status),
            _ => {}
        }
    }
    unsafe extern "C" fn classify_build(
        _context: *mut c_void,
        facts: *const BHCandidateFacts,
        decision: *mut BHCandidateDecision,
    ) -> i32 {
        let facts = unsafe { &*facts };
        let name = unsafe { std::slice::from_raw_parts(facts.node_name, facts.node_name_len) };
        if facts.is_directory != 0 && name == b".build" {
            unsafe {
                *decision = BHCandidateDecision {
                    action: 2,
                    language: 1,
                    kind: 1,
                };
            }
        }
        1
    }
    struct MutationContext {
        events: Events,
        decisions: *mut BHCandidateDecision,
        changed: bool,
    }
    unsafe extern "C" fn mutate_input_on_discovery(
        context: *mut c_void,
        event: *const crate::BHScanEvent,
    ) {
        let state = unsafe { &mut *context.cast::<MutationContext>() };
        let event = unsafe { &*event };
        if event.event_type == 1 && !state.changed {
            state.changed = true;
            for i in 0..CELL_COUNT {
                unsafe {
                    (*state.decisions.add(i)).action = 0;
                }
            }
        }
        unsafe { collect_events((&mut state.events as *mut Events).cast(), event) };
    }
    struct CancelContext {
        control: *mut ScanControl,
        events: Events,
    }
    unsafe extern "C" fn cancel_on_discovery(
        context: *mut c_void,
        event: *const crate::BHScanEvent,
    ) {
        let state = unsafe { &mut *context.cast::<CancelContext>() };
        if unsafe { (*event).event_type } == 1 {
            unsafe { &*state.control }.cancel();
        }
        unsafe { collect_events((&mut state.events as *mut Events).cast(), event) };
    }

    fn facts(name: &[u8], flags: u8) -> CandidateFacts {
        CandidateFacts {
            node_name: name.to_vec(),
            is_directory: flags & 1 != 0,
            is_symbolic_link: flags & 2 != 0,
            has_artifact_ancestor: flags & 4 != 0,
            own_marker_files: if flags & 8 != 0 { MARKER_PYVENV_CFG } else { 0 },
            parent_marker_files: (if flags & 16 != 0 {
                MARKER_CARGO_TOML
            } else {
                0
            }) | (if flags & 32 != 0 {
                MARKER_PYPROJECT_TOML
            } else {
                0
            }) | (if flags & 64 != 0 { MARKER_SETUP_PY } else { 0 })
                | (if flags & 128 != 0 {
                    MARKER_SETUP_CFG
                } else {
                    0
                }),
        }
    }

    #[test]
    fn all_projection_cells_round_trip_and_ignore_unknown_marker_bits() {
        for (class, representative) in REPRESENTATIVES.iter().enumerate() {
            for flags in 0..=255u8 {
                let f = facts(representative, flags);
                assert_eq!(project_cell(&f), class * 256 + usize::from(flags));
                let mut c = BHCandidateFacts {
                    node_name: std::ptr::null(),
                    node_name_len: 0,
                    is_directory: 0,
                    is_symbolic_link: 0,
                    has_artifact_ancestor: 0,
                    own_marker_files: 0,
                    parent_marker_files: 0,
                };
                assert_eq!(
                    unsafe { bh_policy_table_cell_facts(class * 256 + usize::from(flags), &mut c) },
                    1
                );
                assert_eq!(
                    unsafe { std::slice::from_raw_parts(c.node_name, c.node_name_len) },
                    *representative
                );
                assert_eq!(c.is_directory, flags & 1);
                assert_eq!(c.is_symbolic_link, (flags >> 1) & 1);
                assert_eq!(c.has_artifact_ancestor, (flags >> 2) & 1);
                assert_eq!(
                    c.own_marker_files,
                    if flags & 8 != 0 { MARKER_PYVENV_CFG } else { 0 }
                );
                assert_eq!(c.parent_marker_files & 0xf, u32::from(flags >> 4));
            }
        }
        let base = facts(b"different-name", 0);
        let mut with_unknown = base.clone();
        with_unknown.own_marker_files |= 1 << 29;
        with_unknown.parent_marker_files |= 1 << 30;
        assert_eq!(project_cell(&base), project_cell(&with_unknown));
    }

    #[test]
    fn projection_uses_actual_names_exact_precedence_suffixes_and_invalid_cell() {
        for (name, class) in [
            (b".build".as_slice(), 2),
            (b"my.pyc", 16),
            (b"my.pyo", 17),
            (b"anything.egg-info", 15),
            (b"module.pyc.egg-info", 15),
            (b"module.egg-info.pyc", 16),
            (b"odd.build", 0),
            (b"venv.pyc", 16),
        ] {
            assert_eq!(project_cell(&facts(name, 0)) / 256, class, "{name:?}");
        }
        assert_eq!(project_cell(&facts(b"\xff", 0)), INVALID_UTF8_CELL);
        let mut output = BHCandidateFacts {
            node_name: std::ptr::null(),
            node_name_len: 0,
            is_directory: 0,
            is_symbolic_link: 0,
            has_artifact_ancestor: 0,
            own_marker_files: 0,
            parent_marker_files: 0,
        };
        assert_eq!(
            unsafe { bh_policy_table_cell_facts(INVALID_UTF8_CELL, &mut output) },
            1
        );
        assert_eq!(
            unsafe { std::slice::from_raw_parts(output.node_name, output.node_name_len) },
            &[0xff]
        );
        assert_eq!(
            unsafe { bh_policy_table_cell_facts(CELL_COUNT, &mut output) },
            0
        );
        assert_eq!(
            unsafe { bh_policy_table_cell_facts(0, std::ptr::null_mut()) },
            0
        );
    }

    #[test]
    fn table_ffi_rejects_invalid_arguments_without_calling_events() {
        let tree = Tree::new();
        let root = tree.0.as_os_str().as_encoded_bytes();
        let control = crate::bh_scan_control_create();
        let mut table = vec![BHCandidateDecision::default(); CELL_COUNT];
        let mut events = Events::default();
        let context = (&mut events as *mut Events).cast();
        let call = |version, count, decisions, callback| unsafe {
            crate::bh_scan_with_policy_table(
                control,
                root.as_ptr(),
                root.len(),
                1,
                version,
                decisions,
                count,
                callback,
                context,
            )
        };
        assert_eq!(
            call(
                VERSION + 1,
                CELL_COUNT,
                table.as_ptr(),
                Some(collect_events)
            ),
            3
        );
        assert_eq!(
            call(
                VERSION,
                CELL_COUNT - 1,
                table.as_ptr(),
                Some(collect_events)
            ),
            3
        );
        assert_eq!(
            call(VERSION, CELL_COUNT, std::ptr::null(), Some(collect_events)),
            3
        );
        assert_eq!(call(VERSION, CELL_COUNT, table.as_ptr(), None), 3);
        table[0].action = 3;
        assert_eq!(
            call(VERSION, CELL_COUNT, table.as_ptr(), Some(collect_events)),
            3
        );
        unsafe {
            crate::bh_scan_control_destroy(control);
        }
        assert_eq!(events.calls, 0);
    }

    #[test]
    fn table_scan_matches_callback_events_and_uses_an_owned_table_copy() {
        let tree = Tree::new();
        fs::create_dir_all(tree.0.join("one/.build")).unwrap();
        fs::create_dir_all(tree.0.join("two/.build")).unwrap();
        let mut table = vec![BHCandidateDecision::default(); CELL_COUNT];
        table[2 * 256 + 1] = BHCandidateDecision {
            action: 2,
            language: 1,
            kind: 1,
        };
        let root = tree.0.as_os_str().as_encoded_bytes();

        let legacy_control = crate::bh_scan_control_create();
        let mut legacy = Events::default();
        let legacy_status = unsafe {
            let status = crate::bh_scan(
                legacy_control,
                root.as_ptr(),
                root.len(),
                1,
                Some(classify_build),
                Some(collect_events),
                (&mut legacy as *mut Events).cast(),
            );
            crate::bh_scan_control_destroy(legacy_control);
            status
        };
        let table_control = crate::bh_scan_control_create();
        let mut state = MutationContext {
            events: Events::default(),
            decisions: table.as_mut_ptr(),
            changed: false,
        };
        let table_status = unsafe {
            let status = crate::bh_scan_with_policy_table(
                table_control,
                root.as_ptr(),
                root.len(),
                1,
                VERSION,
                table.as_ptr(),
                table.len(),
                Some(mutate_input_on_discovery),
                (&mut state as *mut MutationContext).cast(),
            );
            crate::bh_scan_control_destroy(table_control);
            status
        };
        legacy.discovered.sort();
        state.events.discovered.sort();
        assert_eq!(legacy_status, 0);
        assert_eq!(table_status, 0);
        assert!(state.changed);
        assert_eq!(legacy.discovered, state.events.discovered);
        assert_eq!(
            legacy.discovered,
            ["one", "two"].map(|name| crate::path_bytes(&PathBuf::from(name).join(".build")))
        );
        assert_eq!(legacy.completed, state.events.completed);
        assert_eq!(legacy.finished, Some(0));
        assert_eq!(state.events.finished, Some(0));
        assert_eq!(table.iter().filter(|d| d.action == 2).count(), 0);
    }

    #[test]
    fn table_scan_preserves_cancellation_and_partial_completion_events() {
        let tree = Tree::new();
        fs::create_dir_all(tree.0.join(".build")).unwrap();
        fs::write(tree.0.join(".build/output"), vec![0; 64]).unwrap();
        let mut table = vec![BHCandidateDecision::default(); CELL_COUNT];
        table[2 * 256 + 1] = BHCandidateDecision {
            action: 2,
            language: 1,
            kind: 1,
        };
        let root = tree.0.as_os_str().as_encoded_bytes();
        let control = crate::bh_scan_control_create();
        let mut state = CancelContext {
            control,
            events: Events::default(),
        };
        let status = unsafe {
            let result = crate::bh_scan_with_policy_table(
                control,
                root.as_ptr(),
                root.len(),
                1,
                VERSION,
                table.as_ptr(),
                table.len(),
                Some(cancel_on_discovery),
                (&mut state as *mut CancelContext).cast(),
            );
            crate::bh_scan_control_destroy(control);
            result
        };
        assert_eq!(status, 1);
        assert_eq!(state.events.finished, Some(1));
        assert_eq!(state.events.completed, 1);
        assert_eq!(state.events.partials, [true]);
    }
}
