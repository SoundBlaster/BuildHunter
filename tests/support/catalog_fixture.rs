//! Shared fixture data for the client scan regression and Release benchmark.
use build_hunter::catalog::{CatalogSnapshot, ClientProfile};
use buildhunter_plugin_policy::{Origin, PluginCatalog, PluginInput};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn synthetic_documents(count: usize) -> Vec<Vec<u8>> {
    (0..count.div_ceil(100))
        .map(|batch| {
            let mut facts = vec![json!({"id":"name","type":"string"}), json!({"id":"kind","type":"string"})];
            let mut bindings = json!({"name":{"source":"node.name"},"kind":{"source":"node.kind"}});
            let mut rules = Vec::new();
            let mut targets = Vec::new();
            for i in batch * 100..count.min((batch + 1) * 100) {
                let marker = format!("marker-{i}");
                facts.push(json!({"id":marker,"type":"boolean"}));
                bindings[&marker] = json!({"source":"marker","scope":"own","name":format!("project-{i}.toml")});
                rules.push(json!({"id":format!("rule-{i}"),"when":{"all":[
                    {"fact":"name","op":"eq","value":format!("cache-{i}")},
                    {"fact":"kind","op":"eq","value":"directory"},
                    {"fact":marker,"op":"eq","value":true}
                ]}}));
                targets.push(json!({"id":format!("example.cache-{i}"),"title":format!("Cache {i}"),
                    "ecosystem":"example","kind":"cache","ruleIds":[format!("rule-{i}")]}));
            }
            serde_json::to_vec(&json!({"format":"buildhunter.plugin/1-draft.1", "id":format!("example.bench-{batch}"),
                "version":"1.0.0","title":"Catalog scan fixture","description":"Synthetic benchmark only",
                "program":{"format":"rifle.program/1-draft.1","facts":facts,"rules":rules},
                "bindings":bindings,"targets":targets})).unwrap()
        }).collect()
}

pub fn snapshot(
    profile: ClientProfile,
    synthetic: usize,
    basic: bool,
    excluded: &[&str],
) -> CatalogSnapshot {
    let documents = synthetic_documents(synthetic);
    let mut inputs = if basic {
        profile
            .documents()
            .map(|bytes| PluginInput {
                bytes,
                origin: Origin::Builtin,
            })
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    inputs.extend(documents.iter().map(|bytes| PluginInput {
        bytes,
        origin: Origin::External,
    }));
    CatalogSnapshot::from_catalog(profile, PluginCatalog::load(&inputs, excluded).unwrap())
}

pub fn prepare_synthetic(root: &Path, targets: usize, noise: usize) {
    for i in 0..targets {
        let directory = root.join(format!("cache-{i}"));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("project-{i}.toml")), b"project").unwrap();
        fs::write(directory.join("payload"), [17; 1024]).unwrap();
    }
    for i in 0..noise {
        let directory = root.join(format!("ordinary-{i}"));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("source.txt"), b"not an artifact").unwrap();
    }
}

pub fn semantic_rows(report: &build_hunter::client_scan::CatalogScanReport) -> Vec<Value> {
    let mut rows = report
        .artifacts
        .iter()
        .map(|a| {
            json!({"path":a.relative_path,"target":a.target.id,
        "bytes":a.bytes,"nested":a.nested,"partial":a.partial})
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(Value::to_string);
    rows
}
