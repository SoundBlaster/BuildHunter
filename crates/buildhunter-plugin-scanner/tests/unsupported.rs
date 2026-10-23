#![cfg(not(any(target_os = "macos", target_os = "linux")))]
use build_hunter::{ScanControl, ScanOptions};
use buildhunter_plugin_policy::PluginCatalog;
use buildhunter_plugin_scanner::{ScanError, scan};
#[test]
fn unsupported_backend_fails_before_callbacks_or_filesystem_access() {
    let catalog = PluginCatalog::load(&[], &[]).unwrap();
    let result = scan(
        std::path::Path::new("does-not-exist"),
        &catalog,
        ScanOptions::default(),
        &ScanControl::new(),
        |_| panic!("no scan event before supported preflight"),
    );
    assert!(matches!(result, Err(ScanError::UnsupportedPlatform)));
}
