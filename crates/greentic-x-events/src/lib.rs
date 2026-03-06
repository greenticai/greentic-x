//! Structured event models for Greentic-X resource, contract, and operation lifecycles.
//!
//! ```rust
//! use greentic_x_events::{EventEnvelope, EventMetadata, ResourceCreated};
//! use greentic_x_types::{ActorRef, ContractId, Provenance, ResourceId, ResourceTypeId, Revision};
//! use serde_json::json;
//!
//! let event = EventEnvelope::resource_created(
//!     "evt-1",
//!     EventMetadata::new(Provenance::new(ActorRef::service("runtime").unwrap())),
//!     ResourceCreated {
//!         contract_id: ContractId::new("gx.case").unwrap(),
//!         resource_type: ResourceTypeId::new("case").unwrap(),
//!         resource_id: ResourceId::new("case-42").unwrap(),
//!         revision: Revision::new(1),
//!         document: json!({"title": "Investigate ingress alarm"}),
//!     },
//! );
//!
//! let json = serde_json::to_string_pretty(&event).unwrap();
//! assert!(json.contains("\"event_type\": \"resource_created\""));
//! assert!(json.contains("\"resource_id\": \"case-42\""));
//! ```

use greentic_x_types::{
    CompatibilityReference, ContractId, ContractVersion, OperationId, Provenance, ResourceId,
    ResourceTypeId, Revision,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Envelope common to all Greentic-X domain events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope<T> {
    pub event_id: String,
    pub event_type: EventType,
    pub metadata: EventMetadata,
    pub payload: T,
}

impl<T> EventEnvelope<T> {
    pub fn new(
        event_id: impl Into<String>,
        event_type: EventType,
        metadata: EventMetadata,
        payload: T,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_type,
            metadata,
            payload,
        }
    }
}

impl EventEnvelope<ResourceCreated> {
    pub fn resource_created(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: ResourceCreated,
    ) -> Self {
        Self::new(event_id, EventType::ResourceCreated, metadata, payload)
    }
}

impl EventEnvelope<ResourcePatched> {
    pub fn resource_patched(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: ResourcePatched,
    ) -> Self {
        Self::new(event_id, EventType::ResourcePatched, metadata, payload)
    }
}

impl EventEnvelope<ResourceAppended> {
    pub fn resource_appended(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: ResourceAppended,
    ) -> Self {
        Self::new(event_id, EventType::ResourceAppended, metadata, payload)
    }
}

impl EventEnvelope<ResourceTransitioned> {
    pub fn resource_transitioned(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: ResourceTransitioned,
    ) -> Self {
        Self::new(event_id, EventType::ResourceTransitioned, metadata, payload)
    }
}

impl EventEnvelope<OperationInstalled> {
    pub fn operation_installed(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: OperationInstalled,
    ) -> Self {
        Self::new(event_id, EventType::OperationInstalled, metadata, payload)
    }
}

impl EventEnvelope<OperationExecuted> {
    pub fn operation_executed(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: OperationExecuted,
    ) -> Self {
        Self::new(event_id, EventType::OperationExecuted, metadata, payload)
    }
}

impl EventEnvelope<ContractInstalled> {
    pub fn contract_installed(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: ContractInstalled,
    ) -> Self {
        Self::new(event_id, EventType::ContractInstalled, metadata, payload)
    }
}

impl EventEnvelope<ContractActivated> {
    pub fn contract_activated(
        event_id: impl Into<String>,
        metadata: EventMetadata,
        payload: ContractActivated,
    ) -> Self {
        Self::new(event_id, EventType::ContractActivated, metadata, payload)
    }
}

/// Canonical event kinds emitted by the future Greentic-X runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ResourceCreated,
    ResourcePatched,
    ResourceAppended,
    ResourceTransitioned,
    OperationInstalled,
    OperationExecuted,
    ContractInstalled,
    ContractActivated,
}

/// Event-level metadata used for audit, correlation, and routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMetadata {
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causation_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
}

impl EventMetadata {
    pub fn new(provenance: Provenance) -> Self {
        Self {
            provenance,
            causation_event_id: None,
            partition_key: None,
        }
    }

    pub fn with_causation_event_id(mut self, causation_event_id: impl Into<String>) -> Self {
        self.causation_event_id = Some(causation_event_id.into());
        self
    }

    pub fn with_partition_key(mut self, partition_key: impl Into<String>) -> Self {
        self.partition_key = Some(partition_key.into());
        self
    }
}

/// Payload for initial resource creation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceCreated {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub revision: Revision,
    pub document: Value,
}

/// Payload for a patch-based mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcePatched {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub from_revision: Revision,
    pub to_revision: Revision,
    pub applied_paths: Vec<String>,
}

/// Payload for an append-only mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceAppended {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub collection: String,
    pub revision: Revision,
    pub appended_value: Value,
}

/// Payload for a lifecycle transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceTransitioned {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub from_state: String,
    pub to_state: String,
    pub revision: Revision,
}

/// Payload for op registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationInstalled {
    pub operation_id: OperationId,
    pub version: ContractVersion,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub compatibility: Vec<CompatibilityReference>,
}

/// Payload for op execution results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationExecuted {
    pub operation_id: OperationId,
    pub invocation_id: String,
    pub status: OperationExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

/// Outcome status for operation execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationExecutionStatus {
    Succeeded,
    Failed,
}

/// Payload for contract installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInstalled {
    pub contract_id: ContractId,
    pub version: ContractVersion,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub compatibility: Vec<CompatibilityReference>,
}

/// Payload for contract activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractActivated {
    pub contract_id: ContractId,
    pub version: ContractVersion,
    pub activation_target: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use greentic_x_types::{ActorRef, CompatibilityMode, SchemaReference};
    use serde_json::json;

    #[test]
    fn serializes_resource_created_event() {
        let event = EventEnvelope::resource_created(
            "evt-1",
            EventMetadata::new(Provenance::new(
                ActorRef::service("runtime").expect("static actor id should be valid"),
            ))
            .with_causation_event_id("cmd-1")
            .with_partition_key("case-42"),
            ResourceCreated {
                contract_id: ContractId::new("gx.case")
                    .expect("static contract id should be valid"),
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: ResourceId::new("case-42")
                    .expect("static resource id should be valid"),
                revision: Revision::new(1),
                document: json!({"title": "Investigate ingress alarm"}),
            },
        );

        let value = serde_json::to_value(&event).expect("event must serialize");
        assert_eq!(value["event_type"], "resource_created");
        assert_eq!(value["payload"]["resource_id"], "case-42");
        assert_eq!(value["metadata"]["partition_key"], "case-42");
    }

    #[test]
    fn serializes_contract_and_operation_events() {
        let compatibility = vec![CompatibilityReference {
            schema: SchemaReference::new(
                "greentic-x://contracts/case",
                ContractVersion::new("v1").expect("static version should be valid"),
            )
            .expect("static schema ref should be valid"),
            mode: CompatibilityMode::BackwardCompatible,
        }];

        let contract_event = EventEnvelope::contract_installed(
            "evt-contract",
            EventMetadata::new(Provenance::new(
                ActorRef::system("registry").expect("static actor id should be valid"),
            )),
            ContractInstalled {
                contract_id: ContractId::new("gx.case")
                    .expect("static contract id should be valid"),
                version: ContractVersion::new("v1").expect("static version should be valid"),
                compatibility: compatibility.clone(),
            },
        );

        let op_event = EventEnvelope::operation_executed(
            "evt-op",
            EventMetadata::new(Provenance::new(
                ActorRef::service("runner").expect("static actor id should be valid"),
            )),
            OperationExecuted {
                operation_id: OperationId::new("approval-basic")
                    .expect("static operation id should be valid"),
                invocation_id: "invoke-1".to_owned(),
                status: OperationExecutionStatus::Succeeded,
                output: Some(json!({"approved": true})),
            },
        );

        let contract_json =
            serde_json::to_string(&contract_event).expect("contract event must serialize");
        let op_json = serde_json::to_string(&op_event).expect("operation event must serialize");

        assert!(contract_json.contains("\"event_type\":\"contract_installed\""));
        assert!(contract_json.contains("\"backward_compatible\""));
        assert!(op_json.contains("\"event_type\":\"operation_executed\""));
        assert!(op_json.contains("\"status\":\"succeeded\""));
    }
}
