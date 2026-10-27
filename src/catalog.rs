//! Immutable, versioned catalog identity shared by CLI and the application bridge.
pub use buildhunter_plugin_policy::builtin::ClientProfile;
use buildhunter_plugin_policy::{Origin, PluginCatalog, PluginInput, TargetDescriptor, TargetKind};
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

    /// Built-ins retain their trusted origin; files supplied by a user cannot claim it.
    pub fn with_templates(
        profile: ClientProfile,
        documents: &[&[u8]],
        excluded: &[&str],
    ) -> Result<Self, TemplateLoadError> {
        if documents.len() > 61 {
            return Err(TemplateLoadError::limit(
                "At most 61 user templates may be enabled.",
            ));
        }
        if let Some(index) = documents.iter().position(|bytes| bytes.len() > 1_048_576) {
            let mut error = TemplateLoadError::limit("A template exceeds 1 MiB.");
            error.template_index = Some(index);
            return Err(error);
        }
        if documents.iter().map(|bytes| bytes.len()).sum::<usize>() > 16 * 1_048_576 {
            return Err(TemplateLoadError::limit("Template byte limit exceeded."));
        }
        for (index, bytes) in documents.iter().enumerate() {
            PluginCatalog::load(
                &[PluginInput {
                    bytes,
                    origin: Origin::External,
                }],
                &[],
            )
            .map_err(|e| TemplateLoadError {
                code: e.code.into(),
                location: e.location,
                message: e.message,
                template_index: Some(index),
            })?;
        }
        let builtin = profile.documents();
        let inputs: Vec<_> = builtin
            .iter()
            .map(|bytes| PluginInput {
                bytes,
                origin: Origin::Builtin,
            })
            .chain(documents.iter().map(|bytes| PluginInput {
                bytes,
                origin: Origin::External,
            }))
            .collect();
        let catalog = PluginCatalog::load(&inputs, excluded).map_err(|e| TemplateLoadError {
            code: e.code.into(),
            location: e.location,
            message: e.message,
            template_index: None,
        })?;
        let mut snapshot = Self::from_catalog(profile, catalog);
        let mut metadata: serde_json::Value = serde_json::from_str(snapshot.metadata_json())
            .map_err(|_| TemplateLoadError::internal())?;
        let packages: Vec<_> = inputs
            .iter()
            .map(|input| {
                // Strict admission has already rejected duplicate keys and invalid JSON.
                let value: serde_json::Value = serde_json::from_slice(input.bytes)
                    .map_err(|_| TemplateLoadError::internal())?;
                Ok(serde_json::json!({"id":value["id"], "version":value["version"]}))
            })
            .collect::<Result<_, TemplateLoadError>>()?;
        metadata["packages"] = packages.into();
        snapshot.metadata =
            CString::new(metadata.to_string()).map_err(|_| TemplateLoadError::internal())?;
        Ok(snapshot)
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
            "packages": catalog.packages().iter().map(|package| serde_json::json!({
                "id":package.id, "version":package.version,
            })).collect::<Vec<_>>(),
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

#[derive(Debug)]
pub struct TemplateLoadError {
    pub code: String,
    pub location: String,
    pub message: String,
    pub template_index: Option<usize>,
}

impl TemplateLoadError {
    fn limit(message: &str) -> Self {
        Self {
            code: "limit_exceeded".into(),
            location: String::new(),
            message: message.into(),
            template_index: None,
        }
    }
    fn internal() -> Self {
        Self {
            code: "internal_error".into(),
            location: String::new(),
            message: "Could not prepare the template catalog.".into(),
            template_index: None,
        }
    }
    fn json(&self) -> String {
        serde_json::json!({"code":self.code,"location":self.location,"message":self.message,
            "templateIndex":self.template_index})
        .to_string()
    }
}

#[repr(C)]
pub struct BHPluginDocument {
    pub bytes: *const u8,
    pub len: usize,
}

pub type BHCatalogErrorCallback = unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize);

/// Copy/validate bounded borrowed documents before returning an immutable handle.
/// Failure publishes one borrowed UTF-8 JSON diagnostic and returns null; no partial
/// catalog escapes. ABI 1 is additive: existing callers keep bh_catalog_create.
///
/// # Safety
/// Non-null inputs address the stated readable lengths; documents address count
/// aligned BHPluginDocuments. All payloads live through this synchronous call.
/// The optional callback must not unwind or retain its borrowed bytes. A returned
/// handle has the same ownership/lifetime as bh_catalog_create.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_catalog_create_with_templates_v1(
    profile: u32,
    excluded: *const u8,
    excluded_len: usize,
    documents: *const BHPluginDocument,
    count: usize,
    error_callback: Option<BHCatalogErrorCallback>,
    context: *mut std::ffi::c_void,
) -> *mut CatalogSnapshot {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let profile = match profile {
            1 => ClientProfile::CliV1,
            2 => ClientProfile::GuiV1,
            _ => return Err(TemplateLoadError::internal()),
        };
        if count > 61
            || excluded_len > 65_536
            || (count > 0 && documents.is_null())
            || (excluded_len > 0 && excluded.is_null())
        {
            return Err(TemplateLoadError::limit(
                "Invalid catalog input or byte/package limit exceeded.",
            ));
        }
        let documents = if count == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(documents, count) }
        };
        let mut bytes = Vec::with_capacity(count);
        let mut total = 0usize;
        for document in documents {
            if document.len > 1_048_576 || (document.len > 0 && document.bytes.is_null()) {
                return Err(TemplateLoadError::limit(
                    "A template exceeds 1 MiB or has an invalid buffer.",
                ));
            }
            total += document.len;
            if total > 16 * 1_048_576 {
                return Err(TemplateLoadError::limit(
                    "Templates exceed the catalog byte budget.",
                ));
            }
            bytes.push(if document.len == 0 {
                &[][..]
            } else {
                unsafe { std::slice::from_raw_parts(document.bytes, document.len) }
            });
        }
        let exclusions = if excluded_len == 0 {
            b"[]".as_slice()
        } else {
            unsafe { std::slice::from_raw_parts(excluded, excluded_len) }
        };
        let ids: Vec<String> =
            serde_json::from_slice(exclusions).map_err(|_| TemplateLoadError::internal())?;
        if ids.len() > 2048 {
            return Err(TemplateLoadError::limit("Too many excluded target IDs."));
        }
        let ids: Vec<_> = ids.iter().map(String::as_str).collect();
        CatalogSnapshot::with_templates(profile, &bytes, &ids)
    }))
    .unwrap_or_else(|_| Err(TemplateLoadError::internal()));
    match result {
        Ok(snapshot) => Box::into_raw(Box::new(snapshot)),
        Err(error) => {
            if let Some(callback) = error_callback {
                let json = error.json();
                unsafe {
                    callback(context, json.as_ptr(), json.len());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Static example document; the caller never frees this pointer.
#[unsafe(no_mangle)]
pub extern "C" fn bh_template_example_json() -> *const c_char {
    concat!(
        include_str!("../docs/contracts/plugin-v1/examples/synthetic.json"),
        "\0"
    )
    .as_ptr()
    .cast()
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

    const TEMPLATE: &[u8] = include_bytes!("../docs/contracts/plugin-v1/examples/synthetic.json");

    #[test]
    fn user_templates_extend_builtins_and_keep_immutable_identity() {
        let snapshot =
            CatalogSnapshot::with_templates(ClientProfile::GuiV1, &[TEMPLATE], &[]).unwrap();
        assert_eq!(snapshot.catalog().targets().len(), 14);
        assert!(snapshot.target("example.forge-output").is_some());
        let other = CatalogSnapshot::with_templates(
            ClientProfile::GuiV1,
            &[TEMPLATE],
            &["example.forge-output"],
        )
        .unwrap();
        assert_eq!(
            snapshot.catalog().catalog_digest(),
            other.catalog().catalog_digest()
        );
        let metadata: serde_json::Value = serde_json::from_str(snapshot.metadata_json()).unwrap();
        assert_eq!(metadata["packages"].as_array().unwrap().len(), 4);
        assert_ne!(
            snapshot.catalog().catalog_digest(),
            CatalogSnapshot::basic(ClientProfile::GuiV1, &[])
                .unwrap()
                .catalog()
                .catalog_digest()
        );
        let duplicate =
            CatalogSnapshot::with_templates(ClientProfile::GuiV1, &[TEMPLATE, TEMPLATE], &[]);
        assert_eq!(duplicate.err().unwrap().code, "duplicate_id");
    }

    #[test]
    fn malformed_templates_never_escape_a_partial_catalog() {
        let mut invalid = vec![
            vec![],
            b"{".to_vec(),
            b"null".to_vec(),
            vec![0xff],
            b"{\"id\":1,\"id\":2}".to_vec(),
            vec![b'['; 100],
            vec![b' '; 1_048_577],
        ];
        invalid.push(
            String::from_utf8_lossy(TEMPLATE)
                .replace("1-draft.1", "999")
                .into_bytes(),
        );
        invalid.push(
            String::from_utf8_lossy(TEMPLATE)
                .replace("example.forge-output", "swift.custom")
                .into_bytes(),
        );
        for bytes in invalid {
            let error =
                CatalogSnapshot::with_templates(ClientProfile::GuiV1, &[TEMPLATE, &bytes], &[])
                    .err()
                    .unwrap();
            assert_eq!(error.template_index, Some(1));
            assert!(!error.message.is_empty());
        }
        assert!(
            CatalogSnapshot::with_templates(ClientProfile::GuiV1, &[TEMPLATE; 62], &[]).is_err()
        );
    }

    unsafe extern "C" fn receive_error(
        context: *mut std::ffi::c_void,
        bytes: *const u8,
        len: usize,
    ) {
        let output = unsafe { &mut *context.cast::<Vec<u8>>() };
        output.extend_from_slice(unsafe { std::slice::from_raw_parts(bytes, len) });
    }

    #[test]
    fn template_ffi_owns_source_and_reports_bounded_failures() {
        unsafe {
            let mut error = Vec::<u8>::new();
            let context = (&mut error as *mut Vec<u8>).cast();
            assert!(
                bh_catalog_create_with_templates_v1(
                    2,
                    ptr::null(),
                    0,
                    ptr::null(),
                    62,
                    Some(receive_error),
                    context
                )
                .is_null()
            );
            assert!(String::from_utf8_lossy(&error).contains("limit_exceeded"));
            error.clear();
            let invalid = BHPluginDocument {
                bytes: b"{".as_ptr(),
                len: 1,
            };
            assert!(
                bh_catalog_create_with_templates_v1(
                    2,
                    ptr::null(),
                    0,
                    &invalid,
                    1,
                    Some(receive_error),
                    context
                )
                .is_null()
            );
            let diagnostic: serde_json::Value = serde_json::from_slice(&error).unwrap();
            assert_eq!(diagnostic["templateIndex"], 0);
            let mut source = TEMPLATE.to_vec();
            let doc = BHPluginDocument {
                bytes: source.as_ptr(),
                len: source.len(),
            };
            let handle = bh_catalog_create_with_templates_v1(
                2,
                ptr::null(),
                0,
                &doc,
                1,
                None,
                ptr::null_mut(),
            );
            assert!(!handle.is_null());
            source.fill(0);
            assert!((*handle).target("example.forge-output").is_some());
            bh_catalog_destroy(handle);
        }
    }

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
