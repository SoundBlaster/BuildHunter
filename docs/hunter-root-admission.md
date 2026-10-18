# Root admission sequencing

`Scanner::run` delegates to `admit_root(probes, root, cloud_skip_reason)`.
The generic helper calls the host preflight once on the original path. A reason
returns `Skipped`; otherwise `observe_root` produces `Observed(Directory/Entry)`
or `Unavailable` with the original metadata/error. The callback may have its own
effects; there is no universal no-I/O claim for host preflight itself.

| Outcome | BuildHunter action |
| --- | --- |
| Skipped(reason) | Warn and return before pump/finalization. |
| Observed(Directory) | Enqueue the root with original metadata. |
| Observed(Entry) | Classify/measure using original metadata. |
| Unavailable(error) | Warn, then retain normal pump/finalization path. |

The existing initial profile emission still precedes admission. Pre-cancelled
startup still observes metadata when allowed. Final symlinks remain entries;
intermediate components and directory opens remain pathname based and raceable.
Guard setup failure is separate: product session returns Warning then Failed
before creating workers. Guard lifetime, cloud matching, warnings and final status
are unchanged. Hunter acquires no guard and has no product status vocabulary.

Four new upstream tests and existing product root witnesses verify the seam.
Draft capability `hunter.fs.root_admission` does not claim full reusable startup,
live cloud-provider behavior, UI/home acceptance or a resource improvement.

Guard/backend lifetime and native Darwin policy RAII now live in Hunter `3a3db1e270ff54ba757027df3a55b9d378ee0525`.
Host scope placement, cloud paths/error decoration and product status stay local;
see [guarded backend lifetime](hunter-backend-lifetime.md) and `hunter-lifetime-20261018` evidence.
This supersedes earlier pending guard-lifetime ownership notes for the integrated
Rust coordinator/workers; full engine/backend algorithm assembly remains pending.
