#![cfg(any(target_os = "macos", target_os = "linux"))]
use hunter_fs::{FileFact, Location, ScopedFilesystem};
use hunter_fs_core::{DirectoryEvent, DirectoryVisit, ScanAccess};
use std::{
    ffi::OsStr,
    fs, io,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Tree(PathBuf);

#[test]
fn relative_roots_keep_the_callers_identity_for_all_acquisition_and_markers() {
    use std::os::unix::fs::MetadataExt;
    // Cargo sets this integration test's cwd to its package root. No process-wide
    // chdir or fixture creation in the checkout is needed for this relative case.
    for identity in [Path::new("."), Path::new("crates/hunter-fs-core")] {
        let native = fs::symlink_metadata(identity).unwrap();
        let scope = ScopedFilesystem::open(identity).unwrap();
        assert_eq!(scope.root_metadata(identity).unwrap().ino(), native.ino());
        assert_eq!(
            scope
                .prepare_directory(identity)
                .unwrap()
                .metadata()
                .unwrap()
                .ino(),
            native.ino()
        );
        let paths = entries(&scope, identity);
        assert!(!paths.is_empty());
        assert!(
            paths
                .iter()
                .all(|path| !path.is_absolute() && path.strip_prefix(identity).is_ok())
        );
        assert_eq!(
            scope
                .marker_scope()
                .unwrap()
                .inspect_regular_file(identity, Location::Own, OsStr::new("Cargo.toml"))
                .fact,
            FileFact::Known(true)
        );
    }
}
impl Tree {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hunter-scoped-fs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn entries(scope: &ScopedFilesystem, path: &Path) -> Vec<PathBuf> {
    let mut found = vec![];
    let result = scope.visit_directory(path, &mut || false, &mut |event| match event {
        DirectoryEvent::Entry { path, .. } => found.push(path),
        DirectoryEvent::Error { path, error } => panic!("{path:?}: {error}"),
    });
    assert_eq!(result, DirectoryVisit::Completed);
    found.sort();
    found
}

#[test]
fn root_path_replacement_does_not_redirect_listing_measurement_or_markers() {
    let tree = Tree::new();
    let selected = tree.0.join("selected");
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("original"), b"original").unwrap();
    let outside = Tree::new();
    fs::write(outside.0.join("foreign"), b"foreign payload").unwrap();
    let scope = ScopedFilesystem::open(&selected).unwrap();
    fs::rename(&selected, tree.0.join("renamed")).unwrap();
    symlink(&outside.0, &selected).unwrap();

    assert_eq!(entries(&scope, &selected), [selected.join("original")]);
    assert!(scope.root_metadata(&selected).unwrap().is_dir());
    let pass = scope.prepare_directory(&selected).unwrap();
    assert_eq!(
        pass.entry_metadata(OsStr::new("original"))
            .unwrap()
            .size
            .apparent,
        8
    );
    assert_eq!(
        pass.entry_metadata(OsStr::new("foreign"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::NotFound
    );
    // Construct markers after replacement, not merely before the race.
    let markers = scope.marker_scope().unwrap();
    assert_eq!(
        markers
            .inspect_regular_file(&selected, Location::Own, OsStr::new("original"))
            .fact,
        FileFact::Known(true)
    );
    assert_eq!(
        markers
            .inspect_regular_file(&selected, Location::Own, OsStr::new("foreign"))
            .fact,
        FileFact::Known(false)
    );
}

#[test]
fn intermediate_link_replacement_blocks_every_later_path_acquisition() {
    let tree = Tree::new();
    let outside = Tree::new();
    fs::create_dir_all(tree.0.join("a/b")).unwrap();
    fs::create_dir(outside.0.join("b")).unwrap();
    fs::write(outside.0.join("b/foreign"), b"secret").unwrap();
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    fs::rename(tree.0.join("a"), tree.0.join("old-a")).unwrap();
    symlink(&outside.0, tree.0.join("a")).unwrap();
    assert!(scope.prepare_directory(&tree.0.join("a/b")).is_err());
    assert!(scope.root_metadata(&tree.0.join("a/b/foreign")).is_err());
    let mut errors = vec![];
    assert_eq!(
        scope.visit_directory(&tree.0.join("a/b"), &mut || false, &mut |event| {
            match event {
                DirectoryEvent::Error { path, error } => errors.push((path, error)),
                _ => panic!("replacement target must never be enumerated"),
            }
        }),
        DirectoryVisit::OpenFailed
    );
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, tree.0.join("a/b"));
}

#[test]
fn prepared_measurement_keeps_the_open_directory_after_regular_replacement() {
    let tree = Tree::new();
    let selected = tree.0.join("work");
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("data"), b"original").unwrap();
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    let pass = scope.prepare_directory(&selected).unwrap();
    let before = pass.metadata().unwrap();
    fs::rename(&selected, tree.0.join("old-work")).unwrap();
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("data"), b"replacement is different").unwrap();
    assert_eq!(
        pass.entry_metadata(OsStr::new("data"))
            .unwrap()
            .size
            .apparent,
        8
    );
    use std::os::unix::fs::MetadataExt;
    assert_eq!(pass.metadata().unwrap().ino(), before.ino());
}

#[test]
fn final_links_are_observed_as_links_including_dangling_and_directory_targets() {
    let tree = Tree::new();
    fs::write(tree.0.join("file"), b"unused").unwrap();
    fs::create_dir(tree.0.join("dir")).unwrap();
    for target in ["file", "dir", "missing"] {
        symlink(target, tree.0.join(format!("link-{target}"))).unwrap();
    }
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    let pass = scope.prepare_directory(&tree.0).unwrap();
    let mut links = 0;
    scope.visit_directory(&tree.0, &mut || false, &mut |event| {
        if let DirectoryEvent::Entry { path, file_type } = event {
            if path
                .file_name()
                .unwrap()
                .as_encoded_bytes()
                .starts_with(b"link-")
            {
                assert!(file_type.is_symlink());
                assert!(
                    pass.entry_metadata(path.file_name().unwrap())
                        .unwrap()
                        .file_type
                        .is_symlink()
                );
                links += 1;
            }
        } else {
            panic!("all links should have no-follow metadata");
        }
    });
    assert_eq!(links, 3);
    let selected_link = ScopedFilesystem::open(&tree.0.join("link-dir/")).unwrap();
    assert!(
        selected_link
            .root_metadata(&tree.0.join("link-dir/"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        selected_link
            .prepare_directory(&tree.0.join("link-dir/"))
            .is_err()
    );
}

#[test]
fn parallel_and_repeated_root_listings_have_independent_directory_offsets() {
    let tree = Tree::new();
    for n in 0..64 {
        fs::write(tree.0.join(format!("entry-{n}")), b"x").unwrap();
    }
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    std::thread::scope(|threads| {
        for _ in 0..8 {
            threads.spawn(|| {
                for _ in 0..4 {
                    assert_eq!(entries(&scope, &tree.0).len(), 64);
                }
            });
        }
    });
}

#[test]
fn invalid_candidates_and_child_names_cannot_escape_scope() {
    let tree = Tree::new();
    fs::write(tree.0.join("file"), b"unused").unwrap();
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    let pass = scope.prepare_directory(&tree.0).unwrap();
    for name in [
        "",
        ".",
        "..",
        "../file",
        "dir/file",
        "file/",
        "/absolute",
        "bad\0name",
    ] {
        assert_eq!(
            pass.entry_metadata(OsStr::new(name))
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL),
            "{name:?}"
        );
    }
    for candidate in [
        tree.0.join(".."),
        tree.0.join("dir/../file"),
        tree.0.with_extension("outside"),
    ] {
        assert!(scope.root_metadata(&candidate).is_err(), "{candidate:?}");
        assert!(
            scope.prepare_directory(&candidate).is_err(),
            "{candidate:?}"
        );
    }
}

#[test]
fn metadata_does_not_require_file_read_permission_or_read_fifo_contents() {
    let tree = Tree::new();
    let file = tree.0.join("unreadable");
    fs::write(&file, b"payload").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o0)).unwrap();
    let fifo = std::ffi::CString::new(tree.0.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: the path is a valid terminated string; mkfifo creates a test fixture.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    let pass = scope.prepare_directory(&tree.0).unwrap();
    assert_eq!(
        pass.entry_metadata(OsStr::new("unreadable"))
            .unwrap()
            .size
            .apparent,
        7
    );
    assert!(
        !pass
            .entry_metadata(OsStr::new("fifo"))
            .unwrap()
            .file_type
            .is_file()
    );
    assert_eq!(entries(&scope, &tree.0).len(), 2);
}

#[test]
fn cancellation_and_empty_or_failed_open_keep_the_visit_protocol() {
    let tree = Tree::new();
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    assert_eq!(
        scope.visit_directory(&tree.0, &mut || panic!("empty"), &mut |_| panic!("empty")),
        DirectoryVisit::Completed
    );
    fs::write(tree.0.join("one"), b"x").unwrap();
    assert_eq!(
        scope.visit_directory(&tree.0, &mut || true, &mut |_| panic!("cancelled")),
        DirectoryVisit::Cancelled
    );
    let mut errors = 0;
    assert_eq!(
        scope.visit_directory(
            &tree.0.join("missing"),
            &mut || panic!("failed open first"),
            &mut |event| {
                assert!(matches!(event, DirectoryEvent::Error { .. }));
                errors += 1;
            }
        ),
        DirectoryVisit::OpenFailed
    );
    assert_eq!(errors, 1);
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_names_survive_every_acquisition() {
    use std::os::unix::ffi::OsStrExt;
    let tree = Tree::new();
    let name = OsStr::from_bytes(b"raw-\xff");
    fs::write(tree.0.join(name), b"payload").unwrap();
    let scope = ScopedFilesystem::open(&tree.0).unwrap();
    assert_eq!(entries(&scope, &tree.0), [tree.0.join(name)]);
    assert_eq!(
        scope
            .prepare_directory(&tree.0)
            .unwrap()
            .entry_metadata(name)
            .unwrap()
            .size
            .apparent,
        7
    );
}
