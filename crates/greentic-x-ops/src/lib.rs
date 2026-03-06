//! Operation descriptors and validation helpers for Greentic-X.
//!
//! ```rust
//! use greentic_x_ops::{OperationExample, OperationManifest, PermissionRequirement, SupportedContract};
//! use greentic_x_types::{CompatibilityReference, ContractId, ContractVersion, OperationId, SchemaReference};
//! use serde_json::json;
//!
//! let manifest = OperationManifest {
//!     operation_id: OperationId::new("approval-basic").expect("static operation id should be valid"),
//!     version: ContractVersion::new("v1").expect("static version should be valid"),
//!     description: "Shape a simple approval decision".to_owned(),
//!     input_schema: SchemaReference::new(
//!         "greentic-x://ops/approval-basic/input",
//!         ContractVersion::new("v1").expect("static version should be valid"),
//!     )
//!     .expect("static schema should be valid"),
//!     output_schema: SchemaReference::new(
//!         "greentic-x://ops/approval-basic/output",
//!         ContractVersion::new("v1").expect("static version should be valid"),
//!     )
//!     .expect("static schema should be valid"),
//!     compatibility: Vec::<CompatibilityReference>::new(),
//!     supported_contracts: vec![SupportedContract {
//!         contract_id: ContractId::new("gx.outcome").expect("static contract id should be valid"),
//!         version: ContractVersion::new("v1").expect("static version should be valid"),
//!     }],
//!     permissions: vec![PermissionRequirement::new("decision:write", "outcome")],
//!     examples: vec![OperationExample::new(
//!         "approves default case",
//!         json!({"risk_score": 0.2}),
//!         json!({"approved": true}),
//!     )],
//! };
//!
//! let issues = manifest.validate();
//! assert!(issues.is_empty(), "unexpected validation issues: {issues:?}");
//! ```

use greentic_x_types::{
    CompatibilityReference, ContractId, ContractVersion, OperationId, SchemaReference,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Operation descriptor used for registration and compatibility checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationManifest {
    pub operation_id: OperationId,
    pub version: ContractVersion,
    pub description: String,
    pub input_schema: SchemaReference,
    pub output_schema: SchemaReference,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub compatibility: Vec<CompatibilityReference>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub supported_contracts: Vec<SupportedContract>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub permissions: Vec<PermissionRequirement>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub examples: Vec<OperationExample>,
}

impl OperationManifest {
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        if self.description.trim().is_empty() {
            issues.push(ValidationIssue::new(
                "description",
                "operation description must not be empty",
            ));
        }

        for (index, permission) in self.permissions.iter().enumerate() {
            if permission.capability.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("permissions[{index}].capability"),
                    "permission capability must not be empty",
                ));
            }
            if permission.scope.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("permissions[{index}].scope"),
                    "permission scope must not be empty",
                ));
            }
        }

        for (index, example) in self.examples.iter().enumerate() {
            if example.name.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("examples[{index}].name"),
                    "example name must not be empty",
                ));
            }
        }

        issues
    }
}

/// Declares that an operation supports a contract/version pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportedContract {
    pub contract_id: ContractId,
    pub version: ContractVersion,
}

/// Describes permissions or capabilities required by an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRequirement {
    pub capability: String,
    pub scope: String,
}

impl PermissionRequirement {
    pub fn new(capability: impl Into<String>, scope: impl Into<String>) -> Self {
        Self {
            capability: capability.into(),
            scope: scope.into(),
        }
    }
}

/// Example invocation payloads for an operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationExample {
    pub name: String,
    pub input: Value,
    pub output: Value,
}

impl OperationExample {
    pub fn new(name: impl Into<String>, input: Value, output: Value) -> Self {
        Self {
            name: name.into(),
            input,
            output,
        }
    }
}

/// Validation problem found in an operation manifest.
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

    fn read_operation_manifest(path: &str) -> OperationManifest {
        let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        let data = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|_| panic!("failed to read {}", manifest_path.display()));
        serde_json::from_str(&data)
            .unwrap_or_else(|_| panic!("failed to parse {}", manifest_path.display()))
    }

    #[test]
    fn validates_reference_ops() {
        let manifests = [
            "ops/approval-basic/op.json",
            "ops/playbook-select/op.json",
            "ops/rca-basic/op.json",
        ];

        for path in manifests {
            let manifest = read_operation_manifest(path);
            let issues = manifest.validate();
            assert!(issues.is_empty(), "validation issues in {path}: {issues:?}");
        }
    }

    #[test]
    fn round_trips_reference_op_manifest() {
        let manifest = read_operation_manifest("ops/approval-basic/op.json");
        let json = serde_json::to_value(&manifest).expect("operation manifest must serialize");
        assert_eq!(
            json["operation_id"],
            Value::String("approval-basic".to_owned())
        );
        assert_eq!(
            json["supported_contracts"][0]["contract_id"],
            Value::String("gx.outcome".to_owned())
        );
    }

    #[test]
    fn detects_invalid_permission_entries() {
        let manifest = OperationManifest {
            operation_id: OperationId::new("invalid-op")
                .expect("static operation id should be valid"),
            version: ContractVersion::new("v1").expect("static version should be valid"),
            description: String::new(),
            input_schema: SchemaReference::new(
                "greentic-x://ops/invalid/input",
                ContractVersion::new("v1").expect("static version should be valid"),
            )
            .expect("static schema should be valid"),
            output_schema: SchemaReference::new(
                "greentic-x://ops/invalid/output",
                ContractVersion::new("v1").expect("static version should be valid"),
            )
            .expect("static schema should be valid"),
            compatibility: Vec::new(),
            supported_contracts: Vec::new(),
            permissions: vec![PermissionRequirement::new("", "")],
            examples: vec![OperationExample::new("", Value::Null, Value::Null)],
        };

        let issues = manifest.validate();
        assert!(issues.iter().any(|issue| issue.location == "description"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.location == "permissions[0].capability")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.location == "examples[0].name")
        );
    }
}
