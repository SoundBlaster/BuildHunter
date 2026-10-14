//! Descriptor-relative marker observations. Never open/read marker contents.
use buildhunter_plugin_policy::{MarkerFact, MarkerProbe, Scope as MarkerScope};
use std::{
    ffi::CString,
    fs::{File, OpenOptions},
    io,
    os::fd::{AsRawFd, FromRawFd},
    os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::{Component, Path, PathBuf},
};

pub(super) struct Scope {
    root: PathBuf,
    directory: Option<File>,
}
impl Scope {
    pub(super) fn new(root: &Path, needs_markers: bool) -> io::Result<Self> {
        let directory = if needs_markers && std::fs::symlink_metadata(root)?.is_dir() {
            Some(
                OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(root)?,
            )
        } else {
            None
        };
        Ok(Self {
            root: root.into(),
            directory,
        })
    }
    pub(super) fn observe(
        &self,
        candidate: &Path,
        probe: &MarkerProbe,
    ) -> (MarkerFact<'static>, bool) {
        let relative = match candidate.strip_prefix(&self.root) {
            Ok(relative) => relative,
            Err(_) => return (unknown("out_of_scope"), false),
        };
        let directory = match probe.scope {
            MarkerScope::Own => relative,
            MarkerScope::Parent => {
                if relative.as_os_str().is_empty() {
                    return (unknown("out_of_scope"), false);
                }
                relative.parent().unwrap_or(Path::new(""))
            }
        };
        let fd = match self.directory(directory) {
            Ok(fd) => fd,
            Err(e) => return (unknown(reason(&e)), false),
        };
        let name = CString::new(probe.name.as_bytes()).expect("validated single component");
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fd is owned/alive; name is NUL-terminated; fstatat writes one stat.
        // AT_SYMLINK_NOFOLLOW inspects the entry itself, never its target or contents.
        let result = unsafe {
            libc::fstatat(
                fd.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            let e = io::Error::last_os_error();
            return (
                if e.kind() == io::ErrorKind::NotFound {
                    MarkerFact::Known(false)
                } else {
                    unknown(reason(&e))
                },
                true,
            );
        }
        // SAFETY: fstatat succeeded and initialized the stat value.
        let metadata = unsafe { metadata.assume_init() };
        if dataless(&metadata) {
            return (unknown("cloud_skipped"), true);
        }
        (
            MarkerFact::Known(metadata.st_mode & libc::S_IFMT == libc::S_IFREG),
            true,
        )
    }
    fn directory(&self, relative: &Path) -> io::Result<File> {
        let mut fd = self
            .directory
            .as_ref()
            .ok_or_else(|| io::Error::from_raw_os_error(libc::ENOTDIR))?
            .try_clone()?;
        check_local(&fd)?;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(io::Error::from_raw_os_error(libc::ELOOP));
            };
            let name = CString::new(name.as_bytes())
                .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
            // SAFETY: directory fd stays alive; name is NUL-terminated. Relative,
            // one-component opens with O_NOFOLLOW cannot escape through a link.
            let child = unsafe {
                libc::openat(
                    fd.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if child < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: successful openat returns a new owned fd, adopted exactly once.
            fd = unsafe { File::from_raw_fd(child) };
            check_local(&fd)?;
        }
        Ok(fd)
    }
}
fn unknown(reason: &'static str) -> MarkerFact<'static> {
    MarkerFact::Unavailable(reason)
}
fn reason(e: &io::Error) -> &'static str {
    if e.kind() == io::ErrorKind::PermissionDenied {
        "access_denied"
    } else if matches!(e.raw_os_error(), Some(libc::ELOOP) | Some(libc::ENOTDIR)) {
        "out_of_scope"
    } else if cfg!(target_os = "macos") && matches!(e.raw_os_error(), Some(libc::EWOULDBLOCK)) {
        "cloud_skipped"
    } else {
        "io_error"
    }
}
fn dataless(metadata: &libc::stat) -> bool {
    #[cfg(target_os = "macos")]
    {
        metadata.st_flags & 0x4000_0000 != 0
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = metadata;
        false
    }
}
fn check_local(fd: &File) -> io::Result<()> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: fd is alive; fstat writes an initialized stat on success.
    if unsafe { libc::fstat(fd.as_raw_fd(), metadata.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful fstat initialized metadata.
    if dataless(&unsafe { metadata.assume_init() }) {
        return Err(io::Error::from_raw_os_error(libc::EWOULDBLOCK));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::symlink,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bh-marker-scope-{}-{}",
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
    fn own() -> MarkerProbe {
        MarkerProbe {
            scope: MarkerScope::Own,
            name: "signal".into(),
        }
    }
    #[test]
    fn replacing_intermediate_directory_with_link_cannot_redirect_marker_observation() {
        let root = Tree::new();
        let outside = Tree::new();
        fs::create_dir_all(root.0.join("a/b")).unwrap();
        fs::create_dir(outside.0.join("b")).unwrap();
        fs::write(outside.0.join("b/signal"), b"outside").unwrap();
        let scope = Scope::new(&root.0, true).unwrap();
        fs::rename(root.0.join("a"), root.0.join("old-a")).unwrap();
        symlink(&outside.0, root.0.join("a")).unwrap();
        assert!(matches!(
            scope.observe(&root.0.join("a/b"), &own()),
            (MarkerFact::Unavailable("out_of_scope"), false)
        ));
    }
    #[test]
    fn selected_root_descriptor_stays_anchored_after_path_replacement() {
        let t = Tree::new();
        let outside = Tree::new();
        fs::create_dir(t.0.join("selected")).unwrap();
        fs::write(outside.0.join("signal"), b"outside").unwrap();
        let selected = t.0.join("selected");
        let scope = Scope::new(&selected, true).unwrap();
        fs::rename(&selected, t.0.join("old-selected")).unwrap();
        symlink(&outside.0, &selected).unwrap();
        assert!(matches!(
            scope.observe(&selected, &own()),
            (MarkerFact::Known(false), true)
        ));
    }
    #[test]
    fn paths_outside_root_are_rejected_before_io() {
        let root = Tree::new();
        let outside = Tree::new();
        let scope = Scope::new(&root.0, true).unwrap();
        assert!(matches!(
            scope.observe(&outside.0, &own()),
            (MarkerFact::Unavailable("out_of_scope"), false)
        ));
        assert!(matches!(
            scope.observe(&root.0.join("../outside"), &own()),
            (MarkerFact::Unavailable("out_of_scope"), false)
        ));
    }
    #[test]
    fn inaccessible_directory_is_unknown_not_missing_marker() {
        use std::os::unix::fs::PermissionsExt;
        // root bypasses UNIX mode checks; do not turn that into a false permission witness.
        if unsafe { libc::geteuid() } == 0 {
            return;
        }
        let t = Tree::new();
        let child = t.0.join("restricted");
        fs::create_dir(&child).unwrap();
        let scope = Scope::new(&t.0, true).unwrap();
        fs::set_permissions(&child, fs::Permissions::from_mode(0o0)).unwrap();
        let fact = scope.observe(&child, &own());
        fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(
            fact,
            (MarkerFact::Unavailable("access_denied"), false)
        ));
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn dataless_metadata_is_classified_as_cloud_unavailable() {
        // SAFETY: a zeroed stat is valid as a synthetic metadata fixture.
        let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
        assert!(!dataless(&metadata));
        metadata.st_flags = 0x4000_0000;
        assert!(dataless(&metadata));
    }
}
