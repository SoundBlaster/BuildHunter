# Directory acquisition and marker observation — 2026-10-18

Hunter collect_directory consumes host-opened entries and returns an owned
DirectoryListing plus DirectoryVisit. It collects children/types and errors in
encounter order through the existing visitor. A synchronous observer receives
child Path/FileType and mutable opaque context before the child is retained.
Observer error is stored at that child path without dropping the child or rolling
back context. Iterator/type errors never invoke that observer. Context needs no
Clone/Default/Send bound, and no second listing buffer is introduced.

BuildHunter opens through guarded Probes, supplies zero marker bits and uses local
observe_legacy_marker. Recognized regular files set bits; recognized symlinks still
use the intentional following marker probe. Directories do not set bits; dangling
links retain their error and child. Disabled legacy policy performs no marker probes.
These effects remain per-entry, not delayed until after enumeration. Cancellation
keeps the existing visitor boundary, including a current entry whose observer
requested stop. OpenFailed skips the test post-list hook; other outcomes retain it.

Host keeps guard lifetime, opening, context interpretation, observer effects,
post-list behavior, worker timers, recursion and final status. Final type observations
do not follow links, but the host observer may intentionally do so. Native pathname
replacement/intermediate-link races remain; there is no anchored tree snapshot.
Existing task/worker/result/receiver lifetimes are unchanged.

Seven new macOS upstream tests cover the collection boundary; Linux adds raw names.
The application witness checks regular/symlink/directory/dangling marker behavior,
retained errors/children and disabled-policy I/O suppression. Existing marker/cloud,
profile/cancel/panic, worker join, bulk and plugin tests remain gates. Paired Release
fixtures compare against main f3c66858360aa475690aae9d9445d21b4ca0b72f; no speedup or
memory-budget claim is made. Native UI/home/live-provider testing is outside this pass.

The exact thirteen-file std-only snapshot pins Hunter 29bc8b0f60d0d192fe8e2529216f50ce4c9854c6; offline CLI and the
independent metadata pin remain supported. Runtime bindings identify immutable
BuildHunter c70724d469d823774450bc387ecb90be1c5e2905, including refreshed classification paths with unchanged
client decisions. Maps/contracts stay draft. Frozen upstream/maps and source/tool/
command/performance receipts are under specs/evidence/hunter-listing-20261018 and
specs/validation/hunter-listing-20261018. Merge Hunter preserving the pin before this
consumer. Full startup/backend assembly, default plugin migration and C5 remain pending.
