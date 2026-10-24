//! Reject duplicate keys during decoding, before serde_json::Value can overwrite them.
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

pub(crate) struct Unique(pub Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON without duplicate keys")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
        Ok(Unique(Value::Bool(v)))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
        Ok(Unique(Value::Number(v.into())))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
        Ok(Unique(Value::Number(v.into())))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
        Number::from_f64(v)
            .map(|n| Unique(Value::Number(n)))
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
        Ok(Unique(Value::String(v.into())))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
        Ok(Unique(Value::String(v)))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
        Ok(Unique(Value::Null))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
        let mut items = Vec::new();
        while let Some(Unique(v)) = a.next_element()? {
            items.push(v);
        }
        Ok(Unique(Value::Array(items)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
        let mut map = Map::new();
        while let Some(key) = a.next_key::<String>()? {
            if map.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON key"));
            }
            let Unique(value) = a.next_value()?;
            map.insert(key, value);
        }
        Ok(Unique(Value::Object(map)))
    }
}

use crate::Error;

pub(crate) fn parse(bytes: &[u8], max_bytes: usize) -> Result<Value, Error> {
    if bytes.len() > max_bytes {
        return Err(error("limit_exceeded", "", "JSON exceeds byte budget"));
    }
    // A valid depth-8 expression fits below this defensive JSON parser ceiling.
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 32 {
                        return Err(error("limit_exceeded", "", "JSON nesting exceeds 32"));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let Unique(value) =
        serde_json::from_slice(bytes).map_err(|e| error("invalid_shape", "", e.to_string()))?;
    Ok(value)
}

fn error(code: &'static str, path: &str, message: impl Into<String>) -> Error {
    Error::new(code, path, message)
}
