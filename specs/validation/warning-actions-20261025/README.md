# Warning actions — 2026-10-25

Scope: the dedicated warning table's Copy/Open in Finder menus and exact warning
locations. Current composition is unchanged. `current.bridge.warning_locations`
and `current.application.warning_actions` are draft descriptive constraints; no
owner acceptance, Store distribution or process-wide resource guarantee follows.

## Executed checks

- Debug `build-for-testing` passed. All 146 unit tests passed, including separate
  exact paths, duplicate-message ambiguity, reset/generation, diagnostic locations,
  invalid UTF-8, lossy roots and empty-path regressions.
- The first combined test command exited 65: unit tests passed, but macOS rejected
  the unsigned UI runner as damaged. That command is not a passing UI result.
- A rebuild with local ad-hoc signing passed. App and runner both passed
  `codesign --verify --deep --strict --verbose=2`. No Gatekeeper/quarantine settings
  were changed. This does not establish notarization or Developer ID distribution.
- Signed UI retry and native observation are recorded in `runtime-results.json`.
  A started test is not treated as a pass.
- Apple Development and single-binary Debug retries also failed at application
  launch, before menu assertions. Sampled processes were in `dyld` opening a
  dependency before application code; the exact blocked file was not identified.
- Release built with the project's Apple Development signing and launched.
  Native CUA scanned a disposable directory containing `blocked: folder` with
  mode 000, producing a diagnostic, an I/O warning and an incomplete summary.
  Copy/Open were visible and enabled for the I/O warning; Copy matched the full
  message exactly. Open selected that exact URL in Finder. The locationless summary
  kept Copy enabled and Open disabled. Multi-row keyboard Copy matched both full
  messages in report order. Screenshots and AX were inspected in the tool session.
  Fixture permissions were restored after verification. This bounded observation
  is separate from the blocked automated Debug test and is not broad GUI coverage.

`commands.json` records actual commands, exits, elapsed times and TB5 log hashes;
raw Xcode logs and result bundles remain on the verified external TB5 disk.
`source-identity.json` binds the runtime commit and current file hashes.
`structural-commands.json` and `specpm.json` record fresh structural validation.
Structural validation does not prove menu hit targets, Finder completion or
behavioral coverage beyond the tests/observations listed here. Previous receipts
are preserved with their original source identities.
