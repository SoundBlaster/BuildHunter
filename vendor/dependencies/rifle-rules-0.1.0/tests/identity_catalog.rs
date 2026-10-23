use rifle_rules::{Catalog, JsonIdentity};
use serde_json::{Value, json};

fn program() -> Value {
    json!({"format":"rifle.program/1-draft.1","facts":[{"id":"flag","type":"boolean"}],"rules":[{"id":"result","when":{"fact":"flag","op":"eq","value":true}}]})
}
fn package(id: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"id":id,"version":"1.0.0","program":program()})).unwrap()
}

#[test]
fn jcs_rfc_serialization_vector() {
    // RFC 8785 section 3.2.2, data/expected canonicalization example.
    let raw=br#"{ "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001], "string": "\u20ac$\u000F\nA'B\"\\\\\"/", "literals": [null, true, false] }"#;
    let expected = r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#;
    assert_eq!(
        JsonIdentity::from_json(raw).unwrap().canonical_bytes(),
        expected.as_bytes()
    );
}

#[test]
fn utf16_key_order_and_escaping_are_jcs_not_rust_string_sorting() {
    let input = "{\"\\uE000\":1,\"😀\":2,\"\\r\":3,\"a\":4}";
    let expected = "{\"\\r\":3,\"a\":4,\"😀\":2,\"\u{E000}\":1}";
    assert_eq!(
        JsonIdentity::from_json(input.as_bytes())
            .unwrap()
            .canonical_bytes(),
        expected.as_bytes()
    );
    assert_eq!(
        JsonIdentity::from_json(b"[-0.0,1e-7,1e-6,1e21]")
            .unwrap()
            .canonical_bytes(),
        b"[0,1e-7,0.000001,1e+21]"
    );
}

#[test]
fn identity_is_exact_canonical_sha256_and_preserves_arrays_and_unicode() {
    let a = JsonIdentity::from_json(b"{\"b\":2,\"a\":1}").unwrap();
    let b = JsonIdentity::from_json(b" { \"a\":1.0, \"b\":2 } ").unwrap();
    assert_eq!(a.digest(), b.digest());
    assert_eq!(
        a.digest().to_string(),
        "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
    );
    assert_ne!(
        JsonIdentity::from_json(b"[1,2]").unwrap().digest(),
        JsonIdentity::from_json(b"[2,1]").unwrap().digest()
    );
    assert_ne!(
        JsonIdentity::from_json("\"é\"".as_bytes())
            .unwrap()
            .digest(),
        JsonIdentity::from_json(br#""e\u0301""#).unwrap().digest()
    );
}

#[test]
fn malformed_input_and_unsafe_integer_metadata_are_not_silently_hashed() {
    for input in [
        br#"{"x":1,"x":2}"#.as_slice(),
        br#"{"x":"\uD800"}"#.as_slice(),
        b"[NaN]".as_slice(),
        b"1e999".as_slice(),
        b"{\xff}".as_slice(),
    ] {
        assert_eq!(
            JsonIdentity::from_json(input).unwrap_err().code,
            "invalid_shape"
        );
    }
    for input in [
        b"9007199254740992".as_slice(),
        b"-9007199254740992".as_slice(),
        b"18446744073709551615".as_slice(),
        b"18446744073709551616".as_slice(),
    ] {
        assert_eq!(
            JsonIdentity::from_json(input).unwrap_err().code,
            "unsupported_number"
        );
    }
    assert!(JsonIdentity::from_json(b"9007199254740991").is_ok());
    assert_eq!(
        JsonIdentity::from_json(&vec![b' '; 1_048_577])
            .unwrap_err()
            .code,
        "limit_exceeded"
    );
}

#[test]
fn catalog_order_is_stable_and_full_package_content_is_covered() {
    let a = package("example.alpha");
    let b = package("example.beta");
    let x = Catalog::from_json_documents(&[&a, &b]).unwrap();
    let y = Catalog::from_json_documents(&[&b, &a]).unwrap();
    assert_eq!(x.digest(), y.digest());
    assert_eq!(
        x.packages().iter().map(|p| p.id()).collect::<Vec<_>>(),
        ["example.alpha", "example.beta"]
    );
    let mut doc: Value = serde_json::from_slice(&a).unwrap();
    doc["explanation"] = json!("changed opaque host content");
    let altered = serde_json::to_vec(&doc).unwrap();
    assert_ne!(
        x.digest(),
        Catalog::from_json_documents(&[&altered, &b])
            .unwrap()
            .digest()
    );
    assert_ne!(
        x.packages()[0].digest(),
        Catalog::from_json_documents(&[&altered])
            .unwrap()
            .packages()[0]
            .digest()
    );
    // Program equality does not imply package identity equality.
    assert_eq!(
        x.packages()[0].plan().fact_ids().collect::<Vec<_>>(),
        ["flag"]
    );
    let descriptors: Vec<_> = x
        .packages()
        .iter()
        .map(|p| json!({"id":p.id(),"version":p.version(),"digest":p.digest().to_string()}))
        .collect();
    let expected = JsonIdentity::from_json(&serde_json::to_vec(&descriptors).unwrap()).unwrap();
    assert_eq!(x.digest(), expected.digest());
}

#[test]
fn duplicate_packages_fail_even_across_versions_but_local_rule_ids_are_namespaced() {
    let a = package("example.alpha");
    let b = package("example.beta");
    assert!(Catalog::from_json_documents(&[&a, &b]).is_ok());
    assert_eq!(
        Catalog::from_json_documents(&[&a, &a]).unwrap_err().code,
        "duplicate_id"
    );
    let mut doc: Value = serde_json::from_slice(&a).unwrap();
    doc["version"] = json!("2.0.0");
    let c = serde_json::to_vec(&doc).unwrap();
    assert_eq!(
        Catalog::from_json_documents(&[&a, &c]).unwrap_err().code,
        "duplicate_id"
    );
}

#[test]
fn invalid_package_is_atomic_and_reports_program_location() {
    let valid = package("example.valid");
    let mut bad: Value = serde_json::from_slice(&valid).unwrap();
    bad["id"] = json!("example.bad");
    bad["program"]["rules"][0]["when"]["fact"] = json!("missing");
    let bytes = serde_json::to_vec(&bad).unwrap();
    let error = Catalog::from_json_documents(&[&valid, &bytes]).unwrap_err();
    assert_eq!(error.code, "unknown_fact");
    assert!(error.location.starts_with("/packages/1/program/"));
    assert!(Catalog::from_json_documents(&[&valid]).is_ok());
}

#[test]
fn package_header_and_aggregate_limits_are_enforced() {
    for (field, val) in [
        ("id", json!("Bad")),
        ("version", json!("01.0.0")),
        ("version", json!("1.0")),
        ("program", json!({})),
    ] {
        let mut p: Value = serde_json::from_slice(&package("example.a")).unwrap();
        p[field] = val;
        assert!(Catalog::from_json_documents(&[&serde_json::to_vec(&p).unwrap()]).is_err());
    }
    let documents: Vec<_> = (0..65).map(|i| package(&format!("example.p{i}"))).collect();
    let refs: Vec<_> = documents.iter().map(|x| x.as_slice()).collect();
    assert_eq!(
        Catalog::from_json_documents(&refs).unwrap_err().code,
        "limit_exceeded"
    );
    assert!(Catalog::from_json_documents(&refs[..64]).is_ok());
    let big = vec![b' '; 1_048_576];
    let refs = vec![big.as_slice(); 17];
    assert_eq!(
        Catalog::from_json_documents(&refs).unwrap_err().code,
        "limit_exceeded"
    );
    let empty = Catalog::from_json_documents(&[]).unwrap();
    assert_eq!(
        empty.digest(),
        JsonIdentity::from_json(b"[]").unwrap().digest()
    );
}

#[test]
fn aggregate_rules_and_nodes_are_counted_not_just_package_count() {
    fn many(id: &str, nodes: bool) -> Vec<u8> {
        let expr = if nodes {
            json!({"all":[{"all":vec![json!({"fact":"flag","op":"eq","value":true});31]}, {"all":vec![json!({"fact":"flag","op":"eq","value":true});31]}, {"all":vec![json!({"fact":"flag","op":"eq","value":true});31]}, {"all":vec![json!({"fact":"flag","op":"eq","value":true});30]}]})
        } else {
            json!({"fact":"flag","op":"eq","value":true})
        };
        let rules: Vec<_> = (0..if nodes { 128 } else { 256 })
            .map(|i| json!({"id":format!("r{i}"),"when":expr}))
            .collect();
        serde_json::to_vec(&json!({"id":id,"version":"1.0.0","program":{"format":"rifle.program/1-draft.1","facts":[{"id":"flag","type":"boolean"}],"rules":rules}})).unwrap()
    }
    let docs: Vec<_> = (0..17)
        .map(|i| many(&format!("example.p{i}"), false))
        .collect();
    let refs: Vec<_> = docs.iter().map(|x| x.as_slice()).collect();
    assert!(Catalog::from_json_documents(&refs[..16]).is_ok());
    assert_eq!(
        Catalog::from_json_documents(&refs).unwrap_err().code,
        "limit_exceeded"
    );
    let dense = many("example.dense", true);
    let dense2 = many("example.dense2", true);
    let small = package("example.small");
    assert!(Catalog::from_json_documents(&[&dense, &dense2]).is_ok());
    assert_eq!(
        Catalog::from_json_documents(&[&dense, &dense2, &small])
            .unwrap_err()
            .code,
        "limit_exceeded"
    );
}

#[test]
fn catalog_owns_its_snapshot_and_preserves_evaluator_input_contract() {
    let catalog = {
        let bytes = package("example.owned");
        Catalog::from_json_documents(&[&bytes]).unwrap()
    };
    let plan = catalog.packages()[0].plan();
    let out = plan
        .evaluate(&[rifle_rules::FactValue::Unavailable("io_error")])
        .unwrap();
    assert_eq!(out[0].truth, rifle_rules::Truth::Unknown);
    assert_eq!(out[0].unavailable[0].fact_id, "flag");
    assert_eq!(plan.evaluate(&[]).unwrap_err().code, "missing_fact");
}

#[test]
fn opaque_host_fields_are_hashed_but_not_claimed_valid_by_rifle() {
    let mut doc: Value = serde_json::from_slice(&package("example.host")).unwrap();
    doc["format"] = json!("unknown.host/2");
    doc["binding"] = json!({"arbitrary":"data"});
    assert!(Catalog::from_json_documents(&[&serde_json::to_vec(&doc).unwrap()]).is_ok());
    // Opaque fields do not allow an unvalidated Rifle program through.
    doc["program"]["format"] = json!("unknown.program/2");
    assert_eq!(
        Catalog::from_json_documents(&[&serde_json::to_vec(&doc).unwrap()])
            .unwrap_err()
            .code,
        "unsupported_format"
    );
}

#[test]
fn integer_token_admission_does_not_scan_inside_strings_or_change_jcs_floats() {
    let string_doc = serde_json::to_vec(
        &json!({"text":"18446744073709551616 \" 9999999999999999999999", "slash":"\\"}),
    )
    .unwrap();
    assert!(JsonIdentity::from_json(&string_doc).is_ok());
    assert!(JsonIdentity::from_json(b"[1e30,9007199254740992.0]").is_ok());
    let mut doc: Value = serde_json::from_slice(&package("example.integer")).unwrap();
    doc["opaque"] = json!("INTEGER_TOKEN");
    let text = serde_json::to_string(&doc)
        .unwrap()
        .replace("\"INTEGER_TOKEN\"", "18446744073709551616");
    assert_eq!(
        Catalog::from_json_documents(&[text.as_bytes()])
            .unwrap_err()
            .code,
        "unsupported_number"
    );
}
