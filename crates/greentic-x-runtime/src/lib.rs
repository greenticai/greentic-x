//! Runtime core for Greentic-X resource and operation lifecycles.
//!
//! ```rust
//! use greentic_x_contracts::{ContractManifest, EventDeclaration, MutationRule, ResourceDefinition};
//! use greentic_x_ops::OperationManifest;
//! use greentic_x_runtime::{
//!     CreateResourceRequest, InMemoryResourceStore, NoopEventSink, Runtime, StaticOperationHandler,
//! };
//! use greentic_x_types::{ActorRef, ContractId, ContractVersion, Provenance, ResourceId, ResourceTypeId, Revision, SchemaReference};
//! use serde_json::json;
//! use std::sync::Arc;
//!
//! let mut runtime = Runtime::new(InMemoryResourceStore::default(), NoopEventSink::default());
//! let manifest = ContractManifest {
//!     contract_id: ContractId::new("gx.case").expect("static contract id should be valid"),
//!     version: ContractVersion::new("v1").expect("static version should be valid"),
//!     description: "Shared case contract".to_owned(),
//!     resources: vec![ResourceDefinition {
//!         resource_type: "case".to_owned(),
//!         schema: SchemaReference::new(
//!             "greentic-x://contracts/case/resources/case",
//!             ContractVersion::new("v1").expect("static version should be valid"),
//!         )
//!         .expect("static schema should be valid"),
//!         patch_rules: vec![MutationRule::allow("/title")],
//!         append_collections: vec![],
//!         transitions: vec![],
//!     }],
//!     compatibility: Vec::new(),
//!     event_declarations: vec![EventDeclaration::resource_created()],
//!     policy_hook: None,
//!     migration_from: Vec::new(),
//! };
//!
//! let provenance = Provenance::new(ActorRef::service("runtime").expect("static actor id should be valid"));
//! runtime
//!     .install_contract(manifest.clone(), provenance.clone())
//!     .expect("contract install should succeed");
//! runtime
//!     .activate_contract(&manifest.contract_id, &manifest.version, provenance.clone())
//!     .expect("contract activation should succeed");
//! runtime
//!     .create_resource(CreateResourceRequest {
//!         contract_id: manifest.contract_id.clone(),
//!         resource_type: ResourceTypeId::new("case").expect("static resource type should be valid"),
//!         resource_id: ResourceId::new("case-1").expect("static resource id should be valid"),
//!         document: json!({"title": "Investigate ingress", "state": "new"}),
//!         provenance: provenance.clone(),
//!     })
//!     .expect("resource creation should succeed");
//!
//! runtime
//!     .install_operation(
//!         OperationManifest {
//!             operation_id: greentic_x_types::OperationId::new("approval-basic")
//!                 .expect("static operation id should be valid"),
//!             version: ContractVersion::new("v1").expect("static version should be valid"),
//!             description: "Simple approval op".to_owned(),
//!             input_schema: SchemaReference::new(
//!                 "greentic-x://ops/approval-basic/input",
//!                 ContractVersion::new("v1").expect("static version should be valid"),
//!             )
//!             .expect("static schema should be valid"),
//!             output_schema: SchemaReference::new(
//!                 "greentic-x://ops/approval-basic/output",
//!                 ContractVersion::new("v1").expect("static version should be valid"),
//!             )
//!             .expect("static schema should be valid"),
//!             compatibility: Vec::new(),
//!             supported_contracts: Vec::new(),
//!             permissions: Vec::new(),
//!             examples: Vec::new(),
//!         },
//!         Arc::new(StaticOperationHandler::new(Ok(json!({"approved": true})))),
//!         provenance.clone(),
//!     )
//!     .expect("op registration should succeed");
//!
//! let output = runtime
//!     .invoke_operation(
//!         &greentic_x_types::OperationId::new("approval-basic")
//!             .expect("static operation id should be valid"),
//!         "invoke-1",
//!         json!({"case_id": "case-1"}),
//!         provenance,
//!     )
//!     .expect("op invocation should succeed");
//! assert_eq!(output["approved"], true);
//! ```

use greentic_x_contracts::{
    AppendCollectionDefinition, ContractManifest, MutationRuleKind, ResourceDefinition,
    ValidationIssue,
};
use greentic_x_events::{
    ContractActivated, ContractInstalled, EventEnvelope, EventMetadata, OperationExecuted,
    OperationExecutionStatus, OperationInstalled, ResolverExecuted, ResolverExecutionStatus,
    ResolverInstalled, ResourceAppended, ResourceCreated, ResourceLinked, ResourcePatched,
    ResourceTransitioned,
};
use greentic_x_ops::{OperationManifest, SupportedContract};
use greentic_x_types::{
    AppendRequest, ContractId, ContractVersion, InvocationStatus, OperationCallEnvelope,
    OperationId, OperationResultEnvelope, PatchOperation, PatchOperationKind, Provenance,
    ResolverDescriptor, ResolverQueryEnvelope, ResolverResultEnvelope, ResolverStatus, ResourceId,
    ResourceLink, ResourcePatch, ResourceRef, ResourceTypeId, Revision, SchemaReference,
    TransitionRequest,
};
use jsonschema::Validator;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

/// Request for initial resource creation.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateResourceRequest {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub document: Value,
    pub provenance: Provenance,
}

/// Stored resource state.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRecord {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
    pub revision: Revision,
    pub document: Value,
}

/// Storage key for a resource instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceKey {
    pub contract_id: ContractId,
    pub resource_type: ResourceTypeId,
    pub resource_id: ResourceId,
}

impl ResourceKey {
    fn new(
        contract_id: ContractId,
        resource_type: ResourceTypeId,
        resource_id: ResourceId,
    ) -> Self {
        Self {
            contract_id,
            resource_type,
            resource_id,
        }
    }
}

impl From<&ResourceRecord> for ResourceKey {
    fn from(record: &ResourceRecord) -> Self {
        Self::new(
            record.contract_id.clone(),
            record.resource_type.clone(),
            record.resource_id.clone(),
        )
    }
}

/// Runtime errors for registry, validation, storage, and invocation failures.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    ContractValidation {
        issues: Vec<ValidationIssue>,
    },
    ContractAlreadyInstalled {
        contract_id: ContractId,
        version: ContractVersion,
    },
    ContractNotInstalled {
        contract_id: ContractId,
        version: ContractVersion,
    },
    ContractNotActive {
        contract_id: ContractId,
    },
    ResourceDefinitionNotFound {
        contract_id: ContractId,
        resource_type: ResourceTypeId,
    },
    ResourceAlreadyExists {
        resource_id: ResourceId,
    },
    ResourceNotFound {
        resource_id: ResourceId,
    },
    ResolverAlreadyInstalled {
        resolver_id: greentic_x_types::ResolverId,
    },
    ResolverNotFound {
        resolver_id: greentic_x_types::ResolverId,
    },
    ResolverInvocationFailed {
        resolver_id: greentic_x_types::ResolverId,
        message: String,
    },
    InvalidDocument(&'static str),
    PatchDenied {
        path: String,
    },
    PatchPathInvalid {
        path: String,
    },
    AppendCollectionNotAllowed {
        collection: String,
    },
    TransitionDenied {
        from_state: String,
        to_state: String,
    },
    RevisionConflict {
        expected: Revision,
        actual: Revision,
    },
    OperationAlreadyInstalled {
        operation_id: OperationId,
    },
    OperationValidation {
        issues: Vec<greentic_x_ops::ValidationIssue>,
    },
    OperationCompatibilityMissingContract {
        contract_id: ContractId,
        version: ContractVersion,
    },
    OperationNotFound {
        operation_id: OperationId,
    },
    OperationInvocationFailed {
        operation_id: OperationId,
        message: String,
    },
    SchemaNotRegistered {
        schema_id: String,
    },
    SchemaCompilationFailed {
        schema_id: String,
        message: String,
    },
    SchemaValidationFailed {
        schema_id: String,
        message: String,
    },
    EventSink(String),
    Storage(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractValidation { issues } => {
                write!(
                    formatter,
                    "contract validation failed: {} issue(s)",
                    issues.len()
                )
            }
            Self::ContractAlreadyInstalled {
                contract_id,
                version,
            } => {
                write!(
                    formatter,
                    "contract {contract_id}@{version} is already installed"
                )
            }
            Self::ContractNotInstalled {
                contract_id,
                version,
            } => {
                write!(
                    formatter,
                    "contract {contract_id}@{version} is not installed"
                )
            }
            Self::ContractNotActive { contract_id } => {
                write!(formatter, "contract {contract_id} is not active")
            }
            Self::ResourceDefinitionNotFound {
                contract_id,
                resource_type,
            } => {
                write!(
                    formatter,
                    "resource definition {resource_type} was not found in active contract {contract_id}"
                )
            }
            Self::ResourceAlreadyExists { resource_id } => {
                write!(formatter, "resource {resource_id} already exists")
            }
            Self::ResourceNotFound { resource_id } => {
                write!(formatter, "resource {resource_id} was not found")
            }
            Self::ResolverAlreadyInstalled { resolver_id } => {
                write!(formatter, "resolver {resolver_id} is already installed")
            }
            Self::ResolverNotFound { resolver_id } => {
                write!(formatter, "resolver {resolver_id} was not found")
            }
            Self::ResolverInvocationFailed {
                resolver_id,
                message,
            } => {
                write!(formatter, "resolver {resolver_id} failed: {message}")
            }
            Self::InvalidDocument(message) => formatter.write_str(message),
            Self::PatchDenied { path } => write!(formatter, "patch path {path} is not allowed"),
            Self::PatchPathInvalid { path } => write!(formatter, "patch path {path} is invalid"),
            Self::AppendCollectionNotAllowed { collection } => {
                write!(formatter, "append collection {collection} is not allowed")
            }
            Self::TransitionDenied {
                from_state,
                to_state,
            } => {
                write!(
                    formatter,
                    "transition {from_state} -> {to_state} is not allowed"
                )
            }
            Self::RevisionConflict { expected, actual } => {
                write!(
                    formatter,
                    "revision conflict: expected {}, got {}",
                    expected.value(),
                    actual.value()
                )
            }
            Self::OperationAlreadyInstalled { operation_id } => {
                write!(formatter, "operation {operation_id} is already installed")
            }
            Self::OperationValidation { issues } => {
                write!(
                    formatter,
                    "operation validation failed: {} issue(s)",
                    issues.len()
                )
            }
            Self::OperationCompatibilityMissingContract {
                contract_id,
                version,
            } => {
                write!(
                    formatter,
                    "operation requires missing contract {contract_id}@{version}"
                )
            }
            Self::OperationNotFound { operation_id } => {
                write!(formatter, "operation {operation_id} was not found")
            }
            Self::OperationInvocationFailed {
                operation_id,
                message,
            } => {
                write!(formatter, "operation {operation_id} failed: {message}")
            }
            Self::SchemaNotRegistered { schema_id } => {
                write!(formatter, "schema {schema_id} is not registered")
            }
            Self::SchemaCompilationFailed { schema_id, message } => {
                write!(
                    formatter,
                    "schema {schema_id} could not be compiled: {message}"
                )
            }
            Self::SchemaValidationFailed { schema_id, message } => {
                write!(formatter, "schema {schema_id} validation failed: {message}")
            }
            Self::EventSink(message) | Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Event stream surface exposed by the runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    ContractInstalled(EventEnvelope<ContractInstalled>),
    ContractActivated(EventEnvelope<ContractActivated>),
    ResourceCreated(EventEnvelope<ResourceCreated>),
    ResourcePatched(EventEnvelope<ResourcePatched>),
    ResourceAppended(EventEnvelope<ResourceAppended>),
    ResourceTransitioned(EventEnvelope<ResourceTransitioned>),
    ResourceLinked(EventEnvelope<ResourceLinked>),
    OperationInstalled(EventEnvelope<OperationInstalled>),
    OperationExecuted(EventEnvelope<OperationExecuted>),
    ResolverInstalled(EventEnvelope<ResolverInstalled>),
    ResolverExecuted(EventEnvelope<ResolverExecuted>),
}

/// Storage adapter for persisted resources.
pub trait ResourceStore {
    fn get(&self, key: &ResourceKey) -> Result<Option<ResourceRecord>, RuntimeError>;
    fn list(
        &self,
        contract_id: &ContractId,
        resource_type: &ResourceTypeId,
    ) -> Result<Vec<ResourceRecord>, RuntimeError>;
    fn put(&mut self, record: ResourceRecord) -> Result<(), RuntimeError>;
}

/// Event sink adapter for emitted runtime events.
pub trait EventSink {
    fn publish(&mut self, event: RuntimeEvent) -> Result<(), RuntimeError>;
}

/// Operation invocation extension point.
pub trait OperationHandler: Send + Sync {
    fn invoke(&self, input: Value) -> Result<Value, String>;
}

/// Resolver invocation extension point.
pub trait ResolverHandler: Send + Sync {
    fn resolve(&self, input: ResolverQueryEnvelope) -> Result<ResolverResultEnvelope, String>;
}

/// Static handler used in tests and examples.
pub struct StaticOperationHandler {
    result: Result<Value, String>,
}

impl StaticOperationHandler {
    pub fn new(result: Result<Value, Value>) -> Self {
        Self {
            result: result.map_err(|value| value.to_string()),
        }
    }
}

impl OperationHandler for StaticOperationHandler {
    fn invoke(&self, _input: Value) -> Result<Value, String> {
        self.result.clone()
    }
}

/// Static resolver used in tests and examples.
pub struct StaticResolverHandler {
    result: Result<ResolverResultEnvelope, String>,
}

impl StaticResolverHandler {
    pub fn new(result: Result<ResolverResultEnvelope, String>) -> Self {
        Self { result }
    }
}

impl ResolverHandler for StaticResolverHandler {
    fn resolve(&self, _input: ResolverQueryEnvelope) -> Result<ResolverResultEnvelope, String> {
        self.result.clone()
    }
}

struct RegisteredOperation {
    manifest: OperationManifest,
    handler: Arc<dyn OperationHandler>,
}

struct RegisteredResolver {
    descriptor: ResolverDescriptor,
    handler: Arc<dyn ResolverHandler>,
}

/// In-memory store adapter used by tests and examples.
#[derive(Default)]
pub struct InMemoryResourceStore {
    records: HashMap<ResourceKey, ResourceRecord>,
}

impl ResourceStore for InMemoryResourceStore {
    fn get(&self, key: &ResourceKey) -> Result<Option<ResourceRecord>, RuntimeError> {
        Ok(self.records.get(key).cloned())
    }

    fn list(
        &self,
        contract_id: &ContractId,
        resource_type: &ResourceTypeId,
    ) -> Result<Vec<ResourceRecord>, RuntimeError> {
        let mut records = self
            .records
            .values()
            .filter(|record| {
                &record.contract_id == contract_id && &record.resource_type == resource_type
            })
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.resource_id.as_str().cmp(right.resource_id.as_str()));
        Ok(records)
    }

    fn put(&mut self, record: ResourceRecord) -> Result<(), RuntimeError> {
        self.records.insert(ResourceKey::from(&record), record);
        Ok(())
    }
}

/// No-op sink useful for callers that do not need to observe events.
#[derive(Default)]
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn publish(&mut self, _event: RuntimeEvent) -> Result<(), RuntimeError> {
        Ok(())
    }
}

/// Sink that records events in memory.
#[derive(Default)]
pub struct RecordingEventSink {
    pub events: Vec<RuntimeEvent>,
}

impl EventSink for RecordingEventSink {
    fn publish(&mut self, event: RuntimeEvent) -> Result<(), RuntimeError> {
        self.events.push(event);
        Ok(())
    }
}

/// Generic runtime over a resource store and event sink.
pub struct Runtime<S, E> {
    store: S,
    event_sink: E,
    contracts: HashMap<ContractId, BTreeMap<ContractVersion, ContractManifest>>,
    active_contracts: HashMap<ContractId, ContractVersion>,
    operations: HashMap<OperationId, RegisteredOperation>,
    resolvers: HashMap<greentic_x_types::ResolverId, RegisteredResolver>,
    links: Vec<ResourceLink>,
    schemas: HashMap<String, Value>,
    next_event_id: u64,
}

impl<S, E> Runtime<S, E>
where
    S: ResourceStore,
    E: EventSink,
{
    pub fn new(store: S, event_sink: E) -> Self {
        Self {
            store,
            event_sink,
            contracts: HashMap::new(),
            active_contracts: HashMap::new(),
            operations: HashMap::new(),
            resolvers: HashMap::new(),
            links: Vec::new(),
            schemas: HashMap::new(),
            next_event_id: 1,
        }
    }

    pub fn register_schema_value(
        &mut self,
        schema_id: impl Into<String>,
        schema: Value,
    ) -> Result<(), RuntimeError> {
        let schema_id = schema_id.into();
        compile_validator(&schema_id, &schema)?;
        self.schemas.insert(schema_id, schema);
        Ok(())
    }

    pub fn register_schema_file(
        &mut self,
        schema: &SchemaReference,
        base_dir: impl AsRef<Path>,
    ) -> Result<(), RuntimeError> {
        let uri = schema
            .uri
            .as_deref()
            .ok_or_else(|| RuntimeError::SchemaNotRegistered {
                schema_id: schema.schema_id.clone(),
            })?;
        let schema_path = base_dir.as_ref().join(uri);
        let raw = std::fs::read_to_string(&schema_path).map_err(|err| {
            RuntimeError::Storage(format!("failed to read {}: {err}", schema_path.display()))
        })?;
        let value = serde_json::from_str(&raw).map_err(|err| {
            RuntimeError::Storage(format!("failed to parse {}: {err}", schema_path.display()))
        })?;
        self.register_schema_value(schema.schema_id.clone(), value)
    }

    pub fn register_contract_schemas(
        &mut self,
        manifest: &ContractManifest,
        base_dir: impl AsRef<Path>,
    ) -> Result<(), RuntimeError> {
        for resource in &manifest.resources {
            self.register_schema_file(&resource.schema, base_dir.as_ref())?;
            for collection in &resource.append_collections {
                self.register_schema_file(&collection.item_schema, base_dir.as_ref())?;
            }
        }
        Ok(())
    }

    pub fn register_operation_schemas(
        &mut self,
        manifest: &OperationManifest,
        base_dir: impl AsRef<Path>,
    ) -> Result<(), RuntimeError> {
        self.register_schema_file(&manifest.input_schema, base_dir.as_ref())?;
        self.register_schema_file(&manifest.output_schema, base_dir.as_ref())?;
        Ok(())
    }

    pub fn install_contract(
        &mut self,
        manifest: ContractManifest,
        provenance: Provenance,
    ) -> Result<(), RuntimeError> {
        let issues = manifest.validate();
        if !issues.is_empty() {
            return Err(RuntimeError::ContractValidation { issues });
        }

        let versions = self
            .contracts
            .entry(manifest.contract_id.clone())
            .or_default();
        if versions.contains_key(&manifest.version) {
            return Err(RuntimeError::ContractAlreadyInstalled {
                contract_id: manifest.contract_id.clone(),
                version: manifest.version.clone(),
            });
        }
        versions.insert(manifest.version.clone(), manifest.clone());

        let event = RuntimeEvent::ContractInstalled(EventEnvelope::contract_installed(
            self.allocate_event_id(),
            EventMetadata::new(provenance),
            ContractInstalled {
                contract_id: manifest.contract_id,
                version: manifest.version,
                compatibility: manifest.compatibility,
            },
        ));
        self.emit(event)
    }

    pub fn activate_contract(
        &mut self,
        contract_id: &ContractId,
        version: &ContractVersion,
        provenance: Provenance,
    ) -> Result<(), RuntimeError> {
        let manifest = self
            .contracts
            .get(contract_id)
            .and_then(|versions| versions.get(version))
            .cloned()
            .ok_or_else(|| RuntimeError::ContractNotInstalled {
                contract_id: contract_id.clone(),
                version: version.clone(),
            })?;

        self.active_contracts
            .insert(contract_id.clone(), version.clone());
        let event = RuntimeEvent::ContractActivated(EventEnvelope::contract_activated(
            self.allocate_event_id(),
            EventMetadata::new(provenance),
            ContractActivated {
                contract_id: contract_id.clone(),
                version: version.clone(),
                activation_target: manifest.description,
            },
        ));
        self.emit(event)
    }

    pub fn list_contracts(&self) -> Vec<ContractManifest> {
        let mut manifests = self
            .contracts
            .values()
            .flat_map(|versions| versions.values().cloned())
            .collect::<Vec<_>>();
        manifests.sort_by(|left, right| {
            left.contract_id
                .as_str()
                .cmp(right.contract_id.as_str())
                .then(left.version.as_str().cmp(right.version.as_str()))
        });
        manifests
    }

    pub fn describe_contract(
        &self,
        contract_id: &ContractId,
        version: Option<&ContractVersion>,
    ) -> Option<ContractManifest> {
        let versions = self.contracts.get(contract_id)?;
        let version = match version {
            Some(version) => version.clone(),
            None => self.active_contracts.get(contract_id)?.clone(),
        };
        versions.get(&version).cloned()
    }

    pub fn create_resource(
        &mut self,
        request: CreateResourceRequest,
    ) -> Result<ResourceRecord, RuntimeError> {
        self.ensure_document_object(&request.document)?;
        let definition = self
            .active_resource_definition(&request.contract_id, &request.resource_type)?
            .clone();
        self.validate_against_schema_if_registered(&definition.schema, &request.document)?;

        let key = ResourceKey::new(
            request.contract_id.clone(),
            request.resource_type.clone(),
            request.resource_id.clone(),
        );
        if self.store.get(&key)?.is_some() {
            return Err(RuntimeError::ResourceAlreadyExists {
                resource_id: request.resource_id,
            });
        }

        let record = ResourceRecord {
            contract_id: request.contract_id.clone(),
            resource_type: request.resource_type.clone(),
            resource_id: request.resource_id.clone(),
            revision: Revision::new(1),
            document: request.document,
        };
        self.store.put(record.clone())?;

        let event = RuntimeEvent::ResourceCreated(EventEnvelope::resource_created(
            self.allocate_event_id(),
            EventMetadata::new(request.provenance)
                .with_partition_key(record.resource_id.as_str().to_owned()),
            ResourceCreated {
                contract_id: record.contract_id.clone(),
                resource_type: record.resource_type.clone(),
                resource_id: record.resource_id.clone(),
                revision: record.revision,
                document: record.document.clone(),
            },
        ));
        self.emit(event)?;
        Ok(record)
    }

    pub fn get_resource(
        &self,
        contract_id: &ContractId,
        resource_type: &ResourceTypeId,
        resource_id: &ResourceId,
    ) -> Result<Option<ResourceRecord>, RuntimeError> {
        self.store.get(&ResourceKey::new(
            contract_id.clone(),
            resource_type.clone(),
            resource_id.clone(),
        ))
    }

    pub fn list_resources(
        &self,
        contract_id: &ContractId,
        resource_type: &ResourceTypeId,
    ) -> Result<Vec<ResourceRecord>, RuntimeError> {
        self.store.list(contract_id, resource_type)
    }

    pub fn upsert_link(
        &mut self,
        link: ResourceLink,
        provenance: Provenance,
    ) -> Result<ResourceLink, RuntimeError> {
        self.get_existing_record(
            &link.from.contract_id,
            &link.from.resource_type,
            &link.from.resource_id,
        )?;
        self.get_existing_record(
            &link.to.contract_id,
            &link.to.resource_type,
            &link.to.resource_id,
        )?;

        if let Some(existing) = self.links.iter_mut().find(|existing| {
            existing.link_type == link.link_type
                && existing.from == link.from
                && existing.to == link.to
        }) {
            *existing = link.clone();
        } else {
            self.links.push(link.clone());
        }
        self.links.sort_by(|left, right| {
            left.from
                .resource_id
                .as_str()
                .cmp(right.from.resource_id.as_str())
                .then(left.link_type.as_str().cmp(right.link_type.as_str()))
                .then(
                    left.to
                        .resource_id
                        .as_str()
                        .cmp(right.to.resource_id.as_str()),
                )
        });

        let event = RuntimeEvent::ResourceLinked(EventEnvelope::resource_linked(
            self.allocate_event_id(),
            EventMetadata::new(provenance)
                .with_partition_key(link.from.resource_id.as_str().to_owned()),
            ResourceLinked {
                link_type: link.link_type.clone(),
                from: link.from.clone(),
                to: link.to.clone(),
                metadata: link.metadata.clone(),
            },
        ));
        self.emit(event)?;
        Ok(link)
    }

    pub fn list_links(&self, resource: Option<&ResourceRef>) -> Vec<ResourceLink> {
        match resource {
            Some(resource) => self
                .links
                .iter()
                .filter(|link| &link.from == resource || &link.to == resource)
                .cloned()
                .collect(),
            None => self.links.clone(),
        }
    }

    pub fn patch_resource(
        &mut self,
        request: ResourcePatch,
    ) -> Result<ResourceRecord, RuntimeError> {
        let definition = self
            .active_resource_definition(&request.contract_id, &request.resource_type)?
            .clone();
        let mut record = self.get_existing_record(
            &request.contract_id,
            &request.resource_type,
            &request.resource_id,
        )?;
        self.ensure_revision(record.revision, request.base_revision)?;

        for operation in &request.operations {
            self.ensure_patch_allowed(&definition, operation)?;
            apply_patch_operation(&mut record.document, operation)?;
        }
        self.validate_against_schema_if_registered(&definition.schema, &record.document)?;
        record.revision = record.revision.next();
        self.store.put(record.clone())?;

        let event = RuntimeEvent::ResourcePatched(EventEnvelope::resource_patched(
            self.allocate_event_id(),
            EventMetadata::new(request.provenance)
                .with_partition_key(record.resource_id.as_str().to_owned()),
            ResourcePatched {
                contract_id: record.contract_id.clone(),
                resource_type: record.resource_type.clone(),
                resource_id: record.resource_id.clone(),
                from_revision: request.base_revision,
                to_revision: record.revision,
                applied_paths: request
                    .operations
                    .iter()
                    .map(|op| op.path.clone())
                    .collect(),
            },
        ));
        self.emit(event)?;
        Ok(record)
    }

    pub fn append_resource(
        &mut self,
        request: AppendRequest,
    ) -> Result<ResourceRecord, RuntimeError> {
        let definition = self
            .active_resource_definition(&request.contract_id, &request.resource_type)?
            .clone();
        self.ensure_append_allowed(&definition.append_collections, &request.collection)?;

        let mut record = self.get_existing_record(
            &request.contract_id,
            &request.resource_type,
            &request.resource_id,
        )?;
        self.ensure_revision(record.revision, request.base_revision)?;
        let collection_definition = definition
            .append_collections
            .iter()
            .find(|collection| collection.name == request.collection)
            .ok_or_else(|| RuntimeError::AppendCollectionNotAllowed {
                collection: request.collection.clone(),
            })?;
        self.validate_against_schema_if_registered(
            &collection_definition.item_schema,
            &request.value,
        )?;
        append_to_collection(
            &mut record.document,
            &request.collection,
            request.value.clone(),
        )?;
        self.validate_against_schema_if_registered(&definition.schema, &record.document)?;
        record.revision = record.revision.next();
        self.store.put(record.clone())?;

        let event = RuntimeEvent::ResourceAppended(EventEnvelope::resource_appended(
            self.allocate_event_id(),
            EventMetadata::new(request.provenance)
                .with_partition_key(record.resource_id.as_str().to_owned()),
            ResourceAppended {
                contract_id: record.contract_id.clone(),
                resource_type: record.resource_type.clone(),
                resource_id: record.resource_id.clone(),
                collection: request.collection,
                revision: record.revision,
                appended_value: request.value,
            },
        ));
        self.emit(event)?;
        Ok(record)
    }

    pub fn transition_resource(
        &mut self,
        request: TransitionRequest,
    ) -> Result<ResourceRecord, RuntimeError> {
        let definition = self
            .active_resource_definition(&request.contract_id, &request.resource_type)?
            .clone();
        let mut record = self.get_existing_record(
            &request.contract_id,
            &request.resource_type,
            &request.resource_id,
        )?;
        self.ensure_revision(record.revision, request.base_revision)?;

        let from_state = current_state(&record.document)?;
        ensure_transition_allowed(&definition.transitions, &from_state, &request.target_state)?;
        set_current_state(&mut record.document, request.target_state.clone())?;
        self.validate_against_schema_if_registered(&definition.schema, &record.document)?;
        record.revision = record.revision.next();
        self.store.put(record.clone())?;

        let event = RuntimeEvent::ResourceTransitioned(EventEnvelope::resource_transitioned(
            self.allocate_event_id(),
            EventMetadata::new(request.provenance)
                .with_partition_key(record.resource_id.as_str().to_owned()),
            ResourceTransitioned {
                contract_id: record.contract_id.clone(),
                resource_type: record.resource_type.clone(),
                resource_id: record.resource_id.clone(),
                from_state,
                to_state: request.target_state,
                revision: record.revision,
            },
        ));
        self.emit(event)?;
        Ok(record)
    }

    pub fn install_operation(
        &mut self,
        manifest: OperationManifest,
        handler: Arc<dyn OperationHandler>,
        provenance: Provenance,
    ) -> Result<(), RuntimeError> {
        let issues = manifest.validate();
        if !issues.is_empty() {
            return Err(RuntimeError::OperationValidation { issues });
        }
        self.ensure_supported_contracts_installed(&manifest.supported_contracts)?;

        if self.operations.contains_key(&manifest.operation_id) {
            return Err(RuntimeError::OperationAlreadyInstalled {
                operation_id: manifest.operation_id.clone(),
            });
        }

        self.operations.insert(
            manifest.operation_id.clone(),
            RegisteredOperation {
                manifest: manifest.clone(),
                handler,
            },
        );

        let event = RuntimeEvent::OperationInstalled(EventEnvelope::operation_installed(
            self.allocate_event_id(),
            EventMetadata::new(provenance),
            OperationInstalled {
                operation_id: manifest.operation_id,
                version: manifest.version,
                compatibility: manifest.compatibility,
            },
        ));
        self.emit(event)
    }

    pub fn install_resolver(
        &mut self,
        descriptor: ResolverDescriptor,
        handler: Arc<dyn ResolverHandler>,
        provenance: Provenance,
    ) -> Result<(), RuntimeError> {
        if self.resolvers.contains_key(&descriptor.resolver_id) {
            return Err(RuntimeError::ResolverAlreadyInstalled {
                resolver_id: descriptor.resolver_id.clone(),
            });
        }

        self.resolvers.insert(
            descriptor.resolver_id.clone(),
            RegisteredResolver {
                descriptor: descriptor.clone(),
                handler,
            },
        );

        let event = RuntimeEvent::ResolverInstalled(EventEnvelope::resolver_installed(
            self.allocate_event_id(),
            EventMetadata::new(provenance),
            ResolverInstalled {
                resolver_id: descriptor.resolver_id,
                target_type: descriptor.target_type,
            },
        ));
        self.emit(event)
    }

    pub fn list_operations(&self) -> Vec<OperationManifest> {
        let mut operations = self
            .operations
            .values()
            .map(|operation| operation.manifest.clone())
            .collect::<Vec<_>>();
        operations
            .sort_by(|left, right| left.operation_id.as_str().cmp(right.operation_id.as_str()));
        operations
    }

    pub fn describe_operation(&self, operation_id: &OperationId) -> Option<OperationManifest> {
        self.operations
            .get(operation_id)
            .map(|operation| operation.manifest.clone())
    }

    pub fn list_resolvers(&self) -> Vec<ResolverDescriptor> {
        let mut resolvers = self
            .resolvers
            .values()
            .map(|resolver| resolver.descriptor.clone())
            .collect::<Vec<_>>();
        resolvers.sort_by(|left, right| left.resolver_id.as_str().cmp(right.resolver_id.as_str()));
        resolvers
    }

    pub fn describe_resolver(
        &self,
        resolver_id: &greentic_x_types::ResolverId,
    ) -> Option<ResolverDescriptor> {
        self.resolvers
            .get(resolver_id)
            .map(|resolver| resolver.descriptor.clone())
    }

    pub fn invoke_operation(
        &mut self,
        operation_id: &OperationId,
        invocation_id: impl Into<String>,
        input: Value,
        provenance: Provenance,
    ) -> Result<Value, RuntimeError> {
        self.invoke_operation_enveloped(OperationCallEnvelope::new(
            invocation_id,
            operation_id.clone(),
            input,
            provenance,
        ))
        .and_then(|result| {
            result
                .output
                .ok_or_else(|| RuntimeError::OperationInvocationFailed {
                    operation_id: result.operation_id,
                    message: "operation completed without an output payload".to_owned(),
                })
        })
    }

    pub fn invoke_operation_enveloped(
        &mut self,
        envelope: OperationCallEnvelope,
    ) -> Result<OperationResultEnvelope, RuntimeError> {
        let (input_schema, output_schema, handler) = {
            let operation = self.operations.get(&envelope.operation_id).ok_or_else(|| {
                RuntimeError::OperationNotFound {
                    operation_id: envelope.operation_id.clone(),
                }
            })?;
            (
                operation.manifest.input_schema.clone(),
                operation.manifest.output_schema.clone(),
                Arc::clone(&operation.handler),
            )
        };
        self.validate_against_schema_if_registered(&input_schema, &envelope.input)?;
        let result = handler.invoke(envelope.input.clone());

        let status = if result.is_ok() {
            OperationExecutionStatus::Succeeded
        } else {
            OperationExecutionStatus::Failed
        };
        let output = result.clone().ok();
        let event = RuntimeEvent::OperationExecuted(EventEnvelope::operation_executed(
            self.allocate_event_id(),
            EventMetadata::new(envelope.provenance.clone()),
            OperationExecuted {
                operation_id: envelope.operation_id.clone(),
                invocation_id: envelope.invocation_id.clone(),
                status,
                output: output.clone(),
            },
        ));
        self.emit(event)?;

        match result {
            Ok(output_value) => {
                self.validate_against_schema_if_registered(&output_schema, &output_value)?;
                Ok(OperationResultEnvelope {
                    invocation_id: envelope.invocation_id,
                    operation_id: envelope.operation_id,
                    status: InvocationStatus::Succeeded,
                    output: Some(output_value),
                    evidence_refs: Vec::new(),
                    warnings: Vec::new(),
                    view_hints: Vec::new(),
                })
            }
            Err(message) => Err(RuntimeError::OperationInvocationFailed {
                operation_id: envelope.operation_id,
                message,
            }),
        }
    }

    pub fn resolve(
        &mut self,
        envelope: ResolverQueryEnvelope,
        invocation_id: impl Into<String>,
    ) -> Result<ResolverResultEnvelope, RuntimeError> {
        let invocation_id = invocation_id.into();
        let resolver = self.resolvers.get(&envelope.resolver_id).ok_or_else(|| {
            RuntimeError::ResolverNotFound {
                resolver_id: envelope.resolver_id.clone(),
            }
        })?;
        let result = resolver.handler.resolve(envelope.clone());
        let resolver_status = match &result {
            Ok(result) => map_resolver_status(result.status),
            Err(_) => ResolverExecutionStatus::Failed,
        };
        let candidate_count = result
            .as_ref()
            .map(|result| result.candidates.len())
            .unwrap_or(0);
        let selected = result.as_ref().ok().and_then(|result| {
            result
                .selected
                .as_ref()
                .map(|candidate| candidate.resource.clone())
        });
        let event = RuntimeEvent::ResolverExecuted(EventEnvelope::resolver_executed(
            self.allocate_event_id(),
            EventMetadata::new(envelope.provenance.clone()),
            ResolverExecuted {
                resolver_id: envelope.resolver_id.clone(),
                invocation_id,
                status: resolver_status,
                candidate_count,
                selected,
            },
        ));
        self.emit(event)?;

        result.map_err(|message| RuntimeError::ResolverInvocationFailed {
            resolver_id: envelope.resolver_id,
            message,
        })
    }

    pub fn into_parts(self) -> (S, E) {
        (self.store, self.event_sink)
    }

    fn allocate_event_id(&mut self) -> String {
        let event_id = format!("evt-{}", self.next_event_id);
        self.next_event_id += 1;
        event_id
    }

    fn emit(&mut self, event: RuntimeEvent) -> Result<(), RuntimeError> {
        self.event_sink.publish(event)
    }

    fn active_resource_definition(
        &self,
        contract_id: &ContractId,
        resource_type: &ResourceTypeId,
    ) -> Result<&ResourceDefinition, RuntimeError> {
        let active_version = self.active_contracts.get(contract_id).ok_or_else(|| {
            RuntimeError::ContractNotActive {
                contract_id: contract_id.clone(),
            }
        })?;
        let manifest = self
            .contracts
            .get(contract_id)
            .and_then(|versions| versions.get(active_version))
            .ok_or_else(|| RuntimeError::ContractNotInstalled {
                contract_id: contract_id.clone(),
                version: active_version.clone(),
            })?;
        manifest
            .resources
            .iter()
            .find(|resource| resource.resource_type == resource_type.as_str())
            .ok_or_else(|| RuntimeError::ResourceDefinitionNotFound {
                contract_id: contract_id.clone(),
                resource_type: resource_type.clone(),
            })
    }

    fn get_existing_record(
        &self,
        contract_id: &ContractId,
        resource_type: &ResourceTypeId,
        resource_id: &ResourceId,
    ) -> Result<ResourceRecord, RuntimeError> {
        self.store
            .get(&ResourceKey::new(
                contract_id.clone(),
                resource_type.clone(),
                resource_id.clone(),
            ))?
            .ok_or_else(|| RuntimeError::ResourceNotFound {
                resource_id: resource_id.clone(),
            })
    }

    fn ensure_document_object(&self, document: &Value) -> Result<(), RuntimeError> {
        if document.is_object() {
            Ok(())
        } else {
            Err(RuntimeError::InvalidDocument(
                "resource document must be a JSON object",
            ))
        }
    }

    fn ensure_revision(&self, actual: Revision, expected: Revision) -> Result<(), RuntimeError> {
        if actual == expected {
            Ok(())
        } else {
            Err(RuntimeError::RevisionConflict { expected, actual })
        }
    }

    fn ensure_patch_allowed(
        &self,
        definition: &ResourceDefinition,
        operation: &PatchOperation,
    ) -> Result<(), RuntimeError> {
        let denied = definition
            .patch_rules
            .iter()
            .any(|rule| rule.rule_kind == MutationRuleKind::Deny && rule.path == operation.path);
        if denied {
            return Err(RuntimeError::PatchDenied {
                path: operation.path.clone(),
            });
        }

        let allow_rules = definition
            .patch_rules
            .iter()
            .filter(|rule| rule.rule_kind == MutationRuleKind::Allow)
            .collect::<Vec<_>>();
        if allow_rules.is_empty() || !allow_rules.iter().any(|rule| rule.path == operation.path) {
            return Err(RuntimeError::PatchDenied {
                path: operation.path.clone(),
            });
        }
        Ok(())
    }

    fn ensure_append_allowed(
        &self,
        definitions: &[AppendCollectionDefinition],
        collection: &str,
    ) -> Result<(), RuntimeError> {
        if definitions
            .iter()
            .any(|definition| definition.name == collection)
        {
            Ok(())
        } else {
            Err(RuntimeError::AppendCollectionNotAllowed {
                collection: collection.to_owned(),
            })
        }
    }

    fn ensure_supported_contracts_installed(
        &self,
        supported_contracts: &[SupportedContract],
    ) -> Result<(), RuntimeError> {
        for supported in supported_contracts {
            let installed = self
                .contracts
                .get(&supported.contract_id)
                .is_some_and(|versions| versions.contains_key(&supported.version));
            if !installed {
                return Err(RuntimeError::OperationCompatibilityMissingContract {
                    contract_id: supported.contract_id.clone(),
                    version: supported.version.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_against_schema(
        &self,
        schema: &SchemaReference,
        instance: &Value,
    ) -> Result<(), RuntimeError> {
        let raw = self.schemas.get(&schema.schema_id).ok_or_else(|| {
            RuntimeError::SchemaNotRegistered {
                schema_id: schema.schema_id.clone(),
            }
        })?;
        let validator = compile_validator(&schema.schema_id, raw)?;
        if let Err(error) = validator.validate(instance) {
            return Err(RuntimeError::SchemaValidationFailed {
                schema_id: schema.schema_id.clone(),
                message: error.to_string(),
            });
        }
        Ok(())
    }

    fn validate_against_schema_if_registered(
        &self,
        schema: &SchemaReference,
        instance: &Value,
    ) -> Result<(), RuntimeError> {
        if self.schemas.contains_key(&schema.schema_id) {
            self.validate_against_schema(schema, instance)
        } else {
            Ok(())
        }
    }
}

fn compile_validator(schema_id: &str, schema: &Value) -> Result<Validator, RuntimeError> {
    jsonschema::validator_for(schema).map_err(|err| RuntimeError::SchemaCompilationFailed {
        schema_id: schema_id.to_owned(),
        message: err.to_string(),
    })
}

fn apply_patch_operation(
    document: &mut Value,
    operation: &PatchOperation,
) -> Result<(), RuntimeError> {
    let tokens = parse_pointer(&operation.path)?;
    match operation.op {
        PatchOperationKind::Add => add_value(document, &tokens, operation.value.clone()),
        PatchOperationKind::Replace => replace_value(document, &tokens, operation.value.clone()),
        PatchOperationKind::Remove => remove_value(document, &tokens),
    }
}

fn map_resolver_status(status: ResolverStatus) -> ResolverExecutionStatus {
    match status {
        ResolverStatus::Resolved => ResolverExecutionStatus::Resolved,
        ResolverStatus::Ambiguous => ResolverExecutionStatus::Ambiguous,
        ResolverStatus::NotFound => ResolverExecutionStatus::NotFound,
        ResolverStatus::Error => ResolverExecutionStatus::Failed,
    }
}

fn parse_pointer(path: &str) -> Result<Vec<&str>, RuntimeError> {
    if !path.starts_with('/') {
        return Err(RuntimeError::PatchPathInvalid {
            path: path.to_owned(),
        });
    }
    Ok(path.split('/').skip(1).collect())
}

fn append_to_collection(
    document: &mut Value,
    collection: &str,
    value: Value,
) -> Result<(), RuntimeError> {
    let object = document
        .as_object_mut()
        .ok_or(RuntimeError::InvalidDocument(
            "resource document must be a JSON object",
        ))?;
    let entry = object
        .entry(collection.to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    match entry {
        Value::Array(values) => {
            values.push(value);
            Ok(())
        }
        _ => Err(RuntimeError::InvalidDocument(
            "append collection target must be an array",
        )),
    }
}

fn current_state(document: &Value) -> Result<String, RuntimeError> {
    document
        .get("state")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(RuntimeError::InvalidDocument(
            "resource document must contain a string state field",
        ))
}

fn set_current_state(document: &mut Value, state: String) -> Result<(), RuntimeError> {
    let object = document
        .as_object_mut()
        .ok_or(RuntimeError::InvalidDocument(
            "resource document must be a JSON object",
        ))?;
    object.insert("state".to_owned(), Value::String(state));
    Ok(())
}

fn ensure_transition_allowed(
    transitions: &[greentic_x_contracts::TransitionDefinition],
    from_state: &str,
    to_state: &str,
) -> Result<(), RuntimeError> {
    if transitions
        .iter()
        .any(|transition| transition.from_state == from_state && transition.to_state == to_state)
    {
        Ok(())
    } else {
        Err(RuntimeError::TransitionDenied {
            from_state: from_state.to_owned(),
            to_state: to_state.to_owned(),
        })
    }
}

fn add_value(document: &mut Value, tokens: &[&str], value: Value) -> Result<(), RuntimeError> {
    let (parent_tokens, leaf) = tokens.split_at(tokens.len().saturating_sub(1));
    let leaf = leaf
        .first()
        .copied()
        .ok_or_else(|| RuntimeError::PatchPathInvalid {
            path: "/".to_owned(),
        })?;
    let parent = get_container_mut(document, parent_tokens)?;
    match parent {
        Value::Object(map) => {
            map.insert(leaf.to_owned(), value);
            Ok(())
        }
        Value::Array(array) => {
            if leaf == "-" {
                array.push(value);
                return Ok(());
            }
            let index = leaf
                .parse::<usize>()
                .map_err(|_| RuntimeError::PatchPathInvalid {
                    path: format!("/{}", tokens.join("/")),
                })?;
            if index > array.len() {
                return Err(RuntimeError::PatchPathInvalid {
                    path: format!("/{}", tokens.join("/")),
                });
            }
            array.insert(index, value);
            Ok(())
        }
        _ => Err(RuntimeError::InvalidDocument(
            "patch parent must be a JSON object or array",
        )),
    }
}

fn replace_value(document: &mut Value, tokens: &[&str], value: Value) -> Result<(), RuntimeError> {
    let target = get_value_mut(document, tokens)?;
    *target = value;
    Ok(())
}

fn remove_value(document: &mut Value, tokens: &[&str]) -> Result<(), RuntimeError> {
    let (parent_tokens, leaf) = tokens.split_at(tokens.len().saturating_sub(1));
    let leaf = leaf
        .first()
        .copied()
        .ok_or_else(|| RuntimeError::PatchPathInvalid {
            path: "/".to_owned(),
        })?;
    let parent = get_container_mut(document, parent_tokens)?;
    match parent {
        Value::Object(map) => {
            map.remove(leaf);
            Ok(())
        }
        Value::Array(array) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| RuntimeError::PatchPathInvalid {
                    path: format!("/{}", tokens.join("/")),
                })?;
            if index >= array.len() {
                return Err(RuntimeError::PatchPathInvalid {
                    path: format!("/{}", tokens.join("/")),
                });
            }
            array.remove(index);
            Ok(())
        }
        _ => Err(RuntimeError::InvalidDocument(
            "patch parent must be a JSON object or array",
        )),
    }
}

fn get_container_mut<'a>(
    mut current: &'a mut Value,
    tokens: &[&str],
) -> Result<&'a mut Value, RuntimeError> {
    for token in tokens {
        current = step_mut(current, token)?;
    }
    Ok(current)
}

fn get_value_mut<'a>(
    document: &'a mut Value,
    tokens: &[&str],
) -> Result<&'a mut Value, RuntimeError> {
    get_container_mut(document, tokens)
}

fn step_mut<'a>(current: &'a mut Value, token: &str) -> Result<&'a mut Value, RuntimeError> {
    match current {
        Value::Object(map) => map
            .get_mut(token)
            .ok_or_else(|| RuntimeError::PatchPathInvalid {
                path: token.to_owned(),
            }),
        Value::Array(array) => {
            let index = token
                .parse::<usize>()
                .map_err(|_| RuntimeError::PatchPathInvalid {
                    path: token.to_owned(),
                })?;
            array
                .get_mut(index)
                .ok_or_else(|| RuntimeError::PatchPathInvalid {
                    path: token.to_owned(),
                })
        }
        _ => Err(RuntimeError::InvalidDocument(
            "patch traversal encountered a non-container value",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use greentic_x_contracts::{
        AppendCollectionDefinition, ContractManifest, EventDeclaration, MutationRule,
        ResourceDefinition, TransitionDefinition,
    };
    use greentic_x_ops::OperationManifest;
    use greentic_x_types::{
        ActorRef, LinkTypeId, ResolverCandidate, ResolverId, ResourceLink, ResourceRef,
        SchemaReference,
    };

    fn case_manifest() -> ContractManifest {
        ContractManifest {
            contract_id: ContractId::new("gx.case").expect("static contract id should be valid"),
            version: ContractVersion::new("v1").expect("static version should be valid"),
            description: "Shared case contract".to_owned(),
            resources: vec![ResourceDefinition {
                resource_type: "case".to_owned(),
                schema: SchemaReference::new(
                    "greentic-x://contracts/case/resources/case",
                    ContractVersion::new("v1").expect("static version should be valid"),
                )
                .expect("static schema should be valid")
                .with_uri("schemas/case.schema.json"),
                patch_rules: vec![
                    MutationRule::allow("/title"),
                    MutationRule::allow("/severity"),
                ],
                append_collections: vec![AppendCollectionDefinition::new(
                    "evidence",
                    SchemaReference::new(
                        "greentic-x://contracts/case/resources/evidence-entry",
                        ContractVersion::new("v1").expect("static version should be valid"),
                    )
                    .expect("static schema should be valid")
                    .with_uri("schemas/evidence-entry.schema.json"),
                )],
                transitions: vec![
                    TransitionDefinition::new("new", "triaged"),
                    TransitionDefinition::new("triaged", "resolved"),
                ],
            }],
            compatibility: Vec::new(),
            event_declarations: vec![EventDeclaration::resource_created()],
            policy_hook: None,
            migration_from: Vec::new(),
        }
    }

    fn provenance() -> Provenance {
        Provenance::new(ActorRef::service("runtime").expect("static actor id should be valid"))
    }

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .to_path_buf()
    }

    fn runtime() -> Runtime<InMemoryResourceStore, RecordingEventSink> {
        Runtime::new(
            InMemoryResourceStore::default(),
            RecordingEventSink::default(),
        )
    }

    fn install_case_contract(
        runtime: &mut Runtime<InMemoryResourceStore, RecordingEventSink>,
    ) -> ContractManifest {
        let manifest = case_manifest();
        runtime
            .register_contract_schemas(&manifest, repo_root().join("contracts/case"))
            .expect("contract schemas should register");
        runtime
            .install_contract(manifest.clone(), provenance())
            .expect("contract installation should succeed");
        runtime
            .activate_contract(&manifest.contract_id, &manifest.version, provenance())
            .expect("contract activation should succeed");
        manifest
    }

    fn create_case(
        runtime: &mut Runtime<InMemoryResourceStore, RecordingEventSink>,
        contract_id: &ContractId,
    ) -> ResourceRecord {
        runtime
            .create_resource(CreateResourceRequest {
                contract_id: contract_id.clone(),
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: ResourceId::new("case-1").expect("static resource id should be valid"),
                document: serde_json::json!({
                    "case_id": "case-1",
                    "title": "Investigate ingress",
                    "severity": "high",
                    "state": "new"
                }),
                provenance: provenance(),
            })
            .expect("resource creation should succeed")
    }

    fn approval_basic_manifest() -> OperationManifest {
        OperationManifest {
            operation_id: OperationId::new("approval-basic")
                .expect("static operation id should be valid"),
            version: ContractVersion::new("v1").expect("static version should be valid"),
            description: "Approve simple outcomes".to_owned(),
            input_schema: SchemaReference::new(
                "greentic-x://ops/approval-basic/input",
                ContractVersion::new("v1").expect("static version should be valid"),
            )
            .expect("static schema should be valid")
            .with_uri("schemas/input.schema.json"),
            output_schema: SchemaReference::new(
                "greentic-x://ops/approval-basic/output",
                ContractVersion::new("v1").expect("static version should be valid"),
            )
            .expect("static schema should be valid")
            .with_uri("schemas/output.schema.json"),
            compatibility: Vec::new(),
            supported_contracts: vec![SupportedContract {
                contract_id: ContractId::new("gx.case")
                    .expect("static contract id should be valid"),
                version: ContractVersion::new("v1").expect("static version should be valid"),
            }],
            permissions: Vec::new(),
            examples: Vec::new(),
        }
    }

    fn resolver_descriptor() -> ResolverDescriptor {
        ResolverDescriptor {
            resolver_id: ResolverId::new("resolve.by_name")
                .expect("static resolver id should be valid"),
            description: "Resolve case by exact name".to_owned(),
            target_type: Some(
                ResourceTypeId::new("case").expect("static resource type should be valid"),
            ),
            tags: vec!["exact".to_owned(), "demo".to_owned()],
        }
    }

    fn install_approval_operation(
        runtime: &mut Runtime<InMemoryResourceStore, RecordingEventSink>,
    ) {
        let manifest = approval_basic_manifest();
        runtime
            .register_operation_schemas(&manifest, repo_root().join("ops/approval-basic"))
            .expect("operation schemas should register");
        runtime
            .install_operation(
                manifest,
                Arc::new(StaticOperationHandler::new(Ok(
                    serde_json::json!({"approved": true}),
                ))),
                provenance(),
            )
            .expect("operation install should succeed");
    }

    #[test]
    fn allows_only_declared_patch_paths() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let record = create_case(&mut runtime, &manifest.contract_id);

        let updated = runtime
            .patch_resource(ResourcePatch {
                contract_id: manifest.contract_id.clone(),
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: record.resource_id.clone(),
                base_revision: record.revision,
                operations: vec![PatchOperation::replace(
                    "/title",
                    serde_json::json!("Updated title"),
                )],
                provenance: provenance(),
            })
            .expect("allowed patch should succeed");
        assert_eq!(updated.document["title"], "Updated title");

        let err = runtime
            .patch_resource(ResourcePatch {
                contract_id: manifest.contract_id,
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: updated.resource_id,
                base_revision: updated.revision,
                operations: vec![PatchOperation::replace(
                    "/state",
                    serde_json::json!("triaged"),
                )],
                provenance: provenance(),
            })
            .expect_err("undeclared patch path should be denied");
        assert!(matches!(err, RuntimeError::PatchDenied { .. }));
    }

    #[test]
    fn supports_append_only_collections() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let record = create_case(&mut runtime, &manifest.contract_id);

        let updated = runtime
            .append_resource(
                AppendRequest::new(
                    manifest.contract_id,
                    ResourceTypeId::new("case").expect("static resource type should be valid"),
                    record.resource_id.clone(),
                    record.revision,
                    "evidence",
                    serde_json::json!({"kind": "log", "uri": "s3://case-1/logs.json"}),
                    provenance(),
                )
                .expect("append request should be valid"),
            )
            .expect("append should succeed");

        assert_eq!(updated.document["evidence"][0]["kind"], "log");
    }

    #[test]
    fn enforces_allowed_transitions() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let record = create_case(&mut runtime, &manifest.contract_id);

        let triaged = runtime
            .transition_resource(
                TransitionRequest::new(
                    manifest.contract_id.clone(),
                    ResourceTypeId::new("case").expect("static resource type should be valid"),
                    record.resource_id.clone(),
                    record.revision,
                    "triaged",
                    provenance(),
                )
                .expect("transition request should be valid"),
            )
            .expect("valid transition should succeed");
        assert_eq!(triaged.document["state"], "triaged");

        let err = runtime
            .transition_resource(
                TransitionRequest::new(
                    manifest.contract_id,
                    ResourceTypeId::new("case").expect("static resource type should be valid"),
                    triaged.resource_id,
                    triaged.revision,
                    "new",
                    provenance(),
                )
                .expect("transition request should be valid"),
            )
            .expect_err("invalid transition should be rejected");
        assert!(matches!(err, RuntimeError::TransitionDenied { .. }));
    }

    #[test]
    fn detects_revision_conflicts() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let record = create_case(&mut runtime, &manifest.contract_id);

        let err = runtime
            .patch_resource(ResourcePatch {
                contract_id: manifest.contract_id,
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: record.resource_id,
                base_revision: Revision::new(99),
                operations: vec![PatchOperation::replace(
                    "/title",
                    serde_json::json!("late write"),
                )],
                provenance: provenance(),
            })
            .expect_err("mismatched revision should be rejected");
        assert!(matches!(err, RuntimeError::RevisionConflict { .. }));
    }

    #[test]
    fn rejects_missing_operations() {
        let mut runtime = runtime();
        let err = runtime
            .invoke_operation(
                &OperationId::new("missing-op").expect("static operation id should be valid"),
                "invoke-1",
                serde_json::json!({}),
                provenance(),
            )
            .expect_err("missing operation should fail");
        assert!(matches!(err, RuntimeError::OperationNotFound { .. }));
    }

    #[test]
    fn emits_events_for_contract_resource_and_operation_flows() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let record = create_case(&mut runtime, &manifest.contract_id);

        install_approval_operation(&mut runtime);
        let output = runtime
            .invoke_operation(
                &OperationId::new("approval-basic").expect("static operation id should be valid"),
                "invoke-1",
                serde_json::json!({
                    "case_id": record.resource_id.as_str(),
                    "risk_score": 0.2
                }),
                provenance(),
            )
            .expect("operation invocation should succeed");
        assert_eq!(output["approved"], true);

        let (_store, sink) = runtime.into_parts();
        assert_eq!(sink.events.len(), 5);
        assert!(matches!(sink.events[0], RuntimeEvent::ContractInstalled(_)));
        assert!(matches!(sink.events[1], RuntimeEvent::ContractActivated(_)));
        assert!(matches!(sink.events[2], RuntimeEvent::ResourceCreated(_)));
        assert!(matches!(
            sink.events[3],
            RuntimeEvent::OperationInstalled(_)
        ));
        assert!(matches!(sink.events[4], RuntimeEvent::OperationExecuted(_)));
    }

    #[test]
    fn rejects_operation_registration_for_missing_contract_support() {
        let mut runtime = runtime();
        let err = runtime
            .install_operation(
                OperationManifest {
                    operation_id: OperationId::new("playbook-select")
                        .expect("static operation id should be valid"),
                    version: ContractVersion::new("v1").expect("static version should be valid"),
                    description: "Select a playbook".to_owned(),
                    input_schema: SchemaReference::new(
                        "greentic-x://ops/playbook-select/input",
                        ContractVersion::new("v1").expect("static version should be valid"),
                    )
                    .expect("static schema should be valid"),
                    output_schema: SchemaReference::new(
                        "greentic-x://ops/playbook-select/output",
                        ContractVersion::new("v1").expect("static version should be valid"),
                    )
                    .expect("static schema should be valid"),
                    compatibility: Vec::new(),
                    supported_contracts: vec![SupportedContract {
                        contract_id: ContractId::new("gx.playbook")
                            .expect("static contract id should be valid"),
                        version: ContractVersion::new("v1")
                            .expect("static version should be valid"),
                    }],
                    permissions: Vec::new(),
                    examples: Vec::new(),
                },
                Arc::new(StaticOperationHandler::new(Ok(
                    serde_json::json!({"route": "default"}),
                ))),
                provenance(),
            )
            .expect_err("unsupported contract dependency should fail");
        assert!(matches!(
            err,
            RuntimeError::OperationCompatibilityMissingContract { .. }
        ));
    }

    #[test]
    fn supports_typed_links_between_resources() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let case = create_case(&mut runtime, &manifest.contract_id);
        let second = runtime
            .create_resource(CreateResourceRequest {
                contract_id: manifest.contract_id.clone(),
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: ResourceId::new("case-2").expect("static resource id should be valid"),
                document: serde_json::json!({
                    "case_id": "case-2",
                    "title": "Investigate egress",
                    "severity": "medium",
                    "state": "new"
                }),
                provenance: provenance(),
            })
            .expect("second resource creation should succeed");

        runtime
            .upsert_link(
                ResourceLink::new(
                    LinkTypeId::new("depends_on").expect("static link type should be valid"),
                    ResourceRef::new(
                        manifest.contract_id.clone(),
                        ResourceTypeId::new("case").expect("static resource type should be valid"),
                        case.resource_id.clone(),
                    ),
                    ResourceRef::new(
                        manifest.contract_id,
                        ResourceTypeId::new("case").expect("static resource type should be valid"),
                        second.resource_id.clone(),
                    ),
                ),
                provenance(),
            )
            .expect("link upsert should succeed");

        let links = runtime.list_links(None);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type.as_str(), "depends_on");
        assert_eq!(links[0].to.resource_id, second.resource_id);
    }

    #[test]
    fn registers_and_invokes_resolvers() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);
        let case = create_case(&mut runtime, &manifest.contract_id);

        runtime
            .install_resolver(
                resolver_descriptor(),
                Arc::new(StaticResolverHandler::new(Ok(ResolverResultEnvelope {
                    resolver_id: ResolverId::new("resolve.by_name")
                        .expect("static resolver id should be valid"),
                    status: ResolverStatus::Resolved,
                    selected: Some(ResolverCandidate {
                        resource: ResourceRef::new(
                            manifest.contract_id.clone(),
                            ResourceTypeId::new("case")
                                .expect("static resource type should be valid"),
                            case.resource_id.clone(),
                        ),
                        display: Some("Case 1".to_owned()),
                        confidence: Some(1.0),
                        metadata: None,
                    }),
                    candidates: Vec::new(),
                    warnings: Vec::new(),
                }))),
                provenance(),
            )
            .expect("resolver install should succeed");

        let result = runtime
            .resolve(
                ResolverQueryEnvelope::new(
                    ResolverId::new("resolve.by_name").expect("static resolver id should be valid"),
                    serde_json::json!({"name": "Case 1"}),
                    provenance(),
                ),
                "resolve-1",
            )
            .expect("resolver invocation should succeed");

        assert_eq!(result.status, ResolverStatus::Resolved);
        assert_eq!(
            result
                .selected
                .expect("resolved result should contain a selected candidate")
                .resource
                .resource_id,
            case.resource_id
        );
    }

    #[test]
    fn supports_operation_call_envelopes() {
        let mut runtime = runtime();
        install_case_contract(&mut runtime);
        install_approval_operation(&mut runtime);

        let result = runtime
            .invoke_operation_enveloped(
                OperationCallEnvelope::new(
                    "invoke-1",
                    OperationId::new("approval-basic")
                        .expect("static operation id should be valid"),
                    serde_json::json!({"risk_score": 0.2}),
                    provenance(),
                )
                .with_run_id("run-1"),
            )
            .expect("operation invocation should succeed");

        assert_eq!(result.status, InvocationStatus::Succeeded);
        assert_eq!(result.output.expect("output is present")["approved"], true);
    }

    #[test]
    fn validates_resource_documents_against_registered_schemas() {
        let mut runtime = runtime();
        let manifest = install_case_contract(&mut runtime);

        let err = runtime
            .create_resource(CreateResourceRequest {
                contract_id: manifest.contract_id,
                resource_type: ResourceTypeId::new("case")
                    .expect("static resource type should be valid"),
                resource_id: ResourceId::new("case-invalid")
                    .expect("static resource id should be valid"),
                document: serde_json::json!({
                    "case_id": "case-invalid",
                    "title": "Invalid case"
                }),
                provenance: provenance(),
            })
            .expect_err("missing required state field should fail schema validation");
        assert!(matches!(err, RuntimeError::SchemaValidationFailed { .. }));
    }

    #[test]
    fn validates_operation_payloads_against_registered_schemas() {
        let mut runtime = runtime();
        install_case_contract(&mut runtime);
        install_approval_operation(&mut runtime);

        let err = runtime
            .invoke_operation(
                &OperationId::new("approval-basic").expect("static operation id should be valid"),
                "invoke-bad",
                serde_json::json!({"unexpected": true}),
                provenance(),
            )
            .expect_err("invalid op input should fail schema validation");
        assert!(matches!(err, RuntimeError::SchemaValidationFailed { .. }));
    }
}
