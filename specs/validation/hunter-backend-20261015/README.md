# Native backend/root observation adoption evidence

Local macOS: 66 default / 70 all-feature root tests, all-feature Release, 54 exact
core tests in Debug/Release, 12 plugin scanner tests in Debug/Release, 13 CLI tests
and four vendor mutation tests pass. Clippy, root/vendor fmt and fresh-Cargo-home
offline check/install/launch pass. Separate root/plugin TB5 targets were preflighted
and confirmed with Cargo metadata; test/offline fixtures also use that volume.
Only actual compiler/fixture locations and verified volume identity are recorded.

New application witnesses verify one selected-file metadata observation reused for
measurement, a single terminal event with/without pre-cancellation, missing selected
root as warning/Incomplete, and final symlinks to file/directory/missing targets
without traversal. Existing cloud guards/probe hooks, marker-following exceptions,
bulk fallback, nested sizes, profile/cancel/panic and plugin regression gates pass.
Five macOS backend tests and initial absent-API red phase are recorded upstream;
Linux raw-name behavior is separately tested in hosted CI.

Paired Release fixtures against main 7ef5bd98 pass all eight noise-adjusted budgets.
Both variants, fixture and temporary baseline worktree use the same verified TB5
volume/host. Raw samples/MAD/harness/compiler identity are in performance-report.json;
its candidate SHA pins the immutable runtime commit, not this later docs commit.
No speedup, process RAM reduction or fixed memory bound is inferred.

SpecPM: zero errors / 24 unresolved external-capability warnings, including the new
Hunter backend dependency. Both maps parse and all four profiles validate. All 11
canonical files match pinned upstream Git objects; all 16 implementation binding
paths exist and match the immutable runtime commit. Source/map/tool identities,
frozen maps/upstream draft and actual commands/results are included. Historical
records stay unchanged. Structural success is not runtime or owner acceptance.

Only final-component metadata no-follow is provided. Intermediate components and
read_dir remain path-based and may follow links/replacements. BuildHunter retains
cloud/access admission, guards/error decoration, marker/bulk effects, scheduling,
worker lifetime and final status/events. Full startup/backend assembly and default
plugin migration remain pending. No local Xcode/UI/home/live-provider run occurred.
