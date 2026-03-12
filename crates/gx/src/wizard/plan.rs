use std::collections::BTreeMap;
use std::path::Path;

use crate::i18n::tr;
use crate::{
    WizardAction, WizardAnswerDocument, WizardBundleAnswers, WizardCommonArgs, WizardExecutionMode,
    WizardNormalizedAnswers, WizardTemplateAnswers,
};
use serde_json::Value;

use super::{default_handoff_answers_path, resolve_wizard_path};

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
    locale: &str,
) -> Vec<crate::WizardPlanStep> {
    let mut steps = vec![
        crate::WizardPlanStep {
            kind: "collect_input".to_owned(),
            description: tr(locale, "wizard.step.collect_input"),
        },
        crate::WizardPlanStep {
            kind: "normalize_request".to_owned(),
            description: tr(locale, "wizard.step.normalize_request"),
        },
        crate::WizardPlanStep {
            kind: "validate_plan".to_owned(),
            description: tr(locale, "wizard.step.validate_plan"),
        },
    ];
    if matches!(action, WizardAction::Run | WizardAction::Apply)
        && matches!(execution, WizardExecutionMode::Execute)
    {
        steps.push(crate::WizardPlanStep {
            kind: "execute_plan".to_owned(),
            description: tr(locale, "wizard.step.execute_plan"),
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
    args.bundle_handoff
        && matches!(
            normalized_answers,
            WizardNormalizedAnswers::AssistantBundle(_)
        )
        && matches!(action, WizardAction::Run | WizardAction::Apply)
        && matches!(execution, WizardExecutionMode::Execute)
}

pub(crate) fn wizard_normalized_summary(
    action: WizardAction,
    document: &WizardAnswerDocument,
    normalized_answers: &WizardNormalizedAnswers,
) -> BTreeMap<String, Value> {
    let mut answers_keys = document.answers.keys().cloned().collect::<Vec<_>>();
    answers_keys.sort();
    let mut lock_keys = document.locks.keys().cloned().collect::<Vec<_>>();
    lock_keys.sort();
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
        (
            "lock_keys".to_owned(),
            Value::Array(lock_keys.into_iter().map(Value::String).collect()),
        ),
    ]);
    match normalized_answers {
        WizardNormalizedAnswers::AssistantBundle(bundle_answers) => {
            add_bundle_summary(&mut summary, bundle_answers);
        }
        WizardNormalizedAnswers::Template(template_answers) => {
            add_template_summary(&mut summary, template_answers);
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
        let resolved = resolve_wizard_path(cwd, path);
        writes.push(resolved.display().to_string());
    }
    match normalized_answers {
        WizardNormalizedAnswers::AssistantBundle(bundle_answers) => {
            if should_delegate_bundle_handoff(action, execution, args, normalized_answers) {
                writes.push(
                    resolve_wizard_path(cwd, Path::new(&bundle_answers.bundle_output_path))
                        .display()
                        .to_string(),
                );
                if args.emit_answers.is_none() {
                    writes.push(
                        default_handoff_answers_path(cwd, action)
                            .display()
                            .to_string(),
                    );
                }
                writes.push("<delegated: greentic-bundle artifact writes>".to_owned());
            }
        }
        WizardNormalizedAnswers::Template(template_answers) => {
            writes.push(
                resolve_wizard_path(cwd, Path::new(&template_answers.template_output_path))
                    .display()
                    .to_string(),
            );
        }
    }
    writes.sort();
    writes.dedup();
    writes
}

pub(crate) fn wizard_warnings(
    normalized_answers: &WizardNormalizedAnswers,
    locale: &str,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let (latest_refs, latest_policy) = match normalized_answers {
        WizardNormalizedAnswers::AssistantBundle(bundle_answers) => {
            (&bundle_answers.latest_refs, &bundle_answers.latest_policy)
        }
        WizardNormalizedAnswers::Template(template_answers) => (
            &template_answers.latest_refs,
            &template_answers.latest_policy,
        ),
    };
    if !latest_refs.is_empty()
        && let Some(policy) = latest_policy.as_ref()
    {
        let refs = latest_refs.join(", ");
        warnings.push(format!(
            "{} ({refs}); latest_policy={policy}",
            tr(locale, "wizard.warn.latest_refs")
        ));
    }
    warnings
}

fn add_bundle_summary(summary: &mut BTreeMap<String, Value>, bundle_answers: &WizardBundleAnswers) {
    summary.insert(
        "workflow".to_owned(),
        Value::String(bundle_answers.workflow.clone()),
    );
    summary.insert(
        "bundle_mode".to_owned(),
        Value::String(bundle_answers.bundle_mode.clone()),
    );
    summary.insert(
        "bundle_name".to_owned(),
        Value::String(bundle_answers.bundle_name.clone()),
    );
    summary.insert(
        "bundle_id".to_owned(),
        Value::String(bundle_answers.bundle_id.clone()),
    );
    summary.insert(
        "output_dir".to_owned(),
        Value::String(bundle_answers.output_dir.clone()),
    );
    summary.insert(
        "assistant_template_source".to_owned(),
        Value::String(bundle_answers.assistant_template_source.clone()),
    );
    summary.insert(
        "domain_template_source".to_owned(),
        Value::String(bundle_answers.domain_template_source.clone()),
    );
    summary.insert(
        "provider_categories".to_owned(),
        Value::Array(
            bundle_answers
                .provider_categories
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    summary.insert(
        "bundle_output_path".to_owned(),
        Value::String(bundle_answers.bundle_output_path.clone()),
    );
    summary.insert(
        "latest_policy".to_owned(),
        bundle_answers
            .latest_policy
            .as_ref()
            .map_or(Value::Null, |value| Value::String(value.clone())),
    );
    summary.insert(
        "latest_refs_count".to_owned(),
        Value::from(bundle_answers.latest_refs.len() as u64),
    );
}

fn add_template_summary(
    summary: &mut BTreeMap<String, Value>,
    template_answers: &WizardTemplateAnswers,
) {
    summary.insert(
        "workflow".to_owned(),
        Value::String(template_answers.workflow.clone()),
    );
    summary.insert(
        "template_kind".to_owned(),
        Value::String(template_answers.template_kind.clone()),
    );
    summary.insert(
        "template_action".to_owned(),
        Value::String(template_answers.template_action.clone()),
    );
    summary.insert(
        "template_source".to_owned(),
        Value::String(template_answers.template_source.clone()),
    );
    summary.insert(
        "template_output_path".to_owned(),
        Value::String(template_answers.template_output_path.clone()),
    );
    summary.insert(
        "latest_policy".to_owned(),
        template_answers
            .latest_policy
            .as_ref()
            .map_or(Value::Null, |value| Value::String(value.clone())),
    );
    summary.insert(
        "latest_refs_count".to_owned(),
        Value::from(template_answers.latest_refs.len() as u64),
    );
}
