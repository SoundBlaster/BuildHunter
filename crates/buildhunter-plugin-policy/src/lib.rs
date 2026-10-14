//! Opt-in pure BuildHunter plugin policy. This does not acquire filesystem facts.
mod validation;

use rifle_rules::{Catalog, Error, FactValue, Plan, Sha256Digest, Truth};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Builtin,
    External,
}
/// Origin is supplied by the host, never read from plugin data.
pub struct PluginInput<'a> {
    pub bytes: &'a [u8],
    pub origin: Origin,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Own,
    Parent,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkerProbe {
    pub scope: Scope,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Binding {
    Name,
    Kind,
    Marker(MarkerProbe),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Directory,
    RegularFile,
    Symlink,
    Other,
}
impl NodeKind {
    fn label(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::RegularFile => "regular_file",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum MarkerFact<'a> {
    Known(bool),
    Unavailable(&'a str),
}
/// Supply lossless Unicode name or None. Marker observations must obey the host
/// no-follow/scope policy; unavailable observations must not be changed to false.
pub struct CandidateFacts<'a> {
    pub name: Option<&'a str>,
    pub kind: NodeKind,
    pub markers: &'a BTreeMap<MarkerProbe, MarkerFact<'a>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    BuildOutput,
    Cache,
    Environment,
    Bytecode,
    Metadata,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetDescriptor {
    pub id: String,
    pub title: String,
    pub ecosystem: String,
    pub kind: TargetKind,
}
#[derive(Debug)]
struct Target {
    descriptor: TargetDescriptor,
    rules: Vec<String>,
}
#[derive(Debug)]
struct ActivePlugin {
    id: String,
    plan: Option<Plan>,
    bindings: Vec<Binding>,
    targets: Vec<Target>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    NoMatch,
    Found(String),
    Ambiguous,
    Indeterminate,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FactNotice {
    pub package_id: String,
    pub fact_id: String,
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub decision: Decision,
    pub matched: Vec<String>,
    pub unknown: Vec<String>,
    pub unavailable: Vec<FactNotice>,
}
impl Classification {
    fn empty() -> Self {
        Self {
            decision: Decision::NoMatch,
            matched: Vec::new(),
            unknown: Vec::new(),
            unavailable: Vec::new(),
        }
    }
}
#[derive(Debug)]
pub struct PluginCatalog {
    digest: Sha256Digest,
    exclusions: BTreeSet<String>,
    plugins: Vec<ActivePlugin>,
    targets: Vec<TargetDescriptor>,
    markers: BTreeSet<MarkerProbe>,
}
impl PluginCatalog {
    /// Validate the entire catalog before exclusions. Error never exposes a partial
    /// snapshot. Unknown exclusion IDs are preserved for future catalog updates.
    pub fn load(inputs: &[PluginInput<'_>], excluded: &[&str]) -> Result<Self, Error> {
        let documents: Vec<_> = inputs.iter().map(|x| x.bytes).collect();
        let catalog = Catalog::from_json_documents(&documents)?;
        let mut exclusions = BTreeSet::new();
        for id in excluded {
            validation::id(id, "/exclusions")?;
            exclusions.insert((*id).to_owned());
        }
        let mut source = BTreeMap::new();
        for input in inputs {
            // Rifle already rejected duplicate keys and malformed JSON globally.
            let doc: Value = serde_json::from_slice(input.bytes)
                .map_err(|e| error("invalid_shape", "", e.to_string()))?;
            source.insert(doc["id"].as_str().unwrap().to_owned(), (doc, input.origin));
        }
        let mut plugins = Vec::new();
        let mut targets = Vec::new();
        let mut target_ids = BTreeSet::new();
        let mut declared_markers = BTreeSet::new();
        let mut markers = BTreeSet::new();
        for package in catalog.packages() {
            let (doc, origin) = &source[package.id()];
            let (plugin, descriptors, all_markers) =
                validation::plugin(doc, package.plan(), *origin, &exclusions)?;
            for t in &descriptors {
                if !target_ids.insert(t.id.clone()) {
                    return Err(error("duplicate_id", "/targets", &t.id));
                }
            }
            targets.extend(descriptors);
            declared_markers.extend(all_markers);
            if targets.len() > 2048 || declared_markers.len() > 512 {
                return Err(error(
                    "limit_exceeded",
                    "/catalog",
                    "target/marker budget exceeded",
                ));
            }
            for binding in &plugin.bindings {
                if let Binding::Marker(probe) = binding {
                    markers.insert(probe.clone());
                }
            }
            plugins.push(plugin);
        }
        targets.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self {
            digest: *catalog.digest(),
            exclusions,
            plugins,
            targets,
            markers,
        })
    }
    pub fn catalog_digest(&self) -> &Sha256Digest {
        &self.digest
    }
    pub fn exclusions(&self) -> &BTreeSet<String> {
        &self.exclusions
    }
    /// Includes disabled targets for Settings presentation.
    pub fn targets(&self) -> &[TargetDescriptor] {
        &self.targets
    }
    pub fn marker_requests(&self, kind: NodeKind) -> Vec<&MarkerProbe> {
        if kind == NodeKind::Symlink {
            return Vec::new();
        }
        self.markers
            .iter()
            .filter(|m| kind == NodeKind::Directory || m.scope != Scope::Own)
            .collect()
    }
    /// Pure classification only: never measures or prunes a node. A caller must
    /// keep traversing descendants independently of this result.
    pub fn classify(&self, facts: CandidateFacts<'_>) -> Result<Classification, Error> {
        if facts.kind == NodeKind::Symlink {
            return Ok(Classification::empty());
        }
        let mut result = Classification::empty();
        let mut notices = BTreeSet::new();
        for plugin in &self.plugins {
            let Some(plan) = &plugin.plan else { continue };
            let mut values = Vec::new();
            for (index, binding) in plugin.bindings.iter().enumerate() {
                let value = match binding {
                    Binding::Name => facts
                        .name
                        .map(FactValue::String)
                        .unwrap_or(FactValue::Unavailable("invalid_name")),
                    Binding::Kind => FactValue::String(facts.kind.label()),
                    Binding::Marker(probe)
                        if probe.scope == Scope::Own && facts.kind != NodeKind::Directory =>
                    {
                        FactValue::Boolean(false)
                    }
                    Binding::Marker(probe) => match facts.markers.get(probe) {
                        Some(MarkerFact::Known(v)) => FactValue::Boolean(*v),
                        Some(MarkerFact::Unavailable(reason)) => FactValue::Unavailable(reason),
                        None => {
                            return Err(error(
                                "missing_fact",
                                &format!(
                                    "/packages/{}/bindings/{}",
                                    plugin.id,
                                    plan.facts()[index].id
                                ),
                                "marker observation missing",
                            ));
                        }
                    },
                };
                values.push(value);
            }
            let outcomes = plan.evaluate(&values)?;
            let by_rule: BTreeMap<_, _> =
                outcomes.iter().map(|o| (o.rule_id.as_str(), o)).collect();
            for target in &plugin.targets {
                let mut truth = Truth::False;
                let mut unknowns = BTreeSet::new();
                for rule in &target.rules {
                    let outcome = by_rule[rule.as_str()];
                    if outcome.truth == Truth::True {
                        truth = Truth::True;
                        unknowns.clear();
                        break;
                    }
                    if outcome.truth == Truth::Unknown {
                        truth = Truth::Unknown;
                        for f in &outcome.unavailable {
                            unknowns.insert(FactNotice {
                                package_id: plugin.id.clone(),
                                fact_id: f.fact_id.clone(),
                                reason: f.reason.clone(),
                            });
                        }
                    }
                }
                match truth {
                    Truth::True => result.matched.push(target.descriptor.id.clone()),
                    Truth::Unknown => {
                        result.unknown.push(target.descriptor.id.clone());
                        notices.extend(unknowns);
                    }
                    Truth::False => {}
                }
            }
        }
        result.matched.sort();
        result.unknown.sort();
        result.unavailable = notices.into_iter().collect();
        result.decision = match (result.matched.len(), result.unknown.is_empty()) {
            (0, true) => Decision::NoMatch,
            (1, true) => Decision::Found(result.matched[0].clone()),
            (0 | 1, false) => Decision::Indeterminate,
            _ => Decision::Ambiguous,
        };
        Ok(result)
    }
}

fn error(code: &'static str, location: &str, message: impl Into<String>) -> Error {
    Error {
        code,
        location: location.into(),
        message: message.into(),
    }
}
