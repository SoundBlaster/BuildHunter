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
