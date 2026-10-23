# Prepared directory size lookup

Layer two on the Darwin acquisition commit recorded in source-identity.json. Working-tree source/map/contract hashes are distinct from that immutable baseline. The existing Darwin DirectorySizes API remains a re-export of the portable type. PreparedDirectorySizes performs no I/O and adds no stop check. Tests cover consumed/missing entries, cached entry errors, repeated directory-wide errno/custom errors, Unsupported, Unicode normalization differences and raw Unix names. Host paths must come from the same acquired directory.

Default: 131 workspace unit tests; all-features Debug/Release: 138 (129 core + 9 Unix); two compile-fail doctests. Fmt/all-feature clippy, HC parse/validate and SpecPM pass (zero errors, 14 unresolved-capability warnings). Raw commands and TB5 routing retained. Existing runtime/launch tests remain unchanged. These local macOS results are not fresh hosted Windows/Linux, live cloud/UI, bounded memory or general speedup evidence.
