# R4 default client catalog cutover

CLI/GUI/Settings now use the immutable plugin catalog. Numeric policy APIs remain
compatibility surfaces. JSON schema 2 captures catalog/profile/marker policy,
target IDs, exclusions and typed diagnostics. Current maps/contracts are draft;
this implementation does not accept new requirements implicitly.

Local arm64 macOS results:

- Root Debug and Release: 77 library + 4 CLI unit tests passed in each mode.
- CLI integration: 14 passed; frozen migration corpus driver: 3 passed.
- Clippy all targets/features: passed. Dependency regressions: 10 passed.
- Separate scoped scanner Release/bulk integration: 16 passed.
- Xcode 27.1 build-for-testing: passed, using isolated TB5 products/intermediates.
- HC: 10 parse/profile checks passed. SpecPM: 0 errors, 46 unresolved-capability warnings.

Local XCTest execution is **unverified**: the host stalled before test connection,
in dyld loading a dependency (`open` in the attached two-second sample). It was
terminated after about five minutes; no Swift test pass is claimed. The new GUI
parity/identity/invalid-path/export tests must execute on CI. Source compilation,
synthetic bridge inputs and manual UI behavior are distinct evidence.

`benchmark.json` retains all raw Release rounds: a real 200-target dynamic scan
and seven interleaved 13/200-target pairs, with finding/byte equality each round.
Medians: 10.424 / 10.577 ms, about 1.5% difference. The broad regression limit is
41.001 ms (`2 * baseline + 20 ms + 3 * MAD`), not a promised latency or resource
budget. Sampled process RSS in the paired scans peaked at 11,698,176 bytes; this
short warm fixture excludes GUI and cannot explain or bound a home scan's RAM.
CPU and first-finding timings are in each raw sample; sampled RSS can miss spikes.

The 200-target functional witness also checks exact relevant marker observations
(200, or 199 with one exclusion), independent allocated bytes, completion count,
and absence of spurious matches among 1,024 unrelated directories. CI runs the
Release gate inside existing Rust/CLI jobs; no extra macOS runner job is added.

C3 preserves R2 legacy-file-markers/1 behavior (marker-file links and selected-root
parent context). Default clients do not claim the stricter scoped API's descriptor
confinement. C4 rejects actions for lossy components/descendants, retaining safe
ancestors. No inode/snapshot capability, live iCloud/sandbox/Store acceptance,
user-plugin import or R5 resource budget is established by this receipt.

`commands.json` gives exact commands/log hashes; `tools.json`, `source-identity.json`
and structural logs identify the tested state. Earlier preparation receipts remain
historical. Hosted CI results, once available, are separate revision-specific proof.
