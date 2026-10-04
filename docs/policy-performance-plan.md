# Policy performance series (issue 25)

Source: https://github.com/SoundBlaster/BuildHunter/issues/25 and its performance
addenda, including the corrected Swift specialization diagnosis.

The existing main branches are the baselines. Library changes use feature
branches/local checkouts during development; releases and version bumps happen
after integration validation. PRs 23 and 24 are independent and remain untouched.

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

Snapshot: **2026-10-04**. H1–H21 identify hypotheses and proposed optimizations
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
[application PR #26](https://github.com/SoundBlaster/BuildHunter/pull/26).
All three PRs are open. Their published CI checks passed at Swift `596aa71`,
Rust `b3a02b9`, and application `6edcdb8`; subsequent commits require their own
checks. No row below is marked resolved yet.

| ID | Scope and current state | Evidence and remaining acceptance work |
| --- | --- | --- |
| H1 | Candidate-level specification cost: **partial measurement** | Scanner fixtures record callback counts. A representative real-tree profile is still needed before calling the cost negligible. |
| H2 | Large runtime catalogs: **library benchmark only** | PR #19 compares 14/214 rules. The application catalog and a 200-target scanner scenario are not integrated. |
| H3 | Indexed first-match: **Rust PR #19; app pending** | Ordered keyed/unkeyed parity, duplicate keys, one projection per candidate and shared-worker tests pass. Swift indexing and application adoption remain pending. |
| H4 | Faster hashing: **pending** | No hash replacement or demonstrated need. Compare with the existing index before accepting added complexity. |
| H5 | Static first-match: **local in both libraries** | Rust concrete macro and Swift balanced fixed-arity builder have semantic tests and Release consumers. Publication, CI and application adoption remain pending. |
| H6 | Swift cross-module body visibility: **PR #16; PR #26 uses its branch** | Actual-library first-match consumer improved about 1.76x in the recorded baseline comparison. Other composition APIs showed no material gain. Merge/release and final dependency alignment remain pending. |
| H7 | Existential storage overhead: **local alternative; app pending** | Static APIs retain concrete rule types. Production application dispatch still uses dynamic first-match; measure the adopted path before claiming removal of existential overhead. |
| H8 | Per-call construction/allocation: **partial PR #26** | Candidate/filter specifications are reused and marker literals become masks. Exhaustive policy/filter tests pass. Descriptor lookup is still linear; allocation counts have not been measured. |
| H9 | Swift bridge cost: **partial PR #26** | Marker Set conversion is removed. The 10,000-input projection/classification fixture measured 5.09 vs 0.20 ms. String decoding and FFI callbacks remain; this is not a whole-scan speedup. |
| H10 | Exact shared worker classifier: **pending** | Worker prefetch has not been switched to the shared catalog classifier. Require classification/prefetch parity and bounded candidate counts. |
| H11 | Compiled field-table leaves: **pending** | No field-table implementation in specification-core-serde yet; app marker masks alone do not satisfy this item. |
| H12 | RuleNode evaluation-plan compiler: **pending** | No compiler yet. Require independent direct/compiled semantic parity before flattening, folding or reordering. |
| H13 | Finite Swift policy tables: **pending** | No tabulation or callback elimination yet. Require exhaustive table/direct parity and measured callback counts. |
| H14 | Avoid classifier memoization without useful reuse: **guardrail** | No unbounded classifier cache is being added. A real hit-rate study has not established the hypothesis universally. |
| H15 | Bounded memoization at an expensive boundary: **pending** | Deferred until finite-table work and a pure policy projection contract. No boundary cache is implemented. |
| H16 | Partial evaluation of scan constants: **partial app foundation** | An immutable filter snapshot exists. A compiled plan that removes scan-constant rules remains pending. |
| H17 | Evaluate shared leaves once: **pending** | No DAG/shared-leaf evaluation plan yet. Verify leaf-call counts and parity when implemented. |
| H18 | Cost/selectivity reordering: **pending** | No reordering. Preserve observable first-match/short-circuit behavior and establish leaf purity before changing evaluation order. |
| H19 | Key-aware static dispatch: **Rust local; Swift pending** | Rust macro tests cover linear-reference parity, construction order, unrelated-key skipping and borrowed non-Clone decisions. Final synthetic medians were 4.04/13.09 ns for 14/214 rules; these did not beat the handwritten reference. No app adoption yet. |
| H20 | Swift specialization and balanced construction: **local verification** | Growing-leaf benchmark measured right-nested/balanced medians 160.87/8.13 ns. A separate real-library inline-always experiment improved only about 0.65% and retained specialization remarks. The exact guard diagnosis, binary-size/compile-time impact, publication and integration remain open. |
| H21 | Avoid parameter-pack iteration for this backend: **guardrail** | The local Swift builder uses balanced fixed arities, not pack iteration. Reassess pack support only against a reproducible consumer on a new toolchain. |

For each implementation PR, list **Addresses H…**, the affected layer, named
semantic tests and the benchmark fixture/baseline. Update this ledger when a PR
lands or integration changes; do not use `Closes #25` for an individual stage.
Unpublished local work is intentionally distinguished from reviewable PRs.

CI checks detect regressions in the implemented paths: library parity and
performance gates, scanner comparisons, application unit/UI tests and Release
performance tests. A green check does not verify an unimplemented hypothesis.
Each performance claim must retain its workload, compiler/settings, source SHAs
and raw repeated samples; measure whole scans separately from microbenchmarks.

Next acceptance checkpoint: publish the local static-library implementations,
validate their CI, integrate them in BuildHunter, then implement finite policy
tables and exact shared worker classification. Align released versions only
after consumer validation. The catalog/schema/plugin architecture still needs
its own acceptance checklist before the whole issue can be closed.

## First checkpoint (2026-10-04)

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
