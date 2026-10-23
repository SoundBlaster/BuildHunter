//! Immutable, versioned catalog identity shared by CLI and the application bridge.
pub use buildhunter_plugin_policy::builtin::ClientProfile;
use buildhunter_plugin_policy::{PluginCatalog, TargetDescriptor, TargetKind};
use std::{
    ffi::{CString, c_char},
    ptr,
};

pub const CATALOG_ABI_VERSION: u32 = 1;

/// Caller-owned snapshot. Preferences never mutate a running scan's catalog.
pub struct CatalogSnapshot {
    pub(crate) profile: ClientProfile,
    pub(crate) catalog: PluginCatalog,
    metadata: CString,
}

impl CatalogSnapshot {
    pub fn basic(profile: ClientProfile, excluded: &[&str]) -> Result<Self, String> {
        let catalog = profile.load(excluded).map_err(|e| e.to_string())?;
        Ok(Self::from_catalog(profile, catalog))
    }

    /// Compose a previously validated catalog; plugin loading/origin remains host-owned.
    pub fn from_catalog(profile: ClientProfile, catalog: PluginCatalog) -> Self {
        let targets: Vec<_> = catalog
            .targets()
            .iter()
            .map(|target| {
                serde_json::json!({
                    "id": target.id, "title": target.title, "ecosystem": target.ecosystem,
                    "kind": target_kind_name(target.kind),
                    "presentationKind": presentation_kind(profile, target),
                })
            })
            .collect();
        let metadata = serde_json::json!({
            "format": "buildhunter.catalog/1", "profile": profile.id(),
            "markerPolicy": "legacy-file-markers/1",
            "digest": catalog.catalog_digest().to_string(),
            "excludedTargetIDs": catalog.exclusions(), "targets": targets,
        })
        .to_string();
        Self {
            profile,
            catalog,
            metadata: CString::new(metadata).expect("JSON escapes NUL"),
        }
    }

    pub fn profile(&self) -> ClientProfile {
        self.profile
    }
    pub fn catalog(&self) -> &PluginCatalog {
        &self.catalog
    }
    pub fn metadata_json(&self) -> &str {
        self.metadata.to_str().expect("JSON is UTF-8")
    }
    pub fn target(&self, id: &str) -> Option<&TargetDescriptor> {
        let targets = self.catalog.targets();
        targets
            .binary_search_by(|target| target.id.as_str().cmp(id))
            .ok()
            .map(|i| &targets[i])
    }
}

pub fn presentation_kind(profile: ClientProfile, target: &TargetDescriptor) -> &'static str {
    if profile == ClientProfile::GuiV1 {
        if matches!(target.id.as_str(), "python.tox" | "python.nox") {
            return "test-environment";
        }
        if matches!(target.kind, TargetKind::Bytecode | TargetKind::Metadata) {
            return "cache";
        }
    }
    match target.kind {
        TargetKind::BuildOutput => "build",
        TargetKind::Cache => "cache",
        TargetKind::Environment => "environment",
        TargetKind::Bytecode => "bytecode",
        TargetKind::Metadata => "metadata",
    }
}

pub(crate) fn target_kind_name(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::BuildOutput => "build_output",
        TargetKind::Cache => "cache",
        TargetKind::Environment => "environment",
        TargetKind::Bytecode => "bytecode",
        TargetKind::Metadata => "metadata",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bh_catalog_abi_version() -> u32 {
    CATALOG_ABI_VERSION
}

/// Create a basic snapshot (profile 1 CLI, 2 GUI) from a UTF-8 JSON string array.
/// Unknown valid target IDs are retained; malformed input returns null. The
/// exclusions are copied and validated before return. Limit: 64 KiB / 2048 IDs.
///
/// # Safety
/// `excluded` must address `len` readable bytes for this call. Null is valid only
/// with length zero (equivalent to []). Destroy the returned handle exactly once,
/// after every scan/metadata reader has finished.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_catalog_create(
    profile: u32,
    excluded: *const u8,
    len: usize,
) -> *mut CatalogSnapshot {
    let profile = match profile {
        1 => ClientProfile::CliV1,
        2 => ClientProfile::GuiV1,
        _ => return ptr::null_mut(),
    };
    if len > 65_536 || (excluded.is_null() && len != 0) {
        return ptr::null_mut();
    }
    let bytes = if len == 0 {
        b"[]".as_slice()
    } else {
        unsafe { std::slice::from_raw_parts(excluded, len) }
    };
    let Ok(ids) = serde_json::from_slice::<Vec<String>>(bytes) else {
        return ptr::null_mut();
    };
    if ids.len() > 2048 {
        return ptr::null_mut();
    }
    let ids: Vec<_> = ids.iter().map(String::as_str).collect();
    CatalogSnapshot::basic(profile, &ids).map_or(ptr::null_mut(), |snapshot| {
        Box::into_raw(Box::new(snapshot))
    })
}

/// # Safety
/// `catalog` must be null or a live handle returned by `bh_catalog_create`.
/// The returned immutable UTF-8 JSON pointer is borrowed until handle destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_catalog_json(catalog: *const CatalogSnapshot) -> *const c_char {
    unsafe { catalog.as_ref() }.map_or(ptr::null(), |snapshot| snapshot.metadata.as_ptr())
}

/// # Safety
/// `catalog` must be null or an exclusively owned live handle returned by
/// `bh_catalog_create`; no scan/reader may retain it after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_catalog_destroy(catalog: *mut CatalogSnapshot) {
    if !catalog.is_null() {
        drop(unsafe { Box::from_raw(catalog) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusions_are_copied_and_identity_belongs_to_the_full_profile() {
        let enabled = CatalogSnapshot::basic(ClientProfile::GuiV1, &[]).unwrap();
        let disabled =
            CatalogSnapshot::basic(ClientProfile::GuiV1, &["swift.build", "future.framework"])
                .unwrap();
        assert_eq!(
            enabled.catalog().catalog_digest(),
            disabled.catalog().catalog_digest()
        );
        let metadata: serde_json::Value = serde_json::from_str(disabled.metadata_json()).unwrap();
        assert_eq!(metadata["format"], "buildhunter.catalog/1");
        assert_eq!(metadata["profile"], "buildhunter.gui/1");
        assert_eq!(metadata["targets"].as_array().unwrap().len(), 13);
        assert_eq!(
            metadata["excludedTargetIDs"],
            serde_json::json!(["future.framework", "swift.build"])
        );
        assert_eq!(disabled.target("rust.target").unwrap().ecosystem, "rust");
        assert!(disabled.target("future.framework").is_none());
    }

    #[test]
    fn catalog_abi_refuses_invalid_input_and_owns_its_snapshot() {
        unsafe {
            assert!(bh_catalog_create(9, ptr::null(), 0).is_null());
            assert!(bh_catalog_create(1, ptr::null(), 1).is_null());
            assert!(bh_catalog_create(1, ptr::null(), 65_537).is_null());
            for bytes in [b"{}".as_slice(), b"[null]", b"[\"../bad\"]"] {
                assert!(bh_catalog_create(1, bytes.as_ptr(), bytes.len()).is_null());
            }
            let mut bytes = b"[\"swift.build\"]".to_vec();
            let handle = bh_catalog_create(2, bytes.as_ptr(), bytes.len());
            assert!(!handle.is_null());
            bytes.fill(0);
            let json = std::ffi::CStr::from_ptr(bh_catalog_json(handle))
                .to_str()
                .unwrap();
            assert!(json.contains("swift.build"));
            assert!((*handle).catalog().exclusions().contains("swift.build"));
            bh_catalog_destroy(handle);
            bh_catalog_destroy(ptr::null_mut());
            assert!(bh_catalog_json(ptr::null()).is_null());
        }
    }
}
