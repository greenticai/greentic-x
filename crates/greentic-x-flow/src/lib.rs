//! Flow execution, evidence capture, and neutral view rendering for Greentic-X.
//!
//! ```rust
//! use greentic_x_flow::{
//!     FlowDefinition, FlowEngine, InMemoryEvidenceStore, JoinMode, JoinStep, MapAssignment,
//!     FlowStatus, NoopViewRenderer, OperationCallStep, OperationResult, ReturnStep, StaticFlowRuntime,
//!     Step, ValueSource,
//! };
//! use greentic_x_types::{ActorRef, OperationId, Provenance};
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut operation_results = HashMap::new();
//! operation_results.insert(
//!     "present.summary".to_owned(),
//!     OperationResult::success("invoke-1", "present.summary", json!({"summary": "ok"})),
//! );
//!
//! let flow = FlowDefinition {
//!     flow_id: "demo".to_owned(),
//!     steps: vec![
//!         Step::call(
//!             "present",
//!             OperationCallStep::new(
//!                 OperationId::new("present.summary").expect("static operation id should be valid"),
//!                 json!({"summary": "ok"}),
//!                 "present_result",
//!             ),
//!         ),
//!         Step::return_output(
//!             "return",
//!             ReturnStep::new(ValueSource::context("present_result.output")),
//!         ),
//!     ],
//! };
//! let mut engine = FlowEngine::default();
//! let mut store = InMemoryEvidenceStore::default();
//! let run = engine
//!     .execute(
//!         &flow,
//!         json!({}),
//!         Provenance::new(ActorRef::service("demo").expect("static actor id should be valid")),
//!         &mut StaticFlowRuntime::with_operations(operation_results),
//!         &mut store,
//!         &NoopViewRenderer,
//!     )
//!     .expect("flow should execute");
//! assert_eq!(run.status, FlowStatus::Succeeded);
//! assert_eq!(run.result["summary"], "ok");
//! ```

use greentic_x_runtime::{EventSink, ResourceStore, Runtime};
use greentic_x_types::{
    ContractId, InvocationStatus, OperationCallEnvelope, OperationId, OperationResultEnvelope,
    Provenance, ResolverId, ResolverQueryEnvelope, ResolverResultEnvelope, ResourceRef,
    ResourceTypeId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    Running,
    Succeeded,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepState {
    pub step_id: String,
    pub status: StepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchStatus {
    Succeeded,
    Failed,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchState {
    pub split_step_id: String,
    pub branch_id: String,
    pub status: BranchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub evidence_id: String,
    pub evidence_type: String,
    pub producer: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subject_refs: Vec<ResourceRef>,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewModel {
    pub view_id: String,
    pub view_type: String,
    pub title: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub primary_data_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowRunRecord {
    pub run_id: String,
    pub flow_id: String,
    pub status: FlowStatus,
    pub input: Value,
    pub context: Value,
    pub step_states: Vec<StepState>,
    pub branch_states: Vec<BranchState>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence_refs: Vec<String>,
    pub result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<ViewModel>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub flow_id: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub kind: StepKind,
}

impl Step {
    pub fn resolve(id: impl Into<String>, step: ResolverStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Resolve(step),
        }
    }

    pub fn call(id: impl Into<String>, step: OperationCallStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Call(step),
        }
    }

    pub fn map(id: impl Into<String>, step: MapStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Map(step),
        }
    }

    pub fn branch(id: impl Into<String>, step: BranchStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Branch(step),
        }
    }

    pub fn split(id: impl Into<String>, step: SplitStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Split(step),
        }
    }

    pub fn join(id: impl Into<String>, step: JoinStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Join(step),
        }
    }

    pub fn return_output(id: impl Into<String>, step: ReturnStep) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Return(step),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepKind {
    Resolve(ResolverStep),
    Call(OperationCallStep),
    Map(MapStep),
    Branch(BranchStep),
    Split(SplitStep),
    Join(JoinStep),
    Return(ReturnStep),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolverStep {
    pub resolver_id: ResolverId,
    pub query: ValueSource,
    pub output_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<ResourceTypeId>,
}

impl ResolverStep {
    pub fn new(resolver_id: ResolverId, query: ValueSource, output_key: impl Into<String>) -> Self {
        Self {
            resolver_id,
            query,
            output_key: output_key.into(),
            target_type: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationCallStep {
    pub operation_id: OperationId,
    pub input: ValueSource,
    pub output_key: String,
}

impl OperationCallStep {
    pub fn new(operation_id: OperationId, input: Value, output_key: impl Into<String>) -> Self {
        Self {
            operation_id,
            input: ValueSource::literal(input),
            output_key: output_key.into(),
        }
    }

    pub fn from_source(
        operation_id: OperationId,
        input: ValueSource,
        output_key: impl Into<String>,
    ) -> Self {
        Self {
            operation_id,
            input,
            output_key: output_key.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapStep {
    pub assignments: Vec<MapAssignment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapAssignment {
    pub target_key: String,
    pub value: ValueSource,
}

impl MapAssignment {
    pub fn new(target_key: impl Into<String>, value: ValueSource) -> Self {
        Self {
            target_key: target_key.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchCase {
    pub equals: Value,
    pub next_step_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchStep {
    pub source: ValueSource,
    pub cases: Vec<BranchCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_next_step_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinMode {
    All,
    Any,
    AllOrTimeout,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitBranch {
    pub branch_id: String,
    pub simulated_duration_ms: u64,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitStep {
    pub branches: Vec<SplitBranch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinStep {
    pub split_step_id: String,
    pub mode: JoinMode,
    pub output_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnStep {
    pub output: ValueSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render: Option<RenderSpec>,
}

impl ReturnStep {
    pub fn new(output: ValueSource) -> Self {
        Self {
            output,
            render: None,
        }
    }

    pub fn with_render(mut self, render: RenderSpec) -> Self {
        self.render = Some(render);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSpec {
    pub renderer_id: String,
    pub source: RenderSource,
    pub view_id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RenderSource {
    Result(ValueSource),
    EvidenceRefs,
    AllEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ValueSource {
    Literal(Value),
    Context(String),
    Input(String),
}

impl ValueSource {
    pub fn literal(value: Value) -> Self {
        Self::Literal(value)
    }

    pub fn context(path: impl Into<String>) -> Self {
        Self::Context(path.into())
    }

    pub fn input(path: impl Into<String>) -> Self {
        Self::Input(path.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationResult {
    pub envelope: OperationResultEnvelope,
    pub evidence: Vec<EvidenceItem>,
}

impl OperationResult {
    pub fn success(
        invocation_id: impl Into<String>,
        operation_id: impl Into<String>,
        output: Value,
    ) -> Self {
        let operation_id_string = operation_id.into();
        let operation_id =
            OperationId::new(operation_id_string).expect("static operation id should be valid");
        Self {
            envelope: OperationResultEnvelope {
                invocation_id: invocation_id.into(),
                operation_id,
                status: InvocationStatus::Succeeded,
                output: Some(output),
                evidence_refs: Vec::new(),
                warnings: Vec::new(),
                view_hints: Vec::new(),
            },
            evidence: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SplitExecution {
    branches: HashMap<String, BranchExecution>,
}

#[derive(Debug, Clone, PartialEq)]
struct BranchExecution {
    status: BranchStatus,
    context: Value,
    warnings: Vec<String>,
}

struct SplitRequest<'a> {
    split_step_id: &'a str,
    split: &'a SplitStep,
    input: &'a Value,
    provenance: Provenance,
}

struct BranchRequest<'a> {
    split_step_id: &'a str,
    branch: &'a SplitBranch,
    input: &'a Value,
    provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlowError {
    InvalidFlow(String),
    MissingValue(String),
    MissingStep(String),
    Resolver(String),
    Operation(String),
    Join(String),
    Render(String),
    Evidence(String),
}

impl core::fmt::Display for FlowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFlow(msg)
            | Self::MissingValue(msg)
            | Self::MissingStep(msg)
            | Self::Resolver(msg)
            | Self::Operation(msg)
            | Self::Join(msg)
            | Self::Render(msg)
            | Self::Evidence(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for FlowError {}

pub trait EvidenceStore {
    fn put(&mut self, item: EvidenceItem) -> Result<ResourceRef, FlowError>;
    fn get(&self, evidence_id: &str) -> Result<Option<EvidenceItem>, FlowError>;
    fn list(&self) -> Result<Vec<EvidenceItem>, FlowError>;
}

#[derive(Default)]
pub struct InMemoryEvidenceStore {
    items: HashMap<String, EvidenceItem>,
}

impl EvidenceStore for InMemoryEvidenceStore {
    fn put(&mut self, item: EvidenceItem) -> Result<ResourceRef, FlowError> {
        let evidence_id = item.evidence_id.clone();
        self.items.insert(evidence_id.clone(), item);
        Ok(ResourceRef::new(
            ContractId::new("gx.evidence").map_err(|err| FlowError::Evidence(err.to_string()))?,
            ResourceTypeId::new("evidence").map_err(|err| FlowError::Evidence(err.to_string()))?,
            greentic_x_types::ResourceId::new(evidence_id)
                .map_err(|err| FlowError::Evidence(err.to_string()))?,
        ))
    }

    fn get(&self, evidence_id: &str) -> Result<Option<EvidenceItem>, FlowError> {
        Ok(self.items.get(evidence_id).cloned())
    }

    fn list(&self) -> Result<Vec<EvidenceItem>, FlowError> {
        let mut items = self.items.values().cloned().collect::<Vec<_>>();
        items.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
        Ok(items)
    }
}

pub trait FlowRuntime {
    fn resolve(
        &mut self,
        envelope: ResolverQueryEnvelope,
        invocation_id: &str,
    ) -> Result<ResolverResultEnvelope, FlowError>;
    fn call_operation(
        &mut self,
        envelope: OperationCallEnvelope,
    ) -> Result<OperationResult, FlowError>;
}

pub trait ViewRenderer {
    fn render(
        &self,
        render: &RenderSpec,
        payload: ViewRenderPayload,
    ) -> Result<ViewModel, FlowError>;
}

pub struct NoopViewRenderer;

impl ViewRenderer for NoopViewRenderer {
    fn render(
        &self,
        render: &RenderSpec,
        payload: ViewRenderPayload,
    ) -> Result<ViewModel, FlowError> {
        Ok(ViewModel {
            view_id: render.view_id.clone(),
            view_type: "summary".to_owned(),
            title: render.title.clone(),
            summary: render.summary.clone(),
            primary_data_refs: payload.primary_refs,
            body: Some(payload.body),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewRenderPayload {
    pub primary_refs: Vec<String>,
    pub body: Value,
}

pub struct RuntimeFlowAdapter<'a, S, E> {
    runtime: &'a mut Runtime<S, E>,
}

impl<'a, S, E> RuntimeFlowAdapter<'a, S, E> {
    pub fn new(runtime: &'a mut Runtime<S, E>) -> Self {
        Self { runtime }
    }
}

impl<'a, S, E> FlowRuntime for RuntimeFlowAdapter<'a, S, E>
where
    S: ResourceStore,
    E: EventSink,
{
    fn resolve(
        &mut self,
        envelope: ResolverQueryEnvelope,
        invocation_id: &str,
    ) -> Result<ResolverResultEnvelope, FlowError> {
        self.runtime
            .resolve(envelope, invocation_id)
            .map_err(|err| FlowError::Resolver(err.to_string()))
    }

    fn call_operation(
        &mut self,
        envelope: OperationCallEnvelope,
    ) -> Result<OperationResult, FlowError> {
        let envelope_result = self
            .runtime
            .invoke_operation_enveloped(envelope)
            .map_err(|err| FlowError::Operation(err.to_string()))?;
        Ok(OperationResult {
            envelope: envelope_result,
            evidence: Vec::new(),
        })
    }
}

#[derive(Default)]
pub struct StaticFlowRuntime {
    operation_results: HashMap<String, OperationResult>,
    resolver_results: HashMap<String, ResolverResultEnvelope>,
}

impl StaticFlowRuntime {
    pub fn with_operations(operation_results: HashMap<String, OperationResult>) -> Self {
        Self {
            operation_results,
            resolver_results: HashMap::new(),
        }
    }

    pub fn with_resolvers(resolver_results: HashMap<String, ResolverResultEnvelope>) -> Self {
        Self {
            operation_results: HashMap::new(),
            resolver_results,
        }
    }

    pub fn insert_operation(&mut self, operation_id: impl Into<String>, result: OperationResult) {
        self.operation_results.insert(operation_id.into(), result);
    }

    pub fn insert_resolver(
        &mut self,
        resolver_id: impl Into<String>,
        result: ResolverResultEnvelope,
    ) {
        self.resolver_results.insert(resolver_id.into(), result);
    }
}

impl FlowRuntime for StaticFlowRuntime {
    fn resolve(
        &mut self,
        envelope: ResolverQueryEnvelope,
        _invocation_id: &str,
    ) -> Result<ResolverResultEnvelope, FlowError> {
        self.resolver_results
            .get(envelope.resolver_id.as_str())
            .cloned()
            .ok_or_else(|| {
                FlowError::Resolver(format!("missing resolver {}", envelope.resolver_id))
            })
    }

    fn call_operation(
        &mut self,
        envelope: OperationCallEnvelope,
    ) -> Result<OperationResult, FlowError> {
        self.operation_results
            .get(envelope.operation_id.as_str())
            .cloned()
            .ok_or_else(|| {
                FlowError::Operation(format!("missing operation {}", envelope.operation_id))
            })
    }
}

#[derive(Default)]
pub struct FlowEngine {
    run_counter: u64,
}

impl FlowEngine {
    pub fn execute<R, ES, VR>(
        &mut self,
        flow: &FlowDefinition,
        input: Value,
        provenance: Provenance,
        runtime: &mut R,
        evidence_store: &mut ES,
        renderer: &VR,
    ) -> Result<FlowRunRecord, FlowError>
    where
        R: FlowRuntime,
        ES: EvidenceStore,
        VR: ViewRenderer,
    {
        let run_id = self.allocate_run_id();
        let mut step_states = flow
            .steps
            .iter()
            .map(|step| StepState {
                step_id: step.id.clone(),
                status: StepStatus::Pending,
                message: None,
            })
            .collect::<Vec<_>>();
        let mut branch_states = Vec::new();
        let mut warnings = Vec::new();
        let mut evidence_refs = Vec::new();
        let mut context = Value::Object(Map::new());
        let mut split_results: HashMap<String, SplitExecution> = HashMap::new();
        let mut result = Value::Null;
        let mut view = None;

        let step_index = build_step_index(&flow.steps)?;
        let mut index = 0usize;

        while index < flow.steps.len() {
            let step = &flow.steps[index];
            let step_state = &mut step_states[index];
            step_state.status = StepStatus::Running;

            let next = match &step.kind {
                StepKind::Resolve(resolve) => {
                    let mut envelope = ResolverQueryEnvelope::new(
                        resolve.resolver_id.clone(),
                        resolve.query.resolve(&input, &context)?,
                        provenance.clone(),
                    );
                    if let Some(target_type) = resolve.target_type.clone() {
                        envelope = envelope.with_target_type(target_type);
                    }
                    let resolved = runtime.resolve(envelope, &format!("{run_id}-{}", step.id))?;
                    set_context_key(
                        &mut context,
                        &resolve.output_key,
                        serde_json::to_value(resolved)
                            .map_err(|err| FlowError::Resolver(err.to_string()))?,
                    )?;
                    step_state.status = StepStatus::Succeeded;
                    index + 1
                }
                StepKind::Call(call) => {
                    let envelope = OperationCallEnvelope::new(
                        format!("{run_id}-{}", step.id),
                        call.operation_id.clone(),
                        call.input.resolve(&input, &context)?,
                        provenance.clone(),
                    )
                    .with_run_id(run_id.clone());
                    let result_value = runtime.call_operation(envelope)?;
                    let refs = store_evidence_items(
                        &result_value.evidence,
                        evidence_store,
                        &mut evidence_refs,
                    )?;
                    let mut envelope_value = serde_json::to_value(&result_value.envelope)
                        .map_err(|err| FlowError::Operation(err.to_string()))?;
                    if !refs.is_empty() {
                        envelope_value["evidence_refs"] = serde_json::to_value(refs)
                            .map_err(|err| FlowError::Operation(err.to_string()))?;
                    }
                    set_context_key(&mut context, &call.output_key, envelope_value)?;
                    step_state.status = StepStatus::Succeeded;
                    index + 1
                }
                StepKind::Map(map) => {
                    for assignment in &map.assignments {
                        let value = assignment.value.resolve(&input, &context)?;
                        set_context_key(&mut context, &assignment.target_key, value)?;
                    }
                    step_state.status = StepStatus::Succeeded;
                    index + 1
                }
                StepKind::Branch(branch) => {
                    let branch_value = branch.source.resolve(&input, &context)?;
                    if let Some(case) = branch.cases.iter().find(|case| case.equals == branch_value)
                    {
                        step_state.status = StepStatus::Succeeded;
                        *step_index
                            .get(&case.next_step_id)
                            .ok_or_else(|| FlowError::MissingStep(case.next_step_id.clone()))?
                    } else if let Some(default_step) = &branch.default_next_step_id {
                        step_state.status = StepStatus::Succeeded;
                        *step_index
                            .get(default_step)
                            .ok_or_else(|| FlowError::MissingStep(default_step.clone()))?
                    } else {
                        step_state.status = StepStatus::Failed;
                        return Err(FlowError::InvalidFlow(format!(
                            "branch step {} did not match and has no default",
                            step.id
                        )));
                    }
                }
                StepKind::Split(split) => {
                    let split_result = self.execute_split(
                        SplitRequest {
                            split_step_id: &step.id,
                            split,
                            input: &input,
                            provenance: provenance.clone(),
                        },
                        runtime,
                        evidence_store,
                    )?;
                    for (branch_id, branch) in &split_result.branches {
                        branch_states.push(BranchState {
                            split_step_id: step.id.clone(),
                            branch_id: branch_id.clone(),
                            status: branch.status,
                            message: branch.warnings.first().cloned(),
                        });
                    }
                    split_results.insert(step.id.clone(), split_result);
                    step_state.status = StepStatus::Succeeded;
                    index + 1
                }
                StepKind::Join(join) => {
                    let merged = merge_split_result(
                        split_results.get(&join.split_step_id).ok_or_else(|| {
                            FlowError::Join(format!("missing split {}", join.split_step_id))
                        })?,
                        join,
                    )?;
                    if !merged.warnings.is_empty() {
                        warnings.extend(merged.warnings.clone());
                    }
                    set_context_key(&mut context, &join.output_key, merged.output)?;
                    step_state.status = if merged.partial {
                        StepStatus::TimedOut
                    } else {
                        StepStatus::Succeeded
                    };
                    index + 1
                }
                StepKind::Return(return_step) => {
                    result = return_step.output.resolve(&input, &context)?;
                    if let Some(render_spec) = &return_step.render {
                        let payload = build_render_payload(
                            render_spec,
                            &result,
                            &evidence_refs,
                            evidence_store,
                        )?;
                        view = Some(renderer.render(render_spec, payload)?);
                    }
                    step_state.status = StepStatus::Succeeded;
                    flow.steps.len()
                }
            };

            index = next;
        }

        let status = if warnings.is_empty() {
            FlowStatus::Succeeded
        } else {
            FlowStatus::Partial
        };

        Ok(FlowRunRecord {
            run_id,
            flow_id: flow.flow_id.clone(),
            status,
            input,
            context,
            step_states,
            branch_states,
            evidence_refs,
            result,
            view,
            warnings,
        })
    }

    fn execute_split<R, ES>(
        &mut self,
        request: SplitRequest<'_>,
        runtime: &mut R,
        evidence_store: &mut ES,
    ) -> Result<SplitExecution, FlowError>
    where
        R: FlowRuntime,
        ES: EvidenceStore,
    {
        let mut branches = HashMap::new();
        for branch in &request.split.branches {
            let branch_context = self.execute_branch_steps(
                BranchRequest {
                    split_step_id: request.split_step_id,
                    branch,
                    input: request.input,
                    provenance: request.provenance.clone(),
                },
                runtime,
                evidence_store,
            )?;
            branches.insert(branch.branch_id.clone(), branch_context);
        }
        Ok(SplitExecution { branches })
    }

    fn execute_branch_steps<R, ES>(
        &mut self,
        request: BranchRequest<'_>,
        runtime: &mut R,
        evidence_store: &mut ES,
    ) -> Result<BranchExecution, FlowError>
    where
        R: FlowRuntime,
        ES: EvidenceStore,
    {
        let timeout = request.branch.simulated_duration_ms > 0;
        if timeout {
            return Ok(BranchExecution {
                status: BranchStatus::TimedOut,
                context: json!({
                    "branch_id": request.branch.branch_id,
                    "status": "timed_out"
                }),
                warnings: vec![format!(
                    "branch {} in split {} exceeded simulated duration {}ms",
                    request.branch.branch_id,
                    request.split_step_id,
                    request.branch.simulated_duration_ms
                )],
            });
        }

        let branch_flow = FlowDefinition {
            flow_id: format!("{}.{}", request.split_step_id, request.branch.branch_id),
            steps: request.branch.steps.clone(),
        };
        let run = self.execute(
            &branch_flow,
            request.input.clone(),
            request.provenance,
            runtime,
            evidence_store,
            &NoopViewRenderer,
        )?;
        let status = match run.status {
            FlowStatus::Succeeded | FlowStatus::Partial => BranchStatus::Succeeded,
            FlowStatus::Failed => BranchStatus::Failed,
            FlowStatus::Running => BranchStatus::Failed,
        };
        Ok(BranchExecution {
            status,
            context: run.context,
            warnings: run.warnings,
        })
    }

    fn allocate_run_id(&mut self) -> String {
        self.run_counter += 1;
        format!("run-{}", self.run_counter)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct JoinMerge {
    output: Value,
    warnings: Vec<String>,
    partial: bool,
}

fn merge_split_result(split: &SplitExecution, join: &JoinStep) -> Result<JoinMerge, FlowError> {
    let mut output = Map::new();
    let mut warnings = Vec::new();
    let mut succeeded = 0usize;
    let mut partial = false;

    for (branch_id, branch) in &split.branches {
        match branch.status {
            BranchStatus::Succeeded => {
                succeeded += 1;
                output.insert(branch_id.clone(), branch.context.clone());
            }
            BranchStatus::TimedOut => {
                partial = true;
                warnings.extend(branch.warnings.clone());
            }
            BranchStatus::Failed => {
                warnings.extend(branch.warnings.clone());
            }
        }
    }

    match join.mode {
        JoinMode::All => {
            if succeeded != split.branches.len() {
                return Err(FlowError::Join(format!(
                    "join {} expected all branches to succeed",
                    join.split_step_id
                )));
            }
        }
        JoinMode::Any => {
            if succeeded == 0 {
                return Err(FlowError::Join(format!(
                    "join {} expected at least one successful branch",
                    join.split_step_id
                )));
            }
        }
        JoinMode::AllOrTimeout => {
            if succeeded == 0 && !partial {
                return Err(FlowError::Join(format!(
                    "join {} expected successful or timed out branches",
                    join.split_step_id
                )));
            }
        }
    }

    Ok(JoinMerge {
        output: Value::Object(output),
        warnings,
        partial,
    })
}

fn store_evidence_items<ES: EvidenceStore>(
    items: &[EvidenceItem],
    store: &mut ES,
    evidence_refs: &mut Vec<String>,
) -> Result<Vec<ResourceRef>, FlowError> {
    let mut refs = Vec::new();
    for item in items {
        let reference = store.put(item.clone())?;
        evidence_refs.push(reference.resource_id.as_str().to_owned());
        refs.push(reference);
    }
    Ok(refs)
}

fn build_render_payload<ES: EvidenceStore>(
    render: &RenderSpec,
    result: &Value,
    evidence_refs: &[String],
    evidence_store: &ES,
) -> Result<ViewRenderPayload, FlowError> {
    match &render.source {
        RenderSource::Result(source) => Ok(ViewRenderPayload {
            primary_refs: Vec::new(),
            body: source.resolve(&Value::Null, result)?,
        }),
        RenderSource::EvidenceRefs => Ok(ViewRenderPayload {
            primary_refs: evidence_refs.to_vec(),
            body: serde_json::to_value(evidence_refs)
                .map_err(|err| FlowError::Render(err.to_string()))?,
        }),
        RenderSource::AllEvidence => {
            let items = evidence_store.list()?;
            let refs = items
                .iter()
                .map(|item| item.evidence_id.clone())
                .collect::<Vec<_>>();
            Ok(ViewRenderPayload {
                primary_refs: refs,
                body: serde_json::to_value(items)
                    .map_err(|err| FlowError::Render(err.to_string()))?,
            })
        }
    }
}

fn build_step_index(steps: &[Step]) -> Result<HashMap<String, usize>, FlowError> {
    let mut index = HashMap::new();
    for (offset, step) in steps.iter().enumerate() {
        if index.insert(step.id.clone(), offset).is_some() {
            return Err(FlowError::InvalidFlow(format!(
                "duplicate step id {}",
                step.id
            )));
        }
    }
    Ok(index)
}

fn set_context_key(context: &mut Value, key: &str, value: Value) -> Result<(), FlowError> {
    let object = context
        .as_object_mut()
        .ok_or_else(|| FlowError::InvalidFlow("flow context must be a JSON object".to_owned()))?;
    object.insert(key.to_owned(), value);
    Ok(())
}

impl ValueSource {
    fn resolve(&self, input: &Value, context: &Value) -> Result<Value, FlowError> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Context(path) => get_path(context, path),
            Self::Input(path) => get_path(input, path),
        }
    }
}

fn get_path(root: &Value, path: &str) -> Result<Value, FlowError> {
    let mut current = root;
    for token in path.split('.') {
        if token.is_empty() {
            continue;
        }
        current = current
            .get(token)
            .ok_or_else(|| FlowError::MissingValue(path.to_owned()))?;
    }
    Ok(current.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use greentic_x_types::{ActorRef, PatchOperation, ResolverStatus, ResourceId, Revision};

    fn provenance() -> Provenance {
        Provenance::new(ActorRef::service("flow-engine").expect("static actor id should be valid"))
    }

    struct MemoryRenderer;

    impl ViewRenderer for MemoryRenderer {
        fn render(
            &self,
            render: &RenderSpec,
            payload: ViewRenderPayload,
        ) -> Result<ViewModel, FlowError> {
            Ok(ViewModel {
                view_id: render.view_id.clone(),
                view_type: "summary".to_owned(),
                title: render.title.clone(),
                summary: render.summary.clone(),
                primary_data_refs: payload.primary_refs,
                body: Some(payload.body),
            })
        }
    }

    #[test]
    fn executes_sequential_flow_success() {
        let mut runtime = StaticFlowRuntime::default();
        runtime.insert_operation(
            "present.summary",
            OperationResult::success("invoke-1", "present.summary", json!({"summary": "ok"})),
        );

        let flow = FlowDefinition {
            flow_id: "sequential".to_owned(),
            steps: vec![
                Step::map(
                    "prepare",
                    MapStep {
                        assignments: vec![MapAssignment::new(
                            "summary_input",
                            ValueSource::literal(json!({"summary": "ok"})),
                        )],
                    },
                ),
                Step::call(
                    "present",
                    OperationCallStep::from_source(
                        OperationId::new("present.summary")
                            .expect("static operation id should be valid"),
                        ValueSource::context("summary_input"),
                        "present_result",
                    ),
                ),
                Step::return_output(
                    "return",
                    ReturnStep::new(ValueSource::context("present_result.output.summary")),
                ),
            ],
        };

        let mut engine = FlowEngine::default();
        let mut evidence = InMemoryEvidenceStore::default();
        let run = engine
            .execute(
                &flow,
                json!({}),
                provenance(),
                &mut runtime,
                &mut evidence,
                &NoopViewRenderer,
            )
            .expect("flow should execute");

        assert_eq!(run.status, FlowStatus::Succeeded);
        assert_eq!(run.result, json!("ok"));
    }

    #[test]
    fn executes_branch_selection() {
        let flow = FlowDefinition {
            flow_id: "branch".to_owned(),
            steps: vec![
                Step::branch(
                    "choose",
                    BranchStep {
                        source: ValueSource::input("mode"),
                        cases: vec![BranchCase {
                            equals: json!("a"),
                            next_step_id: "map_a".to_owned(),
                        }],
                        default_next_step_id: Some("map_b".to_owned()),
                    },
                ),
                Step::map(
                    "map_a",
                    MapStep {
                        assignments: vec![MapAssignment::new(
                            "result",
                            ValueSource::literal(json!("branch-a")),
                        )],
                    },
                ),
                Step::return_output("return_a", ReturnStep::new(ValueSource::context("result"))),
                Step::map(
                    "map_b",
                    MapStep {
                        assignments: vec![MapAssignment::new(
                            "result",
                            ValueSource::literal(json!("branch-b")),
                        )],
                    },
                ),
                Step::return_output("return_b", ReturnStep::new(ValueSource::context("result"))),
            ],
        };

        let mut engine = FlowEngine::default();
        let mut evidence = InMemoryEvidenceStore::default();
        let run = engine
            .execute(
                &flow,
                json!({"mode": "a"}),
                provenance(),
                &mut StaticFlowRuntime::default(),
                &mut evidence,
                &NoopViewRenderer,
            )
            .expect("flow should execute");

        assert_eq!(run.result, json!("branch-a"));
    }

    #[test]
    fn executes_split_join_all() {
        let flow = FlowDefinition {
            flow_id: "join-all".to_owned(),
            steps: vec![
                Step::split(
                    "split",
                    SplitStep {
                        branches: vec![
                            SplitBranch {
                                branch_id: "health".to_owned(),
                                simulated_duration_ms: 0,
                                steps: vec![
                                    Step::map(
                                        "set_health",
                                        MapStep {
                                            assignments: vec![MapAssignment::new(
                                                "value",
                                                ValueSource::literal(json!("healthy")),
                                            )],
                                        },
                                    ),
                                    Step::return_output(
                                        "return_health",
                                        ReturnStep::new(ValueSource::context("value")),
                                    ),
                                ],
                            },
                            SplitBranch {
                                branch_id: "attribution".to_owned(),
                                simulated_duration_ms: 0,
                                steps: vec![
                                    Step::map(
                                        "set_attr",
                                        MapStep {
                                            assignments: vec![MapAssignment::new(
                                                "value",
                                                ValueSource::literal(json!("change-window")),
                                            )],
                                        },
                                    ),
                                    Step::return_output(
                                        "return_attr",
                                        ReturnStep::new(ValueSource::context("value")),
                                    ),
                                ],
                            },
                        ],
                    },
                ),
                Step::join(
                    "join",
                    JoinStep {
                        split_step_id: "split".to_owned(),
                        mode: JoinMode::All,
                        output_key: "joined".to_owned(),
                        timeout_ms: None,
                    },
                ),
                Step::return_output("return", ReturnStep::new(ValueSource::context("joined"))),
            ],
        };
        let mut engine = FlowEngine::default();
        let mut evidence = InMemoryEvidenceStore::default();
        let run = engine
            .execute(
                &flow,
                json!({}),
                provenance(),
                &mut StaticFlowRuntime::default(),
                &mut evidence,
                &NoopViewRenderer,
            )
            .expect("flow should execute");

        assert_eq!(run.status, FlowStatus::Succeeded);
        assert_eq!(run.result["health"]["value"], "healthy");
        assert_eq!(run.result["attribution"]["value"], "change-window");
    }

    #[test]
    fn executes_split_timeout_with_warning() {
        let flow = FlowDefinition {
            flow_id: "join-timeout".to_owned(),
            steps: vec![
                Step::split(
                    "split",
                    SplitStep {
                        branches: vec![
                            SplitBranch {
                                branch_id: "fast".to_owned(),
                                simulated_duration_ms: 0,
                                steps: vec![
                                    Step::map(
                                        "set_fast",
                                        MapStep {
                                            assignments: vec![MapAssignment::new(
                                                "value",
                                                ValueSource::literal(json!("done")),
                                            )],
                                        },
                                    ),
                                    Step::return_output(
                                        "return_fast",
                                        ReturnStep::new(ValueSource::context("value")),
                                    ),
                                ],
                            },
                            SplitBranch {
                                branch_id: "slow".to_owned(),
                                simulated_duration_ms: 10,
                                steps: vec![],
                            },
                        ],
                    },
                ),
                Step::join(
                    "join",
                    JoinStep {
                        split_step_id: "split".to_owned(),
                        mode: JoinMode::AllOrTimeout,
                        output_key: "joined".to_owned(),
                        timeout_ms: Some(5),
                    },
                ),
                Step::return_output("return", ReturnStep::new(ValueSource::context("joined"))),
            ],
        };

        let mut engine = FlowEngine::default();
        let mut evidence = InMemoryEvidenceStore::default();
        let run = engine
            .execute(
                &flow,
                json!({}),
                provenance(),
                &mut StaticFlowRuntime::default(),
                &mut evidence,
                &NoopViewRenderer,
            )
            .expect("flow should execute");

        assert_eq!(run.status, FlowStatus::Partial);
        assert_eq!(run.result["fast"]["value"], "done");
        assert!(!run.warnings.is_empty());
    }

    #[test]
    fn propagates_evidence_and_generates_view() {
        let mut runtime = StaticFlowRuntime::default();
        runtime.insert_operation(
            "analyse.threshold",
            OperationResult {
                envelope: OperationResultEnvelope {
                    invocation_id: "invoke-1".to_owned(),
                    operation_id: OperationId::new("analyse.threshold")
                        .expect("static operation id should be valid"),
                    status: InvocationStatus::Succeeded,
                    output: Some(json!({"score": 0.31})),
                    evidence_refs: Vec::new(),
                    warnings: Vec::new(),
                    view_hints: vec!["summary".to_owned()],
                },
                evidence: vec![EvidenceItem {
                    evidence_id: "evidence-17".to_owned(),
                    evidence_type: "summary-stats".to_owned(),
                    producer: "analyse.threshold".to_owned(),
                    timestamp: "2026-03-07T12:00:00Z".to_owned(),
                    subject_refs: vec![ResourceRef::new(
                        ContractId::new("gx.case").expect("static contract id should be valid"),
                        ResourceTypeId::new("case").expect("static resource type should be valid"),
                        ResourceId::new("case-42").expect("static resource id should be valid"),
                    )],
                    summary: "Threshold exceeded".to_owned(),
                    payload: Some(json!({"score": 0.31})),
                }],
            },
        );

        let flow = FlowDefinition {
            flow_id: "evidence-view".to_owned(),
            steps: vec![
                Step::call(
                    "analyse",
                    OperationCallStep::new(
                        OperationId::new("analyse.threshold")
                            .expect("static operation id should be valid"),
                        json!({"metric": "drop_rate"}),
                        "analysis",
                    ),
                ),
                Step::return_output(
                    "return",
                    ReturnStep::new(ValueSource::context("analysis.output")).with_render(
                        RenderSpec {
                            renderer_id: "summary".to_owned(),
                            source: RenderSource::AllEvidence,
                            view_id: "view-1".to_owned(),
                            title: "Threshold View".to_owned(),
                            summary: "Generated from evidence".to_owned(),
                        },
                    ),
                ),
            ],
        };

        let mut engine = FlowEngine::default();
        let mut evidence = InMemoryEvidenceStore::default();
        let run = engine
            .execute(
                &flow,
                json!({}),
                provenance(),
                &mut runtime,
                &mut evidence,
                &MemoryRenderer,
            )
            .expect("flow should execute");

        assert_eq!(run.evidence_refs, vec!["evidence-17".to_owned()]);
        assert_eq!(
            evidence
                .get("evidence-17")
                .expect("store access should succeed")
                .expect("evidence should be present")
                .summary,
            "Threshold exceeded"
        );
        assert_eq!(
            run.view.expect("view should be present").primary_data_refs,
            vec!["evidence-17".to_owned()]
        );
    }

    #[test]
    fn runtime_adapter_calls_current_runtime() {
        let mut runtime = greentic_x_runtime::Runtime::new(
            greentic_x_runtime::InMemoryResourceStore::default(),
            greentic_x_runtime::NoopEventSink,
        );
        let resolver_descriptor = greentic_x_types::ResolverDescriptor {
            resolver_id: ResolverId::new("resolve.by_name")
                .expect("static resolver id should be valid"),
            description: "Resolve by name".to_owned(),
            target_type: Some(
                ResourceTypeId::new("case").expect("static resource type should be valid"),
            ),
            tags: Vec::new(),
        };
        runtime
            .install_resolver(
                resolver_descriptor,
                std::sync::Arc::new(greentic_x_runtime::StaticResolverHandler::new(Ok(
                    ResolverResultEnvelope {
                        resolver_id: ResolverId::new("resolve.by_name")
                            .expect("static resolver id should be valid"),
                        status: ResolverStatus::Resolved,
                        selected: Some(greentic_x_types::ResolverCandidate {
                            resource: ResourceRef::new(
                                ContractId::new("gx.case")
                                    .expect("static contract id should be valid"),
                                ResourceTypeId::new("case")
                                    .expect("static resource type should be valid"),
                                ResourceId::new("case-42")
                                    .expect("static resource id should be valid"),
                            ),
                            display: Some("Case 42".to_owned()),
                            confidence: Some(1.0),
                            metadata: None,
                        }),
                        candidates: Vec::new(),
                        warnings: Vec::new(),
                    },
                ))),
                provenance(),
            )
            .expect("resolver install should succeed");
        runtime
            .install_operation(
                greentic_x_ops::OperationManifest {
                    operation_id: OperationId::new("query.resource")
                        .expect("static operation id should be valid"),
                    version: greentic_x_types::ContractVersion::new("v1")
                        .expect("static version should be valid"),
                    description: "Query resource".to_owned(),
                    input_schema: greentic_x_types::SchemaReference::new(
                        "gx.operation.input",
                        greentic_x_types::ContractVersion::new("v1")
                            .expect("static version should be valid"),
                    )
                    .expect("static schema ref should be valid"),
                    output_schema: greentic_x_types::SchemaReference::new(
                        "gx.operation.output",
                        greentic_x_types::ContractVersion::new("v1")
                            .expect("static version should be valid"),
                    )
                    .expect("static schema ref should be valid"),
                    compatibility: Vec::new(),
                    supported_contracts: Vec::new(),
                    permissions: Vec::new(),
                    examples: Vec::new(),
                },
                std::sync::Arc::new(greentic_x_runtime::StaticOperationHandler::new(Ok(
                    json!({"found": true}),
                ))),
                provenance(),
            )
            .expect("operation install should succeed");

        let mut adapter = RuntimeFlowAdapter::new(&mut runtime);
        let resolve = adapter
            .resolve(
                ResolverQueryEnvelope::new(
                    ResolverId::new("resolve.by_name").expect("static resolver id should be valid"),
                    json!({"name": "Case 42"}),
                    provenance(),
                ),
                "resolve-1",
            )
            .expect("resolve should succeed");
        let call = adapter
            .call_operation(OperationCallEnvelope::new(
                "invoke-1",
                OperationId::new("query.resource").expect("static operation id should be valid"),
                json!({"resource_id": "case-42"}),
                provenance(),
            ))
            .expect("call should succeed");

        assert_eq!(resolve.status, ResolverStatus::Resolved);
        assert_eq!(
            call.envelope.output.expect("output is present")["found"],
            true
        );
    }

    #[test]
    fn example_patch_operation_import_kept_alive() {
        let patch = PatchOperation::replace("/title", json!("value"));
        assert_eq!(patch.path, "/title");
        assert_eq!(Revision::new(1).next(), Revision::new(2));
    }
}
