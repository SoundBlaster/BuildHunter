# Migration parity baseline (R2)

Baseline: merged main `1fd905620c0d5c0317c6b08123cfe5b0653253c2`.
This records current behavior before switching default clients to a plugin catalog;
it does not harmonize CLI/GUI or accept new filesystem/path semantics.

## Corpus and runners

`tests/fixtures/migration-parity-v1.json` contains ten owned trees and explicit
client runs. Expectations are reviewed data, never regenerated from scanner
output during tests. It covers all 13 stable filter IDs, each ID excluded alone,
all-disabled settings, a language selection, nested roots, excluded outer roots,
`.git` outside/inside artifacts, environment defaults/names/precedence, negative
markers, metadata/bytecode files, Unicode, symlink markers/artifacts (Unix),
selected artifact roots, missing roots and selected files.

- `scripts/test_migration_parity.py <compiled-cli>` executes the real CLI for
  every CLI run in apparent and allocated modes. CLI profile JSON supplies terminal
  status; report fields and totals are compared with the golden rows. Catalog IDs
  and per-ID exclusion fixtures cover target identity; CLI artifact JSON itself
  does not yet carry a target ID. That ABI/output change remains R4.
- `MigrationParityTests` loads the same JSON from its test bundle and uses the
  real `RustScanEventSource`, actual Swift policy/table compilation and Rust FFI.
  It checks paths, language/scanner kind, descriptor IDs, allocated bytes, warnings,
  one completion per discovery and Finished last. This is a bridge integration
  test, not visual/UI interaction or signed sandbox acceptance.
- `migration-lifecycle-v1.tsv` freezes five deterministic core fault scenarios:
  completion, cancel at discovery, vanished file, permission failure and missing
  root. The Rust witness checks the real status-code mapping, partial flag,
  completion payload and final event ordering. JSON terminal vectors exercise
  the Swift bridge mapping; the Python test enforces agreement between both files.
  Fault injection is not a live provider or end-user Stop latency measurement.

Byte expectations use an independent traversal of each expected artifact's
fixture subtree: `lstat` length for apparent bytes, `st_blocks * 512` on Unix for
allocated bytes, and length on Windows. Directories are included; symlinks are
not followed or counted. This avoids fixed APFS/Linux directory sizes. Golden
classification/path expectations remain literal and independent of current policy.
Nested CLI bytes contribute to the report only through their outer root; GUI
fixtures contain only outer roots. Timing, IDs assigned by scan order and raw
localized OS error messages are deliberately not golden fields.

## Frozen profile differences (C2)

| Case | CLI baseline | GUI bridge baseline |
| --- | --- | --- |
| Virtual environments | Default excludes/prunes; opt-in recognizes `.venv`/`venv` with own marker | Includes own-marker environments with arbitrary names |
| Environment marker on `.build` | Default prunes; opt-in can classify Swift build | Environment classification has precedence |
| Nested findings | Listed and flagged nested, counted once in totals | Only outer roots reported |
| `__pycache__`, `.pyc`, `.egg-info` | Scanner kinds bytecode/metadata | Cache representation; standalone `.egg-info` file is not a GUI match |
| `.tox` / `.nox` | Cache | Test environment |
| Selected non-directory | CLI preflight rejects before scan/profile output | Bridge can scan an eligible file |
| Missing selected root | CLI preflight rejects (exit 2, no JSON/profile) | Core/bridge yields warning and failed/incomplete result |

A future change to these differences needs an explicit profile/migration decision
and reviewed expected-data diff. Passing this suite does not make the clients
semantically identical or validate plugin-runtime adoption.

## C3/C4 migration gates

**C3 baseline:** traversal and measurement skip artifact symlinks, while default
recognized marker checks follow file links. The opt-in plugin path has stricter
no-follow marker observations. Both contracts remain explicit; R2 preserves the
default behavior. A default plugin cutover must explicitly approve a versioned
marker-link delta or preserve the current profile. Descriptor marker tests do
not establish safety of the full pathname traversal.

**C4 baseline:** Rust paths remain raw bytes. The Swift bridge replaces invalid
UTF-8 for presentation, emits a warning and prevents a successful terminal result;
the synthetic byte vector locks that behavior. Existing Finder/copy URL creation
is not proven to preserve invalid-byte identity. Before a new path/action ABI or
plugin cutover, choose a lossless raw-path/handle contract or disable affected
actions. This remains an owner decision; these tests do not silently accept lossy
paths as safe handles or change Finder behavior.

Shared normal-tree tests, deterministic injected faults and terminal-decoder
vectors are distinct evidence. Live filesystem churn, stalled system I/O, actual
UI cancellation, iCloud and whole-process resource budgets remain outside R2.
Maps/specs stay draft; structural checks are not runtime conformance.
