# Bundled catalog admission — 2026-10-31

The signed Debug app/test targets built successfully with commerce disabled.
The app-hosted focused test attempt timed out before entering any test body;
its runtime result is **unverified**, not passed. The orphaned test host from
this attempt was stopped; the separately running user Release app was retained.

The auxiliary SwiftPM harness executed the six production BundledCatalogTests,
all passed, using the real Rust static library. It copies UserTemplates.swift
with only a C-module import, extracts the actual metadata/kind declarations and
copies the existing test suite. Source, generated-file and archive hashes are in
harness-checks.json. run-harness.py reproduces those transforms. It verifies the
catalog boundary without AppKit; it is not a sandbox, full scan lifecycle or
StoreKit transaction test. The local Rust archive targets macOS 26, so this
harness matches that target and does not establish oldest-OS compatibility.

Checked cases: basic/user rules with locked or corrupt paid resources, user
validation diagnostics, combined allowed catalogs, exact expiry boundary,
immutable admitted handles after revocation, SHA256 integrity and unsafe paths.
No private rule bytes enter this public evidence.

Seven HC/SpecPM structural commands passed; existing unresolved-capability warnings
retain their meaning. The changes are folded into TemplateManagement edition
resources and MacApplication's optional private assembly import. StoreKit itself
remains outside this public package. Recovered constraints stay draft. Historical
receipts are not relabeled. Source identities below identify these changed inputs;
the baseline commit alone does not identify the working changes.
