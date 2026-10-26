use buildhunter_plugin_policy::{
    CandidateFacts, Decision, MarkerFact, MarkerProbe, NodeKind, Origin, PluginCatalog,
    PluginInput, Scope,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const RUST: &[u8] = include_bytes!("../../../docs/contracts/plugin-v1/examples/rust.json");
const FORGE: &[u8] = include_bytes!("../../../docs/contracts/plugin-v1/examples/synthetic.json");
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
    // An invalid name is unknown, but a false required marker rules the target out.
    let markers = BTreeMap::from([(rust_marker(), MarkerFact::Known(false))]);
    let result = c
        .classify(facts(None, NodeKind::Directory, &markers))
        .unwrap();
    assert_eq!(result.decision, Decision::NoMatch);
    assert!(result.unavailable.is_empty());
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

#[test]
fn two_hundred_targets_request_only_the_relevant_marker() {
    let mut doc: Value = serde_json::from_slice(RUST).unwrap();
    doc["id"] = json!("example.dispatch");
    let mut facts = vec![
        json!({"id":"name","type":"string"}),
        json!({"id":"kind","type":"string"}),
    ];
    let mut bindings = json!({"name":{"source":"node.name"},"kind":{"source":"node.kind"}});
    let mut rules = Vec::new();
    let mut targets = Vec::new();
    for i in 0..200 {
        let fact = format!("marker-{i}");
        facts.push(json!({"id":fact,"type":"boolean"}));
        bindings[&fact] =
            json!({"source":"marker","scope":"own","name":format!("project-{i}.toml")});
        rules.push(json!({"id":format!("rule-{i}"),"when":{"all":[
            {"fact":"name","op":"eq","value":format!("cache-{i}")},
            {"fact":"kind","op":"eq","value":"directory"},
            {"fact":fact,"op":"eq","value":true}
        ]}}));
        targets.push(json!({"id":format!("example.target-{i}"),"title":format!("Cache {i}"),"ecosystem":"example","kind":"cache","ruleIds":[format!("rule-{i}")]}));
    }
    doc["program"]["facts"] = json!(facts);
    doc["program"]["rules"] = json!(rules);
    doc["bindings"] = bindings;
    doc["targets"] = json!(targets);
    let documents = (0..2)
        .map(|batch| {
            let mut document = doc.clone();
            document["id"] = json!(format!("example.dispatch-{batch}"));
            document["program"]["rules"] =
                json!(doc["program"]["rules"].as_array().unwrap()[batch * 100..(batch + 1) * 100]);
            document["targets"] =
                json!(doc["targets"].as_array().unwrap()[batch * 100..(batch + 1) * 100]);
            document["program"]["facts"]
                .as_array_mut()
                .unwrap()
                .retain(|f| {
                    let id = f["id"].as_str().unwrap();
                    id.strip_prefix("marker-").is_none_or(|i| {
                        (batch * 100..(batch + 1) * 100).contains(&i.parse::<usize>().unwrap())
                    })
                });
            let used = document["program"]["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f["id"].as_str().unwrap().to_owned())
                .collect::<BTreeSet<_>>();
            document["bindings"]
                .as_object_mut()
                .unwrap()
                .retain(|id, _| used.contains(id));
            serde_json::to_vec(&document).unwrap()
        })
        .collect::<Vec<_>>();
    let inputs = documents
        .iter()
        .map(|bytes| input(bytes, Origin::External))
        .collect::<Vec<_>>();
    for exclusions in [&[][..], &["example.target-137"][..]] {
        let catalog = PluginCatalog::load(&inputs, exclusions).unwrap();
        let requested = catalog.candidate_marker_requests(Some("cache-137"), NodeKind::Directory);
        assert_eq!(requested.len(), usize::from(exclusions.is_empty()));
        if let Some(probe) = requested.first() {
            assert_eq!(probe.name, "project-137.toml");
        }
        let markers = requested
            .into_iter()
            .map(|p| (p.clone(), MarkerFact::Known(true)))
            .collect();
        let result = catalog
            .classify(facts_for_dispatch(Some("cache-137"), &markers))
            .unwrap();
        assert_eq!(
            result.decision,
            if exclusions.is_empty() {
                Decision::Found("example.target-137".into())
            } else {
                Decision::NoMatch
            }
        );
        assert!(
            catalog
                .candidate_marker_requests(Some("ordinary"), NodeKind::Directory)
                .is_empty()
        );
    }
}

fn facts_for_dispatch<'a>(
    name: Option<&'a str>,
    markers: &'a BTreeMap<MarkerProbe, MarkerFact<'a>>,
) -> CandidateFacts<'a> {
    CandidateFacts {
        name,
        kind: NodeKind::Directory,
        markers,
    }
}

#[test]
fn indexed_dispatch_agrees_with_rifle_for_boolean_and_unavailable_facts() {
    let eq = json!({"fact":"name","op":"eq","value":"target"});
    let suffix = json!({"fact":"name","op":"suffix","value":"get"});
    let marker = json!({"fact":"project","op":"eq","value":true});
    let expressions = vec![
        json!({"all":[eq.clone(), marker.clone()]}),
        json!({"any":[eq.clone(),marker.clone()]}),
        json!({"not":eq.clone()}),
        json!({"all":[{"not":eq.clone()},marker.clone()]}),
        json!({"any":[{"all":[eq.clone(),marker.clone()]},suffix.clone()]}),
        json!({"all":[{"any":[eq.clone(),suffix]},marker.clone()]}),
        json!({"all":[{"fact":"name","op":"in","values":["target","other"]},marker]}),
    ];
    for expression in expressions {
        let mut doc: Value = serde_json::from_slice(RUST).unwrap();
        doc["program"]["rules"][0]["when"] = expression;
        // Keep exactly the used facts, as required by Rifle validation.
        let text = doc["program"]["rules"].to_string();
        doc["program"]["facts"]
            .as_array_mut()
            .unwrap()
            .retain(|f| text.contains(&format!("\"fact\":\"{}\"", f["id"].as_str().unwrap())));
        let used = doc["program"]["facts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["id"].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>();
        doc["bindings"]
            .as_object_mut()
            .unwrap()
            .retain(|id, _| used.contains(id));
        let bytes = serde_json::to_vec(&doc).unwrap();
        let catalog = load(&bytes).unwrap();
        let plan =
            rifle_rules::Plan::from_json(&serde_json::to_vec(&doc["program"]).unwrap()).unwrap();
        for name in [Some("target"), Some("other"), Some("unrelated"), None] {
            for marker in [
                MarkerFact::Known(true),
                MarkerFact::Known(false),
                MarkerFact::Unavailable("denied"),
            ] {
                let markers = BTreeMap::from([(rust_marker(), marker)]);
                let actual = catalog
                    .classify(facts_for_dispatch(name, &markers))
                    .unwrap();
                let values = plan
                    .facts()
                    .iter()
                    .map(|f| match f.id.as_str() {
                        "name" => name
                            .map(rifle_rules::FactValue::String)
                            .unwrap_or(rifle_rules::FactValue::Unavailable("invalid_name")),
                        "project" => match marker {
                            MarkerFact::Known(v) => rifle_rules::FactValue::Boolean(v),
                            MarkerFact::Unavailable(reason) => {
                                rifle_rules::FactValue::Unavailable(reason)
                            }
                        },
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>();
                let expected = match plan.evaluate(&values).unwrap()[0].truth {
                    rifle_rules::Truth::True => Decision::Found("rust.target".into()),
                    rifle_rules::Truth::False => Decision::NoMatch,
                    rifle_rules::Truth::Unknown => Decision::Indeterminate,
                };
                assert_eq!(
                    actual.decision, expected,
                    "name={name:?}, expression={}",
                    doc["program"]["rules"][0]["when"]
                );
            }
        }
    }
}

#[test]
fn rule_profile_preserves_decisions_and_separates_dispatch_phase_and_unknowns() {
    use buildhunter_plugin_policy::RulePhase;
    let catalog = load(RUST).unwrap();
    let mut profile = catalog.rule_profile();
    for (phase, name, marker) in [
        (
            RulePhase::Preliminary,
            "unrelated",
            MarkerFact::Known(false),
        ),
        (
            RulePhase::Preliminary,
            "target",
            MarkerFact::Unavailable("not_observed"),
        ),
        (RulePhase::Resolved, "target", MarkerFact::Known(true)),
        (RulePhase::Resolved, "target", MarkerFact::Known(false)),
    ] {
        let markers = BTreeMap::from([(rust_marker(), marker)]);
        assert_eq!(
            catalog
                .classify(facts(Some(name), NodeKind::Directory, &markers))
                .unwrap(),
            catalog
                .classify_profiled(
                    facts(Some(name), NodeKind::Directory, &markers),
                    &mut profile,
                    phase
                )
                .unwrap()
        );
    }
    let value = profile.json();
    assert_eq!(value["preliminary_calls"], 2);
    assert_eq!(value["resolved_calls"], 2);
    let rows = value["rules"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let preliminary = rows.iter().find(|r| r["phase"] == "preliminary").unwrap();
    let resolved = rows.iter().find(|r| r["phase"] == "resolved").unwrap();
    assert_eq!(preliminary["evaluations"], 1);
    assert_eq!(preliminary["skipped_by_dispatch"], 1);
    assert_eq!(preliminary["unknown"], 1);
    assert_eq!(resolved["evaluations"], 2);
    assert_eq!(resolved["true"], 1);
    assert_eq!(resolved["false"], 1);
    for row in rows {
        assert!(row["total_ns"].as_u64().unwrap() >= row["max_ns"].as_u64().unwrap());
    }
    // Histories do not grow with candidate count. Disabled rules have no rows.
    for _ in 0..1000 {
        catalog
            .classify_profiled(
                facts(Some("unrelated"), NodeKind::Directory, &BTreeMap::new()),
                &mut profile,
                RulePhase::Preliminary,
            )
            .unwrap();
    }
    assert_eq!(profile.json()["rules"].as_array().unwrap().len(), 2);
    let excluded = PluginCatalog::load(&[input(RUST, Origin::Builtin)], &["rust.target"]).unwrap();
    assert!(
        excluded.rule_profile().json()["rules"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut disabled_profile = excluded.rule_profile();
    assert_eq!(
        catalog
            .classify_profiled(
                facts(Some("target"), NodeKind::Directory, &BTreeMap::new()),
                &mut disabled_profile,
                RulePhase::Preliminary
            )
            .unwrap_err()
            .code,
        "catalog_mismatch"
    );
    let other = PluginCatalog::load(&[input(FORGE, Origin::External)], &[]).unwrap();
    assert_eq!(
        other
            .classify_profiled(
                facts(Some("x"), NodeKind::Directory, &BTreeMap::new()),
                &mut profile,
                RulePhase::Preliminary
            )
            .unwrap_err()
            .code,
        "catalog_mismatch"
    );
}
