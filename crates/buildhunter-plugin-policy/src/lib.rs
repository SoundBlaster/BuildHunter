//! Opt-in pure BuildHunter plugin policy. This does not acquire filesystem facts.
pub mod builtin;
mod dispatch;
mod validation;

use rifle_rules::{Catalog, Error, Plan, Sha256Digest};
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
    dispatch: dispatch::Dispatch,
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
        let mut dispatch = dispatch::Dispatch::default();
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
            dispatch.add(doc, &plugin)?;
        }
        targets.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self {
            digest: *catalog.digest(),
            exclusions,
            dispatch,
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
    /// Relevant probes for one candidate, derived conservatively from enabled rules.
    /// Unavailable names retain all possible targets instead of becoming NoMatch.
    pub fn candidate_marker_requests(
        &self,
        name: Option<&str>,
        kind: NodeKind,
    ) -> Vec<&MarkerProbe> {
        self.dispatch.markers(name, kind)
    }
    /// Pure classification; an index may exclude only targets proven unable to match.
    pub fn classify(&self, facts: CandidateFacts<'_>) -> Result<Classification, Error> {
        self.dispatch.classify(facts)
    }
}

fn error(code: &'static str, location: &str, message: impl Into<String>) -> Error {
    Error {
        code,
        location: location.into(),
        message: message.into(),
    }
}
