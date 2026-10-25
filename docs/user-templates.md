# File-backed user templates (R7)

The owner accepted this first-release scope on 2026-10-25. These requirements
apply to the free app as well as the Store edition. Arsenal sells a maintained
catalog; authoring and using one's own rules has no purchase requirement.

- One UTF-8 JSON file is one declarative plugin package, possibly with several
  targets. The existing plugin/Rifle formats are reused; no executable hooks,
  Swift compiler, Git client or visual rule builder is added.
- Users choose where files live, including a repository they manage themselves.
  BuildHunter stores file references, access bookmarks and enabled state, not an
  authoritative copy of the source. Disconnecting a file does not delete it.
- The minimal editor opens the complete JSON, highlights syntax, validates and
  saves atomically. Saving compares the loaded bytes under file coordination and
  refuses observed external changes until reload. This is not a filesystem
  compare-and-swap guarantee against an uncoordinated writer racing that check.
  Invalid edits stay in the editor and produce diagnostics.
- File contents are reloaded on request and before each new scan. No watcher or
  automatic Git synchronization belongs to v1. A running scan owns its immutable
  catalog even if a file is changed, replaced, removed or disabled afterwards.
- Missing/unreadable, malformed, incompatible, oversized or conflicting enabled
  packages prevent preparation of the next catalog with actionable diagnostics.
  The user can fix or explicitly disable a package; no silent omission, partial
  activation or fallback to a previous catalog is permitted.
- Input bytes, JSON depth, package/rule/target counts and effective catalog costs
  are bounded before scanning. Files are read off the UI actor. Parse/validation
  errors cross FFI as owned/copied diagnostics, never as an unwinding exception.
- Catalog digest and package versions identify the rules used by the report.
  Git history remains the user's responsibility.

Acceptance includes invalid UTF-8/truncated JSON, duplicate keys and IDs, unsupported
formats, invalid references/operators, byte/depth limits, disappearing/replaced
files, external-edit conflicts, persistent registrations, disabled invalid files,
and current-versus-next-scan isolation. Structural validation, unit tests, native
UI checks and Store acceptance remain separate evidence.

The command-line import UX, Swift authoring SDK, advanced editor, automatic reload
watcher, Git integration and commercial purchase implementation are separate work.

## Using files

1. Open **BuildHunter → Settings… → Templates**. Choose **New…**, or **Connect
   JSON…** for files already in a folder or Git checkout. Connecting a file does
   not copy it into an app-managed source library. Use **Open** to edit it.
2. Edit the full JSON. **Check** validates this draft and the enabled connected
   packages together. A disabled draft is still checked individually but stays
   disabled. Checking never saves or changes an active scan.
3. **Save As…** chooses a destination and connects the saved file; **Save**
   validates the individual package and replaces the loaded file after checking
   it for observed external edits. Use Check to inspect cross-file conflicts.
   **Reload** reads the disk version and asks before discarding a dirty draft.
   Closing Settings retains the draft for this process; quitting asks before
   discarding it. Drafts are not an autosave or crash-recovery store.
4. Run a new scan or Rescan. It reads all enabled files before Rust starts
   traversal. Per-file switches apply immediately to the next scan; admitted
   target types also appear in Search settings after catalog metadata arrives.
5. Commit, branch or restore these JSON files using your own Git tools. An
   external replacement at the same selected path is reread. For a moved file, disconnect the old reference and
   connect its new path; **Disconnect** removes only the app registration.

One broken enabled file blocks the next catalog, with its path/error in the scan
warnings. It does not silently revert to built-ins. Disable that file explicitly
while fixing it. A corrupt registration store requires the explicit **Reset
registrations** action; this does not delete source files.

## Writing a template, including with an agent

Start with [the complete example](contracts/plugin-v1/examples/synthetic.json),
[plugin contract](plugin-contract-v1.md) and its linked schemas. The New command
uses that example with a fresh package/target namespace. Replace the demonstration
names with your own stable identifiers before sharing it. A package has its own
`id`, `version` and `title`; each target has a stable `id`, display `title`,
`ecosystem`, artifact `kind` and references to its `ruleIds`.

The example recognizes a directory named `forge-out` whose parent contains a
`forge.project` marker. `bindings` supplies the node name/kind and marker fact to
the pure `program`. It does not inspect marker contents, execute a command or
grant additional disk access. Add positive and negative fixture directories to
your own repository to make accidental recognition visible.

A useful agent instruction is: “Produce a complete UTF-8 JSON BuildHunter plugin
using the existing `buildhunter.plugin/1-draft.1` and `rifle.program/1-draft.1`
contracts. Use my own package and target IDs, declared supported facts/operators,
and narrow positive and negative fixtures. Return data only; no executable hooks.”
Use **Check** as the authority for the installed app's supported contract. Neither
JSON syntax highlighting nor an agent's answer proves a template valid.

Built-in `swift`, `rust` and `python` namespaces are reserved. Duplicate package
or target IDs reject the catalog; ambiguous matches are diagnosed by the existing
scanner policy. There is no implicit precedence override for user files.

## Limits and identity

- Regular files only; no directory, device or FIFO input. UTF-8 JSON is required.
- At most 61 connected user packages, 1 MiB per file; the effective Rust catalog
  also includes 3 built-ins and has a 16 MiB total byte budget.
- Rifle's existing JSON-depth, expression, rule, target, marker and cost admission
  limits remain in force. Duplicate JSON keys and unsupported operators/formats
  are errors, not ignored extensions.
- A report's JSON `catalog` includes package IDs/versions, the digest, profile and
  exclusions. These identify what was loaded, rather than the current disk files.
- Atomic replacement protects against a partially written destination. File
  coordination plus byte comparison detects observed external changes; it cannot
  arbitrate an uncoordinated writer racing between comparison and replacement.

The editor and explicit template saves add user-selected file writes alongside
existing report exports. Scanning remains read-only; importing data does not add
executable plugins, networking, JIT or permission escalation. App Store acceptance
and the future Arsenal purchase/trial flow require separate validation.
