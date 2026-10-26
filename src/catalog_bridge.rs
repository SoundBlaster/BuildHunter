//! Catalog event ABI 1. Descriptors come from the immutable metadata snapshot;
//! discoveries carry stable target IDs, with no per-candidate Swift policy callback.
use crate::{
    BHScanEvent, ScanControl, ScanOptions, ScanStatus,
    catalog::{CATALOG_ABI_VERSION, CatalogSnapshot},
    client_scan::{CatalogDiagnostic, CatalogScanEvent, scan_client},
};
use std::{ffi::c_void, ptr};

#[repr(C)]
pub struct BHCatalogScanEvent {
    pub version: u32,
    pub base: BHScanEvent,
    pub target_id: *const u8,
    pub target_id_len: usize,
    pub metadata: *const u8,
    pub metadata_len: usize,
}
pub type BHCatalogEventCallback = unsafe extern "C" fn(*mut c_void, *const BHCatalogScanEvent);

pub fn diagnostic_json(diagnostic: &CatalogDiagnostic) -> serde_json::Value {
    let classification = &diagnostic.classification;
    serde_json::json!({
        "kind": match classification.decision { buildhunter_plugin_policy::Decision::Ambiguous => "ambiguous", _ => "indeterminate" },
        "matchedTargetIDs": classification.matched,
        "unknownTargetIDs": classification.unknown,
        "unavailable": classification.unavailable.iter().map(|notice| serde_json::json!({"packageID":notice.package_id,"factID":notice.fact_id,"reason":notice.reason})).collect::<Vec<_>>(),
    })
}

/// Synchronous catalog scan. Event 6 publishes snapshot metadata before any scan
/// event, 1 carries a target ID, 7 carries a typed diagnostic JSON object. Base
/// event types 2/3/4/5 and status codes retain the original ABI meanings. Profile
/// event 8 optionally publishes a bounded rule-profile aggregate before Finished;
/// event 5 optionally carries resource-budget/loss JSON in metadata; the 88-byte
/// numeric profile payload is unchanged.
///
/// # Safety
/// Control/catalog/root/context and callback must remain valid through the call.
/// The callback copies borrowed payloads before returning and must not unwind.
/// Destroy neither handle until return; cancellation may run concurrently with
/// the scan, but must be serialized with control destruction as in the old ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bh_scan_catalog_v1(
    control: *mut ScanControl,
    catalog: *const CatalogSnapshot,
    root: *const u8,
    root_len: usize,
    apparent_size: u8,
    include_environments: u8,
    callback: Option<BHCatalogEventCallback>,
    context: *mut c_void,
) -> i32 {
    let (Some(control), Some(catalog), Some(callback)) = (
        unsafe { control.as_ref() },
        unsafe { catalog.as_ref() },
        callback,
    ) else {
        return 3;
    };
    if root.is_null() || root_len == 0 {
        return 3;
    }
    let root = crate::path_from_bytes(unsafe { std::slice::from_raw_parts(root, root_len) });
    let mut finished = false;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let metadata = catalog.metadata_json().as_bytes();
        let start = BHCatalogScanEvent {
            version: CATALOG_ABI_VERSION,
            base: BHScanEvent {
                event_type: 6,
                ..BHScanEvent::default()
            },
            target_id: ptr::null(),
            target_id_len: 0,
            metadata: metadata.as_ptr(),
            metadata_len: metadata.len(),
        };
        unsafe {
            callback(context, &start);
        }
        let report = scan_client(
            &root,
            catalog,
            ScanOptions {
                apparent_size: apparent_size != 0,
            },
            include_environments != 0,
            control,
            |event| {
                let mut ffi = BHCatalogScanEvent {
                    version: CATALOG_ABI_VERSION,
                    base: BHScanEvent::default(),
                    target_id: ptr::null(),
                    target_id_len: 0,
                    metadata: ptr::null(),
                    metadata_len: 0,
                };
                let mut path = Vec::new();
                let mut message = String::new();
                let mut details = String::new();
                match event {
                    CatalogScanEvent::ArtifactDiscovered(artifact) => {
                        ffi.base.event_type = 1;
                        ffi.base.artifact_id = artifact.id;
                        path = crate::path_bytes(&artifact.relative_path);
                        ffi.target_id = artifact.target.id.as_ptr();
                        ffi.target_id_len = artifact.target.id.len();
                    }
                    CatalogScanEvent::ArtifactCompleted { id, bytes, partial } => {
                        ffi.base.event_type = 2;
                        ffi.base.artifact_id = *id;
                        ffi.base.bytes = *bytes;
                        ffi.base.partial = u8::from(*partial);
                    }
                    CatalogScanEvent::Warning(warning) => {
                        ffi.base.event_type = 3;
                        path = crate::path_bytes(&warning.relative_path);
                        message = warning.message.clone();
                    }
                    CatalogScanEvent::Diagnostic(diagnostic) => {
                        ffi.base.event_type = 7;
                        path = crate::path_bytes(&diagnostic.relative_path);
                        let diagnostic = diagnostic_json(diagnostic);
                        message = format!(
                            "Plugin classification {}: {}",
                            diagnostic["kind"].as_str().unwrap_or("unavailable"),
                            diagnostic
                        );
                        details = diagnostic.to_string();
                    }
                    CatalogScanEvent::Profile(sample) => {
                        ffi.base.event_type = 5;
                        ffi.base.profile = sample;
                        details = control.resource_usage_json().to_string();
                    }
                    CatalogScanEvent::RuleProfile(profile) => {
                        ffi.base.event_type = 8;
                        details = profile.to_string();
                    }
                    CatalogScanEvent::Finished(status) => {
                        ffi.base.event_type = 4;
                        ffi.base.status = crate::status_code(*status) as u32;
                        finished = true;
                    }
                }
                ffi.base.path = path.as_ptr();
                ffi.base.path_len = path.len();
                ffi.base.message = message.as_ptr();
                ffi.base.message_len = message.len();
                ffi.metadata = details.as_ptr();
                ffi.metadata_len = details.len();
                unsafe {
                    callback(context, &ffi);
                }
            },
        );
        crate::status_code(report.status)
    }));
    result.unwrap_or_else(|_| {
        if !finished {
            let terminal = BHCatalogScanEvent {
                version: CATALOG_ABI_VERSION,
                base: BHScanEvent {
                    event_type: 4,
                    status: crate::status_code(ScanStatus::Failed) as u32,
                    ..BHScanEvent::default()
                },
                target_id: ptr::null(),
                target_id_len: 0,
                metadata: ptr::null(),
                metadata_len: 0,
            };
            unsafe {
                callback(context, &terminal);
            }
        }
        3
    })
}
