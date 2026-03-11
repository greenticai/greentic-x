mod answers;
mod handoff;
mod plan;
mod qa;
mod remote;
mod template;

use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::Path;

use crate::i18n::{normalize_locale, resolve_locale};
use crate::{
    GX_WIZARD_ID, GX_WIZARD_SCHEMA_ID, GX_WIZARD_SCHEMA_VERSION, WizardAction,
    WizardAnswerDocument, WizardCommonArgs, WizardExecutionMode, WizardNormalizedAnswers,
    WizardPlanEnvelope, WizardPlanMetadata,
};
use serde_json::Value;

use answers::{load_wizard_answers, normalize_schema_version, normalize_wizard_answers};
use handoff::{
    default_handoff_answers_path, resolve_wizard_path, run_bundle_handoff, write_wizard_answers_at,
};
use plan::{
    should_delegate_bundle_handoff, wizard_action_name, wizard_normalized_summary,
    wizard_plan_steps, wizard_warnings,
};
use qa::collect_interactive_answers;
use template::{materialize_template, should_materialize_template};

#[cfg(test)]
pub(crate) use handoff::bundle_handoff_invocation;
pub(crate) use plan::wizard_expected_writes;

struct WizardSpec {
    ordered_step_list: Vec<crate::WizardPlanStep>,
}

struct WizardApplyResult {
    normalized_input_summary: BTreeMap<String, Value>,
    expected_file_writes: Vec<String>,
    warnings: Vec<String>,
}

pub(crate) fn run_wizard(
    cwd: &Path,
    action: WizardAction,
    args: WizardCommonArgs,
) -> Result<String, String> {
    let preferred_locale = normalize_locale(args.locale.as_deref().unwrap_or("en"));
    let target_schema_version = normalize_schema_version(
        args.schema_version
            .as_deref()
            .unwrap_or(GX_WIZARD_SCHEMA_VERSION),
        &preferred_locale,
    )?;
    let execution = match action {
        WizardAction::Validate => WizardExecutionMode::DryRun,
        WizardAction::Run | WizardAction::Apply => {
            if args.dry_run {
                WizardExecutionMode::DryRun
            } else {
                WizardExecutionMode::Execute
            }
        }
    };
    let mut document = load_wizard_answers(cwd, &args, &target_schema_version, &preferred_locale)?;
    let locale = resolve_locale(
        args.locale.as_deref(),
        args.answers.as_ref().map(|_| document.locale.as_str()),
    );
    document.locale = locale.clone();
    document.schema_version = target_schema_version;
    document.answers.insert(
        "gx_action".to_owned(),
        Value::String(wizard_action_name(action).to_owned()),
    );
    if should_collect_interactive_answers(action, execution, &args) {
        collect_interactive_answers(&mut document, args.mode.as_deref(), &locale)?;
    }
    let resolve_remote = matches!(execution, WizardExecutionMode::Execute)
        && matches!(action, WizardAction::Run | WizardAction::Apply);
    let normalized_answers = normalize_wizard_answers(
        cwd,
        &mut document,
        args.mode.as_deref(),
        &locale,
        resolve_remote,
    )?;

    let emit_answers_path = args
        .emit_answers
        .as_ref()
        .map(|path| resolve_wizard_path(cwd, path));
    if let Some(path) = emit_answers_path.as_ref() {
        write_wizard_answers_at(path, &document)?;
    }

    let spec = wizard_spec(action, execution, &locale);
    let applied = wizard_apply(
        cwd,
        action,
        execution,
        &args,
        &document,
        &normalized_answers,
        &locale,
    );
    wizard_execute_plan(
        cwd,
        action,
        execution,
        &args,
        &document,
        &normalized_answers,
        emit_answers_path.as_deref(),
    )?;

    let plan = WizardPlanEnvelope {
        metadata: WizardPlanMetadata {
            wizard_id: GX_WIZARD_ID.to_owned(),
            schema_id: GX_WIZARD_SCHEMA_ID.to_owned(),
            schema_version: document.schema_version.clone(),
            locale: locale.clone(),
            execution,
        },
        requested_action: wizard_action_name(action).to_owned(),
        target_root: cwd.display().to_string(),
        normalized_input_summary: applied.normalized_input_summary,
        ordered_step_list: spec.ordered_step_list,
        expected_file_writes: applied.expected_file_writes,
        warnings: applied.warnings,
    };
    serde_json::to_string_pretty(&plan)
        .map_err(|err| format!("failed to serialize wizard plan: {err}"))
}

fn should_collect_interactive_answers(
    action: WizardAction,
    execution: WizardExecutionMode,
    args: &WizardCommonArgs,
) -> bool {
    args.answers.is_none()
        && matches!(action, WizardAction::Run | WizardAction::Apply)
        && matches!(execution, WizardExecutionMode::Execute)
        && !is_automated_context()
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
}

fn is_automated_context() -> bool {
    cfg!(test)
        || std::env::var_os("RUST_TEST_THREADS").is_some()
        || std::env::var_os("CI").is_some()
        || std::env::var_os("GX_WIZARD_NON_INTERACTIVE").is_some()
}

fn wizard_spec(action: WizardAction, execution: WizardExecutionMode, locale: &str) -> WizardSpec {
    WizardSpec {
        ordered_step_list: wizard_plan_steps(action, execution, locale),
    }
}

fn wizard_apply(
    cwd: &Path,
    action: WizardAction,
    execution: WizardExecutionMode,
    args: &WizardCommonArgs,
    document: &WizardAnswerDocument,
    normalized_answers: &WizardNormalizedAnswers,
    locale: &str,
) -> WizardApplyResult {
    WizardApplyResult {
        normalized_input_summary: wizard_normalized_summary(action, document, normalized_answers),
        expected_file_writes: wizard_expected_writes(
            cwd,
            action,
            execution,
            args,
            normalized_answers,
        ),
        warnings: wizard_warnings(normalized_answers, locale),
    }
}

fn wizard_execute_plan(
    cwd: &Path,
    action: WizardAction,
    execution: WizardExecutionMode,
    args: &WizardCommonArgs,
    document: &WizardAnswerDocument,
    normalized_answers: &WizardNormalizedAnswers,
    emit_answers_path: Option<&Path>,
) -> Result<(), String> {
    if should_materialize_template(action, execution, normalized_answers)
        && let WizardNormalizedAnswers::Template(template_answers) = normalized_answers
    {
        materialize_template(cwd, template_answers)?;
    }
    if should_delegate_bundle_handoff(action, execution, args, normalized_answers) {
        let handoff_answers_path = emit_answers_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| default_handoff_answers_path(cwd, action));
        if emit_answers_path.is_none() {
            write_wizard_answers_at(&handoff_answers_path, document)?;
        }
        run_bundle_handoff(cwd, action, &handoff_answers_path)?;
    }
    Ok(())
}
