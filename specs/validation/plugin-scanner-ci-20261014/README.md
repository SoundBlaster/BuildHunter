# PR #68 Linux fixture correction

Initial hosted failure: [Ubuntu Rust job](https://github.com/SoundBlaster/BuildHunter/actions/runs/38532277755/job/115746757493),
`invalid_utf8_name_is_unknown_with_lossless_path`: expected 1 diagnostic, actual 0.
The required forge.project marker was absent, so Unknown AND false correctly
produced NoMatch. The corrected fixture provides the positive marker, leaving
only the invalid name unknown. Platform-independent policy coverage now verifies
both true-marker/Indeterminate and false-marker/NoMatch cases with invalid names.

Production code and normative contracts are unchanged. Affected claims are
plugin.policy/classification,fact_boundary and plugin.scanner/candidate_gate,routing.
Fresh identities: [source-identity.json](source-identity.json); frozen maps:
[map snapshots](../../evidence/plugin-scanner-ci-20261014/maps/).
Old records remain historical and unchanged.

[Final local commands](commands.json) passed on macOS ARM64: both packages in
Debug/Release, formatting and clippy; both HC maps/four profiles; SpecPM 0 errors
and 15 capability warnings. The actual Linux filename test cannot run on APFS;
hosted CI rerun is separate evidence and pending at publication. No new runtime,
UI, live cloud or performance claim is made.
