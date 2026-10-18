# Native filesystem backend and selected root — 2026-10-15

Hunter now supplies `FilesystemBackend`, `StdFilesystem`, `RootObservation` and
`observe_root`. The interface exposes native `symlink_metadata` and `read_dir`;
its associated iterator yields native `DirEntry`/I/O errors. It imposes no object
allocation or Send/Sync requirement; a host decides whether a backend is used
serially or through its workers. This is a native path-based seam, not a virtual
filesystem, descriptor-bound traversal or a complete scan constructor.

`observe_root` reads metadata exactly once, returning owned Directory(metadata)
when is_dir, otherwise Entry(metadata), including symlinks/special files. It never
enumerates, canonicalizes, retries or checks cancellation. BuildHunter performs
its cloud admission first, then uses the guarded Probes implementation, retains
directory metadata in DirNode or reuses entry metadata for measurement. Its
pre-cancelled startup still performs that root observation, matching prior behavior.

Probes delegates native calls to StdFilesystem and preserves test hooks, counters,
worker-thread tracking and materialization error decoration. Coordinator/worker
platform guards and their lifetime stay in BuildHunter. Intentional following
of recognized marker symlinks and macOS bulk preparation/fallback stay local.
Only final-component metadata no-follow is provided: intermediate components and
read_dir can follow links or race with path replacement. The host must hold
effective permissions/platform guards through calls and iterator consumption.

New application witnesses verify selected-file metadata reuse and a single Finished
event with/without pre-cancellation, missing selected root as warning/Incomplete,
and selected links to file/directory/missing targets without traversal. Upstream
tests cover native directory/file facts, one-call/error/literal-path preservation,
borrowed backend state, listing paths/open errors, links and Linux raw names.
Existing guarded probes, marker exceptions, bulk metadata, profile/cancel/panic,
nested sizes, CLI and plugin scanner tests remain gates. Paired Release fixtures
compare against main 7ef5bd98 on one host; no speedup/RAM claim is made.

The exact eleven-file std-only distribution pins Hunter `2d94443ae3c0614ff35d4afd98e3f9d0d01e02db`; offline CLI and
the independent metadata Git pin stay supported. Source/maps/commands/performance
receipts live under specs/validation/hunter-backend-20261015; frozen maps/upstream
draft are in specs/evidence/hunter-backend-20261015. Contracts remain draft.
No local native UI/home/live-provider acceptance is implied. Full startup/backend
assembly, descriptor-bound recursion, default plugin migration and resource budgets
remain separate work. Merge Hunter preserving its pinned commit, then BuildHunter.

Follow-up: the [directory worker protocol](hunter-directory-worker.md) now delegates
listing-first measurement publication to Hunter. Platform/backend policy and actual
scope lifetime remain host-owned.
