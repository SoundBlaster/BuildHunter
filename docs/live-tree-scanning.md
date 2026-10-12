# Scanning a changing filesystem

## Cloud materialization protection (#49)

On macOS the coordinator and every filesystem worker install the public Darwin
thread policy `IOPOL_TYPE_VFS_MATERIALIZE_DATALESS_FILES` with
`IOPOL_MATERIALIZE_DATALESS_FILES_OFF` before probing paths. A thread-local RAII
guard restores the caller's previous value, including cancellation and unwind.
Nested scans restore the enclosing scan's disabled policy. The profile sampler
does not access the filesystem and needs no policy change.

If the policy cannot be read or installed, that thread performs no filesystem
work; the scan reports `Failed` and a warning. Existing lexical exclusions for
`Library/Mobile Documents` and `Library/CloudStorage` remain in place. For denied
access to an object whose metadata has `SF_DATALESS`, the warning explains that
its cloud contents were not downloaded. Other I/O errors retain their meaning.
Never infer an iCloud object solely from an errno or substitute a guessed zero
size for an unreadable directory.

Linux and Windows retain their existing filesystem behavior.

### Automated verification

`cargo test --locked` on macOS checks the policy at every instrumented filesystem
probe, including worker listings and metadata, and in coordinator callbacks.
It also checks normal completion, cancellation, nesting and callback unwind.
The coordinator regression failed with policy `0` before protection was wired.
The existing Rust CI matrix runs these tests on macOS and portable tests on
Linux/Windows; no cloud account is required by CI.

### Manual acceptance still required

On a Mac with iCloud Desktop & Documents and Optimize Mac Storage enabled:

1. Choose an owned test folder with a visibly evicted subdirectory. Record its
   download status before scanning; do not open its contents in Finder.
2. Scan its local parent with the CLI and the sandboxed GUI. Confirm no download
   starts and the scan remains cancellable.
3. Record the returned errno and resulting warning/partial status. A normal local
   folder beside it must still be measured.
4. Recheck download status after the scan. Repeat with the evicted directory as
   the selected root.

Policy unit tests prove thread setup/restoration, not live File Provider behavior
or sandbox acceptance. The exact denial errno has not been observed yet.

## Changes during traversal (#50)

An observed child that returns `NotFound` during listing/measurement produces a
"Changed during scan" notice and marks its affected artifact ancestry partial.
If traversal otherwise finishes, the terminal status is `CompletedWithChanges`
(C ABI status 4), rather than `Incomplete`. Missing selected roots, permission
errors and other I/O failures still make the scan incomplete; failure/cancellation
take precedence over changed-tree completion. The GUI retains notices and partial
rows. The CLI returns success for changed-tree completion and includes `partial`
in each JSON artifact, or a partial/changed suffix in its text table.

This is a best-effort observation, not an atomic filesystem snapshot: creations
before a directory listing can be included, creations after it wait for a rescan.
A rename observed between listing and measurement is treated like disappearance
of the old name; the new name is not invented or counted again. Undetected
replacements and changes after measurement are not claimed to be detected.
Partial/changed byte sums are estimates, not guaranteed lower bounds.

Deterministic per-scan probe hooks test deletion, directory removal, rename,
creation before/after listing, selected-root disappearance and real error
precedence without sleeps or a racing test thread. These tests define the
contract that any experimental traversal must preserve.
