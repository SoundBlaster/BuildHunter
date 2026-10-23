# R3 scan-scoped acquisition

`scan_with_candidate_access` uses the existing product engine with an explicit
Hunter `ScanAccess`. The plugin pilot now selects `ScopedFilesystem` through this boundary. Default CLI/GUI and the existing
`scan_with_candidate_observer` retain their standard pathname backend.

The same backend serves root admission, worker directory listings and measurement,
and coordinator missing-result fallback. It is borrowed through worker join.
Workers retain the per-thread non-materializing guard. The caller must install
the guard before constructing a backend that performs I/O during setup.

`ProbedDirectory` preserves product counters and backend errors and rejects a file
slot that has become a directory or symlink, without counting the link or target.
Prepared worker passes hold one directory. Coordinator fallback acquires one
directory lazily per selected pass; supplied results/errors keep their precedence,
and fully cached results do not open it. Acquisition failure is authoritative;
ordinary paths and opt-in pathname bulk metadata cannot repair it. The optional
bulk backend remains available to standard scans exactly as before.

Core routing uses `EntryType` (directory, regular file, symlink, other), converted
from standard native observations for unchanged clients. The facade and verified
std-only core snapshot share one exact Hunter revision. No new dependency enters
the default/offline application graph. Public library versions remain unchanged
until the planned release alignment.

Local spy tests prove backend routing, worker/coordinator preparation, release
before Finished and absence of native retries after scoped failure. They do not
prove no-follow behavior: their backend deliberately uses std paths. Actual
descriptor replacement tests belong to Hunter and the plugin integration.
Default golden profile parity and lifecycle fixtures remain separate evidence.

This does not complete default catalog migration (R4), marker-link/action-path
owner decisions (C3/C4), whole-process budgets (R5), Windows acquisition or live
iCloud acceptance. Contracts remain draft. Current receipts:
`specs/validation/hunter-scoped-access-20261023`; older receipts remain historical.

Scoped-error review correction: all scoped root/listing/directory/file errors
retain their original payload and never use native pathname metadata to diagnose
dataless objects. Only the standard backend retains the legacy pathname error
decoration. A synthetic diagnostic-lookup regression verifies that scoped errors
never invoke that lookup or inherit unrelated SF_DATALESS flags. Current source
and execution identities are in `scoped-error-review-20261023`; earlier receipts
remain historical.

The plugin layer is now integrated on macOS/Linux. End-to-end replacement and
permission fixtures are recorded in `plugin-scoped-scan-20261023`; this supersedes
the earlier pending integration note without accepting R4/R5 or Windows support.
