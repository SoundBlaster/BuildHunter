# Bundled edition catalogs

A composition repository can set the generated Info.plist string
`BuildHunterBundledCatalog` to `CatalogFolder/catalog.json` and include that folder
as an application resource. Without this setting no bundle packages are added.

The UTF-8 manifest has this public wire shape (synthetic filenames only):

```json
{"format":"buildhunter.bundled-catalog/1","packages":[{"file":"package-0.json","sha256":"<64 lowercase hex SHA256 digits>"}]}
```

Names are single ASCII filename components; the setting contains exactly one
folder and filename. Final symlinks and resolved paths outside bundle resources
are rejected. Packages must be regular bounded files. The manifest is limited
to 64 KiB, at least one entry, unique filenames, and at most 61 packages.
SHA256 protects consistency between resource manifest and package bytes; it is
not a signature or a license entitlement. Application signing belongs to release
composition. Total user plus bundled documents share existing package/byte/Rifle
cost limits. An enabled broken resource does not silently fall back to basics.

The Swift catalog constructor admits bundle documents before user documents,
using external origin for both. Duplicate IDs/conflicts use the existing Rust
diagnostics; no priority override is added. User diagnostic indexes are adjusted
so existing path attribution stays correct. Draft Check, metadata preparation,
Save/export validation and new scans use this same constructor. Existing scans
retain their owned immutable Rust handle. Resource reads occur when catalog
preparation is requested, using the existing caller's threading arrangement.

Public tests use synthetic packages only. Private definitions and release pins
are owned by Arsenal/BuildHunterMAS. This enables local edition composition;
StoreKit purchases, trial enforcement, App Store signing and submission are
separate work and are not implemented here.
