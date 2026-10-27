# Optional rule timing — 2026-10-26

Baseline 693a284123faaa1ad3192cd88b259ba97a7b5910. Source/map hashes in source-identity.json.
Executed locally on macOS 27, Swift 6.4/Xcode 27.1 and rustc 1.97.0:
- cargo fmt --check: pass.
- cargo test --locked --all-targets: 28 tests passed.
- cargo test --locked --release --all-targets: 28 tests passed (check-1.log).
- cargo clippy --locked --all-targets -- -D warnings: pass (check-2.log).
- HC parse/validation and SpecPM: actual results in commands.json.
Build outputs routed to verified TB5 worktree-specific Cargo directory.
Tests compare timed/ordinary outcomes, diagnostics/errors/order and concurrent use.
No timing threshold, speed improvement, overhead measurement, filesystem or GUI
acceptance is claimed. Clock observations are advisory and intentionally vary.
BoundarySpec remains draft. CI is separate from these local checks.
