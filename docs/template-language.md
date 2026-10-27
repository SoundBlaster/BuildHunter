# Template language reference

A template is one UTF-8 JSON file in `buildhunter.plugin/1-draft.1` format.
It is declarative data, evaluated by Rifle; it cannot run Swift, shell commands,
network requests or filesystem callbacks. The same rules work in the free app.
This reference describes the implemented JSON contract, not the future Swift SDK.

Start with the [complete Forge example](contracts/plugin-v1/examples/synthetic.json).
In **Settings → Templates → New…**, BuildHunter generates it with fresh IDs.
For file ownership, import/export and conflict handling, see [Using templates](user-templates.md).

## Package, program and target

A package requires `format`, `id`, `version`, `title`, `description`, `program`,
`bindings`, `targets`. Unknown fields are rejected. Package IDs and target IDs
must be stable; change the package version when distributing a changed rule.
Use your own namespace, for example `local.forge` / `local.forge-output`.
The built-in `swift`, `rust`, `python` namespaces are reserved. Duplicates across
files are errors; importing a user package does not override a built-in.

`program.format` is `rifle.program/1-draft.1`. It declares typed facts and rules:

```json
{
  "format": "rifle.program/1-draft.1",
  "facts": [
    {"id": "kind", "type": "string"},
    {"id": "name", "type": "string"},
    {"id": "project", "type": "boolean"}
  ],
  "rules": [{
    "id": "output",
    "when": {"all": [
      {"fact": "kind", "op": "eq", "value": "directory"},
      {"fact": "name", "op": "eq", "value": "forge-out"},
      {"fact": "project", "op": "eq", "value": true}
    ]}
  }]
}
```

This snippet is only the `program`, not a complete importable package. Connect
its facts to host observations with these package-level bindings:

```json
{
  "kind": {"source": "node.kind"},
  "name": {"source": "node.name"},
  "project": {"source": "marker", "scope": "parent", "name": "forge.project"}
}
```

Then declare the package-level target:

```json
[{
  "id": "local.forge-output",
  "title": "Forge build output",
  "ecosystem": "forge",
  "kind": "build_output",
  "ruleIds": ["output"]
}]
```

`ecosystem` is extensible metadata (not a hard-coded language enum). Target kinds
are `build_output`, `cache`, `environment`, `bytecode`, `metadata`. GUI presentation
may group some kinds, but JSON reports keep their scanner values and target IDs.
Each rule must belong to exactly one target. Multiple rules in one target mean
OR. Multiple matching targets are **ambiguous**, not first-match-wins.

## Fact bindings

| Source | Fact type | Meaning |
|---|---|---|
| `node.name` | `string` | Candidate's final path component, case-sensitive |
| `node.kind` | `string` | `directory`, `regular_file`, `symlink`, `other` |
| `marker`, scope `own` | `boolean` | A regular marker file in the candidate directory |
| `marker`, scope `parent` | `boolean` | A regular marker file in the candidate's parent |

Marker names are single components: no `/`, `\\`, `.` or `..`. All declared
facts need matching bindings, and all facts must be used. Own-marker facts are
false on non-directories. Symlink candidates are skipped by the current client.
A missing marker is known false; an inaccessible marker is unavailable, not false.
The current CLI/GUI `legacy-file-markers/1` profile follows marker-file symlinks
and may inspect the selected root's parent for a parent marker. This does not
claim the stronger descriptor-confined policy of the experimental scanner.

Facts do not expose file contents, sizes, timestamps or arbitrary paths. Rule
predicates cannot read files. The host acquires shared marker observations;
profiling does not charge the same I/O independently to every consumer rule.

## Conditions

| JSON condition | Meaning |
|---|---|
| `{"fact":"name","op":"eq","value":"forge-out"}` | Exact equality |
| `{"fact":"name","op":"in","values":["out","dist"]}` | One of the listed values |
| `{"fact":"name","op":"suffix","value":".cache"}` | Literal suffix, strings only |
| `{"all":[condition1, condition2]}` | AND; stops on a decisive false |
| `{"any":[condition1, condition2]}` | OR; stops on a decisive true |
| `{"not":condition}` | Negation; unknown stays unknown |

The words `condition1`/`condition2` above are placeholders, not valid JSON. Use
actual condition objects in files. Comparisons are typed: boolean facts use JSON
`true`/`false`, string facts use strings. `in.values` is a nonempty set of distinct
values of the fact type. `all`/`any` are nonempty. There is no regex, glob, numeric
comparison or implicit case/Unicode normalization. `suffix` is not a glob: `*`
is a literal character. Reordering conditions can change short-circuit cost but
must preserve the intended true/false/unknown behavior.

Three-valued logic matters for protected folders and cloud/provider failures:
`not unknown = unknown`, `false AND unknown = false`, `true OR unknown = true`.
A target with any true rule is true; otherwise any unknown rule makes it unknown.
Zero/one true target together with an unknown target is indeterminate. Unknown
is diagnosed rather than silently reported as a confirmed artifact.

## Test and improve a rule

Keep the JSON and small positive/negative fixture directories in your Git repo:

```text
fixtures/
  positive/forge.project
  positive/forge-out/object.o
  negative/no-marker/forge-out/object.o
  negative/wrong-name/forge.project
  negative/wrong-name/source/source.txt
```

The first case should match; the latter two should not. Include empty directories,
files with the same name as the directory, inaccessible markers, overlapping rules
and excluded targets where relevant. **Check** tests admission, not recognition
against this fixture. Scan the fixture explicitly, compare target IDs and totals,
and use [rule profiling](rule-profiling.md) to compare Release runs. Check does
not itself time a scan. Preserve matching behavior when optimizing costs.

Use narrow literal name conditions so the conservative dispatch index can skip
irrelevant targets before marker acquisition. Do not broaden a rule just to gain
matches, replace unknown facts with false, or assume a high maximum means an
expensive expression: timers and OS scheduling affect nanosecond measurements.

## Validation and limits

The [plugin schema](contracts/plugin-v1/plugin.schema.json) and
[program schema](contracts/plugin-v1/rifle-program.schema.json) define wire shape.
Runtime **Check** also enforces references, namespaces and aggregate budgets.
Duplicate JSON keys, unsupported formats/operators, unused facts, invalid types,
unknown references and oversize expressions fail with code + JSON pointer.

Current upper bounds include 1 MiB per file; 128 facts and 256 rules per program;
128 expression nodes per rule; 32 children per group/membership list; condition
depth 8; 64 packages, 4096 rules and 32768 expression nodes in the effective
catalog. BuildHunter admits at most 61 user files beside its 3 built-ins, 2048
target IDs and 512 declared scope/name markers. Limits are admission constraints,
not promises of whole-process RAM or scan latency. Disabled targets are still
validated; explicit disablement of an entire broken user file excludes it from
next-scan admission.

If an agent authors JSON, give it this reference and the complete example. Ask
for narrow positive/negative fixtures and stable namespaced IDs. Run Check and
fixture scans before relying on its output. Invalid enabled files block the next
catalog with diagnostics; no partial activation or stale fallback is used.
