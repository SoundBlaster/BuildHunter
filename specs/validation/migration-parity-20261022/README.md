# Migration baseline validation — corpus/CLI layer

Baseline main: `1fd905620c0d5c0317c6b08123cfe5b0653253c2`.
Working-tree hashes are in `source-identity.json`; this layer changes test code,
reviewed expected data, CI invocation and documentation only. `scope-check.json`
checks unchanged product manifests/policy/bridge and the preserved primary
checkout scheme. `src/lib.rs` only adds a cfg(test) module.

## Executed

- Debug and Release all-feature tests: 68 BuildHunter library + 4 CLI + 135
  vendored core tests, plus 2 compile-fail doctests, all passed in each mode.
- New real-CLI suite: 3 tests, including 30 CLI runs in each of allocated/apparent
  modes (60 scan invocations on macOS), catalog IDs and lifecycle/terminal-vector
  consistency. Existing CLI suite: 13 tests passed.
- cargo fmt and all-target/all-feature Clippy with warnings denied passed.
- HC current/target/CI parse and seven selected profile validations passed.
  SpecPM: 0 errors, 39 existing unresolved-capability warnings. These are
  structural checks, not acceptance. Exact tools and raw commands are retained.

The initial successful CLI run used checkout-specific TB5 scratch. A repeat with
standard macOS TMPDIR exposed a harness comparison bug (`/var` vs canonical
`/private/var`), not different scanner rows. Normalizing only the relative-path
comparison base fixes it; final standard-TMPDIR run passes. The earlier failing
command is retained. During fixture development, the missing-root expectation
was corrected to the actual CLI preflight rejection, and the fault oracle was
corrected to use directory metadata acquired before child deletion. Neither
correction changes production behavior or regenerates goldens from test output.

## Limits

28 GUI run expectations and invalid-byte/terminal vectors are reviewed data here;
the Swift execution witness is the upper stack layer. Symlink fixtures execute
on Unix; the Python runner skips their case on Windows. Actual Linux/Windows
execution is pending CI. No new plugin/default runtime, safety ABI, live home/
iCloud, UI Stop latency, Finder action identity or resource budget acceptance.
C2 differences are preserved and C3/C4 remain explicit migration gates.

Build outputs and supported caches use verified TB5 routes in `build-routing.json`.
Temporary test trees are owned and cleaned by their harnesses; no user data is
modified. Historical receipts remain historical.
