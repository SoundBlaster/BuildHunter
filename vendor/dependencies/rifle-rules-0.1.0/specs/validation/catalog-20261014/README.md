# Generic catalog and identity validation — 2026-10-14

Baseline: fb319c7ccaa005f6f63ffbeaa034dd3b4ddb7c73 (Rifle #1).
[commands.json](commands.json) records fresh commands/output/tool identity;
[source-identity.json](source-identity.json) pins changed source, dependencies,
contracts and map inputs. Earlier validation records remain historical.

Results: 27 Rust tests in Debug and Release; fmt, clippy with warnings denied,
runnable catalog example, five CI classifier tests and two program-schema fixtures.
HC parse/validation passed. SpecPM has 0 errors / 1 expected unresolved capability
warning: its MVP validator does not resolve the declared local evaluator capability.
The requirement is preserved, with joint runtime tests as separate evidence.

Identity vectors cover RFC 8785 serialization, UTF-16 key ordering, escaping,
negative zero/number boundaries and a known SHA256 value. Catalog tests exercise
input ordering, full-document changes, duplicate IDs, invalid-program atomicity,
exact rule/node budgets, owned plans and the host-schema boundary. The original
15 evaluator tests remain regression coverage after sharing strict JSON parsing.

An extra negative test reproduced integer tokens beyond u64 being parsed as f64
before admission. The implementation now checks raw number tokens outside strings
as well; final results here include that regression. Initial test failures are not
represented as passing results.

Limits: locally executed on macOS ARM64; CI status is separate. No full host-plugin
validation, filesystem integration, Swift SDK or performance claim. JCS input uses
a documented bounded numeric profile, not arbitrary precision preservation. SHA256
identity is not cryptographic provenance/authentication. Full staged whitespace
is checked before commit; source bindings, document links and snapshots checked.
