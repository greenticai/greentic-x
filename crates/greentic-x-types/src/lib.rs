//! Shared type vocabulary for Greentic-X resource contracts and operations.
//!
//! The models in this crate are intentionally generic: they capture identifiers,
//! revisions, provenance, schema references, and mutation requests without
//! embedding any domain-specific runtime logic.
//!
//! ```rust
//! use greentic_x_types::{
//!     ActorRef, AppendRequest, ContractId, PatchOperation, Provenance, ResourceId, ResourcePatch,
//!     ResourceTypeId, Revision,
//! };
//! use serde_json::json;
//!
//! let request = ResourcePatch {
//!     contract_id: ContractId::new("gx.case").unwrap(),
//!     resource_type: ResourceTypeId::new("case").unwrap(),
//!     resource_id: ResourceId::new("case-42").unwrap(),
//!     base_revision: Revision::new(3),
//!     operations: vec![PatchOperation::replace("/title", json!("Investigate ingress alarm"))],
//!     provenance: Provenance::new(ActorRef::service("workflow-engine").unwrap()),
//! };
//!
//! let json = serde_json::to_string_pretty(&request).unwrap();
//! assert!(json.contains("\"resource_id\": \"case-42\""));
//! assert!(json.contains("\"op\": \"replace\""));
//!
//! let append = AppendRequest::new(
//!     ContractId::new("gx.case").unwrap(),
//!     ResourceTypeId::new("case").unwrap(),
//!     ResourceId::new("case-42").unwrap(),
//!     Revision::new(3),
//!     "evidence",
//!     json!({"kind": "log", "uri": "s3://bucket/evidence.json"}),
//!     Provenance::new(ActorRef::user("analyst-1").unwrap()),
//! )
//! .unwrap();
//! assert_eq!(append.collection, "evidence");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Errors returned when constructing validated identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierError {
    message: &'static str,
}

impl IdentifierError {
    fn empty() -> Self {
        Self {
            message: "identifier must not be empty",
        }
    }

    fn whitespace() -> Self {
        Self {
            message: "identifier must not contain whitespace",
        }
    }
}

impl core::fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for IdentifierError {}

fn validate_identifier(value: &str) -> Result<(), IdentifierError> {
    if value.is_empty() {
        return Err(IdentifierError::empty());
    }
    if value.chars().any(char::is_whitespace) {
        return Err(IdentifierError::whitespace());
    }
    Ok(())
}

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdentifierError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentifierError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(ActorId);
string_id!(ContractId);
string_id!(ContractVersion);
string_id!(OperationId);
string_id!(ResourceId);
string_id!(ResourceTypeId);

/// Monotonic revision used for optimistic concurrency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Party that initiated a request or produced an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    User,
    Service,
    System,
}

/// Minimal actor descriptor for audit and provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorRef {
    pub kind: ActorKind,
    pub actor_id: ActorId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl ActorRef {
    pub fn new(kind: ActorKind, actor_id: impl Into<String>) -> Result<Self, IdentifierError> {
        Ok(Self {
            kind,
            actor_id: ActorId::new(actor_id)?,
            display_name: None,
        })
    }

    pub fn user(actor_id: impl Into<String>) -> Result<Self, IdentifierError> {
        Self::new(ActorKind::User, actor_id)
    }

    pub fn service(actor_id: impl Into<String>) -> Result<Self, IdentifierError> {
        Self::new(ActorKind::Service, actor_id)
    }

    pub fn system(actor_id: impl Into<String>) -> Result<Self, IdentifierError> {
        Self::new(ActorKind::System, actor_id)
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }
}

/// Audit and routing context attached to requests and events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub actor: ActorRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl Provenance {
    pub fn new(actor: ActorRef) -> Self {
        Self {
            actor,
            trace_id: None,
            correlation_id: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

/// Reference to a schema or compatibility contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaReference {
    pub schema_id: String,
    pub version: ContractVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

impl SchemaReference {
    pub fn new(
        schema_id: impl Into<String>,
        version: ContractVersion,
    ) -> Result<Self, IdentifierError> {
        let schema_id = schema_id.into();
        validate_identifier(&schema_id)?;
        Ok(Self {
            schema_id,
            version,
            uri: None,
        })
    }

    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }
}

/// Declares whether compatibility is strict or can tolerate additive evolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityMode {
    Exact,
    BackwardCompatible,
    ForwardCompatible,
}

/// Contract or operation compatibility declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReference {
    pub schema: SchemaReference,
    pub mode: CompatibilityMode,
}

/// JSON Patch-like mutation operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchOperation {
    pub op: PatchOperationKind,
    pub path: String,
    pub value: Value,
}

impl PatchOperation {
    pub fn replace(path: impl Into<String>, value: Value) -> Self {
        Self {
            op: PatchOperationKind::Replace,
            path: path.into(),
            value,
        }
    }

    pub fn add(path: impl Into<String>, value: Value) -> Self {
        Self {
            op: PatchOperationKind::Add,
            path: path.into(),
            value,
        }
    }
}

/// Kind of patch operation applied to a resource document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchOperationKind {
    Add,
    Replace,
    Remove,
}

/// Patch request with optimistic concurrency and audit context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcePatch {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub base_revision: Revision,
    pub operations: Vec<PatchOperation>,
    pub provenance: Provenance,
}

/// Append-only collection mutation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppendRequest {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub base_revision: Revision,
    pub collection: String,
    pub value: Value,
    pub provenance: Provenance,
}

impl AppendRequest {
    pub fn new(
        contract_id: ContractId,
        resource_type: ResourceTypeId,
        resource_id: ResourceId,
        base_revision: Revision,
        collection: impl Into<String>,
        value: Value,
        provenance: Provenance,
    ) -> Result<Self, IdentifierError> {
        let collection = collection.into();
        validate_identifier(&collection)?;
        Ok(Self {
            contract_id,
            resource_type,
            resource_id,
            base_revision,
            collection,
            value,
            provenance,
        })
    }
}

/// State transition request for a resource lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionRequest {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub base_revision: Revision,
    pub target_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub provenance: Provenance,
}

impl TransitionRequest {
    pub fn new(
        contract_id: ContractId,
        resource_type: ResourceTypeId,
        resource_id: ResourceId,
        base_revision: Revision,
        target_state: impl Into<String>,
        provenance: Provenance,
    ) -> Result<Self, IdentifierError> {
        let target_state = target_state.into();
        validate_identifier(&target_state)?;
        Ok(Self {
            contract_id,
            resource_type,
            resource_id,
            base_revision,
            target_state,
            reason: None,
            provenance,
        })
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_whitespace_identifiers() {
        let err = ContractId::new("bad id").expect_err("whitespace identifiers must be rejected");
        assert_eq!(err.to_string(), "identifier must not contain whitespace");
    }

    #[test]
    fn serializes_patch_requests() {
        let request = ResourcePatch {
            contract_id: ContractId::new("gx.case").expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("case")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("case-123").expect("static resource id should be valid"),
            base_revision: Revision::new(7),
            operations: vec![
                PatchOperation::replace("/status", json!("investigating")),
                PatchOperation::add("/labels/-", json!("priority-high")),
            ],
            provenance: Provenance::new(
                ActorRef::service("router").expect("static actor id should be valid"),
            )
            .with_trace_id("trace-1")
            .with_correlation_id("corr-2"),
        };

        let json = serde_json::to_value(&request).expect("request must serialize");
        assert_eq!(json["base_revision"], 7);
        assert_eq!(json["operations"][0]["op"], "replace");
        assert_eq!(json["provenance"]["actor"]["kind"], "service");
    }

    #[test]
    fn serializes_append_and_transition_requests() {
        let append = AppendRequest::new(
            ContractId::new("gx.case").expect("static contract id should be valid"),
            ResourceTypeId::new("case").expect("static resource type should be valid"),
            ResourceId::new("case-123").expect("static resource id should be valid"),
            Revision::new(4),
            "evidence",
            json!({"kind": "snapshot"}),
            Provenance::new(ActorRef::user("analyst-1").expect("static actor id should be valid")),
        )
        .expect("append request should be valid");

        let transition = TransitionRequest::new(
            ContractId::new("gx.case").expect("static contract id should be valid"),
            ResourceTypeId::new("case").expect("static resource type should be valid"),
            ResourceId::new("case-123").expect("static resource id should be valid"),
            Revision::new(5),
            "resolved",
            Provenance::new(
                ActorRef::system("policy-engine").expect("static actor id should be valid"),
            ),
        )
        .expect("transition request should be valid")
        .with_reason("triage complete");

        let append_json = serde_json::to_string(&append).expect("append request must serialize");
        let transition_json =
            serde_json::to_string(&transition).expect("transition request must serialize");

        assert!(append_json.contains("\"collection\":\"evidence\""));
        assert!(transition_json.contains("\"target_state\":\"resolved\""));
        assert!(transition_json.contains("\"reason\":\"triage complete\""));
    }
}
