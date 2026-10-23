//! Descriptor-relative acquisition for a single selected root (macOS and Linux).
use crate::{
    ScopedMetadata,
    scoped_metadata::{check_local, dataless},
};
use hunter_fs_core::{
    DirectoryAccess, DirectoryEvent, DirectoryVisit, EntryMetadata, EntryType, MeasuredSize,
    ScanAccess,
};
use std::{
    ffi::{CStr, CString, OsStr},
    fs::{self, File},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
        unix::ffi::OsStrExt,
    },
    path::{Component, Path, PathBuf},
};

/// One selected-root handle, shared by every acquisition in a scan. Report
/// paths are not reopened after initialization. Child resolution opens
/// each directory component separately, without following links. Renames are
/// not a snapshot: independently acquired passes may observe different regular
/// entries, but can never resolve a replacement symlink to its target.
pub struct ScopedFilesystem {
    root: PathBuf,
    selected: SelectedRoot,
}
enum SelectedRoot {
    Directory(File),
    Entry(fs::Metadata),
}
impl ScopedFilesystem {
    /// The host installs its non-materializing policy first and retains it for
    /// every calling thread. Ancestors of the initially selected root are chosen
    /// by the caller; the final component and all subsequent children are no-follow.
    pub fn open(root: &Path) -> io::Result<Self> {
        let absolute = if root.is_absolute() {
            root.to_owned()
        } else {
            std::env::current_dir()?.join(root)
        };
        // Remove trailing separators/`.` without resolving links or `..`.
        // A trailing slash must not turn a selected symlink into its target.
        let opening: PathBuf = absolute.components().collect();
        let metadata = fs::symlink_metadata(&opening)?;
        #[cfg(target_os = "macos")]
        {
            use std::os::macos::fs::MetadataExt;
            if metadata.st_flags() & 0x4000_0000 != 0 {
                return Err(io::Error::from_raw_os_error(libc::EWOULDBLOCK));
            }
        }
        let selected = if metadata.is_dir() {
            SelectedRoot::Directory(open_directory(libc::AT_FDCWD, opening.as_os_str())?)
        } else {
            // Selected files are a single no-follow metadata observation, never
            // opened for content. All recursive access requires the directory case.
            SelectedRoot::Entry(metadata)
        };
        Ok(Self {
            root: root.to_owned(),
            selected,
        })
    }

    /// Marker observations and traversal share the exact same selected-root
    /// object, even if its pathname is replaced between setup and scanning.
    pub fn marker_scope(&self) -> io::Result<ScopedMetadata> {
        Ok(ScopedMetadata::from_handle(
            self.root.clone(),
            match &self.selected {
                SelectedRoot::Directory(file) => Some(file.try_clone()?),
                SelectedRoot::Entry(_) => None,
            },
        ))
    }

    fn relative<'a>(&self, path: &'a Path) -> io::Result<&'a Path> {
        let relative = path.strip_prefix(&self.root).map_err(|_| out_of_scope())?;
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(out_of_scope());
        }
        Ok(relative)
    }

    fn directory(&self, relative: &Path) -> io::Result<OpenedDirectory> {
        let SelectedRoot::Directory(root) = &self.selected else {
            return Err(io::Error::from_raw_os_error(libc::ENOTDIR));
        };
        // openat(".") creates an independent directory offset. dup/try_clone
        // alone would let simultaneous readdir passes consume each other's rows.
        let mut file = open_directory(root.as_raw_fd(), OsStr::new("."))?;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(out_of_scope());
            };
            file = open_directory(file.as_raw_fd(), name)?;
        }
        Ok(OpenedDirectory(file))
    }
}

impl ScanAccess for ScopedFilesystem {
    fn root_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        let relative = self.relative(path)?;
        if relative.as_os_str().is_empty() {
            return match &self.selected {
                SelectedRoot::Directory(file) => file.metadata(),
                SelectedRoot::Entry(metadata) => Ok(metadata.clone()),
            };
        }
        Err(out_of_scope())
    }

    fn prepare_directory(&self, path: &Path) -> io::Result<Box<dyn DirectoryAccess + '_>> {
        Ok(Box::new(self.directory(self.relative(path)?)?))
    }

    fn visit_directory(
        &self,
        path: &Path,
        should_stop: &mut dyn FnMut() -> bool,
        emit: &mut dyn FnMut(DirectoryEvent),
    ) -> DirectoryVisit {
        let stream = self
            .relative(path)
            .and_then(|relative| self.directory(relative))
            .and_then(|directory| DirectoryStream::open(directory.0));
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                emit(DirectoryEvent::Error {
                    path: path.to_owned(),
                    error,
                });
                return DirectoryVisit::OpenFailed;
            }
        };
        loop {
            let name = match stream.next_name() {
                Ok(Some(name)) => name,
                Ok(None) => return DirectoryVisit::Completed,
                Err(error) => {
                    emit(DirectoryEvent::Error {
                        path: path.to_owned(),
                        error,
                    });
                    return DirectoryVisit::Completed;
                }
            };
            if should_stop() {
                return DirectoryVisit::Cancelled;
            }
            let child = path.join(&name);
            let metadata = entry_metadata(stream.fd(), &name);
            match metadata {
                Ok(metadata) => emit(DirectoryEvent::Entry {
                    path: child,
                    file_type: metadata.file_type,
                }),
                Err(error) => emit(DirectoryEvent::Error { path: child, error }),
            }
        }
    }
}

struct OpenedDirectory(File);
impl DirectoryAccess for OpenedDirectory {
    fn metadata(&self) -> io::Result<fs::Metadata> {
        self.0.metadata()
    }
    fn entry_metadata(&self, name: &OsStr) -> io::Result<EntryMetadata> {
        validate_name(name)?;
        entry_metadata(self.0.as_raw_fd(), name)
    }
}

fn out_of_scope() -> io::Error {
    io::Error::from_raw_os_error(libc::ELOOP)
}
fn validate_name(name: &OsStr) -> io::Result<()> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || name.as_bytes().contains(&b'/')
        || name.as_bytes().contains(&0)
    {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }
    Ok(())
}
fn open_directory(parent: RawFd, name: &OsStr) -> io::Result<File> {
    let file = open_at(
        parent,
        name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
    )?;
    check_local(&file)?;
    Ok(file)
}
fn entry_metadata(parent: RawFd, name: &OsStr) -> io::Result<EntryMetadata> {
    validate_name(name)?;
    let name =
        CString::new(name.as_bytes()).map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: parent stays alive; name is a single terminated component; stat is
    // writable for one native value. AT_SYMLINK_NOFOLLOW observes the link itself.
    if unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful fstatat initialized stat.
    let stat = unsafe { stat.assume_init() };
    if dataless(&stat) {
        return Err(io::Error::from_raw_os_error(libc::EWOULDBLOCK));
    }
    let file_type = match stat.st_mode & libc::S_IFMT {
        libc::S_IFDIR => EntryType::Directory,
        libc::S_IFREG => EntryType::RegularFile,
        libc::S_IFLNK => EntryType::Symlink,
        _ => EntryType::Other,
    };
    let nonnegative = |value| {
        u64::try_from(value)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Negative filesystem size"))
    };
    Ok(EntryMetadata {
        file_type,
        size: MeasuredSize {
            apparent: nonnegative(stat.st_size)?,
            allocated: nonnegative(stat.st_blocks)?.saturating_mul(512),
        },
    })
}
fn open_at(parent: RawFd, name: &OsStr, flags: i32) -> io::Result<File> {
    let name =
        CString::new(name.as_bytes()).map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: parent is a live owned descriptor (or AT_FDCWD during setup), name
    // is NUL-terminated. No O_CREAT: the variadic mode argument is unnecessary.
    let fd = unsafe { libc::openat(parent, name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a fresh owned descriptor, adopted exactly once.
    Ok(unsafe { File::from_raw_fd(fd) })
}

struct DirectoryStream(*mut libc::DIR);
impl DirectoryStream {
    fn open(file: File) -> io::Result<Self> {
        let fd = file.into_raw_fd();
        // SAFETY: fd owns a newly opened readable directory. fdopendir takes
        // ownership only on success; failure is closed by the File below.
        let directory = unsafe { libc::fdopendir(fd) };
        if directory.is_null() {
            let error = io::Error::last_os_error();
            // SAFETY: failed fdopendir did not consume fd.
            drop(unsafe { File::from_raw_fd(fd) });
            return Err(error);
        }
        Ok(Self(directory))
    }
    fn fd(&self) -> RawFd {
        // SAFETY: the directory stream stays alive for this synchronous pass.
        unsafe { libc::dirfd(self.0) }
    }
    fn next_name(&self) -> io::Result<Option<std::ffi::OsString>> {
        loop {
            // SAFETY: access is confined to the calling thread. Reset errno to
            // distinguish EOF from error; copy the name before another readdir.
            #[cfg(target_os = "macos")]
            let errno = unsafe { libc::__error() };
            #[cfg(target_os = "linux")]
            let errno = unsafe { libc::__errno_location() };
            unsafe {
                *errno = 0;
            }
            let entry = unsafe { libc::readdir(self.0) };
            if entry.is_null() {
                let code = unsafe { *errno };
                return if code == 0 {
                    Ok(None)
                } else {
                    Err(io::Error::from_raw_os_error(code))
                };
            }
            // SAFETY: readdir returned an initialized dirent with a terminated
            // d_name, valid until the next call on this private stream.
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            return Ok(Some(OsStr::from_bytes(name).to_owned()));
        }
    }
}
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        // SAFETY: this object exclusively owns the successful fdopendir result.
        unsafe {
            libc::closedir(self.0);
        }
    }
}
