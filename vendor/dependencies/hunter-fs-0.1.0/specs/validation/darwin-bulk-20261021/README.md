# Darwin batch acquisition extraction

Baseline Hunter `860d995374d677d68b39093101da25c3cacfff9b`; source BuildHunter 7ab00f16ae437fe88427c2acbe1b651b30b044f1. Working-tree source/map/contract identities are separate from that immutable baseline. The decoder body is byte-identical after the type alias rename. Product ScanControl becomes a borrowed FnMut stop callback; open/buffer/stop/syscall order is unchanged.

Debug and Release: 134 workspace unit tests (125 core + 9 Unix), plus two compile-fail doctests. Seven Darwin batch tests include the four migrated cases, open-before-pre-cancel, stop-after-batch and synthetic non-UTF-8/type records. Fmt and all-feature/workspace clippy pass. HC parse/validate pass; SpecPM: 0 errors, 14 unresolved-capability warnings. Commands, outputs and TB5 routing are recorded. Existing three-platform CI now tests all features so macOS exercises the opt-in implementation; other platforms retain portable compilation. Fresh hosted results remain separate.

The feature stays default-off. Host still owns guard placement, backend selection, fallback, counters and error decoration. Descriptor-bound ancestor traversal, bounded per-directory/scan memory, live iCloud, GUI and a general performance gain are not established. Historical validation remains historical.
