# Accessibility context audit — 2026-11-03

Baseline: BuildHunter `1cfb4ed4547407a9707d21f8d32861ddd1c6ac6f`; the
following SHA-256 identities bind the changed, initially uncommitted sources:

| Source | SHA-256 |
| --- | --- |
| `macos/Sources/BuildHunter/UI/BuildHunterWindow.swift` | `2b077a438e4a732b78b747d3aecc02078f6e27dbcad90dd67b0c29d11ea34e1f` |
| `macos/Sources/BuildHunter/UI/TemplateJSONEditor.swift` | `e8615b5372d1062fa381b29c315df02a086b3a483c974a8a4bf75efd39913f8d` |
| `macos/Sources/BuildHunter/UI/UserTemplatesWindow.swift` | `4f5eae54cd00c3641c2b02c3c6e0e6da5c170424849cada2e44c43ee3239ee67` |
| `macos/Sources/BuildHunter/UI/ScanProfilePanel.swift` | `971cdef9be1d4562e4143581a5c13d9bb572bfc8c1713e9bb06db310816077d5` |
| `macos/Tests/BuildHunterTests/NestedAccessibilityTests.swift` | `545fc51081f09476ffb5eef92e5379ccf599d81f52300ac126da91ecdc889fc8` |
| `macos/Tests/BuildHunterUITests/BuildHunterUITests.swift` | `3f07d60384f49e11e5adc0e0f446bd18b615001e5001427a967ba406d10f6d6e` |
| `specs/hypercode/current.hc` | `69cbc997c36ff5c48a6f9501cda7e5ea758cc2051f53a5f415e15194e252db0f` |
| `specs/hypercode/current.hcs` | `31f3b866d2ff31b4418011dd5b0fe58b45543e2ea0380c0865ef491c49f459f6` |

## Scope and decisions

- `ReportPresentation`, `TemplateManagement` and `RuleProfilePresentation` remain
  within the existing UI composition. Relevant current boundary claims are
  `buildhunter.current.application` and `buildhunter.current.templates`; their
  functional requirements and target topology are unchanged. No new accepted
  product requirement or runtime ownership is inferred.
- SwiftUI template controls use local names under a templates root. The native
  `NSTextView` and `NSOutlineView` take their full identifier from the enclosing
  SwiftUI accessibility environment and refresh it when that context changes.
- Profile statistics now compose `label` and `value` under the actual statistic
  container. The value path remains `current.value`; labels change to
  `average.label` and `peak.label`, with UI-test queries updated accordingly.
- Full strings in the standalone AppKit export panel and the Screener capture
  helper have no enclosing SwiftUI environment and remain explicit boundaries.
  Dynamic filter IDs and row identities describe catalog or data identities;
  they are not fixed enclosing screen paths. Group names derived from titles
  and paths derived from data are not promised as stable cross-version API.
- IDs are folded into existing presentation boundaries; they do not by themselves
  prove VoiceOver usability or agent access to all controls.

## Validation and limits

- Debug app built with Xcode on macOS 27 arm64. The focused
  `BuildHunterTests/NestedAccessibilityTests` passed 1/1: a hosted native editor
  changed from `first.editor` to `second.editor` when its enclosing context
  changed. The Xcode result bundle is local build evidence, not a committed file.
- Focused UI-test invocation for the editor and mock results did not reach the
  tests: XCTest reported timeout enabling Automation mode. Consequently it does
  not establish runtime compatibility of the changed native IDs or profile labels.
- `specpm 0.2.0 validate . --json`: zero errors, 54 warnings. Hypercode CLI was
  unavailable on PATH, so parse and profile validation were not performed.
  Unchanged topology and bindings were reviewed against the cited sources; prior
  receipts remain historical and are not evidence for this change.
