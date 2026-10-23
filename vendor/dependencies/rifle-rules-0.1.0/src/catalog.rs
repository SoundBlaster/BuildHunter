//! Generic program packages. Host-specific fields are opaque, but covered by identity.
use crate::{Error, JsonIdentity, Plan, Sha256Digest, validate};
use serde_json::{Value, json as value};
use std::collections::HashSet;

#[derive(Debug)]
pub struct ProgramPackage {
    id: String,
    version: String,
    digest: Sha256Digest,
    plan: Plan,
}
impl ProgramPackage {
    /// Requires id/version/program. Other fields are opaque to Rifle, not validated
    /// as a BuildHunter plugin. The digest covers the ENTIRE input document.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        let doc = crate::identity::parse(bytes)?;
        let fields = doc
            .as_object()
            .ok_or_else(|| Error::new("invalid_shape", "", "expected package object"))?;
        let id = validate::identifier(fields.get("id").unwrap_or(&Value::Null), "/id")?.to_owned();
        let version = fields
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::new("invalid_shape", "/version", "expected version string"))?;
        if version.len() > 32
            || version.split('.').count() != 3
            || version.split('.').any(|x| {
                x.is_empty()
                    || !x.bytes().all(|b| b.is_ascii_digit())
                    || (x.len() > 1 && x.starts_with('0'))
            })
        {
            return Err(Error::new(
                "invalid_shape",
                "/version",
                "expected major.minor.patch without leading zeroes",
            ));
        }
        let program = fields
            .get("program")
            .ok_or_else(|| Error::new("invalid_shape", "/program", "missing program"))?;
        let plan = validate::compile_value(program).map_err(|mut e| {
            e.location = format!("/program{}", e.location);
            e
        })?;
        let digest = *JsonIdentity::from_value(&doc)?.digest();
        Ok(Self {
            id,
            version: version.into(),
            digest,
            plan,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
}

#[derive(Debug)]
pub struct Catalog {
    packages: Vec<ProgramPackage>,
    digest: Sha256Digest,
}
impl Catalog {
    /// Atomic validation; no partially usable catalog escapes an error.
    pub fn from_json_documents(documents: &[&[u8]]) -> Result<Self, Error> {
        let bytes = documents
            .iter()
            .try_fold(0usize, |sum, doc| sum.checked_add(doc.len()));
        if documents.len() > 64 || bytes.is_none_or(|n| n > 16 * 1_048_576) {
            return Err(Error::new(
                "limit_exceeded",
                "/packages",
                "catalog byte/package budget exceeded",
            ));
        }
        let mut packages = Vec::new();
        let mut ids = HashSet::new();
        let mut rules = 0;
        let mut nodes = 0;
        for (i, doc) in documents.iter().enumerate() {
            let package = ProgramPackage::from_json(doc).map_err(|mut e| {
                e.location = format!("/packages/{i}{}", e.location);
                e
            })?;
            if !ids.insert(package.id.clone()) {
                return Err(Error::new(
                    "duplicate_id",
                    &format!("/packages/{i}/id"),
                    &package.id,
                ));
            }
            rules += package.plan.rule_count();
            nodes += package.plan.expression_nodes();
            if rules > 4096 || nodes > 32768 {
                return Err(Error::new(
                    "limit_exceeded",
                    "/packages",
                    "aggregate rule/node budget exceeded",
                ));
            }
            packages.push(package);
        }
        packages.sort_by(|a, b| a.id.cmp(&b.id));
        let descriptors: Vec<_> = packages
            .iter()
            .map(|p| value!({"id":p.id,"version":p.version,"digest":p.digest.to_string()}))
            .collect();
        let digest = *JsonIdentity::from_value(&Value::Array(descriptors))?.digest();
        Ok(Self { packages, digest })
    }
    pub fn packages(&self) -> &[ProgramPackage] {
        &self.packages
    }
    pub fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
}
