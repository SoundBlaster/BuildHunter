use rifle_rules::{FactValue, Plan, Truth};
use serde_json::{Value, json};

fn program(condition: Value) -> Value {
    json!({"format":"rifle.program/1-draft.1",
        "facts":[{"id":"flag","type":"boolean"}],
        "rules":[{"id":"result","when":condition}]})
}
fn leaf() -> Value {
    json!({"fact":"flag","op":"eq","value":true})
}
fn compile(value: &Value) -> Result<Plan, rifle_rules::Error> {
    Plan::from_json(&serde_json::to_vec(value).unwrap())
}
fn reject(value: &Value, code: &str) {
    let error = compile(value).unwrap_err();
    assert_eq!(error.code, code, "{error}");
    assert!(error.location.is_empty() || error.location.starts_with('/')); // Root JSON pointer is empty.
}

#[test]
fn known_and_unknown_are_distinct() {
    let plan = compile(&program(leaf())).unwrap();
    assert_eq!(
        plan.evaluate(&[FactValue::Boolean(true)]).unwrap()[0].truth,
        Truth::True
    );
    assert_eq!(
        plan.evaluate(&[FactValue::Boolean(false)]).unwrap()[0].truth,
        Truth::False
    );
    let result = plan
        .evaluate(&[FactValue::Unavailable("access_denied")])
        .unwrap();
    assert_eq!(result[0].truth, Truth::Unknown);
    assert_eq!(result[0].unavailable[0].fact_id, "flag");
    assert_eq!(result[0].unavailable[0].reason, "access_denied");
}

#[test]
fn complete_three_valued_truth_tables_and_permutations() {
    for operator in ["all", "any"] {
        for a in [Truth::True, Truth::False, Truth::Unknown] {
            for b in [Truth::True, Truth::False, Truth::Unknown] {
                let mut p =
                    program(json!({operator:[leaf(),{"fact":"other","op":"eq","value":true}]}));
                p["facts"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"id":"other","type":"boolean"}));
                let expected = match operator {
                    "all" if a == Truth::False || b == Truth::False => Truth::False,
                    "any" if a == Truth::True || b == Truth::True => Truth::True,
                    _ if a == Truth::Unknown || b == Truth::Unknown => Truth::Unknown,
                    "all" => Truth::True,
                    _ => Truth::False,
                };
                let fact = |t| match t {
                    Truth::True => FactValue::Boolean(true),
                    Truth::False => FactValue::Boolean(false),
                    Truth::Unknown => FactValue::Unavailable("io_error"),
                };
                for reverse in [false, true] {
                    if reverse {
                        p["rules"][0]["when"][operator]
                            .as_array_mut()
                            .unwrap()
                            .reverse();
                    }
                    let outcome = compile(&p)
                        .unwrap()
                        .evaluate(&[fact(a), fact(b)])
                        .unwrap()
                        .remove(0);
                    assert_eq!(outcome.truth, expected);
                    assert_eq!(outcome.unavailable.is_empty(), expected != Truth::Unknown);
                }
            }
        }
    }
}

#[test]
fn negation_never_matches_unavailable_fact() {
    let plan = compile(&program(json!({"not":leaf()}))).unwrap();
    for (value, expected) in [
        (FactValue::Boolean(true), Truth::False),
        (FactValue::Boolean(false), Truth::True),
        (FactValue::Unavailable("cloud_skipped"), Truth::Unknown),
    ] {
        assert_eq!(plan.evaluate(&[value]).unwrap()[0].truth, expected);
    }
}

#[test]
fn unknown_diagnostics_are_sorted_deduplicated_and_relevant() {
    let mut p = program(json!({"all":[leaf(),leaf(),{"fact":"aardvark","op":"eq","value":true}]}));
    p["facts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"aardvark","type":"boolean"}));
    let plan = compile(&p).unwrap();
    let out = plan
        .evaluate(&[
            FactValue::Unavailable("denied"),
            FactValue::Unavailable("missing"),
        ])
        .unwrap();
    assert_eq!(
        out[0]
            .unavailable
            .iter()
            .map(|x| x.fact_id.as_str())
            .collect::<Vec<_>>(),
        ["aardvark", "flag"]
    );
    let out = plan
        .evaluate(&[FactValue::Unavailable("denied"), FactValue::Boolean(false)])
        .unwrap();
    assert_eq!(out[0].truth, Truth::False);
    assert!(out[0].unavailable.is_empty());
}

#[test]
fn input_errors_are_not_rule_false_and_plan_is_reusable() {
    let plan = compile(&program(leaf())).unwrap();
    assert_eq!(plan.fact_ids().collect::<Vec<_>>(), ["flag"]);
    assert_eq!(plan.evaluate(&[]).unwrap_err().code, "missing_fact");
    assert_eq!(
        plan.evaluate(&[FactValue::String("true")])
            .unwrap_err()
            .code,
        "type_mismatch"
    );
    assert_eq!(
        plan.evaluate(&[FactValue::Boolean(true), FactValue::Boolean(true)])
            .unwrap_err()
            .code,
        "invalid_shape"
    );
    assert_eq!(
        plan.evaluate(&[FactValue::Boolean(true)]).unwrap()[0].truth,
        Truth::True
    );
}

#[test]
fn string_operations_are_exact_and_typed() {
    for (condition, input, expected) in [
        (
            json!({"fact":"flag","op":"suffix","value":".build"}),
            "demo.build",
            Truth::True,
        ),
        (
            json!({"fact":"flag","op":"suffix","value":".build"}),
            "demo.BUILD",
            Truth::False,
        ),
        (
            json!({"fact":"flag","op":"in","values":["α","β"]}),
            "β",
            Truth::True,
        ),
        (
            json!({"fact":"flag","op":"eq","value":"é"}),
            "e\u{301}",
            Truth::False,
        ),
    ] {
        let mut p = program(condition);
        p["facts"][0]["type"] = json!("string");
        assert_eq!(
            compile(&p)
                .unwrap()
                .evaluate(&[FactValue::String(input)])
                .unwrap()[0]
                .truth,
            expected
        );
    }
}

#[test]
fn published_program_examples_need_no_artifact_vocabulary_in_engine() {
    for bytes in [
        include_bytes!("../fixtures/rust-program.json").as_slice(),
        include_bytes!("../fixtures/synthetic-program.json").as_slice(),
    ] {
        let plan = Plan::from_json(bytes).unwrap();
        let name = if bytes.windows(9).any(|x| x == b"forge-out") {
            "forge-out"
        } else {
            "target"
        };
        assert_eq!(
            plan.evaluate(&[
                FactValue::String(name),
                FactValue::String("directory"),
                FactValue::Boolean(true)
            ])
            .unwrap()[0]
                .truth,
            Truth::True
        );
        assert_eq!(
            plan.evaluate(&[
                FactValue::String(name),
                FactValue::String("directory"),
                FactValue::Boolean(false)
            ])
            .unwrap()[0]
                .truth,
            Truth::False
        );
    }
}

#[test]
fn reject_version_fields_references_and_types() {
    let mut p = program(leaf());
    p["format"] = json!("future");
    reject(&p, "unsupported_format");
    let mut p = program(leaf());
    p["execute"] = json!("pwd");
    reject(&p, "invalid_shape");
    let mut p = program(leaf());
    p["rules"][0]["when"]["fact"] = json!("absent");
    reject(&p, "unknown_fact");
    let mut p = program(leaf());
    p["rules"][0]["when"]["value"] = json!("true");
    reject(&p, "type_mismatch");
    let mut p = program(leaf());
    p["facts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unused","type":"boolean"}));
    reject(&p, "unused_fact");
}

#[test]
fn duplicate_json_keys_and_ids_are_rejected_before_information_is_lost() {
    let data = br#"{"format":"wrong","format":"rifle.program/1-draft.1","facts":[],"rules":[]}"#;
    assert_eq!(Plan::from_json(data).unwrap_err().code, "invalid_shape");
    let mut p = program(leaf());
    let fact = p["facts"][0].clone();
    p["facts"].as_array_mut().unwrap().push(fact);
    reject(&p, "duplicate_id");
    let mut p = program(leaf());
    let rule = p["rules"][0].clone();
    p["rules"].as_array_mut().unwrap().push(rule);
    reject(&p, "duplicate_id");
}

#[test]
fn malformed_conditions_and_literals_are_rejected() {
    for expression in [
        json!({"all":[]}),
        json!({"any":[]}),
        json!({"not":leaf(),"all":[leaf()]}),
        json!({"fact":"flag","op":"eq","value":1}),
        json!({"fact":"flag","op":"in","values":[]}),
        json!({"fact":"flag","op":"in","values":[true,true]}),
        json!({"fact":"flag","op":"shell","value":true}),
    ] {
        assert!(compile(&program(expression)).is_err());
    }
    for id in ["x\n", "X", "a..b", "-x", ""] {
        let mut p = program(leaf());
        p["rules"][0]["id"] = json!(id);
        reject(&p, "invalid_shape");
    }
}

#[test]
fn limit_boundaries_are_enforced() {
    let mut expr = leaf();
    for _ in 0..7 {
        expr = json!({"not":expr});
    }
    assert!(compile(&program(expr.clone())).is_ok());
    reject(&program(json!({"not":expr})), "limit_exceeded");
    let mut p = program(json!({"all":vec![leaf();33]}));
    reject(&p, "limit_exceeded");
    p = program(leaf());
    let rules = vec![p["rules"][0].clone(); 257];
    p["rules"] = json!(rules);
    reject(&p, "limit_exceeded");
    assert_eq!(
        Plan::from_json(&vec![b' '; 1_048_577]).unwrap_err().code,
        "limit_exceeded"
    );
    // 1 root + 4 subgroups + 124 leaves = 129 expression nodes.
    reject(
        &program(json!({"all":vec![json!({"all":vec![leaf();31]});4]})),
        "limit_exceeded",
    );
}

#[test]
fn concurrent_read_only_evaluation() {
    let plan = compile(&program(leaf())).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                assert_eq!(
                    plan.evaluate(&[FactValue::Boolean(true)]).unwrap()[0].truth,
                    Truth::True
                );
            });
        }
    });
}

#[test]
fn parser_rejects_invalid_unicode_trailing_data_and_nested_duplicates() {
    for bytes in [
        br#"{"rules":{"when":{"not":true,"not":false}}}"#.as_slice(),
        br#"{"format":"\uD800"}"#.as_slice(),
        b"{\xff}".as_slice(),
        b"{} {}".as_slice(),
        b"{\"x\":1e999}".as_slice(),
    ] {
        assert_eq!(Plan::from_json(bytes).unwrap_err().code, "invalid_shape");
    }
    let deep = format!("{}0{}", "[".repeat(100), "]".repeat(100));
    assert_eq!(
        Plan::from_json(deep.as_bytes()).unwrap_err().code,
        "limit_exceeded"
    );
}

#[test]
fn identifiers_lengths_types_and_subobjects_are_closed() {
    for (path, value) in [
        ("/facts/0/id", json!("x".repeat(129))),
        ("/facts/0/type", json!("number")),
        ("/rules/0/when/value", json!(null)),
        (
            "/rules/0/when",
            json!({"fact":"flag","op":"suffix","value":"x"}),
        ),
    ] {
        let mut p = program(leaf());
        *p.pointer_mut(path).unwrap() = value;
        assert!(compile(&p).is_err());
    }
    for path in ["/facts/0", "/rules/0", "/rules/0/when"] {
        let mut p = program(leaf());
        p.pointer_mut(path).unwrap()["unknown"] = json!(true);
        reject(&p, "invalid_shape");
    }
    let mut p = program(leaf());
    p["facts"][0]["type"] = json!("string");
    p["rules"][0]["when"] = json!({"fact":"flag","op":"in","values":["x",true]});
    reject(&p, "type_mismatch");
    p["rules"][0]["when"] = json!({"fact":"flag","op":"eq","value":"💡".repeat(256)});
    assert!(compile(&p).is_ok());
    p["rules"][0]["when"]["value"] = json!("💡".repeat(257));
    reject(&p, "limit_exceeded");
}

#[test]
fn accepted_count_and_node_boundaries_are_not_off_by_one() {
    let mut p = program(leaf());
    p["rules"] = json!(
        (0..256)
            .map(|i| json!({"id":format!("r{i}"),"when":leaf()}))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        compile(&p)
            .unwrap()
            .evaluate(&[FactValue::Boolean(true)])
            .unwrap()
            .len(),
        256
    );
    let mut groups = vec![json!({"all":vec![leaf();31]}); 3];
    groups.push(json!({"all":vec![leaf();30]}));
    assert!(compile(&program(json!({"all":groups}))).is_ok()); // 128 nodes
    p = program(leaf());
    p["facts"] = json!(
        (0..128)
            .map(|i| json!({"id":format!("f{i}"),"type":"boolean"}))
            .collect::<Vec<_>>()
    );
    p["rules"]=json!((0..128).map(|i|json!({"id":format!("r{i}"),"when":{"fact":format!("f{i}"),"op":"eq","value":true}})).collect::<Vec<_>>());
    assert!(compile(&p).is_ok());
    p["facts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"extra","type":"boolean"}));
    reject(&p, "limit_exceeded");
}
