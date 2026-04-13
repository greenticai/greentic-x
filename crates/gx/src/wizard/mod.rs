mod answers;
mod bundle;
mod catalog;
mod compose;
mod handoff;
mod intent_to_pack;
mod launcher;
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
use qa::{build_runtime_form_spec_for_document, collect_interactive_answers};

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
    let fetcher = DistributorCatalogFetcher;
    if should_collect_interactive_answers(action, execution, &args)
        && !collect_interactive_answers(cwd, &mut document, &fetcher)?
    {
        return Ok(String::new());
    }
    document.answers.insert(
        "gx_action".to_owned(),
        Value::String(wizard_action_name(action).to_owned()),
    );

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

pub(crate) fn render_answer_schema(cwd: &Path, args: WizardCommonArgs) -> Result<String, String> {
    let preferred_locale = normalize_locale(args.locale.as_deref().unwrap_or("en"));
    let target_schema_version = normalize_schema_version(
        args.schema_version
            .as_deref()
            .unwrap_or(GX_WIZARD_SCHEMA_VERSION),
        &preferred_locale,
    )?;
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
    document.locale = locale;

    let fetcher = DistributorCatalogFetcher;
    let runtime_form = build_runtime_form_spec_for_document(cwd, &document, &fetcher)?;
    let schema = wizard_answer_schema(&target_schema_version, &runtime_form);
    serde_json::to_string_pretty(&schema)
        .map(|rendered| format!("{rendered}\n"))
        .map_err(|err| format!("failed to serialize wizard schema: {err}"))
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

fn wizard_answer_schema(schema_version: &str, runtime_form: &Value) -> Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://greenticai.github.io/greentic-x/schemas/wizard.answers.schema.json",
        "title": "Greentic-X Wizard AnswerDocument",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "wizard_id": {
                "type": "string",
                "const": GX_WIZARD_ID
            },
            "schema_id": {
                "type": "string",
                "const": GX_WIZARD_SCHEMA_ID
            },
            "schema_version": {
                "type": "string",
                "const": schema_version
            },
            "locale": {
                "type": "string",
                "minLength": 1
            },
            "answers": {
                "$ref": "#/$defs/gx_answers"
            },
            "locks": {
                "type": "object"
            }
        },
        "required": ["wizard_id", "schema_id", "schema_version", "locale", "answers", "locks"],
        "$defs": {
            "gx_runtime_form": runtime_form,
            "gx_answers": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "gx_action": {
                        "type": "string",
                        "enum": ["run", "validate", "apply"]
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["create", "update"]
                    },
                    "existing_solution_path": {
                        "type": "string",
                        "minLength": 1
                    },
                    "solution_name": {
                        "type": "string",
                        "minLength": 2
                    },
                    "solution_id": {
                        "type": "string",
                        "minLength": 2
                    },
                    "description": {
                        "type": "string"
                    },
                    "output_dir": {
                        "type": "string",
                        "minLength": 1
                    },
                    "catalog_oci_refs": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1 }
                    },
                    "template_mode": {
                        "type": "string",
                        "enum": ["basic_empty", "catalog", "manual"]
                    },
                    "template_entry_id": {
                        "type": "string",
                        "minLength": 1
                    },
                    "assistant_template_ref": {
                        "type": "string",
                        "minLength": 1
                    },
                    "domain_template_ref": {
                        "type": "string",
                        "minLength": 1
                    },
                    "provider_selection": {
                        "type": "string",
                        "enum": ["webchat", "teams", "webex", "slack", "all", "catalog", "manual"]
                    },
                    "provider_preset_entry_id": {
                        "type": "string",
                        "minLength": 1
                    },
                    "provider_refs": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1 }
                    },
                    "template_display_name": {
                        "type": "string"
                    },
                    "provider_preset_display_name": {
                        "type": "string"
                    },
                    "overlay_entry_id": {
                        "type": "string"
                    },
                    "overlay_display_name": {
                        "type": "string"
                    },
                    "overlay_default_locale": {
                        "type": "string"
                    },
                    "overlay_tenant_id": {
                        "type": "string"
                    },
                    "catalog_resolution_policy": {
                        "type": "string"
                    }
                },
                "required": ["mode", "solution_name", "solution_id", "output_dir", "provider_selection"],
                "allOf": [
                    {
                        "if": {
                            "properties": {
                                "mode": { "const": "update" }
                            },
                            "required": ["mode"]
                        },
                        "then": {
                            "required": ["existing_solution_path"]
                        }
                    },
                    {
                        "if": {
                            "properties": {
                                "mode": { "const": "create" }
                            },
                            "required": ["mode"]
                        },
                        "then": {
                            "required": ["template_mode"]
                        }
                    },
                    {
                        "if": {
                            "properties": {
                                "mode": { "const": "create" },
                                "template_mode": { "const": "catalog" }
                            },
                            "required": ["mode", "template_mode"]
                        },
                        "then": {
                            "required": ["template_entry_id"]
                        }
                    },
                    {
                        "if": {
                            "properties": {
                                "mode": { "const": "create" },
                                "template_mode": { "const": "manual" }
                            },
                            "required": ["mode", "template_mode"]
                        },
                        "then": {
                            "required": ["assistant_template_ref", "domain_template_ref"]
                        }
                    },
                    {
                        "if": {
                            "properties": {
                                "provider_selection": { "const": "catalog" }
                            },
                            "required": ["provider_selection"]
                        },
                        "then": {
                            "required": ["provider_preset_entry_id"]
                        }
                    },
                    {
                        "if": {
                            "properties": {
                                "provider_selection": { "const": "manual" }
                            },
                            "required": ["provider_selection"]
                        },
                        "then": {
                            "required": ["provider_refs"]
                        }
                    }
                ]
            }
        }
    })
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
        warnings: wizard_warnings(action, execution, args, normalized_answers, locale),
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
        eprintln!(
            "warning: GX is invoking a deprecated downstream bundle compatibility bridge; prefer emitted handoff artifacts and downstream launcher integration."
        );
        let bundle_answers_path = emit_answers_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| resolve_wizard_path(cwd, Path::new(&request.bundle_answers_path)));
        if emit_answers_path.is_none() {
            write_answer_document(&bundle_answers_path, &generated.handoff.bundle_answers)?;
        }
        run_bundle_handoff(cwd, &bundle_answers_path)?;
    }
    Ok(())
}

fn run_interactive_session(cwd: &Path, args: WizardCommonArgs) -> Result<(), String> {
    let plan_json = run_wizard(cwd, WizardAction::Run, args)?;
    if plan_json.trim().is_empty() {
        return Ok(());
    }
    let plan: WizardPlanEnvelope = serde_json::from_str(&plan_json)
        .map_err(|err| format!("failed to parse interactive wizard result: {err}"))?;
    print_completion_message(cwd, &plan)
}

fn print_completion_message(cwd: &Path, plan: &WizardPlanEnvelope) -> Result<(), String> {
    let solution_manifest = plan
        .normalized_input_summary
        .get("solution_manifest_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "wizard result missing solution_manifest_path".to_owned())?;
    let handoff = plan
        .normalized_input_summary
        .get("toolchain_handoff_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "wizard result missing toolchain_handoff_path".to_owned())?;
    let launcher_answers = plan
        .normalized_input_summary
        .get("launcher_answers_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "wizard result missing launcher_answers_path".to_owned())?;
    let pack_input = plan
        .normalized_input_summary
        .get("pack_input_path")
        .and_then(Value::as_str)
        .ok_or_else(|| "wizard result missing pack_input_path".to_owned())?;

    let resolved_solution_manifest = resolve_wizard_path(cwd, Path::new(solution_manifest));
    if !resolved_solution_manifest.exists() {
        return Err(format!(
            "wizard reported solution manifest {}, but the file was not created",
            resolved_solution_manifest.display()
        ));
    }
    println!("Solution intent created successfully.");
    println!();
    println!(
        "Solution manifest: {}",
        resolved_solution_manifest.display()
    );
    println!(
        "Toolchain handoff: {}",
        resolve_wizard_path(cwd, Path::new(handoff)).display()
    );
    println!(
        "Launcher answers: {}",
        resolve_wizard_path(cwd, Path::new(launcher_answers)).display()
    );
    println!(
        "Pack input: {}",
        resolve_wizard_path(cwd, Path::new(pack_input)).display()
    );
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
