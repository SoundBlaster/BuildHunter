//! Streaming child selection for host-coordinated recursive traversal.
use crate::DirectoryTask;
use std::{
    fs,
    iter::Enumerate,
    path::{Path, PathBuf},
    vec::IntoIter,
};

/// A host-selected directory without an assigned coordinator ID.
#[derive(Debug)]
pub struct DirectoryRequest<Context = ()> {
    pub path: PathBuf,
    pub context: Context,
    pub measure: bool,
}
impl<Context> DirectoryRequest<Context> {
    /// The host assigns identity and retains responsibility for scheduling/accounting.
    pub fn into_task<Id>(self, id: Id) -> DirectoryTask<Id, Context> {
        DirectoryTask {
            id,
            path: self.path,
            context: self.context,
            measure: self.measure,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildDecision {
    Visit,
    Skip,
    Stop,
}

/// Original listing positions survive host filtering and are usable for measurement results.
#[derive(Debug)]
pub enum PlannedChild<Context = ()> {
    Directory {
        index: usize,
        request: DirectoryRequest<Context>,
    },
    Entry {
        index: usize,
        path: PathBuf,
        file_type: fs::FileType,
    },
}

/// Consumes the existing listing without building a second child/task vector.
/// Context is opaque and cloned only for admitted directory requests. Observed
/// types are used as supplied, without I/O; later replacements are not detected.
/// Does not recurse, enqueue, assign IDs or enforce host permissions itself.
/// Drop the plan to release the listing buffer and any unconsumed suffix after Stop.
#[derive(Debug)]
pub struct ChildTraversal<Context = ()> {
    children: Enumerate<IntoIter<(PathBuf, fs::FileType)>>,
    context: Context,
    measure: bool,
    stopped: bool,
}
impl<Context: Clone> ChildTraversal<Context> {
    pub fn new(children: Vec<(PathBuf, fs::FileType)>, context: Context, measure: bool) -> Self {
        Self {
            children: children.into_iter().enumerate(),
            context,
            measure,
            stopped: false,
        }
    }

    /// Calls the host before routing each original slot, including entries it skips.
    /// Host may emit checkpoints, apply access/selection policy and return Stop.
    /// Stop is terminal; an empty/exhausted listing never invokes the callback.
    /// Directory symlinks remain Entry values when supplied as symlink observations.
    pub fn next_child(
        &mut self,
        mut select: impl FnMut(usize, &Path, fs::FileType) -> ChildDecision,
    ) -> Option<PlannedChild<Context>> {
        if self.stopped {
            return None;
        }
        for (index, (path, file_type)) in self.children.by_ref() {
            match select(index, &path, file_type) {
                ChildDecision::Skip => continue,
                ChildDecision::Stop => {
                    self.stopped = true;
                    return None;
                }
                ChildDecision::Visit => {}
            }
            return Some(if file_type.is_dir() {
                PlannedChild::Directory {
                    index,
                    request: DirectoryRequest {
                        path,
                        context: self.context.clone(),
                        measure: self.measure,
                    },
                }
            } else {
                PlannedChild::Entry {
                    index,
                    path,
                    file_type,
                }
            });
        }
        None
    }

    /// Distinguishes host cancellation from ordinary completion or filtering all slots.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        fs,
        path::PathBuf,
        rc::Rc,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "hunter-children-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            fs::create_dir(p.join("dir")).unwrap();
            fs::write(p.join("file"), b"x").unwrap();
            Self(p)
        }
        fn entries(&self, names: &[&str]) -> Vec<(PathBuf, fs::FileType)> {
            names
                .iter()
                .map(|n| {
                    let p = self.0.join(n);
                    let kind = fs::symlink_metadata(&p).unwrap().file_type();
                    (p, kind)
                })
                .collect()
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[derive(Debug)]
    struct Context {
        value: u32,
        clones: Rc<Cell<usize>>,
    }
    impl Clone for Context {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                value: self.value,
                clones: self.clones.clone(),
            }
        }
    }
    #[test]
    fn selection_preserves_order_indices_and_clones_context_only_for_directories() {
        let t = Tree::new();
        let clones = Rc::new(Cell::new(0));
        let mut plan = ChildTraversal::new(
            t.entries(&["dir", "dir", "file"]),
            Context {
                value: 17,
                clones: clones.clone(),
            },
            true,
        );
        let mut checked = Vec::new();
        let first = plan
            .next_child(|index, _, _| {
                checked.push(index);
                if index == 0 {
                    ChildDecision::Skip
                } else {
                    ChildDecision::Visit
                }
            })
            .unwrap();
        match first {
            PlannedChild::Directory { index, request } => {
                assert_eq!(index, 1);
                let task = request.into_task(42);
                assert_eq!(task.id, 42);
                assert_eq!(task.path, t.0.join("dir"));
                assert!(task.measure);
                assert_eq!(task.context.value, 17);
            }
            _ => panic!("directory request"),
        }
        match plan
            .next_child(|index, _, _| {
                checked.push(index);
                ChildDecision::Visit
            })
            .unwrap()
        {
            PlannedChild::Entry {
                index,
                path,
                file_type,
            } => {
                assert_eq!(index, 2);
                assert_eq!(path, t.0.join("file"));
                assert!(file_type.is_file());
            }
            _ => panic!("leaf entry"),
        }
        assert!(plan.next_child(|_, _, _| panic!("exhausted")).is_none());
        assert!(!plan.is_stopped());
        assert_eq!(checked, [0, 1, 2]);
        assert_eq!(clones.get(), 1);
    }
    #[test]
    fn stop_is_terminal_and_keeps_already_returned_prefix() {
        let t = Tree::new();
        let mut plan = ChildTraversal::new(t.entries(&["file", "dir", "file"]), (), false);
        assert!(plan.next_child(|_, _, _| ChildDecision::Visit).is_some());
        let mut seen = Vec::new();
        assert!(
            plan.next_child(|i, _, _| {
                seen.push(i);
                ChildDecision::Stop
            })
            .is_none()
        );
        assert!(plan.is_stopped());
        assert_eq!(seen, [1]);
        assert!(
            plan.next_child(|_, _, _| panic!("must not resume"))
                .is_none()
        );
    }
    #[test]
    fn empty_or_all_skipped_listing_completes_without_stop() {
        let t = Tree::new();
        let mut plan = ChildTraversal::new(Vec::new(), (), false);
        assert!(plan.next_child(|_, _, _| panic!("empty")).is_none());
        assert!(!plan.is_stopped());
        let mut plan = ChildTraversal::new(t.entries(&["dir", "file"]), (), false);
        assert!(plan.next_child(|_, _, _| ChildDecision::Skip).is_none());
        assert!(!plan.is_stopped());
    }
    #[test]
    fn routing_uses_supplied_observation_without_new_io() {
        let t = Tree::new();
        let entries = t.entries(&["dir", "file"]);
        fs::remove_dir(t.0.join("dir")).unwrap();
        fs::remove_file(t.0.join("file")).unwrap();
        let mut plan = ChildTraversal::new(entries, 7, false);
        match plan.next_child(|_, _, _| ChildDecision::Visit).unwrap() {
            PlannedChild::Directory { request, .. } => {
                assert!(!request.measure);
                assert_eq!(request.context, 7);
            }
            _ => panic!("observed directory"),
        }
        assert!(matches!(
            plan.next_child(|_, _, _| ChildDecision::Visit),
            Some(PlannedChild::Entry { index: 1, .. })
        ));
    }
    #[cfg(unix)]
    #[test]
    fn directory_symlink_is_a_leaf_and_raw_path_is_preserved() {
        use std::os::unix::ffi::OsStrExt;
        let t = Tree::new();
        std::os::unix::fs::symlink("dir", t.0.join("link")).unwrap();
        let mut entries = t.entries(&["link"]);
        let raw = PathBuf::from(std::ffi::OsStr::from_bytes(b"raw-\xff"));
        entries[0].0 = raw.clone();
        let mut plan = ChildTraversal::new(entries, (), true);
        match plan.next_child(|_, _, _| ChildDecision::Visit).unwrap() {
            PlannedChild::Entry {
                path,
                file_type,
                index,
            } => {
                assert_eq!(path, raw);
                assert!(file_type.is_symlink());
                assert_eq!(index, 0);
            }
            _ => panic!("links never become directory requests"),
        }
    }
}
