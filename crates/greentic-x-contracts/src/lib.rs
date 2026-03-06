//! Contract descriptors and structural validation helpers for Greentic-X.
//!
//! ```rust
//! use greentic_x_contracts::{
//!     ContractManifest, EventDeclaration, MutationRule, ResourceDefinition, TransitionDefinition,
//!     ValidationIssue,
//! };
//! use greentic_x_types::{CompatibilityMode, CompatibilityReference, ContractId, ContractVersion, SchemaReference};
//!
//! let manifest = ContractManifest {
//!     contract_id: ContractId::new("gx.case").expect("static contract id should be valid"),
//!     version: ContractVersion::new("v1").expect("static version should be valid"),
//!     description: "Shared operational case contract".to_owned(),
//!     resources: vec![ResourceDefinition {
//!         resource_type: "case".to_owned(),
//!         schema: SchemaReference::new(
//!             "greentic-x://contracts/case/resources/case",
//!             ContractVersion::new("v1").expect("static version should be valid"),
//!         )
//!         .expect("static schema should be valid"),
//!         patch_rules: vec![MutationRule::allow("/title"), MutationRule::allow("/severity")],
//!         append_collections: vec![],
//!         transitions: vec![TransitionDefinition::new("triaged", "resolved")],
//!     }],
//!     compatibility: vec![CompatibilityReference {
//!         schema: SchemaReference::new(
//!             "greentic-x://contracts/case/compatibility",
//!             ContractVersion::new("v1").expect("static version should be valid"),
//!         )
//!         .expect("static schema should be valid"),
//!         mode: CompatibilityMode::BackwardCompatible,
//!     }],
//!     event_declarations: vec![EventDeclaration::resource_created()],
//!     policy_hook: None,
//!     migration_from: Vec::new(),
//! };
//!
//! let issues = manifest.validate();
//! assert!(issues.is_empty(), "unexpected validation issues: {issues:?}");
//! ```

use greentic_x_events::EventType;
use greentic_x_types::{CompatibilityReference, ContractId, ContractVersion, SchemaReference};
use serde::{Deserialize, Serialize};

/// Top-level descriptor for a contract package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractManifest {
    pub contract_id: ContractId,
    pub version: ContractVersion,
    pub description: String,
    pub resources: Vec<ResourceDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub compatibility: Vec<CompatibilityReference>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub event_declarations: Vec<EventDeclaration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_hook: Option<PolicyHookReference>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub migration_from: Vec<MigrationReference>,
}

impl ContractManifest {
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if self.description.trim().is_empty() {
            issues.push(ValidationIssue::new(
                "description",
                "contract description must not be empty",
            ));
        }

        if self.resources.is_empty() {
            issues.push(ValidationIssue::new(
                "resources",
                "contract must declare at least one resource",
            ));
        }

        for (index, resource) in self.resources.iter().enumerate() {
            resource.validate(index, &mut issues);
        }

        issues
    }
}

/// Definition of a resource managed by a contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub resource_type: String,
    pub schema: SchemaReference,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub patch_rules: Vec<MutationRule>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub append_collections: Vec<AppendCollectionDefinition>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub transitions: Vec<TransitionDefinition>,
}

impl ResourceDefinition {
    fn validate(&self, index: usize, issues: &mut Vec<ValidationIssue>) {
        let prefix = format!("resources[{index}]");

        if self.resource_type.trim().is_empty() {
            issues.push(ValidationIssue::new(
                format!("{prefix}.resource_type"),
                "resource_type must not be empty",
            ));
        }

        for (rule_index, rule) in self.patch_rules.iter().enumerate() {
            if rule.path.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{prefix}.patch_rules[{rule_index}].path"),
                    "patch rule path must not be empty",
                ));
            }
        }

        for (collection_index, collection) in self.append_collections.iter().enumerate() {
            if collection.name.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{prefix}.append_collections[{collection_index}].name"),
                    "append collection name must not be empty",
                ));
            }
        }

        for (transition_index, transition) in self.transitions.iter().enumerate() {
            if transition.from_state.trim().is_empty() || transition.to_state.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{prefix}.transitions[{transition_index}]"),
                    "transition states must not be empty",
                ));
            }
        }
    }
}

/// Patchable field declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationRule {
    pub path: String,
    #[serde(rename = "kind")]
    pub rule_kind: MutationRuleKind,
}

impl MutationRule {
    pub fn allow(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            rule_kind: MutationRuleKind::Allow,
        }
    }
}

/// Whether a path is allowed or denied for patch operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationRuleKind {
    Allow,
    Deny,
}

/// Append-only collection declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendCollectionDefinition {
    pub name: String,
    pub item_schema: SchemaReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AppendCollectionDefinition {
    pub fn new(name: impl Into<String>, item_schema: SchemaReference) -> Self {
        Self {
            name: name.into(),
            item_schema,
            description: None,
        }
    }
}

/// Resource transition declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDefinition {
    pub from_state: String,
    pub to_state: String,
}

impl TransitionDefinition {
    pub fn new(from_state: impl Into<String>, to_state: impl Into<String>) -> Self {
        Self {
            from_state: from_state.into(),
            to_state: to_state.into(),
        }
    }
}

/// Event declaration exposed by the contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventDeclaration {
    pub event_type: EventType,
}

impl EventDeclaration {
    pub fn resource_created() -> Self {
        Self {
            event_type: EventType::ResourceCreated,
        }
    }
}

/// Optional policy integration hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyHookReference {
    pub hook_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Compatibility or migration source reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReference {
    pub from_version: ContractVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Validation problem found in a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub location: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn read_contract_manifest(path: &str) -> ContractManifest {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        let data = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|_| panic!("failed to read {}", manifest_path.display()));
        serde_json::from_str(&data)
            .unwrap_or_else(|_| panic!("failed to parse {}", manifest_path.display()))
    }

    #[test]
    fn validates_reference_contract_manifests() {
        let manifests = [
            "contracts/case/contract.json",
            "contracts/evidence/contract.json",
            "contracts/outcome/contract.json",
            "contracts/playbook/contract.json",
        ];

        for path in manifests {
            let manifest = read_contract_manifest(path);
            let issues = manifest.validate();
            assert!(issues.is_empty(), "validation issues in {path}: {issues:?}");
        }
    }

    #[test]
    fn reference_contract_payloads_round_trip() {
        let manifest = read_contract_manifest("contracts/case/contract.json");
        let json = serde_json::to_value(&manifest).expect("contract manifest must serialize");
        assert_eq!(json["contract_id"], Value::String("gx.case".to_owned()));
        assert_eq!(
            json["resources"][0]["resource_type"],
            Value::String("case".to_owned())
        );
    }

    #[test]
    fn detects_missing_resource_definitions() {
        let manifest = ContractManifest {
            contract_id: ContractId::new("gx.invalid").expect("static contract id should be valid"),
            version: ContractVersion::new("v1").expect("static version should be valid"),
            description: String::new(),
            resources: Vec::new(),
            compatibility: Vec::new(),
            event_declarations: Vec::new(),
            policy_hook: None,
            migration_from: Vec::new(),
        };

        let issues = manifest.validate();
        assert!(issues.iter().any(|issue| issue.location == "description"));
        assert!(issues.iter().any(|issue| issue.location == "resources"));
    }
}
