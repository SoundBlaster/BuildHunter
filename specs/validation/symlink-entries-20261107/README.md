# Symlink entry validation — 2026-11-07

Baseline revision and SHA-256 identities for the changed source, map and contract
inputs are recorded in `source-identity.json`. The baseline is not the complete
working-tree identity; use the hashes for edited files. Previous dated receipts
and the frozen migration corpus remain historical.

Locally on macOS/arm64 with builds and test fixtures on the verified TB5 volume:

- `cargo test --locked --all-features`: 103 library and 5 CLI unit tests passed.
- Policy package: 2 unit and 20 integration tests passed; strict scanner: 20
  integration tests passed. Clippy passed with warnings denied for all three.
- Release CLI integration: 16 tests; migration corpus: 3 tests; dependency graph
  tests: 10; vendor tests: 4. Fresh Cargo home offline check, install and launch passed.
- Xcode selected `MigrationParityTests`, `ArtifactFolderLocationTests`,
  `ScanReportExportTests`, and `WindowScanModelTests`: 52 tests passed.
  Result bundle: TB5 `xcode/Symlink-policy-20261107.xcresult` under the
  per-worktree build root.
- String Catalog Doctor: zero issues. Hypercode parsed `current.hc` and validated
  CLI and GUI contexts without issues. `specpm validate . --json`: zero errors,
  54 unresolved-capability warnings, unchanged from the earlier baseline.
- `cargo bench --locked --bench catalog_scan`: seven warm synthetic rounds;
  13-target median 11,256 µs, 200-target median 11,339 µs, gate 42,737 µs.
  This compares catalog sizes on this revision, not before/after symlink cost or
  real-user performance.

The execution evidence covers direct catalog, scoped scanner and Swift event
consumers, including missing and looping targets, marker links and explicitly
included/excluded symlink-kind rules. Finder selection
is a request: tests establish the selected URL, not observed Finder completion.
The default pathname client does not claim descriptor confinement. No signed
MAS build, live provider or TestFlight run is implied by these local results.
