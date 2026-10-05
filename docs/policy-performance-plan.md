# Policy performance series (issue 25)

Source: https://github.com/SoundBlaster/BuildHunter/issues/25 and its performance
addenda, including the corrected Swift specialization diagnosis.

The existing main branches are the baselines. Library changes use feature
branches/local checkouts during development; releases and version bumps happen
after integration validation. Independent PRs 23 (cloud guards) and 24 (language
badges) have also merged; neither implements the declarative plugin catalog.

## Ordered delivery

1. **Library foundations**: Swift cross-module inlining on real SpecificationCore
   (H6), Rust indexed first-match with exact priority (H3). Parity, short-circuit,
   tracing-on/off tests and reproducible Release measurements precede adoption.
2. **Application hot path**: bitmask facts instead of marker String sets (H9),
   reuse immutable specifications/filter plans (H8), and candidate/callback
   counters. Preserve CLI/app behavior, including their currently different
   environment defaults and kind presentation.
3. **Static dispatch**: keyed generated dispatch and balanced/fixed-arity Swift
   construction (H19/H20). Do not use parameter-pack iteration (H21). Test
   original priority across keyed and unkeyed rules and growing leaf types.
4. **Finite policy**: explicit pure decision contracts and per-scan decision
   tables over target identity/nesting (H13). Keep callbacks for policies that
   need unrestricted input. Require exhaustive table/direct parity and callback
   counts before claiming removal of the FFI cost.
5. **Runtime catalog integration**: shared classification for worker prefetch,
   indexed candidates and 200-target benchmark (H1/H2/H10). Declarative catalog,
   linter/compiled field tables and plugins are their own architectural stages;
   they must not silently change classification during a performance fix.
6. **Release alignment**: run application and library CI/performance gates, then
   raise versions and replace temporary dependencies together.

## Evidence requirements

- Use actual libraries, separate-module Swift compilation, Release settings,
  warm-up, repeated/interleaved samples and result parity outside timed regions.
- Record compiler, SHA, dirty state and raw samples. Best-of-only scratch numbers
  in the issue are motivating evidence, not acceptance thresholds.
- Keep semantic checks independent from noisy timing gates. Existing scanner
  comparison and Swift scaling gates continue to run in GitHub Actions.
- No unbounded classifier caches (H14). Faster hashing, leaf deduplication,
  predicate reordering and memoization (H4/H15–H18) require measurements and an
  explicit purity/key contract before implementation.
- Compilation, hostless unit tests, hosted Xcode/UI tests and signed sandbox
  runtime are reported separately.

## Issue 25 status ledger

Snapshot: **2026-10-05**, audited against BuildHunter main `57b9d51`. H1–H21 identify hypotheses and proposed optimizations
from the issue comments, not 21 confirmed bugs. The declarative target/plugin
proposal in the issue body is a separate scope and remains open.

Use these states consistently:

- **Pending**: no accepted implementation or verification yet.
- **Local**: implementation and tests exist in a worktree; no published PR yet.
- **PR / partial**: a published change covers the stated part; it is not merged
  or released, and remaining integration is named explicitly.
- **Guardrail**: a development constraint, not a claim of a measured speedup.
- **Resolved**: the required library and application changes are merged,
  semantic tests pass, and the claimed effect has reproducible evidence. Track
  released dependency versions separately; a library-only fix does not imply
  the app uses it.

References: [Swift library PR #16](https://github.com/SoundBlaster/SpecificationCore/pull/16),
[Rust library PR #19](https://github.com/SoundBlaster/specification-core-rs/pull/19),
[application PR #26](https://github.com/SoundBlaster/BuildHunter/pull/26),
[balanced Swift PR #17](https://github.com/SoundBlaster/SpecificationCore/pull/17),
[keyed static Rust PR #20](https://github.com/SoundBlaster/specification-core-rs/pull/20).
All referenced PRs are merged. Swift #16/#17 merged at `3cfe27b`, Rust
#19/#20 at `043a0ba`, and application #26/#27/#29/#30 through `aaa99f1`.
Cloud guard #23 and badges #24 bring application main to `57b9d51`.
Their final pre-merge CI checks passed and review threads were resolved.
This is not a claim about post-merge main CI or a completed home-directory scan.

The app still resolves Swift `perf/balanced-decisions` at `bacc4d6`, containing
the implemented performance APIs. After the library rebase, that commit was
preserved by `codex/pinned-performance-baseline` so fresh public HTTPS fetches
remain possible. Dependency release/version alignment is still pending.
BuildHunter's Cargo manifest has no dependencies: neither new Rust library API
has been adopted by the scanner.

| ID | Scope and current state | Evidence and remaining acceptance work |
| --- | --- | --- |
| H1 | Candidate-level specification cost: **partial** | Fixture callback counts exist. A representative completed real-tree profile remains necessary; the previous home scan stalled in cloud filesystem calls. |
| H2 | Large runtime catalogs: **partial, library only** | Merged Rust #19 benchmarks 14/214 rules. No application catalog or 200-target scanner scenario yet. |
| H3 | Indexed first-match: **partial, library merged** | Rust #19 has ordered keyed/unkeyed parity, duplicate-key, projection-count and shared-worker tests. Scanner adoption and Swift indexing remain pending. |
| H4 | Faster hashing: **pending** | No replacement or measured need; compare against the existing index first. |
| H5 | Static first-match: **partial** | Swift #17 and Rust #20 are merged with semantic and Release consumer tests. Swift app adoption merged in #27 and passed CI. Rust scanner adoption remains pending. |
| H6 | Swift cross-module body visibility: **resolved for the adopted source-package path** | Swift #16 and app #26 are merged. Recorded actual-library first-match medians 346.82/197.34 ns establish about 1.76x on that consumer, not a scan-speed gain. App uses the implemented branch revision; releases remain separate. Resilient binary support is not established. |
| H7 | Existential first-match storage: **resolved for the nine fixed app rules** | App #27 replaces the FirstMatchSpec collection with concrete balanced builder types. Exhaustive parity and hosted Release regression tests passed. Predicate closures remain; no claim of eliminating all closure costs. |
| H8 | Per-call construction/allocation: **partial** | #26 reuses candidate/filter specifications and replaces marker literals with masks. Descriptor lookup remains linear on the callback/table-construction path; allocation counts have not been measured. |
| H9 | Swift bridge cost: **partial** | #26 removes marker Set conversion; the 10,000-input fixture measured 5.09/0.20 ms. #29 removes policy callbacks for supported finite policies. Event decoding and custom-policy callbacks remain; no whole-scan speedup demonstrated. |
| H10 | Exact shared worker classifier: **pending** | `may_classify_directory` remains separate handwritten prefetch logic. Worker-thread listing (#21) does not satisfy shared-classifier adoption. Require classification/prefetch parity and bounded counts. |
| H11 | Compiled field-table leaves: **pending** | No specification-core-serde field-table implementation. App marker masks and finite decision cells do not implement this generic backend. |
| H12 | RuleNode evaluation-plan compiler: **pending** | No direct/compiled RuleNode backend or parity suite yet. |
| H13 | Finite Swift policy tables: **resolved for supported finite policies** | #29 merged. Exhaustive cell/exclusion parity, ownership, cancellation, invalid-input and fallback tests passed. Whole-scan fixture: policy callbacks 640 → 0, 897 events in both paths. No material whole-scan speedup demonstrated; custom descriptors retain callbacks. |
| H14 | Avoid memoization without useful reuse: **guardrail** | No unbounded classifier cache added; no universal hit-rate conclusion. |
| H15 | Bounded memoization at an expensive boundary: **pending** | No boundary cache. Measure reuse/cost after finite-table adoption before implementing one. |
| H16 | Partial evaluation of scan constants: **partial** | #29 compiles the captured filter snapshot into per-scan finite decisions. No generic constant-folding/plan compiler; unrestricted policies retain callbacks. |
| H17 | Evaluate shared leaves once: **pending** | No DAG/shared-leaf evaluation plan or call-count acceptance tests. |
| H18 | Cost/selectivity reordering: **pending** | No reordering; purity and observable priority/short-circuit semantics must be preserved. |
| H19 | Key-aware static dispatch: **partial, Rust library merged** | Rust #20 covers ordered reference parity, construction, unrelated-key skipping, outer generic/lifetime contexts and borrowed non-Clone decisions. CI passed. Scanner adoption and Swift keyed backend remain pending; local 3.84/12.27 ns samples did not beat handwritten dispatch. |
| H20 | Swift specialization/balanced construction: **partial** | Swift #17 and app #27 merged with CI parity/performance gates. Growing-leaf right-nested/balanced medians 162.50/8.33 ns support balanced construction. Forced evaluation inlining has a separate measured effect: on the actual library, in separate-module microbenchmarks, about 62–65x faster nested-chain and about 3.1x faster balanced evaluation; dynamic `FirstMatchSpec` did not materially improve ([SpecificationCore #20](https://github.com/SoundBlaster/SpecificationCore/pull/20), open, stacked on #19; not yet adopted by the app). *Historical:* the earlier ~0.65% came from a narrow experiment with forced inlining on `And`/`Or`/`Not` only and is not a verdict on the annotation set. No whole-scan speedup is demonstrated. The upstream guard report, resilient ABI support and the compile-time trade-off (one cell +18.8%; a fresh median +3.3% with wide spread) remain open. |
| H21 | Avoid parameter-pack iteration here: **guardrail** | Adopted builder uses balanced fixed arities. Reassess only with reproducible new-toolchain evidence. |


**Current Swift annotation recommendation.** Balanced construction and forced
evaluation inlining are separate, measured levers. Adopt forced inlining through
SpecificationCore's `AggressiveInlining` trait (#20: on by default, consumer
opt-out, disabled under Tracing, no new `@frozen` types) once it merges and the
app's Release gates pass. Do not add blanket `@frozen`: it is an ABI commitment
without a measured need, and library-evolution builds are not yet comparable.
Replica-library figures in issue 25 comments and the ~0.65% experiment are kept
as historical evidence only.

Within the explicitly scoped H ledger: **3 resolved, 9 partial, 7 pending,
2 guardrails**. These are hypotheses and engineering tasks, not a count of
confirmed bugs. Library API completion, app adoption and releases remain distinct.

For each implementation PR, list **Addresses H…**, the affected layer, named
semantic tests and the benchmark fixture/baseline. Update this ledger when a PR
lands or integration changes; do not use `Closes #25` for an individual stage.
Unpublished local work is intentionally distinguished from reviewable PRs.

CI checks detect regressions in the implemented paths: library parity and
performance gates, scanner comparisons, application unit/UI tests and Release
performance tests. A green check does not verify an unimplemented hypothesis.
Each performance claim must retain its workload, compiler/settings, source SHAs
and raw repeated samples; measure whole scans separately from microbenchmarks.

## Remaining work in dependency order

1. **Finish runtime acceptance**: rebuild current main, repeat `/Users/egor`
   scanning with #23 cloud guards, verify completion/incomplete warnings, cancellation,
   responsiveness, QoS diagnostics and chart behavior. The previous GUI scan was
   responsive but stalled in iCloud Books filesystem calls. The new guard has CI
   coverage; its merged home-directory behavior has not been verified yet.
2. **Shared classifier and catalog contract**: design the shared facts/index seam,
   adopt the appropriate Rust library backend, unify worker prefetch and candidate
   selection (H3/H5/H10/H19), then add the 200-target scanner workload (H2).
   Indexed runtime and keyed static dispatch are alternative backends to compare,
   not two implementations that must both be forced into the same hot path.
3. **Measured optional backends**: field tables/RuleNode compilation (H11/H12),
   shared leaves (H17), further partial evaluation (H16), then only demonstrated
   hashing/cache/reordering needs (H4/H15/H18). Preserve independent parity tests.
4. **Release alignment**: tag validated library releases, replace temporary Swift
   branch pins and any adopted Rust branches with released versions, and rerun
   consumer CI/performance gates. Version bumps have not happened.

The issue body's **declarative plugin proposal remains unimplemented**: schema,
manifest fixtures, catalog loader/linter/index, target-ID FFI, plugin Settings,
new ecosystem manifests and user-plugin loading/diagnostics are all pending.
ADR 0002 in this repository describes the finite policy table; it is not the
plugin ADR proposed in the issue. Allocate a new ADR number for that design.

## Merged runtime and UI optimizations outside the H ledger

- #13/#14: avoid metadata reads outside artifacts and derive marker facts from listings.
- #15/#22: lossless bounded event delivery; ring slots release consumed payloads.
- #16: set-based warning deduplication; #17: pure suffix bytecode classification.
- #18/#19/#20: incremental sorted rows, byte-based stable IDs, incremental diagram snapshot.
- #21: worker-thread directory listing; #30: propagate requested macOS worker QoS.
- #23: skip known cloud-managed roots before filesystem probes, emit warnings and
  mark the scan incomplete. This is a targeted guard, not universal File Provider detection.
- #24: descendant language badges; a UX change, not a claimed scanner optimization.

The QoS production-bridge harness verified all eight workers at User Initiated
and a responsive heartbeat. The previous GUI run had no observed priority-inversion
message, but never completed. Negative-size AppKit diagnostics remain a separate
unresolved runtime investigation; neither cloud guards nor QoS propagation prove
those diagnostics fixed.

## Historical evidence: first checkpoint (2026-10-04)

Library feature branches and PRs:

- Swift `perf/cross-module-policy`: [SpecificationCore #16](https://github.com/SoundBlaster/SpecificationCore/pull/16).
  Default and Tracing suites passed locally (92 and 111 tests). The separate-module
  Release consumer compared actual baseline `9a791d1` with `c3a3f59`: first-match
  medians were 346.82 and 197.34 ns/candidate. Concrete composition showed no
  material gain. A subsequent CI harness correction adds its deployment target;
  absolute timings vary across toolchains/targets and are not scan-speed claims.
- Rust `perf/indexed-first-match`: [specification-core-rs #19](https://github.com/SoundBlaster/specification-core-rs/pull/19).
  At 214 rules, linear/indexed medians were 369.80/23.49 ns/candidate in the
  synthetic benchmark; 14-rule indexed median was 19.98 ns. Ordered parity,
  projection counts, worker sharing, workspace tests, Clippy, rustdoc and coverage
  passed. GitHub Linux/macOS/Windows, Miri and performance jobs passed.

The app now uses the existing ABI's UInt32 marker facts directly. Compatibility
Set projections are restricted to non-hot-path callers. It reuses candidate and
filter specifications; root priority, environment defaults and kind mapping are
unchanged. Exhaustive marker parity covers all 32 x 32 marker combinations with
node names/types, nesting and symlink cases. All 17 focused policy/filter tests
passed against production sources and the actual library/Rust archive.

The arm64 Release app/performance target built with the branch dependency pinned
in Package.resolved. All 81 hosted unit tests and 11 XCTest performance tests passed on the local
Mac (macOS 27, Swift 6.4). On the same 10,000-input fixture, Set projection plus
classification took a median 5.09 ms; direct masks plus classification took
0.20 ms. These are separate five-iteration XCTest measurements, not an interleaved
whole-scan benchmark. XCTMemoryMetric records physical memory, not allocation
counts; no allocation-count reduction has been measured. UI/sandbox runtime is
not established by these unsigned tests.

Existing macOS CI runs the new clock/memory/scaling tests automatically. Static
keyed Rust dispatch and balanced Swift construction are the next library stage;
finite policy tables and shared worker classification remain pending. No version
bumps or releases have occurred.

Public SPM dependencies use HTTPS so GitHub-hosted runners can resolve them
without SSH credentials. The temporary SpecificationCore branch and locked
revision remain unchanged. Anonymous remote access and Xcode's locked package
resolution passed after the transport correction. Repository clone/push uses SSH.

## CMO and resilient-layout follow-up

[The later issue update](https://github.com/SoundBlaster/BuildHunter/issues/25#issuecomment-5980860908)
separates linking, body visibility and generic specialization. Its measurements
use a replica; they motivate actual-library verification, not a new speed claim.

The local Xcode Release build-for-testing compiler driver uses `-O` and
`-whole-module-optimization`. Replaying its SpecificationCore arm64 invocation
with `-driver-print-jobs` shows `-enable-default-cmo` on the effective frontend
command, with neither aggressive `-cross-module-optimization` nor
`-enable-library-evolution`. Looking only at driver flags would have missed the
default CMO flag. This test build also uses `-enable-testing`; it is not evidence
for every future distribution configuration.

The local follow-up compared non-tracing `@inline(__always)` on And/Or/Not
evaluations using a real separate-module consumer and growing leaf types.
Across 24 alternating pairs, baseline/candidate medians were 638.299/634.675 ms;
the paired median ratio was 0.99349. Result checksums matched, but both variants
still emitted 32 specialization-limit remarks. This bounded experiment does
not establish the same diagnosis as the issue's replica. The annotation changes
were not retained.

Freezing And/Or/Not storage resolved their own library-evolution initializer
errors, but other existing inlinable initializers still failed. No library-wide
resilient binary support or blanket freezing is claimed. Frozen layout commits
stored fields to the binary ABI and needs a separate compatibility decision.
Balanced construction and keyed dispatch remain independently useful; their
measured effects must be reassessed after future annotation changes.

## Historical evidence: second checkpoint (2026-10-04)

The static library PRs target their foundation branches: Swift #17 over #16,
Rust #20 over #19. Existing APIs and versions remain unchanged. New CI jobs
retain independent parity, repeated raw samples, noise-aware gates and
source/compiler/harness metadata. Rust and Swift checks passed on their current heads.

BuildHunter's nine fixed Swift classification rules now use the balanced
builder in their original order. The branch dependency is pinned to Swift
`bacc4d65174bcf680e10954c0eed788c410553c0` over HTTPS. Xcode locked package resolution
confirmed that revision. Eight classification tests, including exhaustive marker
parity, passed in a Release SwiftPM validation package built from copied actual
domain sources. This is separate from hosted application validation.

A new Release XCTest compares the adopted policy against the old dynamic
FirstMatchSpec implementation on 32,768 mixed names/types/marker inputs. It
checks complete result parity outside timing, then alternates seven timed rounds
with eight fixture passes per sample. The gate permits 1.5x baseline + 2 ms +
six combined MADs; it does not demand a particular speedup. Hosted Release
performance and unit/UI tests passed in GitHub Actions at `2b620c8`. The local
Release test build compiled, but its runner hung before establishing a connection
and no local timing result was produced. No signed UI/home-directory scan or
whole-scan improvement is claimed by this checkpoint.


## Historical evidence: third checkpoint (2026-10-04)

Addresses H13, and the per-scan filter binding portion of H16. Application [PR #29](https://github.com/SoundBlaster/BuildHunter/pull/29)
targets `codex/static-policy` above application #27; library release versions
remain unchanged. ADR 0002 defines the finite name/fact projection and additive
versioned C ABI. Rust holds an owned immutable table; Swift computes its cells
using the existing policy and captured filter snapshot on a detached task.
Unsupported exact/suffix descriptors retain the callback path. No Rust copy of
Swift classification rules or unbounded classifier memoization is introduced.

Rust validates table version, length, pointers and actions before filesystem work.
Its tests cover all projection cells, invalid UTF-8, suffixes, ignored marker bits,
caller-storage mutation, cancellation and callback/table event parity. Swift tests
cover every cell for each canonical exclusion and all exclusions together, plus
custom fallback, root priority and captured snapshots.

A Release integration test compares complete normalized events and policy callback
counts on 64 mixed projects. Seven alternating scan pairs retain raw timings;
table construction is measured separately and included in a broad regression gate.
Filesystem work is included; no particular speedup is required or claimed.
The existing macOS CI performance job discovers this test and exports its JSON
attachment together with the xcresult. Local hosted validation is in progress.

## macOS worker QoS follow-up (2026-10-05)

The local synchronous MainActor performance test reported a high-priority
caller waiting on Default-priority Rust workers. A separate Release harness
using the production asynchronous `RustScanEventSource` reproduced the
requested-priority mismatch: coordinator User Initiated (25), all eight
workers Default (21). Its 1,000-project scan discovered and measured all
1,000 artifacts without warnings; the MainActor heartbeat executed 30 times
during the 88.5 ms scan. This establishes the worker mismatch independently
of the synchronous test, but does not establish a GUI hang or a whole-scan
speedup. The harness uses the actual Swift bridge and Rust scanner outside
the signed App Sandbox application.

The fix captures the caller's requested Darwin QoS class and relative priority
once per scan and applies them on each dedicated Rust worker before filesystem
work. It leaves the Swift task and caller unchanged, skips unspecified QoS,
and treats unsupported/failed QoS requests as best effort. Other platforms
keep their existing scheduling. Requested QoS does not include temporary
scheduler overrides.

After the fix, the same harness reported User Initiated (25) on the coordinator
and all eight workers. All 1,000 artifacts were discovered and measured with
zero warnings; the MainActor heartbeat executed 19 times during the 50.1 ms
scan. These single runs validate propagation and responsiveness, not a timing
comparison or disappearance of Xcode's diagnostic in the signed GUI.

Local acceptance: `cargo test --locked` passed 25 tests (including
`propagates_user_initiated_class_and_relative_priority`,
`propagates_default_class_and_relative_priority`, and
`unspecified_qos_is_a_no_op`); formatting, all-targets Clippy with warnings
denied, and all-targets Linux cross-compilation checks passed. Existing
macOS/Linux/Windows Rust CI runs this layer without a new workflow. This is
a runtime scheduling follow-up to issue 25, not resolution of another H row.
