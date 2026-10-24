use crate::{Error, Expression, FORMAT, Fact, FactType, Plan, Predicate, Rule, Scalar};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

pub(crate) fn compile(bytes: &[u8]) -> Result<Plan, Error> {
    let value = crate::json::parse(bytes, 1_048_576)?;
    compile_value(&value)
}

pub(crate) fn compile_value(value: &Value) -> Result<Plan, Error> {
    let root = object(value, &["format", "facts", "rules"], "")?;
    if string(&root["format"], "/format")? != FORMAT {
        return Err(error(
            "unsupported_format",
            "/format",
            "unsupported program version",
        ));
    }
    let declarations = array(&root["facts"], 0, 128, "/facts")?;
    let mut facts = Vec::new();
    let mut indexes = HashMap::new();
    for (i, v) in declarations.iter().enumerate() {
        let path = format!("/facts/{i}");
        let o = object(v, &["id", "type"], &path)?;
        let id = identifier(&o["id"], &format!("{path}/id"))?.to_owned();
        let kind = match string(&o["type"], &format!("{path}/type"))? {
            "string" => FactType::String,
            "boolean" => FactType::Boolean,
            _ => {
                return Err(error(
                    "invalid_shape",
                    &format!("{path}/type"),
                    "unknown fact type",
                ));
            }
        };
        if indexes.insert(id.clone(), i).is_some() {
            return Err(error("duplicate_id", &format!("{path}/id"), id));
        }
        facts.push(Fact { id, kind });
    }
    let input_rules = array(&root["rules"], 1, 256, "/rules")?;
    let mut ids = HashSet::new();
    let mut rules = Vec::new();
    let mut used = HashSet::new();
    let mut expression_nodes = 0;
    for (i, v) in input_rules.iter().enumerate() {
        let path = format!("/rules/{i}");
        let o = object(v, &["id", "when"], &path)?;
        let id = identifier(&o["id"], &format!("{path}/id"))?.to_owned();
        if !ids.insert(id.clone()) {
            return Err(error("duplicate_id", &format!("{path}/id"), id));
        }
        let mut parser = Parser {
            facts: &facts,
            indexes: &indexes,
            used: &mut used,
            nodes: 0,
        };
        let expression = parser.expression(&o["when"], 1, &format!("{path}/when"))?;
        expression_nodes += parser.nodes;
        rules.push(Rule { id, expression });
    }
    for (i, fact) in facts.iter().enumerate() {
        if !used.contains(&i) {
            return Err(error("unused_fact", &format!("/facts/{i}"), &fact.id));
        }
    }
    Ok(Plan {
        facts,
        rules,
        expression_nodes,
    })
}
struct Parser<'a> {
    facts: &'a [Fact],
    indexes: &'a HashMap<String, usize>,
    used: &'a mut HashSet<usize>,
    nodes: usize,
}
impl Parser<'_> {
    fn expression(&mut self, v: &Value, depth: usize, path: &str) -> Result<Expression, Error> {
        self.nodes += 1;
        if depth > 8 || self.nodes > 128 {
            return Err(error(
                "limit_exceeded",
                path,
                "expression depth/nodes exceeded",
            ));
        }
        let o = v
            .as_object()
            .ok_or_else(|| error("invalid_shape", path, "expected condition object"))?;
        for group in ["all", "any", "not"] {
            if o.contains_key(group) {
                object(v, &[group], path)?;
                if group == "not" {
                    return Ok(Expression::Not(Box::new(self.expression(
                        &o[group],
                        depth + 1,
                        &format!("{path}/not"),
                    )?)));
                }
                let children = array(&o[group], 1, 32, &format!("{path}/{group}"))?;
                let expressions = children
                    .iter()
                    .enumerate()
                    .map(|(i, v)| self.expression(v, depth + 1, &format!("{path}/{group}/{i}")))
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(if group == "all" {
                    Expression::All(expressions)
                } else {
                    Expression::Any(expressions)
                });
            }
        }
        let op = o
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| error("invalid_shape", path, "missing operator"))?;
        let field = match op {
            "eq" | "suffix" => "value",
            "in" => "values",
            _ => return Err(error("invalid_shape", path, "unsupported operator")),
        };
        object(v, &["fact", "op", field], path)?;
        let fact = identifier(&o["fact"], &format!("{path}/fact"))?;
        let index = *self
            .indexes
            .get(fact)
            .ok_or_else(|| error("unknown_fact", &format!("{path}/fact"), fact))?;
        self.used.insert(index);
        let kind = self.facts[index].kind;
        let predicate = match op {
            "eq" => Predicate::Eq(scalar(&o[field], kind, &format!("{path}/{field}"))?),
            "in" => {
                let items = array(&o[field], 1, 32, &format!("{path}/{field}"))?;
                let mut choices = Vec::new();
                for (i, v) in items.iter().enumerate() {
                    let p = format!("{path}/{field}/{i}");
                    let choice = scalar(v, kind, &p)?;
                    if choices.contains(&choice) {
                        return Err(error("invalid_shape", &p, "duplicate literal"));
                    }
                    choices.push(choice);
                }
                Predicate::In(choices)
            }
            "suffix" => {
                if kind != FactType::String {
                    return Err(error("type_mismatch", path, "suffix requires string fact"));
                }
                let Scalar::String(s) = scalar(&o[field], kind, &format!("{path}/{field}"))? else {
                    unreachable!()
                };
                if s.is_empty() {
                    return Err(error("invalid_shape", path, "empty suffix"));
                }
                Predicate::Suffix(s)
            }
            _ => unreachable!(),
        };
        Ok(Expression::Compare(index, predicate))
    }
}
fn error(code: &'static str, path: &str, message: impl Into<String>) -> Error {
    Error::new(code, path, message)
}
fn object<'a>(v: &'a Value, fields: &[&str], path: &str) -> Result<&'a Map<String, Value>, Error> {
    let o = v
        .as_object()
        .ok_or_else(|| error("invalid_shape", path, "expected object"))?;
    if o.len() != fields.len() || fields.iter().any(|f| !o.contains_key(*f)) {
        return Err(error("invalid_shape", path, "unexpected or missing fields"));
    }
    Ok(o)
}
fn array<'a>(v: &'a Value, min: usize, max: usize, path: &str) -> Result<&'a [Value], Error> {
    let a = v
        .as_array()
        .ok_or_else(|| error("invalid_shape", path, "expected array"))?;
    if a.len() > max {
        return Err(error("limit_exceeded", path, "too many items"));
    }
    if a.len() < min {
        return Err(error("invalid_shape", path, "empty required array"));
    }
    Ok(a)
}
fn string<'a>(v: &'a Value, path: &str) -> Result<&'a str, Error> {
    v.as_str()
        .ok_or_else(|| error("invalid_shape", path, "expected string"))
}
pub(crate) fn identifier<'a>(v: &'a Value, path: &str) -> Result<&'a str, Error> {
    let s = string(v, path)?;
    if s.len() > 128
        || !s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || s.split(['.', '-']).any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
        return Err(error("invalid_shape", path, "invalid identifier"));
    }
    Ok(s)
}
fn scalar(v: &Value, kind: FactType, path: &str) -> Result<Scalar, Error> {
    match (v, kind) {
        (Value::String(s), FactType::String) if s.chars().count() <= 256 => {
            Ok(Scalar::String(s.clone()))
        }
        (Value::String(_), FactType::String) => Err(error(
            "limit_exceeded",
            path,
            "literal exceeds 256 characters",
        )),
        (Value::Bool(b), FactType::Boolean) => Ok(Scalar::Boolean(*b)),
        _ => Err(error(
            "type_mismatch",
            path,
            "literal does not match fact type",
        )),
    }
}
