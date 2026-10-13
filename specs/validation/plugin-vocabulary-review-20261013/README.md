# Plugin vocabulary review correction

PR #65 discussion r4263521987: align v1 facts advertised by the architecture/ADR
with the existing schema. Artifact ancestry remains host policy, not a binding.

Fresh results in [commands.json](commands.json): 12 schema tests passed, including
ancestor binding rejection; two HC parses and four profiles passed; SpecPM zero
errors / 11 existing dependency warnings. [specpm.json](specpm.json) contains the
full diagnostics. [source-identity.json](source-identity.json) pins current inputs.

No source/runtime/schema/map behavior changes. Current runtime contracts remain
unchanged. Earlier evidence is historical. Tests validate wire shape, not semantic
matching or filesystem behavior; draft acceptance is unchanged. Full staged PR
whitespace is checked before committing. Unrelated Xcode scheme edits are excluded.
