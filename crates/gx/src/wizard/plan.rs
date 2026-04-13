use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::{
    CompositionRequest, WizardAction, WizardAnswerDocument, WizardCommonArgs, WizardExecutionMode,
    WizardNormalizedAnswers,
};

use super::compose::{downstream_output_paths, generated_output_paths};
use super::handoff::default_handoff_answers_path;
use super::resolve_wizard_path;

pub(crate) fn wizard_action_name(action: WizardAction) -> &'static str {
    match action {
        WizardAction::Run => "run",
        WizardAction::Validate => "validate",
        WizardAction::Apply => "apply",
    }
}

pub(crate) fn wizard_plan_steps(
    action: WizardAction,
    execution: WizardExecutionMode,
    _locale: &str,
) -> Vec<crate::WizardPlanStep> {
    let mut steps = vec![
        crate::WizardPlanStep {
            kind: "collect_input".to_owned(),
            description: "Collect wizard inputs and answer state".to_owned(),
        },
        crate::WizardPlanStep {
            kind: "load_catalogs".to_owned(),
            description: "Load local and OCI catalogs".to_owned(),
        },
        crate::WizardPlanStep {
            kind: "build_outputs".to_owned(),
            description: "Build GX composition outputs and downstream handoff artifacts".to_owned(),
        },
    ];
    if matches!(action, WizardAction::Run | WizardAction::Apply)
        && matches!(execution, WizardExecutionMode::Execute)
    {
        steps.push(crate::WizardPlanStep {
            kind: "bundle_handoff".to_owned(),
            description: "Invoke downstream bundle generation through greentic-bundle".to_owned(),
        });
    }
    steps
}

pub(crate) fn should_delegate_bundle_handoff(
    action: WizardAction,
    execution: WizardExecutionMode,
    args: &WizardCommonArgs,
    normalized_answers: &WizardNormalizedAnswers,
) -> bool {
    matches!(normalized_answers, WizardNormalizedAnswers::Composition(_))
        && (matches!(action, WizardAction::Apply)
            || (matches!(action, WizardAction::Run) && args.bundle_handoff))
        && matches!(execution, WizardExecutionMode::Execute)
}

pub(crate) fn wizard_normalized_summary(
    action: WizardAction,
    document: &WizardAnswerDocument,
    normalized_answers: &WizardNormalizedAnswers,
) -> BTreeMap<String, Value> {
    let mut answers_keys = document.answers.keys().cloned().collect::<Vec<_>>();
    answers_keys.sort();
    let mut summary = BTreeMap::from([
        (
            "mode".to_owned(),
            Value::String(wizard_action_name(action).to_owned()),
        ),
        (
            "schema_version".to_owned(),
            Value::String(document.schema_version.clone()),
        ),
        ("locale".to_owned(), Value::String(document.locale.clone())),
        (
            "answers_keys".to_owned(),
            Value::Array(answers_keys.into_iter().map(Value::String).collect()),
        ),
    ]);
    match normalized_answers {
        WizardNormalizedAnswers::Composition(request) => {
            add_composition_summary(&mut summary, request);
        }
    }
    summary
}

pub(crate) fn wizard_expected_writes(
    cwd: &Path,
    action: WizardAction,
    execution: WizardExecutionMode,
    args: &WizardCommonArgs,
    normalized_answers: &WizardNormalizedAnswers,
) -> Vec<String> {
    let mut writes = Vec::new();
    if let Some(path) = args.emit_answers.as_ref() {
        writes.push(resolve_wizard_path(cwd, path).display().to_string());
    }
    match normalized_answers {
        WizardNormalizedAnswers::Composition(request) => {
            for path in generated_output_paths(request) {
                writes.push(
                    resolve_wizard_path(cwd, Path::new(&path))
                        .display()
                        .to_string(),
                );
            }
            if should_delegate_bundle_handoff(action, execution, args, normalized_answers) {
                for path in downstream_output_paths(request) {
                    writes.push(
                        resolve_wizard_path(cwd, Path::new(&path))
                            .display()
                            .to_string(),
                    );
                }
            }
            if should_delegate_bundle_handoff(action, execution, args, normalized_answers)
                && args.emit_answers.is_none()
            {
                writes.push(
                    default_handoff_answers_path(cwd, action)
                        .display()
                        .to_string(),
                );
            }
        }
    }
    writes.sort();
    writes.dedup();
    writes
}

pub(crate) fn wizard_warnings(
    action: WizardAction,
    execution: WizardExecutionMode,
    args: &WizardCommonArgs,
    normalized_answers: &WizardNormalizedAnswers,
    _locale: &str,
) -> Vec<String> {
    match normalized_answers {
        WizardNormalizedAnswers::Composition(request) => {
            let mut warnings = Vec::new();
            if !request.catalog_oci_refs.is_empty() {
                warnings.push(format!(
                    "remote catalog sources configured: {}",
                    request.catalog_oci_refs.join(", ")
                ));
            }
            if matches!(action, WizardAction::Apply) {
                warnings.push(
                    "`gx wizard apply` is a compatibility bridge for downstream replay. Prefer `gx wizard run` and consume emitted handoff artifacts.".to_owned(),
                );
            }
            if matches!(execution, WizardExecutionMode::Execute)
                && (args.bundle_handoff || matches!(action, WizardAction::Apply))
            {
                warnings.push(
                    "Direct `greentic-bundle` invocation from GX is deprecated compatibility behavior; long-term integration should happen through `greentic-dev` and downstream tools.".to_owned(),
                );
            }
            warnings
        }
    }
}

fn add_composition_summary(summary: &mut BTreeMap<String, Value>, request: &CompositionRequest) {
    summary.insert(
        "workflow".to_owned(),
        Value::String("compose_solution".to_owned()),
    );
    summary.insert(
        "ownership_boundary".to_owned(),
        Value::String("gx_composition_only".to_owned()),
    );
    summary.insert(
        "compose_mode".to_owned(),
        Value::String(request.mode.clone()),
    );
    summary.insert(
        "template_mode".to_owned(),
        Value::String(request.template_mode.clone()),
    );
    summary.insert(
        "solution_name".to_owned(),
        Value::String(request.solution_name.clone()),
    );
    summary.insert(
        "solution_id".to_owned(),
        Value::String(request.solution_id.clone()),
    );
    summary.insert(
        "output_dir".to_owned(),
        Value::String(request.output_dir.clone()),
    );
    summary.insert(
        "provider_selection".to_owned(),
        Value::String(request.provider_selection.clone()),
    );
    summary.insert(
        "provider_refs_count".to_owned(),
        Value::from(request.provider_refs.len() as u64),
    );
    summary.insert(
        "bundle_output_path".to_owned(),
        Value::String(request.bundle_output_path.clone()),
    );
    summary.insert(
        "solution_manifest_path".to_owned(),
        Value::String(request.solution_manifest_path.clone()),
    );
    summary.insert(
        "toolchain_handoff_path".to_owned(),
        Value::String(request.toolchain_handoff_path.clone()),
    );
    summary.insert(
        "launcher_answers_path".to_owned(),
        Value::String(request.launcher_answers_path.clone()),
    );
    summary.insert(
        "pack_input_path".to_owned(),
        Value::String(request.pack_input_path.clone()),
    );
    summary.insert(
        "catalog_oci_sources_count".to_owned(),
        Value::from(request.catalog_oci_refs.len() as u64),
    );
    summary.insert(
        "catalog_resolution_policy".to_owned(),
        Value::String(request.catalog_resolution_policy.clone()),
    );
}
