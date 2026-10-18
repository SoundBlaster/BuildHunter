//! Host-native metadata preparation and fallback. Caller retains the thread guard.
use crate::{MeasuredSize, Probes, ScanControl};
use hunter_fs_core::{
    DirectoryMeasurement, DirectoryMeasurementBackend, DirectoryMeasurementPlan,
    FileMeasurementBackend,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn measure_directory<Id>(
    plan: DirectoryMeasurementPlan<'_, Id>,
    control: &ScanControl,
    probes: &Probes,
) -> DirectoryMeasurement<Id> {
    plan.measure_with(
        &mut MetadataBackend {
            probes,
            #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
            control,
        },
        || control.should_stop(),
    )
}

struct MetadataBackend<'a> {
    probes: &'a Probes,
    #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
    control: &'a ScanControl,
}
#[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
type BulkMeasurements = io::Result<std::collections::HashMap<Vec<u8>, io::Result<MeasuredSize>>>;

struct PreparedFiles<'a> {
    probes: &'a Probes,
    #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
    control: &'a ScanControl,
    #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
    bulk: Option<BulkMeasurements>,
    #[cfg(all(test, target_os = "macos", feature = "macos-bulk-metadata"))]
    injected: std::collections::HashMap<PathBuf, io::Error>,
}
impl<'a> DirectoryMeasurementBackend for MetadataBackend<'a> {
    type Prepared = PreparedFiles<'a>;
    fn directory_metadata(&mut self, path: &Path) -> io::Result<fs::Metadata> {
        use hunter_fs_core::FilesystemBackend;
        self.probes.symlink_metadata(path)
    }
    fn prepare_files(&mut self, path: &Path, files: &[Option<PathBuf>]) -> Self::Prepared {
        let _ = (path, files);
        #[cfg(all(test, target_os = "macos", feature = "macos-bulk-metadata"))]
        let mut injected = std::collections::HashMap::new();
        #[cfg(all(test, target_os = "macos", feature = "macos-bulk-metadata"))]
        for file in files.iter().flatten() {
            if let Err(error) = self.probes.hook(crate::ProbePoint::Metadata, file) {
                injected.insert(file.clone(), error);
            }
        }
        #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
        let bulk = if files.iter().any(Option::is_some) {
            use std::sync::atomic::Ordering;
            if self.control.profiling.load(Ordering::Relaxed) {
                self.control
                    .bulk_directories
                    .fetch_add(1, Ordering::Relaxed);
            }
            #[cfg(test)]
            self.probes.record_policy();
            Some(crate::bulk_metadata::read(path, self.control))
        } else {
            None
        };
        PreparedFiles {
            probes: self.probes,
            #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
            control: self.control,
            #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
            bulk,
            #[cfg(all(test, target_os = "macos", feature = "macos-bulk-metadata"))]
            injected,
        }
    }
}
impl FileMeasurementBackend for PreparedFiles<'_> {
    fn measure(&mut self, file: &Path) -> io::Result<MeasuredSize> {
        use hunter_fs_core::FilesystemBackend;
        #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
        if let Some(result) = &mut self.bulk {
            #[cfg(test)]
            if let Some(error) = self.injected.remove(file) {
                return Err(error);
            }
            match result {
                Ok(entries) => {
                    let name = file
                        .file_name()
                        .map(crate::os_str_bytes)
                        .unwrap_or_default();
                    if let Some(size) = entries.remove(name.as_ref()) {
                        return size
                            .map_err(|error| crate::materialization::describe_error(file, error));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Unsupported => {}
                Err(error) => {
                    let error = error.raw_os_error().map_or_else(
                        || io::Error::new(error.kind(), error.to_string()),
                        io::Error::from_raw_os_error,
                    );
                    return Err(crate::materialization::describe_error(file, error));
                }
            }
            use std::sync::atomic::Ordering;
            if self.control.profiling.load(Ordering::Relaxed) {
                self.control.bulk_fallbacks.fetch_add(1, Ordering::Relaxed);
            }
            // Unsupported/missing batch metadata keeps the portable probe authoritative.
        }
        self.probes.symlink_metadata(file).map(MeasuredSize::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hunter_fs_core::DirectoryTask;
    #[test]
    fn precancelled_pass_still_prepares_after_directory_failure() {
        let tree = crate::tests::TempTree::new();
        let path = tree.0.join("gone");
        fs::create_dir(&path).unwrap();
        let file = path.join("data");
        fs::write(&file, b"data").unwrap();
        let entries = vec![(
            file.clone(),
            fs::symlink_metadata(&file).unwrap().file_type(),
        )];
        fs::remove_file(file).unwrap();
        fs::remove_dir(&path).unwrap();
        let task = DirectoryTask {
            id: 17,
            path,
            measure: true,
            context: (),
        };
        let control = ScanControl::new();
        control.set_profiling(true);
        control.cancel();
        let probes = Probes::default();
        let _guard = crate::materialization::Guard::enter().unwrap();
        let result = measure_directory(
            task.measurement_plan(&entries, false).unwrap(),
            &control,
            &probes,
        );
        assert_eq!(result.id, 17);
        assert_eq!(
            result.directory.unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(result.files.is_empty());
        assert_eq!(probes.counts().metadata, 1);
        #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
        assert_eq!(
            control.work_timings().bulk_directories,
            1,
            "preparation must not be suppressed by pre-cancellation/directory error"
        );
    }
    #[test]
    #[cfg(all(target_os = "macos", feature = "macos-bulk-metadata"))]
    fn prepared_batch_preserves_consumption_fallbacks_and_error_provenance() {
        use std::collections::HashMap;
        let tree = crate::tests::TempTree::new();
        let file = tree.0.join("data");
        fs::write(&file, b"data").unwrap();
        let control = ScanControl::new();
        control.set_profiling(true);
        let probes = Probes::default();
        let _guard = crate::materialization::Guard::enter().unwrap();
        let size = MeasuredSize {
            apparent: 7,
            allocated: 512,
        };
        let mut prepared = PreparedFiles {
            probes: &probes,
            control: &control,
            bulk: Some(Ok(HashMap::from([(b"data".to_vec(), Ok(size))]))),
            injected: HashMap::new(),
        };
        assert_eq!(prepared.measure(&file).unwrap(), size);
        assert_eq!(probes.counts().metadata, 0);
        assert_eq!(prepared.measure(&file).unwrap().apparent, 4);
        assert_eq!(control.work_timings().bulk_fallbacks, 1);
        prepared.bulk = Some(Err(io::Error::from(io::ErrorKind::Unsupported)));
        assert_eq!(prepared.measure(&file).unwrap().apparent, 4);
        assert_eq!(control.work_timings().bulk_fallbacks, 2);
        prepared.bulk = Some(Err(io::Error::from_raw_os_error(13)));
        assert_eq!(
            prepared.measure(&file).unwrap_err().raw_os_error(),
            Some(13)
        );
        assert_eq!(probes.counts().metadata, 2);
        prepared
            .injected
            .insert(file.clone(), io::Error::from_raw_os_error(5));
        assert_eq!(prepared.measure(&file).unwrap_err().raw_os_error(), Some(5));
        assert_eq!(control.work_timings().bulk_fallbacks, 2);
        assert!(
            probes
                .counts()
                .materialization_policies
                .iter()
                .all(|p| *p == crate::materialization::current_policy())
        );
    }
}
