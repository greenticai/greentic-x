use greentic_x_flow::{
    FlowDefinition, JoinMode, JoinStep, OperationCallStep, RenderSource, RenderSpec, ResolverStep,
    ReturnStep, SplitBranch, SplitStep, Step, ValueSource,
};
use greentic_x_types::{OperationId, ResolverId};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ObservabilityPlaybookProfile {
    pub profile_id: String,
    pub resolver: String,
    #[serde(default)]
    pub query_ops: Vec<String>,
    #[serde(default)]
    pub analysis_ops: Vec<String>,
    pub present_op: String,
    #[serde(default)]
    pub split_join: Option<SplitJoinProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SplitJoinProfile {
    pub branches: Vec<SplitJoinBranch>,
    #[serde(default = "default_join_mode")]
    pub join_mode: JoinMode,
    #[serde(default = "default_join_output_key")]
    pub join_output_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SplitJoinBranch {
    pub branch_id: String,
    #[serde(default)]
    pub query_ops: Vec<String>,
    #[serde(default)]
    pub analysis_ops: Vec<String>,
    #[serde(default)]
    pub simulated_duration_ms: u64,
}

fn default_join_mode() -> JoinMode {
    JoinMode::All
}

fn default_join_output_key() -> String {
    "merged".to_owned()
}

pub(crate) fn read_profile(path: &Path) -> Result<ObservabilityPlaybookProfile, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&data).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub(crate) fn validate_profile(profile: &ObservabilityPlaybookProfile) -> Vec<String> {
    let mut issues = Vec::new();
    if profile.profile_id.trim().is_empty() {
        issues.push("profile_id must not be empty".to_owned());
    }
    if ResolverId::new(profile.resolver.clone()).is_err() {
        issues.push(format!(
            "resolver {} is not a valid identifier",
            profile.resolver
        ));
    }
    for operation_id in profile.query_ops.iter().chain(profile.analysis_ops.iter()) {
        if OperationId::new(operation_id.clone()).is_err() {
            issues.push(format!(
                "operation {} is not a valid identifier",
                operation_id
            ));
        }
    }
    if OperationId::new(profile.present_op.clone()).is_err() {
        issues.push(format!(
            "present_op {} is not a valid identifier",
            profile.present_op
        ));
    }
    if profile.query_ops.is_empty() && profile.split_join.is_none() {
        issues.push("query_ops must not be empty when split_join is absent".to_owned());
    }
    if profile.analysis_ops.is_empty() && profile.split_join.is_none() {
        issues.push("analysis_ops must not be empty when split_join is absent".to_owned());
    }
    if let Some(split_join) = &profile.split_join {
        if split_join.branches.len() < 2 {
            issues.push("split_join must define at least two branches".to_owned());
        }
        for branch in &split_join.branches {
            if branch.branch_id.trim().is_empty() {
                issues.push("split_join branch_id must not be empty".to_owned());
            }
            if branch.query_ops.is_empty() {
                issues.push(format!(
                    "split_join branch {} must define at least one query op",
                    branch.branch_id
                ));
            }
            if branch.analysis_ops.is_empty() {
                issues.push(format!(
                    "split_join branch {} must define at least one analysis op",
                    branch.branch_id
                ));
            }
            for operation_id in branch.query_ops.iter().chain(branch.analysis_ops.iter()) {
                if OperationId::new(operation_id.clone()).is_err() {
                    issues.push(format!(
                        "split_join branch {} contains invalid operation {}",
                        branch.branch_id, operation_id
                    ));
                }
            }
        }
    }
    issues
}

pub(crate) fn compile_profile(
    profile: &ObservabilityPlaybookProfile,
) -> Result<FlowDefinition, String> {
    let issues = validate_profile(profile);
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }

    let resolver_id =
        ResolverId::new(profile.resolver.clone()).map_err(|err| format!("resolver id: {err}"))?;
    let present_op = OperationId::new(profile.present_op.clone())
        .map_err(|err| format!("present op id {}: {err}", profile.present_op))?;

    let mut steps = Vec::new();
    steps.push(Step::resolve(
        "resolve",
        ResolverStep::new(resolver_id, ValueSource::input("query"), "resolved"),
    ));

    if let Some(split_join) = &profile.split_join {
        let mut branches = Vec::new();
        for branch in &split_join.branches {
            let branch_steps = compile_linear_steps(
                &branch.query_ops,
                &branch.analysis_ops,
                ValueSource::input("query"),
                &format!("branch_{}", branch.branch_id),
            )?;
            let final_output_key = last_output_key(&branch_steps).ok_or_else(|| {
                format!(
                    "split_join branch {} does not produce an output key",
                    branch.branch_id
                )
            })?;
            let mut branch_steps = branch_steps;
            branch_steps.push(Step::return_output(
                format!("return_{}", branch.branch_id),
                ReturnStep::new(ValueSource::context(final_output_key)),
            ));
            branches.push(SplitBranch {
                branch_id: branch.branch_id.clone(),
                simulated_duration_ms: branch.simulated_duration_ms,
                steps: branch_steps,
            });
        }
        steps.push(Step::split("split", SplitStep { branches }));
        steps.push(Step::join(
            "join",
            JoinStep {
                split_step_id: "split".to_owned(),
                mode: split_join.join_mode,
                output_key: split_join.join_output_key.clone(),
                timeout_ms: None,
            },
        ));
        steps.push(Step::call(
            "present",
            OperationCallStep::from_source(
                present_op,
                ValueSource::context(split_join.join_output_key.clone()),
                "present_result",
            ),
        ));
    } else {
        let linear_steps = compile_linear_steps(
            &profile.query_ops,
            &profile.analysis_ops,
            ValueSource::context("resolved"),
            "",
        )?;
        steps.extend(linear_steps);
        let input_source = last_output_key(&steps)
            .map(ValueSource::context)
            .ok_or_else(|| "compiled profile has no operation output".to_owned())?;
        steps.push(Step::call(
            "present",
            OperationCallStep::from_source(present_op, input_source, "present_result"),
        ));
    }

    steps.push(Step::return_output(
        "return",
        ReturnStep::new(ValueSource::context("present_result.output")).with_render(RenderSpec {
            renderer_id: "noop.summary".to_owned(),
            source: RenderSource::AllEvidence,
            view_id: "summary-card".to_owned(),
            title: format!("{} summary", profile.profile_id),
            summary: "Compiled from gx.observability.playbook.v1".to_owned(),
        }),
    ));

    Ok(FlowDefinition {
        flow_id: profile.profile_id.clone(),
        steps,
    })
}

fn compile_linear_steps(
    query_ops: &[String],
    analysis_ops: &[String],
    initial_source: ValueSource,
    prefix: &str,
) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    let mut source = initial_source;
    for (index, operation_id) in query_ops.iter().enumerate() {
        let op_id = OperationId::new(operation_id.clone())
            .map_err(|err| format!("query op {}: {err}", operation_id))?;
        let step_id = prefixed(prefix, "query", index);
        let output_key = format!("{}_result", step_id);
        steps.push(Step::call(
            step_id.clone(),
            OperationCallStep::from_source(op_id, source, output_key.clone()),
        ));
        source = ValueSource::context(format!("{output_key}.output"));
    }
    for (index, operation_id) in analysis_ops.iter().enumerate() {
        let op_id = OperationId::new(operation_id.clone())
            .map_err(|err| format!("analysis op {}: {err}", operation_id))?;
        let step_id = prefixed(prefix, "analyse", index);
        let output_key = format!("{}_result", step_id);
        steps.push(Step::call(
            step_id.clone(),
            OperationCallStep::from_source(op_id, source, output_key.clone()),
        ));
        source = ValueSource::context(format!("{output_key}.output"));
    }
    Ok(steps)
}

fn prefixed(prefix: &str, stem: &str, index: usize) -> String {
    if prefix.is_empty() {
        format!("{stem}_{index}")
    } else {
        format!("{prefix}_{stem}_{index}")
    }
}

fn last_output_key(steps: &[Step]) -> Option<String> {
    steps.iter().rev().find_map(|step| match &step.kind {
        greentic_x_flow::StepKind::Call(call) => Some(call.output_key.clone()),
        greentic_x_flow::StepKind::Resolve(resolve) => Some(resolve.output_key.clone()),
        _ => None,
    })
}
