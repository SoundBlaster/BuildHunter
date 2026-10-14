use super::error;
use super::{
    ActivePlugin, Binding, MarkerProbe, Origin, Scope, Target, TargetDescriptor, TargetKind,
};
use rifle_rules::{Error, FactType, Plan};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn id(s: &str, path: &str) -> Result<(), Error> {
    if s.len() > 128
        || !s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || s.split(['.', '-']).any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
        return Err(error("invalid_shape", path, "invalid identifier"));
    }
    Ok(())
}
fn object<'a>(v: &'a Value, keys: &[&str], path: &str) -> Result<&'a Map<String, Value>, Error> {
    let o = v
        .as_object()
        .ok_or_else(|| error("invalid_shape", path, "expected object"))?;
    if o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)) {
        return Err(error("invalid_shape", path, "unexpected or missing fields"));
    }
    Ok(o)
}
fn text<'a>(v: &'a Value, max: usize, path: &str) -> Result<&'a str, Error> {
    let s = v
        .as_str()
        .ok_or_else(|| error("invalid_shape", path, "expected string"))?;
    if s.is_empty() || s.chars().count() > max {
        return Err(error("invalid_shape", path, "text length outside contract"));
    }
    Ok(s)
}
fn identifier(v: &Value, path: &str) -> Result<String, Error> {
    let s = text(v, 128, path)?;
    id(s, path)?;
    Ok(s.into())
}
fn reserved(target: &str, package: &str, origin: Origin) -> bool {
    [
        ("rust.", "org.buildhunter.rust"),
        ("swift.", "org.buildhunter.swift"),
        ("python.", "org.buildhunter.python"),
    ]
    .iter()
    .any(|(prefix, owner)| {
        target.starts_with(prefix) && (origin != Origin::Builtin || package != *owner)
    })
}
fn binding(v: &Value, kind: FactType, path: &str) -> Result<Binding, Error> {
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| error("invalid_shape", path, "missing source"))?;
    match source {
        "node.name" | "node.kind" => {
            object(v, &["source"], path)?;
            if kind != FactType::String {
                return Err(error(
                    "type_mismatch",
                    path,
                    "node source requires string fact",
                ));
            }
            Ok(if source == "node.name" {
                Binding::Name
            } else {
                Binding::Kind
            })
        }
        "marker" => {
            let o = object(v, &["source", "scope", "name"], path)?;
            if kind != FactType::Boolean {
                return Err(error(
                    "type_mismatch",
                    path,
                    "marker source requires boolean fact",
                ));
            }
            let scope = match o["scope"].as_str() {
                Some("own") => Scope::Own,
                Some("parent") => Scope::Parent,
                _ => return Err(error("invalid_shape", path, "unknown marker scope")),
            };
            let name = text(&o["name"], 128, path)?;
            if name == "."
                || name == ".."
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            {
                return Err(error(
                    "invalid_shape",
                    path,
                    "marker must be a single ASCII component",
                ));
            }
            Ok(Binding::Marker(MarkerProbe {
                scope,
                name: name.into(),
            }))
        }
        _ => Err(error("invalid_shape", path, "unsupported binding source")),
    }
}
pub(super) fn plugin(
    doc: &Value,
    original: &Plan,
    origin: Origin,
    excluded: &BTreeSet<String>,
) -> Result<(ActivePlugin, Vec<TargetDescriptor>, BTreeSet<MarkerProbe>), Error> {
    let root = object(
        doc,
        &[
            "format",
            "id",
            "version",
            "title",
            "description",
            "program",
            "bindings",
            "targets",
        ],
        "",
    )?;
    if root["format"].as_str() != Some("buildhunter.plugin/1-draft.1") {
        return Err(error(
            "unsupported_format",
            "/format",
            "unsupported plugin version",
        ));
    }
    text(&root["title"], 256, "/title")?;
    text(&root["description"], 2048, "/description")?;
    let package = root["id"].as_str().unwrap();
    let bindings = root["bindings"]
        .as_object()
        .ok_or_else(|| error("invalid_shape", "/bindings", "expected binding map"))?;
    if bindings.len() != original.facts().len() {
        return Err(error(
            "invalid_reference",
            "/bindings",
            "facts and bindings must correspond exactly",
        ));
    }
    let mut all_bindings = BTreeMap::new();
    let mut markers = BTreeSet::new();
    for fact in original.facts() {
        let path = format!("/bindings/{}", fact.id);
        let value = bindings
            .get(&fact.id)
            .ok_or_else(|| error("invalid_reference", &path, "binding missing"))?;
        let b = binding(value, fact.kind, &path)?;
        if let Binding::Marker(probe) = &b {
            markers.insert(probe.clone());
        }
        all_bindings.insert(fact.id.clone(), b);
    }
    let rules = root["program"]["rules"].as_array().unwrap();
    let rule_ids: BTreeSet<_> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let targets = root["targets"]
        .as_array()
        .ok_or_else(|| error("invalid_shape", "/targets", "expected target array"))?;
    if targets.is_empty() || targets.len() > 256 {
        return Err(error(
            "limit_exceeded",
            "/targets",
            "target count outside contract",
        ));
    }
    let mut assigned = BTreeSet::new();
    let mut target_ids = BTreeSet::new();
    let mut active = Vec::new();
    let mut descriptors = Vec::new();
    let mut active_rules = BTreeSet::new();
    for (i, v) in targets.iter().enumerate() {
        let path = format!("/targets/{i}");
        let t = object(v, &["id", "title", "ecosystem", "kind", "ruleIds"], &path)?;
        let tid = identifier(&t["id"], &format!("{path}/id"))?;
        if !target_ids.insert(tid.clone()) {
            return Err(error("duplicate_id", &path, &tid));
        }
        if reserved(&tid, package, origin) {
            return Err(error("reserved_id", &path, &tid));
        }
        let title = text(&t["title"], 256, &path)?.to_owned();
        let ecosystem = identifier(&t["ecosystem"], &path)?;
        let kind = match t["kind"].as_str() {
            Some("build_output") => TargetKind::BuildOutput,
            Some("cache") => TargetKind::Cache,
            Some("environment") => TargetKind::Environment,
            Some("bytecode") => TargetKind::Bytecode,
            Some("metadata") => TargetKind::Metadata,
            _ => {
                return Err(error("invalid_shape", &path, "unsupported target kind"));
            }
        };
        let refs = t["ruleIds"]
            .as_array()
            .ok_or_else(|| error("invalid_shape", &path, "expected rule references"))?;
        if refs.is_empty() || refs.len() > 256 {
            return Err(error(
                "invalid_reference",
                &path,
                "rule count outside contract",
            ));
        }
        let mut mapped = Vec::new();
        for r in refs {
            let rid = identifier(r, &path)?;
            if !rule_ids.contains(rid.as_str()) || !assigned.insert(rid.clone()) {
                return Err(error(
                    "invalid_reference",
                    &path,
                    "rule must exist and belong to one target exactly once",
                ));
            }
            mapped.push(rid);
        }
        let descriptor = TargetDescriptor {
            id: tid,
            title,
            ecosystem,
            kind,
        };
        descriptors.push(descriptor.clone());
        if !excluded.contains(&descriptor.id) {
            active_rules.extend(mapped.iter().cloned());
            active.push(Target {
                descriptor,
                rules: mapped,
            });
        }
    }
    if assigned.len() != rule_ids.len() {
        return Err(error(
            "invalid_reference",
            "/targets",
            "every rule must map to a target",
        ));
    }
    let (plan, bindings) = if active_rules.is_empty() {
        (None, Vec::new())
    } else {
        let mut program = root["program"].clone();
        program["rules"]
            .as_array_mut()
            .unwrap()
            .retain(|r| active_rules.contains(r["id"].as_str().unwrap()));
        let mut used = BTreeSet::new();
        for rule in program["rules"].as_array().unwrap() {
            collect_facts(&rule["when"], &mut used);
        }
        program["facts"]
            .as_array_mut()
            .unwrap()
            .retain(|f| used.contains(f["id"].as_str().unwrap()));
        let p = Plan::from_json(
            &serde_json::to_vec(&program)
                .map_err(|e| error("invalid_shape", "/program", e.to_string()))?,
        )?;
        let b = p
            .facts()
            .iter()
            .map(|f| all_bindings[&f.id].clone())
            .collect();
        (Some(p), b)
    };
    Ok((
        ActivePlugin {
            id: package.into(),
            plan,
            bindings,
            targets: active,
        },
        descriptors,
        markers,
    ))
}
fn collect_facts(expr: &Value, used: &mut BTreeSet<String>) {
    if let Some(fact) = expr.get("fact").and_then(Value::as_str) {
        used.insert(fact.into());
    }
    for key in ["all", "any"] {
        if let Some(children) = expr.get(key).and_then(Value::as_array) {
            for child in children {
                collect_facts(child, used);
            }
        }
    }
    if let Some(child) = expr.get("not") {
        collect_facts(child, used);
    }
}
