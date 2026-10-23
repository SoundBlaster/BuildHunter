# Scoped error diagnostics review

BuildHunter #106 thread PRRT_kwDOU2hwxM6vs-qm found native pathname stat in
legacy error decoration after scoped failures. ErrorScope is now explicit:
scoped root/listing/prepared/fallback errors preserve original errno/kind/message
without diagnostic metadata I/O. Standard scans retain native dataless decoration.
A macOS regression simulates unrelated SF_DATALESS metadata and verifies no lookup
or relabeling for missing, denied and would-block errors.

Local Debug/Release: 71 root library tests and 4 CLI tests pass, plus clippy,
explicit rebuilt Release CLI with 13 CLI/3 golden checks, HC and SpecPM (0 errors,
39 retained capability warnings). Fresh exact inputs/commands are included. The
prior seam receipts remain historical; plugin tests run again after stack rebase.
No live cloud provider or new resource/performance claim. Contracts stay draft.
