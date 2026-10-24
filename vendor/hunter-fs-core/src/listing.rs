//! Owned listing acquisition with an opaque host context observer.
use crate::{DirectoryEvent, DirectoryListing, DirectoryVisit, visit_directory};
use std::{fs, io, path::Path};

/// Host-selected retention limits. Weight counts owned path capacities, entry slots
/// and formatted error length; it is an estimate, not allocator/RSS accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListingLimits {
    pub max_entries: usize,
    pub max_errors: usize,
    pub max_weight: usize,
}
impl ListingLimits {
    pub const UNBOUNDED: Self = Self {
        max_entries: usize::MAX,
        max_errors: usize::MAX,
        max_weight: usize::MAX,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListingLimit {
    Entries,
    Errors,
    Weight,
}

/// A retained prefix plus the acquisition outcome and optional retention limit.
/// A limit is not successful exhaustion and must be interpreted by the host.
pub struct BoundedDirectoryListing<Context> {
    pub listing: DirectoryListing<Context>,
    pub visit: DirectoryVisit,
    pub limit: Option<ListingLimit>,
    pub retained_weight: usize,
}

/// Limit owned retention while consuming an ordered visitor. The visitor must check
/// the supplied stop predicate between observations to stop acquisition promptly.
/// Uncooperative visitors can keep doing I/O, but subsequent events are not retained.
/// The rejected entry does not invoke observe. If observe fails after an entry is
/// admitted, that entry/context remain; an error that exceeds its budget is reported
/// as a limit. No extra stat/open, path normalization or fallback is introduced.
/// The already-created current event and opaque context are outside this estimate.
pub fn collect_directory_events_with_limits<Context>(
    context: Context,
    limits: ListingLimits,
    visit: impl FnOnce(&mut dyn FnMut(DirectoryEvent), &dyn Fn() -> bool) -> DirectoryVisit,
    mut observe: impl FnMut(&Path, crate::EntryType, &mut Context) -> io::Result<()>,
) -> BoundedDirectoryListing<Context> {
    let mut listing = DirectoryListing {
        children: Vec::new(),
        errors: Vec::new(),
        context,
    };
    let limit = std::cell::Cell::new(None);
    let mut retained_weight = 0;
    let visit = visit(
        &mut |event| {
            if limit.get().is_some() {
                return;
            }
            match event {
                DirectoryEvent::Error { path, error } => retain_error(
                    &mut listing,
                    path,
                    error,
                    limits,
                    &mut retained_weight,
                    &limit,
                ),
                DirectoryEvent::Entry { path, file_type } => {
                    if listing.children.len() >= limits.max_entries {
                        limit.set(Some(ListingLimit::Entries));
                        return;
                    }
                    let weight = path
                        .capacity()
                        .saturating_add(
                            std::mem::size_of::<(std::path::PathBuf, crate::EntryType)>(),
                        );
                    if weight > limits.max_weight.saturating_sub(retained_weight) {
                        limit.set(Some(ListingLimit::Weight));
                        return;
                    }
                    retained_weight += weight;
                    if let Err(error) = observe(&path, file_type, &mut listing.context) {
                        retain_error(
                            &mut listing,
                            path.clone(),
                            error,
                            limits,
                            &mut retained_weight,
                            &limit,
                        );
                    }
                    listing.children.push((path, file_type));
                }
            }
        },
        &|| limit.get().is_some(),
    );
    BoundedDirectoryListing {
        listing,
        visit,
        limit: limit.get(),
        retained_weight,
    }
}

fn retain_error<Context>(
    listing: &mut DirectoryListing<Context>,
    path: std::path::PathBuf,
    error: io::Error,
    limits: ListingLimits,
    weight: &mut usize,
    limit: &std::cell::Cell<Option<ListingLimit>>,
) {
    if listing.errors.len() >= limits.max_errors {
        limit.set(Some(ListingLimit::Errors));
        return;
    }
    let cost = path
        .capacity()
        .saturating_add(std::mem::size_of::<(std::path::PathBuf, io::Error)>())
        .saturating_add(error.to_string().len());
    if cost > limits.max_weight.saturating_sub(*weight) {
        limit.set(Some(ListingLimit::Weight));
        return;
    }
    *weight += cost;
    listing.errors.push((path, error));
}

/// Portable equivalent of the bounded event collector; preserves open/iterator/type
/// errors and combines the caller stop predicate with the retention stop request.
pub fn collect_directory_with_limits<I, Context>(
    path: &Path,
    entries: io::Result<I>,
    context: Context,
    limits: ListingLimits,
    mut should_stop: impl FnMut() -> bool,
    observe: impl FnMut(&Path, crate::EntryType, &mut Context) -> io::Result<()>,
) -> BoundedDirectoryListing<Context>
where
    I: IntoIterator<Item = io::Result<fs::DirEntry>>,
{
    collect_directory_events_with_limits(
        context,
        limits,
        |emit, budget_stop| visit_directory(path, entries, || should_stop() || budget_stop(), emit),
        observe,
    )
}

/// Consume host-opened entries in order, observing each successful entry before
/// retaining it. Observer failure adds a child-qualified error without removing
/// the child or rolling back context changes. Iterator/type failures keep the
/// existing visit_directory path/error semantics and never invoke the observer.
///
/// Context is opaque and may borrow non-Send state. Host owns open/guard lifetime,
/// observer effects and cancellation policy; callback panics propagate. Cancellation
/// retains the current observed entry and context before the next visitor stop check.
/// Failed opens are returned as OpenFailed even when stopped; empty input completes.
/// This adds no second listing buffer, path normalization, content reads or snapshot.
pub fn collect_directory<I, Context>(
    path: &Path,
    entries: io::Result<I>,
    context: Context,
    should_stop: impl FnMut() -> bool,
    observe: impl FnMut(&Path, crate::EntryType, &mut Context) -> io::Result<()>,
) -> (DirectoryListing<Context>, DirectoryVisit)
where
    I: IntoIterator<Item = io::Result<fs::DirEntry>>,
{
    collect_directory_events(
        context,
        |emit| visit_directory(path, entries, should_stop, emit),
        observe,
    )
}

/// Collect the same owned listing from an alternative acquisition backend. The
/// visitor owns opening, stop checks and error ordering; the observer and retained
/// context follow exactly the same rules as `collect_directory`.
pub fn collect_directory_events<Context>(
    context: Context,
    visit: impl FnOnce(&mut dyn FnMut(DirectoryEvent)) -> DirectoryVisit,
    mut observe: impl FnMut(&Path, crate::EntryType, &mut Context) -> io::Result<()>,
) -> (DirectoryListing<Context>, DirectoryVisit) {
    let mut listing = DirectoryListing {
        children: Vec::new(),
        errors: Vec::new(),
        context,
    };
    let outcome = visit(&mut |event| match event {
        DirectoryEvent::Error { path, error } => listing.errors.push((path, error)),
        DirectoryEvent::Entry { path, file_type } => {
            if let Err(error) = observe(&path, file_type, &mut listing.context) {
                listing.errors.push((path.clone(), error));
            }
            listing.children.push((path, file_type));
        }
    });
    (listing, outcome)
}

#[cfg(test)]
mod tests {
    use crate::{
        DirectoryEvent, DirectoryVisit, EntryType, ListingLimit, ListingLimits, collect_directory,
        collect_directory_events_with_limits, collect_directory_with_limits,
    };
    use std::{
        cell::Cell,
        fs, io,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "hunter-listing-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn entries(&self) -> Vec<io::Result<fs::DirEntry>> {
            let mut entries: Vec<_> = fs::read_dir(&self.0).unwrap().map(Result::unwrap).collect();
            entries.sort_by_key(fs::DirEntry::path);
            entries.into_iter().map(Ok).collect()
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn bounded_retention_does_not_observe_or_keep_the_rejected_entry() {
        let tree = Tree::new();
        for name in ["a", "b", "c"] {
            fs::write(tree.0.join(name), b"x").unwrap();
        }
        let calls = Cell::new(0);
        let result = collect_directory_with_limits(
            &tree.0,
            Ok(tree.entries()),
            0,
            ListingLimits {
                max_entries: 1,
                ..ListingLimits::UNBOUNDED
            },
            || false,
            |_, _, count| {
                calls.set(calls.get() + 1);
                *count += 1;
                Ok(())
            },
        );
        assert_eq!(result.limit, Some(ListingLimit::Entries));
        assert_eq!(result.visit, DirectoryVisit::Cancelled);
        assert_eq!(result.listing.children.len(), 1);
        assert_eq!(result.listing.children[0].0, tree.0.join("a"));
        assert_eq!((calls.get(), result.listing.context), (1, 1));
    }

    #[test]
    fn bounded_weight_and_errors_are_explicit_even_for_uncooperative_visitors() {
        let result = collect_directory_events_with_limits(
            (),
            ListingLimits {
                max_weight: 1,
                ..ListingLimits::UNBOUNDED
            },
            |emit, _| {
                for _ in 0..10_000 {
                    emit(DirectoryEvent::Entry {
                        path: "large-path".into(),
                        file_type: EntryType::RegularFile,
                    });
                }
                DirectoryVisit::Completed
            },
            |_, _, _| panic!("over-budget entries must not be observed"),
        );
        assert_eq!(result.limit, Some(ListingLimit::Weight));
        assert!(result.listing.children.is_empty());
        assert_eq!(result.retained_weight, 0);
        let result = collect_directory_events_with_limits(
            (),
            ListingLimits {
                max_errors: 1,
                ..ListingLimits::UNBOUNDED
            },
            |emit, _| {
                for _ in 0..10_000 {
                    emit(DirectoryEvent::Error {
                        path: "entry".into(),
                        error: io::Error::from_raw_os_error(13),
                    });
                }
                DirectoryVisit::Completed
            },
            |_, _, _| Ok(()),
        );
        assert_eq!(result.limit, Some(ListingLimit::Errors));
        assert_eq!(result.listing.errors.len(), 1);
        assert_eq!(result.listing.errors[0].1.raw_os_error(), Some(13));
    }

    #[test]
    fn admitted_entry_and_context_survive_an_observer_error_budget() {
        let result = collect_directory_events_with_limits(
            0,
            ListingLimits {
                max_errors: 0,
                ..ListingLimits::UNBOUNDED
            },
            |emit, stop| {
                emit(DirectoryEvent::Entry {
                    path: "entry".into(),
                    file_type: EntryType::Directory,
                });
                assert!(stop());
                DirectoryVisit::Cancelled
            },
            |_, _, count| {
                *count += 1;
                Err(io::Error::from_raw_os_error(5))
            },
        );
        assert_eq!(result.limit, Some(ListingLimit::Errors));
        assert_eq!(result.listing.children.len(), 1);
        assert_eq!(result.listing.context, 1);
        assert!(result.listing.errors.is_empty());
    }

    #[test]
    fn bounded_open_failure_and_exact_limit_are_not_rewritten_as_exhaustion() {
        let tree = Tree::new();
        fs::write(tree.0.join("one"), b"x").unwrap();
        let result = collect_directory_with_limits(
            &tree.0,
            Ok(tree.entries()),
            (),
            ListingLimits {
                max_entries: 1,
                ..ListingLimits::UNBOUNDED
            },
            || false,
            |_, _, _| Ok(()),
        );
        assert_eq!(result.limit, None);
        assert_eq!(result.visit, DirectoryVisit::Completed);
        let result = collect_directory_with_limits::<Vec<io::Result<fs::DirEntry>>, _>(
            &tree.0,
            Err(io::Error::from_raw_os_error(13)),
            (),
            ListingLimits::UNBOUNDED,
            || true,
            |_, _, _| Ok(()),
        );
        assert_eq!(result.visit, DirectoryVisit::OpenFailed);
        assert_eq!(result.limit, None);
        assert_eq!(result.listing.errors[0].1.raw_os_error(), Some(13));
    }

    #[test]
    fn ordered_entries_and_observer_errors_retain_children_and_borrowed_context() {
        let tree = Tree::new();
        fs::write(tree.0.join("a"), b"a").unwrap();
        fs::write(tree.0.join("b"), b"b").unwrap();
        let entries = std::iter::once(Err(io::Error::from_raw_os_error(13))).chain(tree.entries());
        // No Clone, Default or Send bound is needed for the opaque context.
        struct Context<'a>(&'a Cell<usize>);
        let calls = Cell::new(0);
        let mut visited = vec![];
        let (listing, outcome) = collect_directory(
            &tree.0,
            Ok(entries),
            Context(&calls),
            || false,
            |path, kind, context| {
                assert!(kind.is_file());
                visited.push(path.to_owned());
                context.0.set(context.0.get() + 1);
                if path.file_name() == Some(std::ffi::OsStr::new("a")) {
                    Err(io::Error::from_raw_os_error(5))
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(outcome, DirectoryVisit::Completed);
        assert_eq!(
            listing
                .children
                .iter()
                .map(|(p, _)| p.clone())
                .collect::<Vec<_>>(),
            visited
        );
        assert_eq!(visited, [tree.0.join("a"), tree.0.join("b")]);
        assert_eq!(listing.context.0.get(), 2);
        assert_eq!(listing.errors.len(), 2);
        assert_eq!(listing.errors[0].0, tree.0);
        assert_eq!(listing.errors[0].1.raw_os_error(), Some(13));
        assert_eq!(listing.errors[1].0, tree.0.join("a"));
        assert_eq!(listing.errors[1].1.raw_os_error(), Some(5));
    }
    #[test]
    fn observer_cancellation_keeps_current_entry_and_stops_before_next_observer() {
        let tree = Tree::new();
        for name in ["a", "b"] {
            fs::write(tree.0.join(name), b"data").unwrap();
        }
        let stop = Cell::new(false);
        let (listing, outcome) = collect_directory(
            &tree.0,
            Ok(tree.entries()),
            0,
            || stop.get(),
            |_, _, count| {
                *count += 1;
                stop.set(true);
                Ok(())
            },
        );
        assert_eq!(outcome, DirectoryVisit::Cancelled);
        assert_eq!(listing.context, 1);
        assert_eq!(listing.children.len(), 1);
        assert_eq!(listing.children[0].0, tree.0.join("a"));
    }
    #[test]
    fn open_failure_precedes_stop_and_preserves_context_path_and_error() {
        let path = Path::new("literal/../missing");
        let (listing, outcome) = collect_directory(
            path,
            Err::<fs::ReadDir, _>(io::Error::from_raw_os_error(13)),
            17,
            || true,
            |_, _, _| panic!("failed open"),
        );
        assert_eq!(outcome, DirectoryVisit::OpenFailed);
        assert_eq!(listing.context, 17);
        assert!(listing.children.is_empty());
        assert_eq!(listing.errors.len(), 1);
        assert_eq!(listing.errors[0].0, path);
        assert_eq!(listing.errors[0].1.raw_os_error(), Some(13));
    }
    #[test]
    fn empty_precancelled_listing_is_completed_without_callbacks() {
        let (listing, outcome) = collect_directory(
            Path::new("unused"),
            Ok(std::iter::empty()),
            (),
            || panic!("empty stop"),
            |_, _, _| panic!("empty observer"),
        );
        assert_eq!(outcome, DirectoryVisit::Completed);
        assert!(listing.children.is_empty() && listing.errors.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn observed_links_remain_links_without_resolving_targets() {
        let tree = Tree::new();
        fs::create_dir(tree.0.join("dir")).unwrap();
        fs::write(tree.0.join("file"), b"data").unwrap();
        for target in ["dir", "file", "missing"] {
            std::os::unix::fs::symlink(target, tree.0.join(format!("link-{target}"))).unwrap();
        }
        let (listing, outcome) = collect_directory(
            &tree.0,
            Ok(tree.entries()),
            0,
            || false,
            |_, kind, links| {
                if kind.is_symlink() {
                    *links += 1;
                }
                Ok(())
            },
        );
        assert_eq!(outcome, DirectoryVisit::Completed);
        assert_eq!(listing.context, 3);
        assert_eq!(listing.children.len(), 5);
        assert!(listing.errors.is_empty());
    }
    #[test]
    fn observer_panic_propagates_before_visiting_later_entries() {
        let tree = Tree::new();
        for name in ["a", "b"] {
            fs::write(tree.0.join(name), b"data").unwrap();
        }
        let visited = Cell::new(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            collect_directory(
                &tree.0,
                Ok(tree.entries()),
                (),
                || false,
                |_, _, _| {
                    visited.set(visited.get() + 1);
                    panic!("observer panic")
                },
            )
        }));
        assert!(result.is_err());
        assert_eq!(visited.get(), 1);
    }
    #[test]
    fn consumes_supplied_iterator_without_opening_or_rewriting_its_paths() {
        let tree = Tree::new();
        fs::write(tree.0.join("file"), b"data").unwrap();
        let (listing, outcome) = collect_directory(
            &tree.0.join("never-created/.."),
            fs::read_dir(&tree.0),
            (),
            || false,
            |_, _, _| Ok(()),
        );
        assert_eq!(outcome, DirectoryVisit::Completed);
        assert_eq!(listing.children[0].0, tree.0.join("file"));
        assert!(listing.errors.is_empty());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn raw_names_reach_observer_and_listing_without_decoding() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let tree = Tree::new();
        let path = tree.0.join(OsStr::from_bytes(b"raw-\xff"));
        fs::write(&path, b"data").unwrap();
        let (listing, outcome) = collect_directory(
            &tree.0,
            fs::read_dir(&tree.0),
            (),
            || false,
            |seen, _, _| {
                assert_eq!(seen, path);
                Ok(())
            },
        );
        assert_eq!(outcome, DirectoryVisit::Completed);
        assert_eq!(listing.children[0].0, path);
    }
}
