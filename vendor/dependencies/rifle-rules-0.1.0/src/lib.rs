//! Pure rule validation and evaluation. The caller supplies facts in declared order.
//! No filesystem, product vocabulary or mutable scan state belongs to this crate.
mod catalog;
mod identity;
mod json;
pub use catalog::{Catalog, ProgramPackage};
pub use identity::{JsonIdentity, Sha256Digest};
mod validate;

use std::collections::BTreeSet;
use std::fmt;

pub const FORMAT: &str = "rifle.program/1-draft.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    /// JSON pointer for load errors; `/facts/<index>` for supplied input errors.
    pub location: String,
    pub message: String,
}
impl Error {
    pub(crate) fn new(code: &'static str, location: &str, message: impl Into<String>) -> Self {
        Self {
            code,
            location: location.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.location, self.message)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactType {
    String,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub id: String,
    pub kind: FactType,
}

#[derive(Debug, Clone, Copy)]
pub enum FactValue<'a> {
    String(&'a str),
    Boolean(bool),
    Unavailable(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Truth {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableFact {
    pub fact_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub rule_id: String,
    pub truth: Truth,
    /// Sorted by fact ID, deduplicated, and empty for a definite result.
    pub unavailable: Vec<UnavailableFact>,
}

/// Advisory wall time for one rule, including construction of its owned outcome.
/// Excludes input validation, catalog loading and host fact acquisition. Clock
/// overhead and scheduling affect the measurement; it is not CPU attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedOutcome {
    pub outcome: Outcome,
    pub elapsed: std::time::Duration,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Scalar {
    String(String),
    Boolean(bool),
}
impl Scalar {
    fn matches(&self, value: FactValue<'_>) -> bool {
        match (self, value) {
            (Self::String(a), FactValue::String(b)) => a == b,
            (Self::Boolean(a), FactValue::Boolean(b)) => *a == b,
            _ => false,
        }
    }
}
#[derive(Debug)]
pub(crate) enum Predicate {
    Eq(Scalar),
    In(Vec<Scalar>),
    Suffix(String),
}
#[derive(Debug)]
pub(crate) enum Expression {
    Compare(usize, Predicate),
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
}
#[derive(Debug)]
pub(crate) struct Rule {
    pub id: String,
    pub expression: Expression,
}

/// Immutable, validated program; safe to share between concurrent callers.
#[derive(Debug)]
pub struct Plan {
    pub(crate) facts: Vec<Fact>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) expression_nodes: usize,
}
impl Plan {
    /// Accept only the pure Rifle program, not the BuildHunter plugin envelope.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        validate::compile(bytes)
    }
    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    pub fn expression_nodes(&self) -> usize {
        self.expression_nodes
    }
    pub fn fact_ids(&self) -> impl Iterator<Item = &str> {
        self.facts.iter().map(|x| x.id.as_str())
    }

    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        self.rules.iter().map(|rule| rule.id.as_str())
    }

    /// Inputs correspond to `facts()` order. Missing values are integration errors.
    /// Unavailable values carry a host-owned reason and propagate as unknown.
    pub fn evaluate(&self, values: &[FactValue<'_>]) -> Result<Vec<Outcome>, Error> {
        self.validate_values(values)?;
        Ok(self
            .rules
            .iter()
            .map(|rule| self.outcome(rule, values))
            .collect())
    }

    /// Explicit instrumentation mode. Evaluate the same rules once, in the same
    /// order, with the same outcomes and errors as `evaluate`. No timers run on
    /// the ordinary path. Retains one timing per rule, never an event history.
    pub fn evaluate_timed(&self, values: &[FactValue<'_>]) -> Result<Vec<TimedOutcome>, Error> {
        self.validate_values(values)?;
        Ok(self
            .rules
            .iter()
            .map(|rule| {
                let start = std::time::Instant::now();
                let outcome = self.outcome(rule, values);
                TimedOutcome {
                    outcome,
                    elapsed: start.elapsed(),
                }
            })
            .collect())
    }

    fn validate_values(&self, values: &[FactValue<'_>]) -> Result<(), Error> {
        if values.len() < self.facts.len() {
            return Err(Error::new(
                "missing_fact",
                &format!("/facts/{}", values.len()),
                format!("missing {}", self.facts[values.len()].id),
            ));
        }
        if values.len() > self.facts.len() {
            return Err(Error::new(
                "invalid_shape",
                "/facts",
                "too many supplied facts",
            ));
        }
        for (index, (fact, value)) in self.facts.iter().zip(values).enumerate() {
            if !matches!(
                (fact.kind, value),
                (FactType::String, FactValue::String(_))
                    | (FactType::Boolean, FactValue::Boolean(_))
                    | (_, FactValue::Unavailable(_))
            ) {
                return Err(Error::new(
                    "type_mismatch",
                    &format!("/facts/{index}"),
                    &fact.id,
                ));
            }
        }
        Ok(())
    }

    fn outcome(&self, rule: &Rule, values: &[FactValue<'_>]) -> Outcome {
        let (truth, missing) = rule.expression.evaluate(values);
        let mut unavailable: Vec<_> = missing
            .into_iter()
            .map(|index| {
                let FactValue::Unavailable(reason) = values[index] else {
                    unreachable!()
                };
                UnavailableFact {
                    fact_id: self.facts[index].id.clone(),
                    reason: reason.into(),
                }
            })
            .collect();
        unavailable.sort_by(|a, b| a.fact_id.cmp(&b.fact_id));
        Outcome {
            rule_id: rule.id.clone(),
            truth,
            unavailable,
        }
    }
}
impl Expression {
    fn evaluate(&self, values: &[FactValue<'_>]) -> (Truth, BTreeSet<usize>) {
        match self {
            Self::Compare(index, predicate) => {
                if matches!(values[*index], FactValue::Unavailable(_)) {
                    return (Truth::Unknown, BTreeSet::from([*index]));
                }
                let matched = match predicate {
                    Predicate::Eq(value) => value.matches(values[*index]),
                    Predicate::In(choices) => choices.iter().any(|x| x.matches(values[*index])),
                    Predicate::Suffix(suffix) => {
                        matches!(values[*index],FactValue::String(s) if s.ends_with(suffix))
                    }
                };
                (
                    if matched { Truth::True } else { Truth::False },
                    BTreeSet::new(),
                )
            }
            Self::Not(child) => {
                let (truth, missing) = child.evaluate(values);
                (
                    match truth {
                        Truth::True => Truth::False,
                        Truth::False => Truth::True,
                        Truth::Unknown => Truth::Unknown,
                    },
                    missing,
                )
            }
            Self::All(children) | Self::Any(children) => {
                let all = matches!(self, Self::All(_));
                let decisive = if all { Truth::False } else { Truth::True };
                let mut missing = BTreeSet::new();
                for child in children {
                    let (truth, unknown) = child.evaluate(values);
                    if truth == decisive {
                        return (decisive, BTreeSet::new());
                    }
                    missing.extend(unknown);
                }
                let truth = if !missing.is_empty() {
                    Truth::Unknown
                } else if all {
                    Truth::True
                } else {
                    Truth::False
                };
                (truth, missing)
            }
        }
    }
}
