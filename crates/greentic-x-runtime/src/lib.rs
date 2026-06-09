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
//! use std::sync::{Arc, Mutex};
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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Portable runtime classes for Greentic-X component invocation.
///
/// The enum is intentionally independent from any one transport. A playbook or
/// catalog can declare the runtime class, while the host decides which provider
/// is configured to execute it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentRuntimeKind {
    LocalBuiltin,
    WasmWasi,
    McpAdapter,
    ExternalWorker,
}

/// Descriptor for a component that can be invoked by a GX host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    pub component_id: String,
    pub kind: String,
    pub runtime: ComponentRuntimeKind,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resilience: Option<ResilienceStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caching: Option<CachingStrategy>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl ComponentDescriptor {
    pub fn new(
        component_id: impl Into<String>,
        kind: impl Into<String>,
        runtime: ComponentRuntimeKind,
        reference: impl Into<String>,
    ) -> Self {
        Self {
            component_id: component_id.into(),
            kind: kind.into(),
            runtime,
            reference: reference.into(),
            interface: None,
            resilience: None,
            caching: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = Some(interface.into());
        self
    }

    pub fn with_resilience(mut self, strategy: ResilienceStrategy) -> Self {
        self.resilience = Some(strategy);
        self
    }

    pub fn with_caching(mut self, strategy: CachingStrategy) -> Self {
        self.caching = Some(strategy);
        self
    }
}

/// Host-executed resilience policy for component calls that touch external
/// systems. Hosts are responsible for enforcing the policy because retry,
/// health, and success checks are transport-specific.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResilienceStrategy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_check: Option<HealthCheckStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_check: Option<SuccessCheckStrategy>,
}

/// Optional pre-invocation health check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckStrategy {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Retry settings for retryable component invocation failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryStrategy {
    pub max_attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_delay_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_delay_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backoff_multiplier: Option<u32>,
}

/// Optional post-failure verification for update-style operations where the
/// transport may fail after the external system accepted the change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessCheckStrategy {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Host-executed cache policy for deterministic component calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachingStrategy {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_entries: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_template: Option<String>,
}

/// Standard invocation envelope for resolver, adapter, analyser, renderer, MCP,
/// WASM/WASI, and external worker components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentInvocationEnvelope {
    pub invocation_id: String,
    pub component_id: String,
    pub runtime: ComponentRuntimeKind,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resilience: Option<ResilienceStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caching: Option<CachingStrategy>,
    pub input: Value,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl ComponentInvocationEnvelope {
    pub fn new(
        invocation_id: impl Into<String>,
        component: &ComponentDescriptor,
        input: Value,
        provenance: Provenance,
    ) -> Self {
        Self {
            invocation_id: invocation_id.into(),
            component_id: component.component_id.clone(),
            runtime: component.runtime,
            reference: component.reference.clone(),
            interface: component.interface.clone(),
            resilience: component.resilience.clone(),
            caching: component.caching.clone(),
            input,
            provenance,
            run_id: None,
            metadata: component.metadata.clone(),
        }
    }

    pub fn with_run_id(mut self, run_id: impl Into<String>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_resilience(mut self, strategy: ResilienceStrategy) -> Self {
        self.resilience = Some(strategy);
        self
    }

    pub fn with_caching(mut self, strategy: CachingStrategy) -> Self {
        self.caching = Some(strategy);
        self
    }
}

/// Standard result envelope returned by every component provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentInvocationResultEnvelope {
    pub invocation_id: String,
    pub component_id: String,
    pub status: InvocationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl ComponentInvocationResultEnvelope {
    pub fn success(
        invocation_id: impl Into<String>,
        component_id: impl Into<String>,
        output: Value,
    ) -> Self {
        Self {
            invocation_id: invocation_id.into(),
            component_id: component_id.into(),
            status: InvocationStatus::Succeeded,
            output: Some(output),
            error: None,
            warnings: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn failed(
        invocation_id: impl Into<String>,
        component_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            invocation_id: invocation_id.into(),
            component_id: component_id.into(),
            status: InvocationStatus::Failed,
            output: None,
            error: Some(error.into()),
            warnings: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Provider boundary for executable components.
///
/// Implementations may back this with local built-ins, WASM/WASI, MCP adapter
/// calls, remote workers, or test fixtures. The runtime contract stays the same:
/// JSON envelope in, JSON result envelope out.
pub trait ComponentProvider: Send + Sync {
    fn invoke_component(
        &self,
        envelope: ComponentInvocationEnvelope,
    ) -> Result<ComponentInvocationResultEnvelope, RuntimeError>;
}

/// Minimal provider for hosts that have not configured external execution yet.
#[derive(Debug, Default, Clone)]
pub struct UnsupportedComponentProvider;

impl ComponentProvider for UnsupportedComponentProvider {
    fn invoke_component(
        &self,
        envelope: ComponentInvocationEnvelope,
    ) -> Result<ComponentInvocationResultEnvelope, RuntimeError> {
        Err(RuntimeError::ComponentInvocationFailed {
            component_id: envelope.component_id,
            message: format!(
                "component runtime {:?} for reference {} is not configured",
                envelope.runtime, envelope.reference
            ),
        })
    }
}

/// Deterministic fixture provider for tests and host integration scaffolding.
#[derive(Debug, Default, Clone)]
pub struct StaticComponentProvider {
    outputs: BTreeMap<String, Value>,
}

impl StaticComponentProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_component_output(mut self, component_id: impl Into<String>, output: Value) -> Self {
        self.outputs.insert(component_id.into(), output);
        self
    }
}

impl ComponentProvider for StaticComponentProvider {
    fn invoke_component(
        &self,
        envelope: ComponentInvocationEnvelope,
    ) -> Result<ComponentInvocationResultEnvelope, RuntimeError> {
        let output = self
            .outputs
            .get(&envelope.component_id)
            .cloned()
            .ok_or_else(|| RuntimeError::ComponentInvocationFailed {
                component_id: envelope.component_id.clone(),
                message: "no static output registered for component".to_owned(),
            })?;
        Ok(ComponentInvocationResultEnvelope::success(
            envelope.invocation_id,
            envelope.component_id,
            output,
        ))
    }
}

/// Adapter for host-owned component runtimes.
///
/// This lets a host wire Greentic-X component invocation to an executor that
/// lives outside this crate, such as greentic-runner-host, MCP transport, or an
/// agentic worker pool, without adding those heavyweight dependencies to the
/// portable runtime contract crate.
pub struct DelegatingComponentProvider<F>
where
    F: Fn(ComponentInvocationEnvelope) -> Result<ComponentInvocationResultEnvelope, RuntimeError>
        + Send
        + Sync,
{
    handler: F,
}

impl<F> DelegatingComponentProvider<F>
where
    F: Fn(ComponentInvocationEnvelope) -> Result<ComponentInvocationResultEnvelope, RuntimeError>
        + Send
        + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> ComponentProvider for DelegatingComponentProvider<F>
where
    F: Fn(ComponentInvocationEnvelope) -> Result<ComponentInvocationResultEnvelope, RuntimeError>
        + Send
        + Sync,
{
    fn invoke_component(
        &self,
        envelope: ComponentInvocationEnvelope,
    ) -> Result<ComponentInvocationResultEnvelope, RuntimeError> {
        (self.handler)(envelope)
    }
}

/// Host-visible execution metadata produced by the generic strategy executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentExecutionMetadata {
    pub attempts: u32,
    pub cache_enabled: bool,
    pub cache_hit: bool,
    pub cache_key: Option<String>,
    pub elapsed_ms: u64,
    pub timed_out: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl ComponentExecutionMetadata {
    fn new(cache_enabled: bool, cache_key: Option<String>) -> Self {
        Self {
            attempts: 0,
            cache_enabled,
            cache_hit: false,
            cache_key,
            elapsed_ms: 0,
            timed_out: false,
            warnings: Vec::new(),
        }
    }
}

/// Result plus strategy execution metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentExecutionOutcome {
    pub result: ComponentInvocationResultEnvelope,
    pub metadata: ComponentExecutionMetadata,
}

/// Cache boundary for deterministic component invocation results.
pub trait ComponentCache: Send + Sync {
    fn get(&self, key: &str) -> Option<ComponentInvocationResultEnvelope>;
    fn insert(
        &self,
        key: String,
        value: ComponentInvocationResultEnvelope,
        ttl_ms: Option<u64>,
        max_entries: Option<u64>,
    );
}

#[derive(Debug, Clone)]
struct CachedComponentValue {
    value: ComponentInvocationResultEnvelope,
    expires_at: Option<std::time::Instant>,
}

/// Small in-memory cache for hosts, tests, and smoke demos.
#[derive(Debug, Default)]
pub struct InMemoryComponentCache {
    values: Mutex<HashMap<String, CachedComponentValue>>,
}

impl InMemoryComponentCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.values
            .lock()
            .expect("component cache mutex should not be poisoned")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ComponentCache for InMemoryComponentCache {
    fn get(&self, key: &str) -> Option<ComponentInvocationResultEnvelope> {
        let mut values = self
            .values
            .lock()
            .expect("component cache mutex should not be poisoned");
        let cached = values.get(key)?;
        if cached
            .expires_at
            .is_some_and(|expires_at| std::time::Instant::now() >= expires_at)
        {
            values.remove(key);
            return None;
        }
        Some(cached.value.clone())
    }

    fn insert(
        &self,
        key: String,
        value: ComponentInvocationResultEnvelope,
        ttl_ms: Option<u64>,
        max_entries: Option<u64>,
    ) {
        let expires_at =
            ttl_ms.map(|ttl| std::time::Instant::now() + std::time::Duration::from_millis(ttl));
        let mut values = self
            .values
            .lock()
            .expect("component cache mutex should not be poisoned");
        values.insert(key, CachedComponentValue { value, expires_at });
        if let Some(max_entries) = max_entries.and_then(|value| usize::try_from(value).ok()) {
            while max_entries > 0 && values.len() > max_entries {
                if let Some(oldest_key) = values.keys().next().cloned() {
                    values.remove(&oldest_key);
                } else {
                    break;
                }
            }
            if max_entries == 0 {
                values.clear();
            }
        }
    }
}

/// Execute a component provider with generic resilience and caching strategy support.
///
/// This helper intentionally stays transport-neutral: hosts can wrap WASM, MCP,
/// local fixture, or external worker providers with the same policy handling.
pub fn execute_component_with_strategies<P>(
    provider: &P,
    envelope: ComponentInvocationEnvelope,
    cache: Option<&dyn ComponentCache>,
) -> Result<ComponentExecutionOutcome, RuntimeError>
where
    P: ComponentProvider + ?Sized,
{
    let started = std::time::Instant::now();
    let caching = envelope.caching.clone().filter(|strategy| strategy.enabled);
    let cache_key = caching
        .as_ref()
        .map(|strategy| component_cache_key(&envelope, strategy));
    let mut metadata = ComponentExecutionMetadata::new(caching.is_some(), cache_key.clone());

    if let (Some(cache), Some(key)) = (cache, cache_key.as_deref()) {
        if let Some(mut result) = cache.get(key) {
            metadata.cache_hit = true;
            metadata.elapsed_ms = elapsed_ms(started);
            attach_execution_metadata(&mut result, &metadata);
            return Ok(ComponentExecutionOutcome { result, metadata });
        }
    }

    let retry = envelope
        .resilience
        .as_ref()
        .and_then(|strategy| strategy.retry.as_ref());
    let max_attempts = retry.map_or(1, |strategy| strategy.max_attempts.max(1));
    let timeout_ms = envelope
        .resilience
        .as_ref()
        .and_then(|strategy| strategy.health_check.as_ref())
        .and_then(|strategy| strategy.timeout_ms);

    let mut last_error = None;
    for attempt in 1..=max_attempts {
        metadata.attempts = attempt;
        let result = provider.invoke_component(envelope.clone());
        match result {
            Ok(mut result) if result.status == InvocationStatus::Succeeded => {
                metadata.elapsed_ms = elapsed_ms(started);
                metadata.timed_out = timeout_ms.is_some_and(|limit| metadata.elapsed_ms > limit);
                if let (Some(cache), Some(strategy), Some(key)) =
                    (cache, caching.as_ref(), cache_key.clone())
                {
                    cache.insert(key, result.clone(), strategy.ttl_ms, strategy.max_entries);
                }
                attach_execution_metadata(&mut result, &metadata);
                return Ok(ComponentExecutionOutcome { result, metadata });
            }
            Ok(result) => {
                last_error = Some(RuntimeError::ComponentInvocationFailed {
                    component_id: result.component_id,
                    message: result
                        .error
                        .unwrap_or_else(|| "component returned failed status".to_owned()),
                });
            }
            Err(err) => {
                last_error = Some(err);
            }
        }

        if attempt < max_attempts {
            let delay_ms = retry_delay_ms(retry, attempt);
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
    }

    metadata.elapsed_ms = elapsed_ms(started);
    metadata.timed_out = timeout_ms.is_some_and(|limit| metadata.elapsed_ms > limit);
    Err(
        last_error.unwrap_or_else(|| RuntimeError::ComponentInvocationFailed {
            component_id: envelope.component_id,
            message: "component invocation failed without an error detail".to_owned(),
        }),
    )
}

fn retry_delay_ms(retry: Option<&RetryStrategy>, completed_attempt: u32) -> u64 {
    let Some(retry) = retry else {
        return 0;
    };
    let initial = retry.initial_delay_ms.unwrap_or(0);
    if initial == 0 {
        return 0;
    }
    let multiplier = retry.backoff_multiplier.unwrap_or(1).max(1) as u64;
    let mut delay = initial;
    for _ in 1..completed_attempt {
        delay = delay.saturating_mul(multiplier);
    }
    retry.max_delay_ms.map_or(delay, |max| delay.min(max))
}

fn component_cache_key(
    envelope: &ComponentInvocationEnvelope,
    strategy: &CachingStrategy,
) -> String {
    if let Some(template) = strategy.key_template.as_deref() {
        return render_cache_key_template(template, envelope);
    }
    format!(
        "{}:{}:{}",
        envelope.component_id, envelope.reference, envelope.input
    )
}

fn render_cache_key_template(template: &str, envelope: &ComponentInvocationEnvelope) -> String {
    let mut rendered = template
        .replace("${component_id}", &envelope.component_id)
        .replace("${runtime}", &format!("{:?}", envelope.runtime))
        .replace("${reference}", &envelope.reference)
        .replace("${invocation_id}", &envelope.invocation_id);

    for (key, value) in envelope.input.as_object().into_iter().flatten() {
        let replacement = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        rendered = rendered.replace(&format!("${{input.{key}}}"), &replacement);
    }
    rendered
}

fn attach_execution_metadata(
    result: &mut ComponentInvocationResultEnvelope,
    metadata: &ComponentExecutionMetadata,
) {
    result.metadata.insert(
        "greentic_x.execution".to_owned(),
        serde_json::to_value(metadata).expect("execution metadata should serialize"),
    );
}

fn elapsed_ms(started: std::time::Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

/// Incoming message envelope used by the Fast2Flow routing boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fast2FlowMessageEnvelope {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl Fast2FlowMessageEnvelope {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            channel: None,
            provider: None,
        }
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }
}

/// Greentic-X host request for Fast2Flow-style intent routing.
///
/// This mirrors the Fast2Flow hook contract but keeps Greentic-X decoupled from
/// a concrete Fast2Flow crate or runner implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fast2FlowRouteRequest {
    pub scope: String,
    pub envelope: Fast2FlowMessageEnvelope,
    pub session_active: bool,
    pub input_locale: String,
    pub time_budget_ms: u64,
    pub registry_path: String,
    pub indexes_path: String,
    pub now_unix_ms: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl Fast2FlowRouteRequest {
    pub fn new(scope: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            envelope: Fast2FlowMessageEnvelope::new(text),
            session_active: false,
            input_locale: "en".to_owned(),
            time_budget_ms: 250,
            registry_path: String::new(),
            indexes_path: String::new(),
            now_unix_ms: 0,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_mounts(
        mut self,
        registry_path: impl Into<String>,
        indexes_path: impl Into<String>,
    ) -> Self {
        self.registry_path = registry_path.into();
        self.indexes_path = indexes_path.into();
        self
    }

    pub fn with_session_active(mut self, session_active: bool) -> Self {
        self.session_active = session_active;
        self
    }

    pub fn with_locale(mut self, input_locale: impl Into<String>) -> Self {
        self.input_locale = input_locale.into();
        self
    }

    pub fn with_time_budget_ms(mut self, time_budget_ms: u64) -> Self {
        self.time_budget_ms = time_budget_ms;
        self
    }

    pub fn with_now_unix_ms(mut self, now_unix_ms: u64) -> Self {
        self.now_unix_ms = now_unix_ms;
        self
    }
}

/// Routing decision returned by a Fast2Flow-compatible host integration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fast2FlowDirective {
    Continue,
    Dispatch {
        target: String,
        confidence: f32,
        reason: String,
    },
    Respond {
        message: String,
    },
    Deny {
        reason: String,
    },
}

/// Standard route result envelope for Greentic-X intent routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fast2FlowRouteResult {
    pub directive: Fast2FlowDirective,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl Fast2FlowRouteResult {
    pub fn continue_route() -> Self {
        Self {
            directive: Fast2FlowDirective::Continue,
            metadata: BTreeMap::new(),
        }
    }

    pub fn dispatch(target: impl Into<String>, confidence: f32, reason: impl Into<String>) -> Self {
        Self {
            directive: Fast2FlowDirective::Dispatch {
                target: target.into(),
                confidence,
                reason: reason.into(),
            },
            metadata: BTreeMap::new(),
        }
    }

    pub fn respond(message: impl Into<String>) -> Self {
        Self {
            directive: Fast2FlowDirective::Respond {
                message: message.into(),
            },
            metadata: BTreeMap::new(),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            directive: Fast2FlowDirective::Deny {
                reason: reason.into(),
            },
            metadata: BTreeMap::new(),
        }
    }
}

/// Provider boundary for Fast2Flow-compatible intent routing.
///
/// Hosts can back this with `greentic-fast2flow`, a WASM routing component, or
/// a remote router service. Greentic-X core owns the stable request/result
/// contract; the host owns index loading, policy evaluation, and dispatch.
pub trait Fast2FlowRoutingProvider: Send + Sync {
    fn route_intent(
        &self,
        request: Fast2FlowRouteRequest,
    ) -> Result<Fast2FlowRouteResult, RuntimeError>;
}

/// Fail-fast router for hosts that have not configured Fast2Flow routing.
#[derive(Debug, Default, Clone)]
pub struct UnsupportedFast2FlowRoutingProvider;

impl Fast2FlowRoutingProvider for UnsupportedFast2FlowRoutingProvider {
    fn route_intent(
        &self,
        request: Fast2FlowRouteRequest,
    ) -> Result<Fast2FlowRouteResult, RuntimeError> {
        Err(RuntimeError::Fast2FlowRoutingFailed {
            scope: request.scope,
            message: "Fast2Flow routing provider is not configured".to_owned(),
        })
    }
}

/// Adapter for host-owned Fast2Flow routing implementations.
pub struct DelegatingFast2FlowRoutingProvider<F>
where
    F: Fn(Fast2FlowRouteRequest) -> Result<Fast2FlowRouteResult, RuntimeError> + Send + Sync,
{
    handler: F,
}

impl<F> DelegatingFast2FlowRoutingProvider<F>
where
    F: Fn(Fast2FlowRouteRequest) -> Result<Fast2FlowRouteResult, RuntimeError> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> Fast2FlowRoutingProvider for DelegatingFast2FlowRoutingProvider<F>
where
    F: Fn(Fast2FlowRouteRequest) -> Result<Fast2FlowRouteResult, RuntimeError> + Send + Sync,
{
    fn route_intent(
        &self,
        request: Fast2FlowRouteRequest,
    ) -> Result<Fast2FlowRouteResult, RuntimeError> {
        (self.handler)(request)
    }
}

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
    ComponentInvocationFailed {
        component_id: String,
        message: String,
    },
    Fast2FlowRoutingFailed {
        scope: String,
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
            Self::ComponentInvocationFailed {
                component_id,
                message,
            } => {
                write!(formatter, "component {component_id} failed: {message}")
            }
            Self::Fast2FlowRoutingFailed { scope, message } => {
                write!(
                    formatter,
                    "Fast2Flow routing failed for scope {scope}: {message}"
                )
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

    #[test]
    fn component_invocation_envelope_captures_runtime_boundary() {
        let descriptor = ComponentDescriptor::new(
            "example.analyser.rca",
            "analyser",
            ComponentRuntimeKind::WasmWasi,
            "oci://ghcr.io/greenticai/components/example-analyser-rca:latest",
        )
        .with_interface("gx.operation.descriptor.v1");

        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"evidence": []}),
            provenance(),
        )
        .with_run_id("run-1");

        assert_eq!(envelope.component_id, "example.analyser.rca");
        assert_eq!(envelope.runtime, ComponentRuntimeKind::WasmWasi);
        assert_eq!(
            envelope.interface.as_deref(),
            Some("gx.operation.descriptor.v1")
        );
        assert_eq!(envelope.run_id.as_deref(), Some("run-1"));
    }

    #[test]
    fn component_invocation_envelope_carries_resilience_and_caching_strategies() {
        let resilience = ResilienceStrategy {
            health_check: Some(HealthCheckStrategy {
                enabled: true,
                interval_ms: Some(30_000),
                timeout_ms: Some(2_000),
            }),
            retry: Some(RetryStrategy {
                max_attempts: 3,
                initial_delay_ms: Some(100),
                max_delay_ms: Some(1_000),
                backoff_multiplier: Some(2),
            }),
            success_check: Some(SuccessCheckStrategy {
                enabled: false,
                timeout_ms: None,
            }),
        };
        let caching = CachingStrategy {
            enabled: true,
            ttl_ms: Some(60_000),
            max_entries: Some(512),
            key_template: Some("${component_id}:${input.prefix}".to_owned()),
        };
        let descriptor = ComponentDescriptor::new(
            "tx.query.arbor.flows",
            "adapter",
            ComponentRuntimeKind::McpAdapter,
            "mcp://arbor/run_template",
        )
        .with_resilience(resilience.clone())
        .with_caching(caching.clone());

        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"prefix": "203.0.113.0/24"}),
            provenance(),
        );

        assert_eq!(envelope.resilience, Some(resilience));
        assert_eq!(envelope.caching, Some(caching));
    }

    #[test]
    fn unsupported_component_provider_returns_runtime_error() {
        let descriptor = ComponentDescriptor::new(
            "tx.query.arbor",
            "adapter",
            ComponentRuntimeKind::McpAdapter,
            "mcp://arbor/run_template",
        );
        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"template": "inbound_by_prefix"}),
            provenance(),
        );

        let err = UnsupportedComponentProvider
            .invoke_component(envelope)
            .expect_err("unsupported provider should return a runtime error");
        assert!(matches!(
            err,
            RuntimeError::ComponentInvocationFailed { .. }
        ));
        assert!(err.to_string().contains("tx.query.arbor"));
    }

    #[test]
    fn unsupported_fast2flow_routing_provider_returns_runtime_error() {
        let request = Fast2FlowRouteRequest::new("demo", "show inbound traffic")
            .with_mounts("/mnt/registry", "/mnt/indexes")
            .with_time_budget_ms(250);

        let err = UnsupportedFast2FlowRoutingProvider
            .route_intent(request)
            .expect_err("unsupported router should fail fast");

        assert!(matches!(err, RuntimeError::Fast2FlowRoutingFailed { .. }));
        assert!(err.to_string().contains("demo"));
    }

    #[test]
    fn delegating_fast2flow_routing_provider_forwards_request_to_host_handler() {
        let provider = DelegatingFast2FlowRoutingProvider::new(|request| {
            assert_eq!(request.scope, "tenant-a");
            assert_eq!(request.envelope.text, "show inbound traffic");
            assert_eq!(request.envelope.channel.as_deref(), Some("webchat"));
            assert_eq!(request.indexes_path, "/mnt/indexes");
            Ok(Fast2FlowRouteResult::dispatch(
                "telco-x/tx.playbook.prefix_traffic",
                0.92,
                "matched prefix traffic metadata",
            ))
        });
        let mut request = Fast2FlowRouteRequest::new("tenant-a", "show inbound traffic")
            .with_mounts("/mnt/registry", "/mnt/indexes")
            .with_session_active(false)
            .with_locale("en-GB")
            .with_now_unix_ms(1_779_000_000);
        request.envelope = request.envelope.with_channel("webchat");

        let result = provider
            .route_intent(request)
            .expect("delegating router should return host result");

        assert_eq!(
            result.directive,
            Fast2FlowDirective::Dispatch {
                target: "telco-x/tx.playbook.prefix_traffic".to_owned(),
                confidence: 0.92,
                reason: "matched prefix traffic metadata".to_owned(),
            }
        );
    }

    #[test]
    fn static_component_provider_returns_registered_output() {
        let descriptor = ComponentDescriptor::new(
            "tx.analyse.top_peers",
            "analyser",
            ComponentRuntimeKind::LocalBuiltin,
            "local://telco-x/analysers/top-peers",
        );
        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"flows": []}),
            provenance(),
        );
        let provider = StaticComponentProvider::new()
            .with_component_output("tx.analyse.top_peers", serde_json::json!({"ranked": []}));

        let result = provider
            .invoke_component(envelope)
            .expect("static provider should return output");
        assert_eq!(result.status, InvocationStatus::Succeeded);
        assert_eq!(
            result.output.expect("output should be present")["ranked"],
            serde_json::json!([])
        );
    }

    #[test]
    fn delegating_component_provider_forwards_envelope_to_host_handler() {
        let descriptor = ComponentDescriptor::new(
            "zain.analyser.rca",
            "analyser",
            ComponentRuntimeKind::WasmWasi,
            "oci://ghcr.io/greenticai/components/zain-analyser-rca:latest",
        );
        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"evidence": []}),
            provenance(),
        );
        let provider = DelegatingComponentProvider::new(|envelope| {
            assert_eq!(envelope.component_id, "zain.analyser.rca");
            assert_eq!(envelope.runtime, ComponentRuntimeKind::WasmWasi);
            Ok(ComponentInvocationResultEnvelope::success(
                envelope.invocation_id,
                envelope.component_id,
                serde_json::json!({"summary": "runner invoked"}),
            ))
        });

        let result = provider
            .invoke_component(envelope)
            .expect("delegating provider should return host output");
        assert_eq!(result.status, InvocationStatus::Succeeded);
        assert_eq!(
            result.output.expect("output should be present")["summary"],
            serde_json::json!("runner invoked")
        );
    }

    #[test]
    fn execute_component_with_strategies_retries_until_success() {
        let descriptor = ComponentDescriptor::new(
            "tx.query.arbor.flows",
            "adapter",
            ComponentRuntimeKind::McpAdapter,
            "mcp://arbor/run_template",
        )
        .with_resilience(ResilienceStrategy {
            health_check: None,
            retry: Some(RetryStrategy {
                max_attempts: 3,
                initial_delay_ms: Some(0),
                max_delay_ms: None,
                backoff_multiplier: None,
            }),
            success_check: None,
        });
        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"prefix": "203.0.113.0/24"}),
            provenance(),
        );
        let attempts = Arc::new(Mutex::new(0_u32));
        let provider_attempts = attempts.clone();
        let provider = DelegatingComponentProvider::new(move |envelope| {
            let mut count = provider_attempts
                .lock()
                .expect("attempt counter should not be poisoned");
            *count += 1;
            if *count < 3 {
                return Err(RuntimeError::ComponentInvocationFailed {
                    component_id: envelope.component_id,
                    message: "temporary failure".to_owned(),
                });
            }
            Ok(ComponentInvocationResultEnvelope::success(
                envelope.invocation_id,
                envelope.component_id,
                serde_json::json!({"ok": true}),
            ))
        });

        let outcome = execute_component_with_strategies(&provider, envelope, None)
            .expect("third attempt should succeed");

        assert_eq!(outcome.metadata.attempts, 3);
        assert_eq!(
            *attempts
                .lock()
                .expect("attempt counter should not be poisoned"),
            3
        );
        assert_eq!(outcome.result.status, InvocationStatus::Succeeded);
        assert_eq!(
            outcome.result.metadata["greentic_x.execution"]["attempts"],
            serde_json::json!(3)
        );
    }

    #[test]
    fn execute_component_with_strategies_serves_cache_hit_without_provider_call() {
        let descriptor = ComponentDescriptor::new(
            "tx.query.inventory.interfaces",
            "adapter",
            ComponentRuntimeKind::WasmWasi,
            "oci://ghcr.io/greenticai/components/inventory-interfaces:latest",
        )
        .with_caching(CachingStrategy {
            enabled: true,
            ttl_ms: Some(60_000),
            max_entries: Some(128),
            key_template: Some("${component_id}:${input.device_id}".to_owned()),
        });
        let envelope = ComponentInvocationEnvelope::new(
            "component-invoke-1",
            &descriptor,
            serde_json::json!({"device_id": "aci-pod1-node2201"}),
            provenance(),
        );
        let calls = Arc::new(Mutex::new(0_u32));
        let provider_calls = calls.clone();
        let provider = DelegatingComponentProvider::new(move |envelope| {
            *provider_calls
                .lock()
                .expect("provider counter should not be poisoned") += 1;
            Ok(ComponentInvocationResultEnvelope::success(
                envelope.invocation_id,
                envelope.component_id,
                serde_json::json!({"interfaces": ["eth1/1"]}),
            ))
        });
        let cache = InMemoryComponentCache::new();

        let first = execute_component_with_strategies(&provider, envelope.clone(), Some(&cache))
            .expect("first call should populate cache");
        let second = execute_component_with_strategies(&provider, envelope, Some(&cache))
            .expect("second call should use cache");

        assert!(!first.metadata.cache_hit);
        assert!(second.metadata.cache_hit);
        assert_eq!(
            second.metadata.cache_key.as_deref(),
            Some("tx.query.inventory.interfaces:aci-pod1-node2201")
        );
        assert_eq!(
            *calls
                .lock()
                .expect("provider counter should not be poisoned"),
            1
        );
    }

    #[test]
    fn in_memory_component_cache_respects_max_entries() {
        let cache = InMemoryComponentCache::new();
        cache.insert(
            "a".to_owned(),
            ComponentInvocationResultEnvelope::success(
                "invoke-a",
                "component",
                serde_json::json!({"n": 1}),
            ),
            None,
            Some(1),
        );
        cache.insert(
            "b".to_owned(),
            ComponentInvocationResultEnvelope::success(
                "invoke-b",
                "component",
                serde_json::json!({"n": 2}),
            ),
            None,
            Some(1),
        );

        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn in_memory_component_cache_expires_entries_by_ttl() {
        let cache = InMemoryComponentCache::new();
        cache.insert(
            "short".to_owned(),
            ComponentInvocationResultEnvelope::success(
                "component-invoke-1",
                "tx.query.inventory.interfaces",
                serde_json::json!({"interfaces": []}),
            ),
            Some(1),
            None,
        );

        std::thread::sleep(std::time::Duration::from_millis(5));

        assert!(cache.get("short").is_none());
        assert!(cache.is_empty());
    }
}
