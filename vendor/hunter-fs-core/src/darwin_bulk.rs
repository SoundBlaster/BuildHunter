//! Opt-in Darwin batch measurement. Discovery still uses the portable listing.
//! No persistent index; all names and sizes belong to this one directory pass.
use crate::MeasuredSize;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
};

const NAME: u32 = 0x1;
const OBJTYPE: u32 = 0x8;
const ERROR: u32 = 0x2000_0000;
const RETURNED: u32 = 0x8000_0000;
const ALLOCSIZE: u32 = 0x4;
const DATALENGTH: u32 = 0x200;
const COMMON: u32 = NAME | OBJTYPE | ERROR | RETURNED;
const FILE: u32 = ALLOCSIZE | DATALENGTH;

#[repr(C)]
struct AttrList {
    bitmap_count: u16,
    reserved: u16,
    common: u32,
    volume: u32,
    directory: u32,
    file: u32,
    fork: u32,
}

unsafe extern "C" {
    fn getattrlistbulk(
        fd: i32,
        attrs: *mut AttrList,
        buffer: *mut std::ffi::c_void,
        size: usize,
        options: u64,
    ) -> i32;
}

/// One directory pass keyed by lossless Unix basename bytes. Not a persistent cache.
pub use crate::DirectorySizes;

/// Read apparent/allocated sizes without opening file contents. The caller holds
/// permissions and the thread's non-materializing I/O guard throughout this call.
/// Opens the directory before checking stop, then checks once before each batch.
/// Cancellation returns the accumulated prefix; an in-flight syscall is not interrupted.
/// The final directory component is opened no-follow; ancestors remain pathname-based.
pub fn read_directory_sizes(
    path: &Path,
    should_stop: impl FnMut() -> bool,
) -> io::Result<DirectorySizes> {
    read_directory_sizes_with_limits(path, should_stop, crate::ListingLimits::UNBOUNDED)
        .map(|collected| collected.sizes)
}

/// Bounded batch result. A retention limit is truncation, independently of cancellation.
pub struct BoundedDirectorySizes {
    pub sizes: DirectorySizes,
    pub limit: Option<crate::ListingLimit>,
    pub retained_weight: usize,
}

/// Optional count/estimated-payload limits, checked before retaining each decoded
/// observation. The fixed native buffer and hash-table overhead are outside weight.
/// `max_errors` is unused: errors occupy the same observation slots as sizes.
pub fn read_directory_sizes_with_limits(
    path: &Path,
    mut should_stop: impl FnMut() -> bool,
    limits: crate::ListingLimits,
) -> io::Result<BoundedDirectorySizes> {
    // Public <sys/fcntl.h>: no symlink following for the opened directory.
    const OPEN_FLAGS: i32 = 0x100 | 0x10_0000 | 0x100_0000;
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_FLAGS)
        .open(path)?;
    let mut attrs = AttrList {
        bitmap_count: 5,
        reserved: 0,
        common: COMMON,
        volume: 0,
        directory: 0,
        file: FILE,
        fork: 0,
    };
    // u64 elements provide the alignment Darwin requires for each record.
    let mut buffer = vec![0u64; 32_768];
    let mut result = HashMap::new();
    let mut retained_weight = 0;
    let mut limit = None;
    while !should_stop() {
        // SAFETY: fd owns an open directory; attrlist has the documented C layout;
        // the initialized, aligned buffer is writable for the supplied byte count.
        let count = unsafe {
            getattrlistbulk(
                directory.as_raw_fd(),
                &mut attrs,
                buffer.as_mut_ptr().cast(),
                buffer.len() * 8,
                0,
            )
        };
        if count == -1 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            break;
        }
        // SAFETY: buffer consists of initialized u64 values; borrowing its bytes
        // is valid until the next syscall. Parsing below checks every range.
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), buffer.len() * 8) };
        limit = parse_batch_with_limits(
            bytes,
            count as usize,
            &mut result,
            limits,
            &mut retained_weight,
        )?;
        if limit.is_some() {
            break;
        }
    }
    Ok(BoundedDirectorySizes {
        sizes: result,
        limit,
        retained_weight,
    })
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid getattrlistbulk metadata record",
    )
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let end = self.offset.checked_add(N).ok_or_else(invalid)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_| invalid())?;
        self.offset = end;
        Ok(value)
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_ne_bytes(self.take()?))
    }
    fn size(&mut self) -> io::Result<u64> {
        u64::try_from(i64::from_ne_bytes(self.take()?)).map_err(|_| invalid())
    }
}

#[cfg(test)]
fn parse_batch(bytes: &[u8], count: usize, result: &mut DirectorySizes) -> io::Result<()> {
    parse_batch_with_limits(
        bytes,
        count,
        result,
        crate::ListingLimits::UNBOUNDED,
        &mut 0,
    )
    .map(|_| ())
}

fn parse_batch_with_limits(
    bytes: &[u8],
    count: usize,
    result: &mut DirectorySizes,
    limits: crate::ListingLimits,
    retained_weight: &mut usize,
) -> io::Result<Option<crate::ListingLimit>> {
    let mut offset = 0usize;
    for _ in 0..count {
        let length = u32::from_ne_bytes(
            bytes
                .get(offset..offset.checked_add(4).ok_or_else(invalid)?)
                .ok_or_else(invalid)?
                .try_into()
                .map_err(|_| invalid())?,
        ) as usize;
        if length < 24 || !length.is_multiple_of(8) {
            return Err(invalid());
        }
        let end = offset.checked_add(length).ok_or_else(invalid)?;
        let record = bytes.get(offset..end).ok_or_else(invalid)?;
        offset = end;
        let mut cursor = Cursor {
            bytes: record,
            offset: 4,
        };
        let common = cursor.u32()?;
        let volume = cursor.u32()?;
        let directory = cursor.u32()?;
        let file = cursor.u32()?;
        let fork = cursor.u32()?;
        if common & !COMMON != 0
            || common & (RETURNED | NAME) != (RETURNED | NAME)
            || volume != 0
            || directory != 0
            || fork != 0
            || file & !FILE != 0
        {
            return Err(invalid());
        }
        // ATTR_CMN_ERROR is specially packed immediately after RETURNED_ATTRS.
        let error = if common & ERROR != 0 {
            cursor.u32()?
        } else {
            0
        };
        let reference = cursor.offset;
        let displacement = i32::from_ne_bytes(cursor.take()?);
        let name_len = cursor.u32()? as usize;
        let name_start = reference
            .checked_add_signed(displacement as isize)
            .ok_or_else(invalid)?;
        let name_end = name_start.checked_add(name_len).ok_or_else(invalid)?;
        let kind = if common & OBJTYPE != 0 {
            Some(cursor.u32()?)
        } else {
            None
        };
        let allocated = if file & ALLOCSIZE != 0 {
            Some(cursor.size()?)
        } else {
            None
        };
        let apparent = if file & DATALENGTH != 0 {
            Some(cursor.size()?)
        } else {
            None
        };
        if name_start < cursor.offset {
            return Err(invalid());
        }
        let name = record.get(name_start..name_end).ok_or_else(invalid)?;
        let Some((&0, name)) = name.split_last() else {
            return Err(invalid());
        };
        if name.is_empty() || name.contains(&0) || name.contains(&b'/') {
            return Err(invalid());
        }
        let value = if error != 0 {
            Some(Err(io::Error::from_raw_os_error(error as i32)))
        } else if kind.is_some_and(|kind| kind != 2 && kind != 5)
            && let (Some(apparent), Some(allocated)) = (apparent, allocated)
        {
            // VDIR=2 and VLNK=5 are not regular file slots.
            Some(Ok(MeasuredSize {
                apparent,
                allocated,
            }))
        } else {
            None
        };
        if let Some(value) = value {
            // Duplicate records conservatively consume additional weight. This
            // avoids under-counting replacement storage under concurrent mutation.
            if !result.contains_key(name) && result.len() >= limits.max_entries {
                return Ok(Some(crate::ListingLimit::Entries));
            }
            let weight = name
                .len()
                .saturating_add(std::mem::size_of::<(Vec<u8>, io::Result<MeasuredSize>)>());
            if weight > limits.max_weight.saturating_sub(*retained_weight) {
                return Ok(Some(crate::ListingLimit::Weight));
            }
            *retained_weight += weight;
            result.insert(name.to_vec(), value);
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{MetadataExt, symlink},
    };

    struct Tree(std::path::PathBuf);
    impl Tree {
        fn new() -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "hunter-bulk-{}-{}",
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
    #[test]
    fn limited_batch_stops_before_retaining_an_over_budget_observation() {
        let mut bytes = record(0);
        let mut second = record(13);
        second[56..60].copy_from_slice(b"next");
        bytes.extend(second);
        let mut sizes = HashMap::new();
        let mut weight = 0;
        let limits = crate::ListingLimits {
            max_entries: 1,
            ..crate::ListingLimits::UNBOUNDED
        };
        assert_eq!(
            parse_batch_with_limits(&bytes, 2, &mut sizes, limits, &mut weight).unwrap(),
            Some(crate::ListingLimit::Entries)
        );
        assert_eq!(sizes.len(), 1);
        assert!(weight > 0);
        let mut sizes = HashMap::new();
        let mut weight = 0;
        let limits = crate::ListingLimits {
            max_weight: 1,
            ..crate::ListingLimits::UNBOUNDED
        };
        assert_eq!(
            parse_batch_with_limits(&bytes, 2, &mut sizes, limits, &mut weight).unwrap(),
            Some(crate::ListingLimit::Weight)
        );
        assert!(sizes.is_empty());
        assert_eq!(weight, 0);
    }

    #[test]
    fn native_limited_read_reports_truncation_and_exact_limit_exhaustion() {
        let _guard = crate::NonMaterializingIoGuard::enter().unwrap();
        let tree = Tree::new();
        fs::write(tree.0.join("first"), b"one").unwrap();
        let limits = crate::ListingLimits {
            max_entries: 1,
            ..crate::ListingLimits::UNBOUNDED
        };
        let exact = read_directory_sizes_with_limits(&tree.0, || false, limits).unwrap();
        assert_eq!(exact.sizes.len(), 1);
        assert_eq!(exact.limit, None);
        fs::write(tree.0.join("second"), b"two").unwrap();
        let truncated = read_directory_sizes_with_limits(&tree.0, || false, limits).unwrap();
        assert_eq!(truncated.sizes.len(), 1);
        assert_eq!(truncated.limit, Some(crate::ListingLimit::Entries));
    }

    #[test]
    fn precancellation_keeps_open_before_checkpoint_and_returns_empty_batch() {
        let _guard = crate::NonMaterializingIoGuard::enter().unwrap();
        let tree = Tree::new();
        fs::write(tree.0.join("data"), b"payload").unwrap();
        let mut checks = 0;
        let sizes = read_directory_sizes(&tree.0, || {
            checks += 1;
            true
        })
        .unwrap();
        assert!(sizes.is_empty());
        assert_eq!(checks, 1);
        let error = read_directory_sizes(&tree.0.join("missing"), || {
            panic!("failed open must not check stop")
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
    #[test]
    fn stop_after_a_batch_returns_owned_observations_without_another_checkpoint() {
        let _guard = crate::NonMaterializingIoGuard::enter().unwrap();
        let tree = Tree::new();
        fs::write(tree.0.join("data"), b"payload").unwrap();
        let mut checks = 0;
        let sizes = read_directory_sizes(&tree.0, || {
            checks += 1;
            checks == 2
        })
        .unwrap();
        assert_eq!(checks, 2);
        assert_eq!(sizes[b"data".as_slice()].as_ref().unwrap().apparent, 7);
    }
    #[test]
    fn decoder_keeps_non_utf8_names_and_excludes_directory_and_link_sizes() {
        let mut bytes = record(0);
        bytes[56..60].copy_from_slice(b"raw\xff");
        let mut sizes = HashMap::new();
        parse_batch(&bytes, 1, &mut sizes).unwrap();
        assert!(sizes.contains_key(b"raw\xff".as_slice()));
        for kind in [2u32, 5] {
            let mut bytes = record(0);
            bytes[36..40].copy_from_slice(&kind.to_ne_bytes());
            let mut sizes = HashMap::new();
            parse_batch(&bytes, 1, &mut sizes).unwrap();
            assert!(sizes.is_empty());
        }
    }
    #[test]
    fn batch_sizes_match_stat_for_files_sparse_files_and_hard_links() {
        let _guard = crate::NonMaterializingIoGuard::enter().unwrap();
        let tree = Tree::new();
        fs::write(tree.0.join("small"), b"payload").unwrap();
        fs::write(tree.0.join("small/..namedfork/rsrc"), vec![42; 128 * 1024]).unwrap();
        fs::hard_link(tree.0.join("small"), tree.0.join("hard-link")).unwrap();
        fs::File::create(tree.0.join("sparse"))
            .unwrap()
            .set_len(32 * 1024 * 1024)
            .unwrap();
        fs::create_dir(tree.0.join("directory")).unwrap();
        symlink("small", tree.0.join("link")).unwrap();
        let mut sizes = read_directory_sizes(&tree.0, || false).unwrap();
        for name in ["small", "hard-link", "sparse"] {
            let actual = sizes.remove(name.as_bytes()).unwrap().unwrap();
            let metadata = fs::symlink_metadata(tree.0.join(name)).unwrap();
            assert_eq!(actual.apparent, metadata.len());
            assert_eq!(actual.allocated, metadata.blocks() * 512);
        }
        assert!(!sizes.contains_key(b"link".as_slice()));
        assert!(!sizes.contains_key(b"directory".as_slice()));
    }

    #[test]
    fn rejects_truncated_overflowing_and_negative_records() {
        for bytes in [
            vec![],
            vec![0; 64],
            vec![255; 64],
            24u32.to_ne_bytes().to_vec(),
        ] {
            assert!(parse_batch(&bytes, 1, &mut HashMap::new()).is_err());
        }
        assert!(
            Cursor {
                bytes: &(-1i64).to_ne_bytes(),
                offset: 0
            }
            .size()
            .is_err()
        );
    }

    #[test]
    fn does_not_open_a_symlinked_directory() {
        let _guard = crate::NonMaterializingIoGuard::enter().unwrap();
        let tree = Tree::new();
        fs::create_dir(tree.0.join("real")).unwrap();
        symlink("real", tree.0.join("link")).unwrap();
        assert!(read_directory_sizes(&tree.0.join("link"), || false).is_err());
    }

    fn record(error: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [64u32, COMMON, 0, 0, FILE, 0, error, 28, 5, 1] {
            bytes.extend(value.to_ne_bytes());
        }
        bytes.extend(4096i64.to_ne_bytes());
        bytes.extend(7i64.to_ne_bytes());
        bytes.extend(b"name\0");
        bytes.resize(64, 0);
        bytes
    }

    #[test]
    fn validates_name_references_sizes_and_per_entry_errors() {
        let mut sizes = HashMap::new();
        parse_batch(&record(0), 1, &mut sizes).unwrap();
        assert_eq!(
            sizes.remove(b"name".as_slice()).unwrap().unwrap(),
            MeasuredSize {
                apparent: 7,
                allocated: 4096
            }
        );
        parse_batch(&record(13), 1, &mut sizes).unwrap();
        assert_eq!(
            sizes
                .remove(b"name".as_slice())
                .unwrap()
                .unwrap_err()
                .raw_os_error(),
            Some(13)
        );
        for (offset, value) in [(28, u32::MAX), (32, u32::MAX)] {
            let mut bytes = record(0);
            bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
            assert!(parse_batch(&bytes, 1, &mut sizes).is_err());
        }
        let mut bytes = record(0);
        bytes[40..48].copy_from_slice(&(-1i64).to_ne_bytes());
        assert!(parse_batch(&bytes, 1, &mut sizes).is_err());
        assert!(parse_batch(&record(0)[..63], 1, &mut sizes).is_err());
    }
}
