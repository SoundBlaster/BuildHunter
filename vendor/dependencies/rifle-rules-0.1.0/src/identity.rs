//! Bounded strict JSON admission, JCS canonicalization and SHA-256 content identity.
use crate::{Error, json};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256Digest([u8; 32]);
impl Sha256Digest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }
}
impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct JsonIdentity {
    canonical: Vec<u8>,
    digest: Sha256Digest,
}
impl JsonIdentity {
    /// Identity of arbitrary JSON in the bounded Rifle JCS input profile.
    /// This validates JSON admission, not a host's plugin format or permissions.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_value(&parse(bytes)?)
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
    pub(crate) fn from_value(value: &Value) -> Result<Self, Error> {
        admit_numbers(value, "")?;
        let canonical = serde_json_canonicalizer::to_vec(value)
            .map_err(|e| Error::new("invalid_shape", "", e.to_string()))?;
        let digest = Sha256Digest::of(&canonical);
        Ok(Self { canonical, digest })
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Value, Error> {
    // Syntax, byte/depth limits and duplicate keys are checked first. A plain
    // integer beyond u64 can already have become f64 in Value: inspect tokens
    // before relying on Value's numeric representation for identity admission.
    let value = json::parse(bytes, 1_048_576)?;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() {
                    match bytes[index] {
                        b'\\' => index += 2,
                        b'"' => {
                            index += 1;
                            break;
                        }
                        _ => index += 1,
                    }
                }
            }
            b'-' | b'0'..=b'9' => {
                let start = index;
                while index < bytes.len()
                    && matches!(bytes[index], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
                {
                    index += 1;
                }
                let token = &bytes[start..index];
                if !token.iter().any(|b| matches!(b, b'.' | b'e' | b'E')) {
                    let digits = token.strip_prefix(b"-").unwrap_or(token);
                    let limit = b"9007199254740991";
                    if digits.len() > limit.len()
                        || (digits.len() == limit.len() && digits > limit.as_slice())
                    {
                        return Err(Error::new(
                            "unsupported_number",
                            "",
                            "integer token outside binary64 safe range; use a string",
                        ));
                    }
                }
            }
            _ => index += 1,
        }
    }
    Ok(value)
}

fn admit_numbers(value: &Value, location: &str) -> Result<(), Error> {
    match value {
        Value::Number(n) => {
            let unsafe_integer = n.as_u64().is_some_and(|x| x > 9_007_199_254_740_991)
                || n.as_i64().is_some_and(|x| x < -9_007_199_254_740_991);
            if unsafe_integer {
                return Err(Error::new(
                    "unsupported_number",
                    location,
                    "integer tokens beyond the binary64 safe range must be strings",
                ));
            }
        }
        Value::Array(values) => {
            for (i, value) in values.iter().enumerate() {
                admit_numbers(value, &format!("{location}/{i}"))?;
            }
        }
        Value::Object(fields) => {
            for (key, value) in fields {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                admit_numbers(value, &format!("{location}/{escaped}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}
