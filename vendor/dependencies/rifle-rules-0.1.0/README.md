# Rifle

Rifle validates and evaluates declarative rules over facts supplied by a host.
It is a Rust library independent of filesystem traversal, build artifacts and UI.
The initial format is `rifle.program/1-draft.1`; the crate is not published yet.

```rust
use rifle_rules::{FactValue, Plan, Truth};

let plan = Plan::from_json(include_bytes!("fixtures/rust-program.json"))?;
// Match the declared fact order, available from plan.facts().
let results = plan.evaluate(&[
    FactValue::String("target"),
    FactValue::String("directory"),
    FactValue::Unavailable("access_denied"),
])?;
assert_eq!(results[0].truth, Truth::Unknown);
```

Run the complete example: `cargo run --locked --example evaluate`.
Validation resolves fact names to indexes once. Evaluation borrows input values,
checks their count/types, and returns owned outcomes. A plan can be shared across
threads. There are no per-object filesystem callbacks or host effects.

Supported conditions: typed `eq`, `in`, string `suffix`, `all`, `any`, `not`.
Unknown input remains unknown under negation; decisive false in AND or true in OR
suppresses irrelevant unknown diagnostics. IDs in diagnostics are sorted and
unique. Missing input is an adapter error, never a false match.

The loader rejects duplicate JSON keys, unknown fields/operators/formats, wrong
types, duplicate IDs, unused/unknown facts, empty conditions and excessive plans.
It bounds input bytes and JSON nesting before recursive decoding. See
[implemented contract and limits](docs/contract.md), [schema](schemas/program.schema.json),
[architecture](specs/hypercode/architecture.hc) and
[verification progress](specs/specification-progress.md).

## Scope and provenance

The program schema is copied unchanged from BuildHunter commit
`79fa49303e73a595330fcb7441d5f8c2f430098d`; the Rust and synthetic fixtures extract
only the `program` field from its public examples. Source:
[plugin contract](https://github.com/SoundBlaster/BuildHunter/blob/79fa49303e73a595330fcb7441d5f8c2f430098d/docs/plugin-contract-v1.md).
Product target IDs, marker bindings, exclusions and ambiguous target resolution
remain host responsibilities. This crate does not load a BuildHunter envelope.

Generic package/catalog validation and JCS identity are available: see
[catalog API and limits](docs/catalog-identity.md). Full plugin validation remains
a host responsibility. Next stages: SDK authoring, host-specific catalog validation
and measured execution-plan optimization. Hunter/BuildHunter integration is separate.
No speed, whole-process memory bound, filesystem correctness or SDK availability
is claimed by this initial evaluator.

## Development

`cargo fmt --check`, `cargo test --locked --all-targets`,
`cargo test --locked --release --all-targets`,
`cargo clippy --locked --all-targets -- -D warnings`.
CI checks Linux, macOS and Windows. GitHub workflows are defined here; fresh CI
results must be checked on the PR rather than inferred from local macOS results.

MIT license. See [AGENTS.md](AGENTS.md) for architecture/spec maintenance.
