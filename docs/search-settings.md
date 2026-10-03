# Search settings

BuildHunter selects artifact roots using its existing project markers and path
rules. Search filters let users exclude recognized root types; they do not change
classification or weaken the marker checks.

## CLI

No exclusions preserves the previous behavior. `--language all` remains the default;
virtual environments still require the existing `--include-envs` flag.

```sh
build-hunter --list-filters
build-hunter --list-filters --json
build-hunter . --exclude swift.build
build-hunter . --exclude python.pytest-cache,python.ruff-cache
build-hunter . --exclude rust --exclude python.bytecode --json
```

`--exclude` is repeatable and accepts comma-separated filter IDs. Language aliases
`swift`, `rust`, and `python` expand to all registered types in that language.
Unknown IDs and missing/empty values are argument errors. `--language` and exclusions
intersect: excluding a type cannot enable a different language. Disabling all
eligible types returns an empty successful report, subject to normal read errors.

## macOS

Use **BuildHunter → Settings…** or **⌘,** to open the native Settings window.
Switches enable individual search types or entire language groups. All types are enabled
on first launch, including the GUI's previously supported Python environments.
Preferences store excluded IDs in UserDefaults and survive application restarts.
Changes apply to new scans and **Rescan**, across all windows. Running scans and
completed reports keep their original selection. Reports and folder targets are
still not persisted.

An excluded root remains traversable. For example, excluding `swift.build` can expose
a Python `__pycache__` inside a `.build` tree. An included artifact is still measured
as a whole: excluding Python bytecode does not subtract those bytes from an included
Swift build directory. GUI reports contain outer artifact roots only; CLI retains
its existing nested-row reporting and counts nested sizes once.

## Extensibility and contract

The Rust descriptor catalog is the source of supported filter IDs, titles, groups
and candidate match metadata. CLI listing and exclusion validation consume it;
the macOS app decodes the same catalog through `bh_search_filter_catalog_json`.
The C function returns immutable, NUL-terminated UTF-8 JSON valid for the lifetime
of the process. Swift does not maintain another list of supported filter types.

IDs identify semantics rather than UI position. Preferences store exclusions, so
newly added types are enabled by default. Unknown stored IDs are retained across
catalog upgrades; group toggles operate on current descriptors. A full reset restores
defaults. Adding a future language or framework requires a recognition policy plus
its descriptor and integration tests, not another hard-coded Settings section.

The Swift bridge captures an immutable filter snapshot when a scan starts and uses
SpecificationCore's `IsIncludedSearchArtifact` after classification. An excluded
candidate is traversed rather than pruned, preserving discovery of other supported
types beneath it. A future classification with no descriptor remains included.

## Verification

Rust and CLI tests cover exclusions, defaults, invalid IDs, language intersections,
listing, and nested discovery. Swift tests cover preference persistence, grouped
selection, future IDs, immutable snapshots and SpecificationCore decisions. Bridge
tests scan real temporary fixtures through the Rust FFI, including an excluded
outer root, an included nested cache, and an all-disabled scan.
