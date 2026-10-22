//! Owned one-directory batch observations and consume-once lookup, without I/O.
use crate::MeasuredSize;
use std::{collections::HashMap, io, path::Path};

/// Basename keys use the current platform's `OsStr::as_encoded_bytes` representation.
/// Keep them inside this process/platform; they are not portable serialized paths.
pub type DirectorySizes = HashMap<Vec<u8>, io::Result<MeasuredSize>>;

/// A prepared batch, including a directory-wide acquisition error.
/// The host keeps guard lifetime, acquisition strategy, fallback effects and counters.
pub struct PreparedDirectorySizes {
    result: io::Result<DirectorySizes>,
}

impl PreparedDirectorySizes {
    /// Take ownership of an already acquired batch; this performs no I/O.
    pub fn new(result: io::Result<DirectorySizes>) -> Self {
        Self { result }
    }

    /// Consume one basename observation. `None` asks the host to use its fallback:
    /// the name was absent/already consumed, or acquisition was Unsupported.
    /// A cached entry error is returned once without retry. Other directory-wide
    /// errors are reconstructed per request, preserving errno or kind/message.
    /// Only the basename is looked up: the host must supply original slots from
    /// the same directory, not an arbitrary path with an identical basename.
    pub fn take_file(&mut self, file: &Path) -> Option<io::Result<MeasuredSize>> {
        match &mut self.result {
            Ok(entries) => {
                let name = file
                    .file_name()
                    .map(|name| name.as_encoded_bytes())
                    .unwrap_or_default();
                entries.remove(name)
            }
            Err(error) if error.kind() == io::ErrorKind::Unsupported => None,
            Err(error) => Some(Err(error.raw_os_error().map_or_else(
                || io::Error::new(error.kind(), error.to_string()),
                io::Error::from_raw_os_error,
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SIZE: MeasuredSize = MeasuredSize {
        apparent: 7,
        allocated: 512,
    };

    #[test]
    fn values_and_entry_errors_are_consumed_once_and_missing_names_fall_back() {
        let mut batch = PreparedDirectorySizes::new(Ok(HashMap::from([
            (b"data".to_vec(), Ok(SIZE)),
            (b"denied".to_vec(), Err(io::Error::from_raw_os_error(13))),
        ])));
        assert_eq!(
            batch.take_file(Path::new("dir/data")).unwrap().unwrap(),
            SIZE
        );
        assert!(batch.take_file(Path::new("dir/data")).is_none());
        assert_eq!(
            batch
                .take_file(Path::new("dir/denied"))
                .unwrap()
                .unwrap_err()
                .raw_os_error(),
            Some(13)
        );
        assert!(batch.take_file(Path::new("dir/denied")).is_none());
        assert!(batch.take_file(Path::new("dir/missing")).is_none());
    }

    #[test]
    fn unsupported_requests_fallback_but_other_directory_errors_repeat_without_retry() {
        let mut unsupported = PreparedDirectorySizes::new(Err(io::ErrorKind::Unsupported.into()));
        let mut native = PreparedDirectorySizes::new(Err(io::Error::from_raw_os_error(13)));
        let mut custom = PreparedDirectorySizes::new(Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bad batch",
        )));
        for name in ["a", "b", "a"] {
            assert!(unsupported.take_file(Path::new(name)).is_none());
            assert_eq!(
                native
                    .take_file(Path::new(name))
                    .unwrap()
                    .unwrap_err()
                    .raw_os_error(),
                Some(13)
            );
            let error = custom.take_file(Path::new(name)).unwrap().unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert_eq!(error.to_string(), "bad batch");
        }
    }

    #[test]
    fn keys_use_lossless_platform_basenames_without_normalization() {
        let path = Path::new("dir/é");
        let key = path.file_name().unwrap().as_encoded_bytes().to_vec();
        let mut batch = PreparedDirectorySizes::new(Ok(HashMap::from([(key, Ok(SIZE))])));
        assert!(batch.take_file(Path::new("dir/e\u{301}")).is_none());
        assert_eq!(batch.take_file(path).unwrap().unwrap(), SIZE);
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_basename_is_not_lossily_replaced() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let path = Path::new(OsStr::from_bytes(b"raw-\xff"));
        let mut batch =
            PreparedDirectorySizes::new(Ok(HashMap::from([(b"raw-\xff".to_vec(), Ok(SIZE))])));
        assert!(batch.take_file(Path::new("raw-�")).is_none());
        assert_eq!(batch.take_file(path).unwrap().unwrap(), SIZE);
    }
}
