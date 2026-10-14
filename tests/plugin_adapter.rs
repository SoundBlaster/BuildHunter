#![cfg(feature = "plugin-catalog")]
use build_hunter::plugins::{
    CandidateFacts, Decision, MarkerFact, MarkerProbe, NodeKind, Origin, PluginCatalog,
    PluginInput, Scope,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const RUST: &[u8] = include_bytes!("../docs/contracts/plugin-v1/examples/rust.json");
const FORGE: &[u8] = include_bytes!("../docs/contracts/plugin-v1/examples/synthetic.json");
fn input(bytes: &[u8], origin: Origin) -> PluginInput<'_> {
    PluginInput { bytes, origin }
}
fn rust_marker() -> MarkerProbe {
    MarkerProbe {
        scope: Scope::Parent,
        name: "Cargo.toml".into(),
    }
}
fn load(bytes: &[u8]) -> Result<PluginCatalog, rifle_rules::Error> {
    PluginCatalog::load(&[input(bytes, Origin::Builtin)], &[])
}
fn edited(f: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut v: Value = serde_json::from_slice(RUST).unwrap();
    f(&mut v);
    serde_json::to_vec(&v).unwrap()
}
fn facts<'a>(
    name: Option<&'a str>,
    kind: NodeKind,
    markers: &'a BTreeMap<MarkerProbe, MarkerFact<'a>>,
) -> CandidateFacts<'a> {
    CandidateFacts {
        name,
        kind,
        markers,
    }
}

#[test]
fn current_examples_validate_and_reserve_builtin_ids_by_host_origin() {
    assert!(load(RUST).is_ok());
    assert!(PluginCatalog::load(&[input(FORGE, Origin::External)], &[]).is_ok());
    assert_eq!(
        PluginCatalog::load(&[input(RUST, Origin::External)], &[])
            .unwrap_err()
            .code,
        "reserved_id"
    );
    let wrong = edited(|v| {
        v["id"] = json!("org.buildhunter.python");
    });
    assert_eq!(load(&wrong).unwrap_err().code, "reserved_id");
}

#[test]
fn plugin_shape_bindings_and_fact_types_are_strict() {
    for edit in [
        |v: &mut Value| {
            v["run"] = json!("pwd");
        },
        |v: &mut Value| {
            v["bindings"]["project"] =
                json!({"source":"marker","scope":"parent","name":"../Cargo.toml"});
        },
        |v: &mut Value| {
            v["bindings"]["project"] = json!({"source":"node.name"});
        },
        |v: &mut Value| {
            v["bindings"]["unused"] = json!({"source":"node.kind"});
        },
        |v: &mut Value| {
            v["bindings"].as_object_mut().unwrap().remove("project");
        },
        |v: &mut Value| {
            v["bindings"]["project"]["execute"] = json!(true);
        },
        |v: &mut Value| {
            v["bindings"]["project"] = json!({"source":"artifact.ancestor"});
        },
        |v: &mut Value| {
            v["targets"][0]["kind"] = json!("custom_action");
        },
        |v: &mut Value| {
            v["format"] = json!("future/2");
        },
    ] {
        assert!(load(&edited(edit)).is_err());
    }
    let bad = edited(|v| {
        v["bindings"]["project"]["name"] = json!(".");
    });
    assert!(load(&bad).is_err());
}

#[test]
fn targets_cover_rules_once_with_no_dangling_or_duplicate_ids() {
    for edit in [
        |v: &mut Value| {
            v["targets"][0]["ruleIds"] = json!(["absent"]);
        },
        |v: &mut Value| {
            v["targets"][0]["ruleIds"] = json!(["output", "output"]);
        },
        |v: &mut Value| {
            let t = v["targets"][0].clone();
            v["targets"].as_array_mut().unwrap().push(t);
        },
        |v: &mut Value| {
            let mut t = v["targets"][0].clone();
            t["id"] = json!("example.other");
            v["targets"].as_array_mut().unwrap().push(t);
        },
        |v: &mut Value| {
            let mut rule = v["program"]["rules"][0].clone();
            rule["id"] = json!("unmapped");
            v["program"]["rules"].as_array_mut().unwrap().push(rule);
        },
    ] {
        assert!(load(&edited(edit)).is_err());
    }
}

#[test]
fn duplicate_target_ids_across_packages_fail_even_if_excluded() {
    let forge = serde_json::from_slice::<Value>(FORGE).unwrap();
    let mut other = forge.clone();
    other["id"] = json!("example.other");
    let bytes = serde_json::to_vec(&other).unwrap();
    let excludes = BTreeSet::from(["example.forge-output".to_owned()]);
    assert_eq!(
        PluginCatalog::load(
            &[
                input(FORGE, Origin::External),
                input(&bytes, Origin::External)
            ],
            &excludes.iter().map(String::as_str).collect::<Vec<_>>()
        )
        .unwrap_err()
        .code,
        "duplicate_id"
    );
}

#[test]
fn exclusions_remove_rules_and_marker_requests_and_preserve_unknown_ids() {
    let c = PluginCatalog::load(
        &[input(RUST, Origin::Builtin)],
        &["rust.target", "future.target"],
    )
    .unwrap();
    assert!(c.marker_requests(NodeKind::Directory).is_empty());
    assert!(c.exclusions().contains("future.target"));
    assert_eq!(
        c.classify(facts(None, NodeKind::Directory, &BTreeMap::new()))
            .unwrap()
            .decision,
        Decision::NoMatch
    );
    assert_eq!(c.catalog_digest(), load(RUST).unwrap().catalog_digest());
}

#[test]
fn true_false_unknown_and_invalid_name_follow_policy() {
    let c = load(RUST).unwrap();
    for (m, expected) in [
        (
            MarkerFact::Known(true),
            Decision::Found("rust.target".into()),
        ),
        (MarkerFact::Known(false), Decision::NoMatch),
        (
            MarkerFact::Unavailable("out_of_scope"),
            Decision::Indeterminate,
        ),
    ] {
        let markers = BTreeMap::from([(rust_marker(), m)]);
        assert_eq!(
            c.classify(facts(Some("target"), NodeKind::Directory, &markers))
                .unwrap()
                .decision,
            expected
        );
    }
    let markers = BTreeMap::from([(rust_marker(), MarkerFact::Known(true))]);
    let result = c
        .classify(facts(None, NodeKind::Directory, &markers))
        .unwrap();
    assert_eq!(result.decision, Decision::Indeterminate);
    assert_eq!(result.unavailable[0].reason, "invalid_name");
    assert_eq!(
        c.classify(facts(Some("target"), NodeKind::Directory, &BTreeMap::new()))
            .unwrap_err()
            .code,
        "missing_fact"
    );
}

fn multi() -> Vec<u8> {
    edited(|v| {
        let mut rule = v["program"]["rules"][0].clone();
        rule["id"] = json!("second");
        v["program"]["rules"].as_array_mut().unwrap().push(rule);
        let mut t = v["targets"][0].clone();
        t["id"] = json!("example.second");
        t["ruleIds"] = json!(["second"]);
        v["targets"].as_array_mut().unwrap().push(t);
    })
}

#[test]
fn multiple_true_targets_are_ambiguous_and_exclusions_can_remove_overlap() {
    let bytes = multi();
    let c = load(&bytes).unwrap();
    let m = BTreeMap::from([(rust_marker(), MarkerFact::Known(true))]);
    let result = c
        .classify(facts(Some("target"), NodeKind::Directory, &m))
        .unwrap();
    assert_eq!(result.decision, Decision::Ambiguous);
    assert_eq!(result.matched, ["example.second", "rust.target"]);
    let c = PluginCatalog::load(&[input(&bytes, Origin::Builtin)], &["example.second"]).unwrap();
    assert_eq!(
        c.classify(facts(Some("target"), NodeKind::Directory, &m))
            .unwrap()
            .decision,
        Decision::Found("rust.target".into())
    );
}

#[test]
fn one_true_and_one_unknown_is_indeterminate_not_a_confirmed_finding() {
    let bytes = edited(|v| {
        v["program"]["facts"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"second","type":"boolean"}));
        v["program"]["rules"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"second-rule","when":{"fact":"second","op":"eq","value":true}}));
        v["bindings"]["second"] = json!({"source":"marker","scope":"own","name":"another.marker"});
        v["targets"].as_array_mut().unwrap().push(json!({"id":"example.other","title":"Other","ecosystem":"example","kind":"cache","ruleIds":["second-rule"]}));
    });
    let c = load(&bytes).unwrap();
    let m = BTreeMap::from([
        (rust_marker(), MarkerFact::Known(true)),
        (
            MarkerProbe {
                scope: Scope::Own,
                name: "another.marker".into(),
            },
            MarkerFact::Unavailable("access_denied"),
        ),
    ]);
    let r = c
        .classify(facts(Some("target"), NodeKind::Directory, &m))
        .unwrap();
    assert_eq!(r.decision, Decision::Indeterminate);
    assert_eq!(r.matched, ["rust.target"]);
    assert_eq!(r.unknown, ["example.other"]);
}

#[test]
fn rules_for_one_target_use_or_and_true_suppresses_irrelevant_unknowns() {
    let bytes = edited(|v| {
        v["program"]["facts"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"unknown","type":"boolean"}));
        v["bindings"]["unknown"] = json!({"source":"marker","scope":"own","name":"unknown.marker"});
        v["program"]["rules"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"alternative","when":{"fact":"unknown","op":"eq","value":true}}));
        v["targets"][0]["ruleIds"] = json!(["output", "alternative"]);
    });
    let c = load(&bytes).unwrap();
    let m = BTreeMap::from([
        (rust_marker(), MarkerFact::Known(true)),
        (
            MarkerProbe {
                scope: Scope::Own,
                name: "unknown.marker".into(),
            },
            MarkerFact::Unavailable("io_error"),
        ),
    ]);
    let r = c
        .classify(facts(Some("target"), NodeKind::Directory, &m))
        .unwrap();
    assert_eq!(r.decision, Decision::Found("rust.target".into()));
    assert!(r.unavailable.is_empty());
}

#[test]
fn marker_requests_are_deduplicated_and_symlinks_never_classify() {
    let c = load(RUST).unwrap();
    assert_eq!(c.marker_requests(NodeKind::Directory), vec![&rust_marker()]);
    assert!(c.marker_requests(NodeKind::Symlink).is_empty());
    assert_eq!(
        c.classify(facts(Some("target"), NodeKind::Symlink, &BTreeMap::new()))
            .unwrap()
            .decision,
        Decision::NoMatch
    );
}

#[test]
fn input_order_does_not_change_catalog_identity_or_classification() {
    let a = PluginCatalog::load(
        &[input(RUST, Origin::Builtin), input(FORGE, Origin::External)],
        &[],
    )
    .unwrap();
    let b = PluginCatalog::load(
        &[input(FORGE, Origin::External), input(RUST, Origin::Builtin)],
        &[],
    )
    .unwrap();
    assert_eq!(a.catalog_digest(), b.catalog_digest());
    let m = BTreeMap::from([
        (rust_marker(), MarkerFact::Known(true)),
        (
            MarkerProbe {
                scope: Scope::Parent,
                name: "forge.project".into(),
            },
            MarkerFact::Known(false),
        ),
    ]);
    assert_eq!(
        a.classify(facts(Some("target"), NodeKind::Directory, &m))
            .unwrap(),
        b.classify(facts(Some("target"), NodeKind::Directory, &m))
            .unwrap()
    );
}

#[test]
fn partial_exclusion_removes_only_the_disabled_fact_bindings() {
    let bytes = edited(|v| {
        v["program"]["facts"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"extra","type":"boolean"}));
        v["program"]["rules"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"extra-rule","when":{"fact":"extra","op":"eq","value":true}}));
        v["bindings"]["extra"] = json!({"source":"marker","scope":"own","name":"disabled.marker"});
        v["targets"].as_array_mut().unwrap().push(json!({"id":"example.extra","title":"Extra","ecosystem":"example","kind":"cache","ruleIds":["extra-rule"]}));
    });
    let c = PluginCatalog::load(&[input(&bytes, Origin::Builtin)], &["example.extra"]).unwrap();
    assert_eq!(c.marker_requests(NodeKind::Directory), vec![&rust_marker()]);
    let m = BTreeMap::from([(rust_marker(), MarkerFact::Known(true))]);
    assert_eq!(
        c.classify(facts(Some("target"), NodeKind::Directory, &m))
            .unwrap()
            .decision,
        Decision::Found("rust.target".into())
    );
}

#[test]
fn own_marker_on_files_is_false_without_a_probe() {
    let bytes = edited(|v| {
        v["bindings"]["project"]["scope"] = json!("own");
        v["program"]["rules"][0]["when"] = json!({"all":[{"fact":"name","op":"eq","value":"target"},{"fact":"kind","op":"eq","value":"regular_file"},{"fact":"project","op":"eq","value":false}]});
    });
    let c = load(&bytes).unwrap();
    assert!(c.marker_requests(NodeKind::RegularFile).is_empty());
    assert_eq!(
        c.classify(facts(
            Some("target"),
            NodeKind::RegularFile,
            &BTreeMap::new()
        ))
        .unwrap()
        .decision,
        Decision::Found("rust.target".into())
    );
}

#[test]
fn aggregate_target_and_marker_budgets_cannot_be_bypassed_by_exclusions() {
    fn generated(package: usize, marker_count: usize, target_count: usize) -> Vec<u8> {
        let facts: Vec<_> = (0..marker_count)
            .map(|i| json!({"id":format!("f{i}"),"type":"boolean"}))
            .collect();
        let bindings: serde_json::Map<_, _> = (0..marker_count)
            .map(|i| {
                (
                    format!("f{i}"),
                    json!({"source":"marker","scope":"parent","name":format!("m{package}-{i}")}),
                )
            })
            .collect();
        let rules:Vec<_>=(0..target_count).map(|i|json!({"id":format!("r{i}"),"when":{"fact":format!("f{}",i%marker_count),"op":"eq","value":true}})).collect();
        let targets:Vec<_>=(0..target_count).map(|i|json!({"id":format!("example.p{package}.t{i}"),"title":"Target","ecosystem":"example","kind":"cache","ruleIds":[format!("r{i}")]})).collect();
        serde_json::to_vec(&json!({"format":"buildhunter.plugin/1-draft.1","id":format!("example.p{package}"),"version":"1.0.0","title":"Example","description":"Fixture","program":{"format":"rifle.program/1-draft.1","facts":facts,"rules":rules},"bindings":bindings,"targets":targets})).unwrap()
    }
    let docs: Vec<_> = (0..9).map(|i| generated(i, 1, 256)).collect();
    let inputs: Vec<_> = docs.iter().map(|x| input(x, Origin::External)).collect();
    assert!(PluginCatalog::load(&inputs[..8], &[]).is_ok());
    assert_eq!(
        PluginCatalog::load(&inputs, &[]).unwrap_err().code,
        "limit_exceeded"
    );
    let docs: Vec<_> = (0..5).map(|i| generated(i, 128, 128)).collect();
    let inputs: Vec<_> = docs.iter().map(|x| input(x, Origin::External)).collect();
    assert!(PluginCatalog::load(&inputs[..4], &[]).is_ok());
    let excludes: Vec<_> = (0..5)
        .flat_map(|i| (0..128).map(move |t| format!("example.p{i}.t{t}")))
        .collect();
    assert_eq!(
        PluginCatalog::load(
            &inputs,
            &excludes.iter().map(String::as_str).collect::<Vec<_>>()
        )
        .unwrap_err()
        .code,
        "limit_exceeded"
    );
}

#[test]
fn new_ecosystem_rule_is_evaluated_without_fixed_marker_vocabulary() {
    let c = PluginCatalog::load(&[input(FORGE, Origin::External)], &[]).unwrap();
    let m = BTreeMap::from([(
        MarkerProbe {
            scope: Scope::Parent,
            name: "forge.project".into(),
        },
        MarkerFact::Known(true),
    )]);
    assert_eq!(
        c.classify(facts(Some("forge-out"), NodeKind::Directory, &m))
            .unwrap()
            .decision,
        Decision::Found("example.forge-output".into())
    );
}
