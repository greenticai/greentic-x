mod answers;
mod catalog;
mod compose;
mod handoff;
mod plan;
mod qa;
mod remote;

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
use catalog::{DistributorCatalogFetcher, load_catalogs};
use compose::{generate_artifacts, write_generated_artifacts};
use handoff::{resolve_wizard_path, run_bundle_handoff};
use plan::{
    should_delegate_bundle_handoff, wizard_action_name, wizard_expected_writes,
    wizard_normalized_summary, wizard_plan_steps, wizard_warnings,
};
use qa::collect_interactive_answers;

#[allow(unused_imports)]
pub(crate) use handoff::bundle_handoff_invocation;

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
    if !args.catalog.is_empty() {
        let mut refs = document
            .answers
            .get("catalog_oci_refs")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for catalog in &args.catalog {
            if !refs.iter().any(|existing| existing == catalog) {
                refs.push(catalog.clone());
            }
        }
        document.answers.insert(
            "catalog_oci_refs".to_owned(),
            Value::Array(refs.into_iter().map(Value::String).collect()),
        );
    }
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
    let fetcher = DistributorCatalogFetcher;
    if should_collect_interactive_answers(action, execution, &args)
        && !collect_interactive_answers(cwd, &mut document, &fetcher)?
    {
        return Ok(String::new());
    }

    let normalized_answers = normalize_wizard_answers(
        cwd,
        &mut document,
        args.mode.as_deref(),
        &locale,
        matches!(execution, WizardExecutionMode::Execute),
    )?;

    let emit_answers_path = args
        .emit_answers
        .as_ref()
        .map(|path| resolve_wizard_path(cwd, path));
    if let Some(path) = emit_answers_path.as_ref() {
        write_answer_document(path, &document)?;
    }

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

    let spec = WizardSpec {
        ordered_step_list: wizard_plan_steps(action, execution, &locale),
    };
    let plan = WizardPlanEnvelope {
        metadata: WizardPlanMetadata {
            wizard_id: GX_WIZARD_ID.to_owned(),
            schema_id: GX_WIZARD_SCHEMA_ID.to_owned(),
            schema_version: document.schema_version.clone(),
            locale,
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

pub(crate) fn run_default_wizard(cwd: &Path, args: WizardCommonArgs) -> Result<String, String> {
    if should_run_interactive_session(&args) {
        run_interactive_session(cwd, args)?;
        return Ok(String::new());
    }
    run_wizard(cwd, WizardAction::Run, args)
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

fn should_run_interactive_session(args: &WizardCommonArgs) -> bool {
    args.answers.is_none()
        && args.emit_answers.is_none()
        && !args.dry_run
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
    _document: &WizardAnswerDocument,
    normalized_answers: &WizardNormalizedAnswers,
    emit_answers_path: Option<&Path>,
) -> Result<(), String> {
    if !matches!(execution, WizardExecutionMode::Execute) {
        return Ok(());
    }
    let fetcher = DistributorCatalogFetcher;
    let WizardNormalizedAnswers::Composition(request) = normalized_answers;
    let catalogs = load_catalogs(cwd, &request.catalog_oci_refs, &fetcher)?;
    let generated = generate_artifacts(cwd, request, &catalogs, "en", true, &fetcher)?;
    write_generated_artifacts(cwd, request, &generated)?;

    if should_delegate_bundle_handoff(action, execution, args, normalized_answers) {
        let bundle_answers_path = emit_answers_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| resolve_wizard_path(cwd, Path::new(&request.bundle_answers_path)));
        if emit_answers_path.is_none() {
            write_answer_document(&bundle_answers_path, &generated.bundle_answers)?;
        }
        run_bundle_handoff(cwd, &bundle_answers_path)?;
    }
    Ok(())
}

fn run_interactive_session(cwd: &Path, args: WizardCommonArgs) -> Result<(), String> {
    loop {
        let mut session_args = args.clone();
        session_args.bundle_handoff = true;
        let plan_json = run_wizard(cwd, WizardAction::Apply, session_args)?;
        if plan_json.trim().is_empty() {
            return Ok(());
        }
        let plan: WizardPlanEnvelope = serde_json::from_str(&plan_json)
            .map_err(|err| format!("failed to parse interactive wizard result: {err}"))?;
        print_completion_message(cwd, &plan)?;
    }
}

fn print_completion_message(cwd: &Path, plan: &WizardPlanEnvelope) -> Result<(), String> {
    let path = plan
        .normalized_input_summary
        .get("bundle_output_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "wizard result missing bundle_output_path".to_owned())?;
    let resolved = resolve_wizard_path(cwd, Path::new(path));
    println!("Solution created successfully.");
    println!();
    println!("Generated bundle: {}", resolved.display());
    println!();
    println!("M) Main menu");
    println!("0) Exit");
    println!();
    Ok(())
}

fn write_answer_document(path: &Path, document: &WizardAnswerDocument) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(document)
        .map_err(|err| format!("failed to serialize answer document: {err}"))?;
    std::fs::write(path, format!("{rendered}\n"))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}
