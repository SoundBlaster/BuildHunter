# R3 descriptor filesystem acquisition — 2026-10-23

Consumer goal: make enumeration and sizing use the same selected-root anchor as
marker observations, without moving product policy into Hunter.

Implemented on macOS/Linux: ScopedFilesystem, independent directory streams,
prepared no-follow stat passes, shared marker root identity and portable scan
access interfaces. EntryType replaces opaque std FileType in core routing;
directory/file/link/other decisions stay unchanged for the standard backend.

Executed locally on arm64 macOS 27 / Rust stable: workspace/all-features Debug
and Release, clippy with warnings denied, rustfmt, HC parse/validation and SpecPM.
All returned zero (see commands.json). Debug/Release each ran 9 marker unit tests,
8 descriptor integration tests, 138 core tests and 2 compile-fail doc tests.
Linux/Windows are separate CI evidence, not inferred from this macOS run.

The first prototype opened metadata with O_EVTONLY; a permission regression test
correctly failed on macOS. Final child observations use fstatat and the unreadable
file/FIFO regression passes without opening contents.

Root and intermediate link replacement, regular replacement within an already
prepared pass, selected symlinks, cancellation and parallel stream offsets have
deterministic fixtures. Coordinator tests prove one lazy preparation, no native
retry after a scoped failure and no new preparation for fully cached results.

Independent passes may observe different regular directory entries. There is no
atomic snapshot, persistent inode ledger, proof of Finder/path action identity,
whole-scan RSS/descriptor bound or live iCloud acceptance. Windows/other Unix
descriptor acquisition remains unsupported. Host thread-policy placement is an
integration obligation. Specs remain draft; structural validation is not runtime
conformance or owner acceptance.

Exact source/map/file identities are in source-identity.json. It identifies the
worktree inputs by SHA256, independently of the baseline commit. Raw logs remain
on TB5 and have their SHA256 in commands.json. Previous receipts are historical.
