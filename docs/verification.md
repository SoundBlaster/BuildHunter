# BuildHunter verification

## Local checks

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
python3 scripts/test_cli.py target/release/build-hunter
bash scripts/ci/macos.sh test
bash scripts/ci/macos.sh release
```

Windows uses `target/release/build-hunter.exe`. CLI integration fixtures cover
language filters, marker-based classification, nested aggregation, JSON escaping,
environment opt-in, symlink/.git exclusion, invalid arguments and read-only behavior.

macOS commands require Apple Silicon and Xcode with the macOS 26+ SDK. The shared
project pins SpecificationCore and commits `Package.resolved`. The CI script uses
`-skipMacroValidation`, explicitly authorized for this project's builds: this
disables validation for every macro in that invocation, not just SpecificationCore.
No persistent Xcode trust defaults are changed.

The test script checks `.xcresult` for at least one passing executed test, so a
successful build with zero discovered tests cannot pass as test evidence. Test
results, summary and logs live under ignored `macos/.build/ci/`. A repeat test run
requires moving the prior result bundle aside or using a clean output directory.

## GitHub checks

- **Rust CI:** fmt, unit tests, Clippy, Release build and actual CLI process/fixture
  integration tests on Ubuntu, macOS and Windows.
- **macOS CI / macOS unit and UI tests:** unsigned arm64 app/test compilation and
  execution on macOS 26 with Xcode 26.6. UI tests query NestedA11yIDs identifiers,
  exercise all Debug mock states, and attach screenshots to `.xcresult`; the
  script exports and counts the seven scenario/picker screenshots. The workflow
  publishes the bundle, screenshots, summary and log.
- **macOS CI / macOS Release build:** independent unsigned Release compilation;
  publishes the log.

These workflows run for every pull request, main push and manual dispatch.
No path filters suppress checks needed by a PR. GitHub branch protection settings
are separate from workflow creation; configuring required checks is not implied.

## TDD and evidence boundaries

For new domain rules, state transitions and integration contracts, add a meaningful
failing behavior test before implementing the change, capture its failure, then
run the targeted test and applicable broader checks after the minimal fix.
Use injected event sources and explicit continuations for asynchronous tests,
without timing sleeps or dependence on the live filesystem unless testing that
boundary. Policy tests exercise supported outcomes, overlaps/priority and no-match.

The default GUI scan now uses the Rust static library through C callbacks; Debug
mock states remain available for deterministic UI testing. Rust core tests cover
candidate policy adapters, nested roots, cancellation, and lossless Unix path bytes.
A Swift integration test exercises classification, streaming, and measured results
through the real FFI. The current unsigned test run does not prove signed sandbox
access, Finder integration, clipboard behavior, or App Store readiness.

Unsigned builds do not establish signed sandbox runtime behavior or App Store
readiness. A separate signed-app check must exercise folder drop/open grants,
multiwindow lifetime, Finder/clipboard and absence of filesystem writes. App Review
is a distinct gate.
