# CI gating validation — 2026-10-14

[commands.json](commands.json) records commands, outputs, tool identities and input
hashes. Five classifier tests pass, including real Git cumulative PR, rename and
delete scenarios. Twelve plugin schema tests pass. Both HC parses/four profiles
pass; SpecPM has zero errors and 11 existing dependency warnings.

All four workflow YAML files were parsed locally; reusable-output wiring, heavy
job conditions and absence of workflow-level path filtering were asserted.
Actionlint was unavailable. GitHub execution/skip outcomes are not established by
local YAML parsing. The CI change itself is deliberately classified as heavy.
Full staged whitespace is checked before commit. Application runtime/ABI/contracts
and HC maps are unchanged; the local Xcode scheme edit is excluded.
