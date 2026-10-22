# Hunter process platform boundary

Hunter revision `c5354e973b5d1d90c438c94b78c06898ffe62500` owns the two std-only OS modules:

| Module | Hunter provides | BuildHunter retains |
| --- | --- | --- |
| `thread_qos` | Capture requested macOS class/relative priority; apply to current thread, best-effort | Capture point in `scan_session`, worker/sampler placement and join/status |
| `process_resources` | Independent optional cumulative user/system CPU nanoseconds and resident bytes for self | Profile baseline, interval, deltas, CPU %, RAM time weighting, summaries, profile ABI and UI |

`src/lib.rs` imports both modules under their existing private names. The old
local implementations are deleted. The complete vendored snapshot is byte-equal
to the pinned upstream commit; the offline verifier covers all 27 files.

QoS is requested rather than effective priority. Apply does not restore anything
and query/set failures remain best-effort. Non-macOS QoS is a no-op. Resource
snapshots cover macOS, 64-bit Linux and Windows; unsupported platforms return
None. RSS is resident bytes/Windows working set, not peak memory or physical
footprint. Samples include unrelated work in this process, exclude child
processes and do not form atomic CPU/RAM pairs. Linux reads `/proc/self/statm`;
no selected scan-tree content is read by these helpers.

The same native QoS and CPU/RSS tests run upstream. Existing host profile tests
still verify unavailable data, decreasing counters, multi-core CPU percentages,
time-weighted RAM, stalled consumers and final profile-before-Finished ordering.
The relocation adds no automatic thread setup, sampler, retry or budget.

Draft contracts: `hunter.platform.process` upstream, and
`buildhunter.current.scan/process_platform_assembly` here. Source hashes and
actual validation are retained in `specs/validation/hunter-process-20261022`.
This completes the concrete OS-primitive relocation identified by the audit;
C1–C5 acceptance, descriptor recursion and default-plugin migration remain separate.
