# Plugin contract draft validation — 2026-10-13

Scope: proposed plugin/program format, not a working plugin runtime.
[commands.json](commands.json) records actual commands, outputs and tool identity.
[source-identity.json](source-identity.json) pins the input documents and schemas.

- Hypercode: both maps parse and all four profiles validate. Composition unchanged.
- SpecPM: zero errors; 11 pre-existing unresolved capability warnings retained.
- `scripts/test_plugin_contract.py`: 11 passing tests, including positive examples
  and malformed inputs. Local Python 3.10 / jsonschema 4.26.0; the new CI job uses
  Python 3.12 and the same direct dependency pin. CI outcome is not claimed here.
- Both schemas and all examples are UTF-8 JSON. External schema references in the
  tests resolve only from an in-memory registry; no remote schema loading.
- Full staged PR whitespace is checked before commit.

Limits: schema tests do not check type references, duplicate JSON keys before
parsing, expression depth/aggregate budgets, catalog conflicts, matching, marker
I/O, SDK export or JCS. These are normative requirements/scenarios in the draft,
not implemented assertions. No Rust/Swift runtime or app behavior changed; no app
build, scan, performance claim or owner acceptance is implied.

Implementation baseline remains 48badf9e097783505151dd47fbafe290f0af25e2;
parent documentation commit is 6ec3053debfc65383d231051bdd154d26e7be194.
The user's Xcode scheme change is excluded. Earlier evidence stays historical.
