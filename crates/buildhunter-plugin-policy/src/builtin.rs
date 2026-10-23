//! Public basic plugins, with versioned client profiles preserving C2 semantics.
//! Acquisition, environment admission and nested-root admission belong to the host.
use crate::{Origin, PluginCatalog, PluginInput};
use rifle_rules::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientProfile {
    CliV1,
    GuiV1,
}

impl ClientProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::CliV1 => "buildhunter.cli/1",
            Self::GuiV1 => "buildhunter.gui/1",
        }
    }

    pub fn documents(self) -> [&'static [u8]; 3] {
        match self {
            Self::CliV1 => [
                include_bytes!("../../../plugins/basic/cli-v1/swift.json"),
                include_bytes!("../../../plugins/basic/cli-v1/rust.json"),
                include_bytes!("../../../plugins/basic/cli-v1/python.json"),
            ],
            Self::GuiV1 => [
                include_bytes!("../../../plugins/basic/gui-v1/swift.json"),
                include_bytes!("../../../plugins/basic/gui-v1/rust.json"),
                include_bytes!("../../../plugins/basic/gui-v1/python.json"),
            ],
        }
    }

    /// Every document is validated before applying the immutable exclusion snapshot.
    pub fn load(self, excluded: &[&str]) -> Result<PluginCatalog, Error> {
        let documents = self.documents();
        let inputs = documents.map(|bytes| PluginInput {
            bytes,
            origin: Origin::Builtin,
        });
        PluginCatalog::load(&inputs, excluded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CandidateFacts, Decision, MarkerFact, NodeKind};
    use std::collections::BTreeMap;

    #[test]
    fn profiles_keep_the_thirteen_stable_ids_and_have_distinct_identities() {
        let cli = ClientProfile::CliV1.load(&[]).unwrap();
        let gui = ClientProfile::GuiV1.load(&[]).unwrap();
        assert_eq!(cli.targets().len(), 13);
        assert_eq!(gui.targets(), cli.targets());
        assert_ne!(cli.catalog_digest(), gui.catalog_digest());
        for target in cli.targets() {
            let disabled = ClientProfile::CliV1.load(&[&target.id]).unwrap();
            assert_eq!(disabled.catalog_digest(), cli.catalog_digest());
            assert!(disabled.exclusions().contains(&target.id));
        }
    }

    #[test]
    fn gui_environment_precedence_is_retained_even_when_the_environment_is_excluded() {
        for excluded in [&[][..], &["python.environment"][..]] {
            let catalog = ClientProfile::GuiV1.load(excluded).unwrap();
            let markers = catalog
                .marker_requests(NodeKind::Directory)
                .into_iter()
                .map(|m| (m.clone(), MarkerFact::Known(m.name == "pyvenv.cfg")))
                .collect::<BTreeMap<_, _>>();
            let result = catalog
                .classify(CandidateFacts {
                    name: Some(".build"),
                    kind: NodeKind::Directory,
                    markers: &markers,
                })
                .unwrap();
            assert_eq!(
                result.decision,
                if excluded.is_empty() {
                    Decision::Found("python.environment".into())
                } else {
                    Decision::NoMatch
                }
            );
        }
    }
}
