//! Conservative name dispatch. Only necessary positive name conditions are indexed;
//! negation/unknown projections retain a fallback. Rifle remains the decision engine.
use crate::{
    ActivePlugin, Binding, CandidateFacts, Classification, Decision, FactNotice, MarkerFact,
    MarkerProbe, NodeKind, Scope, error,
};
use rifle_rules::{Error, FactValue, Plan, Truth};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NameKey {
    Exact(String),
    Suffix(String),
}

#[derive(Debug)]
struct CompiledTarget {
    package: String,
    id: String,
    plan: Plan,
    bindings: Vec<Binding>,
}

#[derive(Default, Debug)]
pub(super) struct Dispatch {
    targets: Vec<CompiledTarget>,
    exact: BTreeMap<String, Vec<usize>>,
    suffix: BTreeMap<String, Vec<usize>>,
    fallback: Vec<usize>,
}

impl Dispatch {
    pub(super) fn add(&mut self, doc: &Value, plugin: &ActivePlugin) -> Result<(), Error> {
        let Some(original) = &plugin.plan else {
            return Ok(());
        };
        let binding_map: BTreeMap<_, _> = original
            .facts()
            .iter()
            .zip(&plugin.bindings)
            .map(|(fact, binding)| (fact.id.as_str(), binding))
            .collect();
        let name_facts: BTreeSet<_> = binding_map
            .iter()
            .filter_map(|(id, binding)| matches!(binding, Binding::Name).then_some(*id))
            .collect();
        for target in &plugin.targets {
            let mut program = doc["program"].clone();
            program["rules"].as_array_mut().unwrap().retain(|r| {
                target
                    .rules
                    .iter()
                    .any(|id| Some(id.as_str()) == r["id"].as_str())
            });
            let names = union(
                program["rules"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| selector(&r["when"], &name_facts)),
            );
            let mut used = BTreeSet::new();
            for rule in program["rules"].as_array().unwrap() {
                crate::validation::collect_facts(&rule["when"], &mut used);
            }
            program["facts"]
                .as_array_mut()
                .unwrap()
                .retain(|f| used.contains(f["id"].as_str().unwrap()));
            let plan = Plan::from_json(
                &serde_json::to_vec(&program)
                    .map_err(|e| error("invalid_shape", "/program", e.to_string()))?,
            )?;
            let bindings = plan
                .facts()
                .iter()
                .map(|f| binding_map[f.id.as_str()].clone())
                .collect();
            let index = self.targets.len();
            match names {
                None => self.fallback.push(index),
                Some(names) => {
                    for key in names {
                        match key {
                            NameKey::Exact(name) => self.exact.entry(name).or_default().push(index),
                            NameKey::Suffix(name) => {
                                self.suffix.entry(name).or_default().push(index)
                            }
                        }
                    }
                }
            }
            self.targets.push(CompiledTarget {
                package: plugin.id.clone(),
                id: target.descriptor.id.clone(),
                plan,
                bindings,
            });
        }
        Ok(())
    }

    fn candidates(&self, name: Option<&str>) -> Vec<usize> {
        let Some(name) = name else {
            return (0..self.targets.len()).collect();
        };
        let mut indices = self.fallback.clone();
        if let Some(exact) = self.exact.get(name) {
            indices.extend(exact);
        }
        for (suffix, targets) in &self.suffix {
            if name.ends_with(suffix) {
                indices.extend(targets);
            }
        }
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    pub(super) fn markers(&self, name: Option<&str>, kind: NodeKind) -> Vec<&MarkerProbe> {
        if kind == NodeKind::Symlink {
            return Vec::new();
        }
        let mut markers = BTreeSet::new();
        for i in self.candidates(name) {
            for binding in &self.targets[i].bindings {
                if let Binding::Marker(probe) = binding
                    && (kind == NodeKind::Directory || probe.scope != Scope::Own)
                {
                    markers.insert(probe);
                }
            }
        }
        markers.into_iter().collect()
    }

    pub(super) fn classify(&self, facts: CandidateFacts<'_>) -> Result<Classification, Error> {
        let mut result = Classification::empty();
        if facts.kind == NodeKind::Symlink {
            return Ok(result);
        }
        let mut notices = BTreeSet::new();
        for index in self.candidates(facts.name) {
            let target = &self.targets[index];
            let mut values = Vec::with_capacity(target.bindings.len());
            for (index, binding) in target.bindings.iter().enumerate() {
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
                                    target.package,
                                    target.plan.facts()[index].id
                                ),
                                "marker observation missing",
                            ));
                        }
                    },
                };
                values.push(value);
            }
            let outcomes = target.plan.evaluate(&values)?;
            if outcomes.iter().any(|outcome| outcome.truth == Truth::True) {
                result.matched.push(target.id.clone());
            } else if outcomes
                .iter()
                .any(|outcome| outcome.truth == Truth::Unknown)
            {
                result.unknown.push(target.id.clone());
                for outcome in outcomes {
                    if outcome.truth == Truth::Unknown {
                        for fact in outcome.unavailable {
                            notices.insert(FactNotice {
                                package_id: target.package.clone(),
                                fact_id: fact.fact_id,
                                reason: fact.reason,
                            });
                        }
                    }
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

fn union(values: impl Iterator<Item = Option<BTreeSet<NameKey>>>) -> Option<BTreeSet<NameKey>> {
    let mut merged = BTreeSet::new();
    for value in values {
        merged.extend(value?);
    }
    Some(merged)
}

fn selector(expr: &Value, names: &BTreeSet<&str>) -> Option<BTreeSet<NameKey>> {
    if let Some(children) = expr.get("all").and_then(Value::as_array) {
        return children
            .iter()
            .filter_map(|child| selector(child, names))
            .min_by_key(BTreeSet::len);
    }
    if let Some(children) = expr.get("any").and_then(Value::as_array) {
        return union(children.iter().map(|child| selector(child, names)));
    }
    if !expr
        .get("fact")
        .and_then(Value::as_str)
        .is_some_and(|id| names.contains(id))
    {
        return None;
    }
    match expr["op"].as_str()? {
        "eq" => Some(BTreeSet::from([NameKey::Exact(
            expr["value"].as_str()?.into(),
        )])),
        "suffix" => Some(BTreeSet::from([NameKey::Suffix(
            expr["value"].as_str()?.into(),
        )])),
        "in" => Some(
            expr["values"]
                .as_array()?
                .iter()
                .map(|v| NameKey::Exact(v.as_str().unwrap().into()))
                .collect(),
        ),
        _ => None,
    }
}
