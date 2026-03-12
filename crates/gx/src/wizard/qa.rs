use std::io::{self, Write};

use greentic_qa_lib::{I18nConfig, WizardDriver, WizardFrontend, WizardRunConfig};
use serde_json::{Map, Value, json};

use crate::WizardAnswerDocument;
use crate::i18n::tr;

pub(crate) fn collect_interactive_answers(
    document: &mut WizardAnswerDocument,
    cli_mode: Option<&str>,
    locale: &str,
) -> Result<bool, String> {
    let workflow = match cli_mode {
        Some(mode) if !mode.trim().is_empty() => mode.trim().to_owned(),
        _ => {
            let mode_answers = run_qa_form(workflow_spec(locale), locale)?;
            mode_answers
                .get("workflow")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| "assistant_bundle".to_owned())
        }
    };
    if workflow == "__exit__" {
        return Ok(false);
    }
    document
        .answers
        .insert("workflow".to_owned(), Value::String(workflow.clone()));

    let detail_answers = run_qa_form(detail_spec(&workflow, locale), locale)?;
    merge_answers(&mut document.answers, detail_answers);
    normalize_qa_field_shapes(&mut document.answers);
    Ok(true)
}

fn run_qa_form(spec: Value, locale: &str) -> Result<Map<String, Value>, String> {
    let config = WizardRunConfig {
        spec_json: spec.to_string(),
        initial_answers_json: None,
        frontend: WizardFrontend::Text,
        i18n: I18nConfig {
            locale: Some(locale.to_owned()),
            resolved: None,
            debug: false,
        },
        verbose: false,
    };
    let mut driver = WizardDriver::new(config)
        .map_err(|err| format!("initialize greentic-qa-lib wizard: {err}"))?;

    loop {
        driver
            .next_payload_json()
            .map_err(|err| format!("render greentic-qa-lib payload: {err}"))?;
        if driver.is_complete() {
            break;
        }
        let ui_raw = driver
            .last_ui_json()
            .ok_or_else(|| "greentic-qa-lib payload missing UI state".to_owned())?;
        let ui: Value = serde_json::from_str(ui_raw)
            .map_err(|err| format!("parse greentic-qa-lib UI payload: {err}"))?;
        let question_id = ui
            .get("next_question_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "greentic-qa-lib UI missing next_question_id".to_owned())?
            .to_owned();
        let question = ui
            .get("questions")
            .and_then(Value::as_array)
            .and_then(|questions| {
                questions.iter().find(|question| {
                    question.get("id").and_then(Value::as_str) == Some(question_id.as_str())
                })
            })
            .ok_or_else(|| format!("greentic-qa-lib UI missing question {question_id}"))?;
        let answer = prompt_question(question, locale)?;
        driver
            .submit_patch_json(&json!({ question_id: answer }).to_string())
            .map_err(|err| format!("submit greentic-qa-lib answer: {err}"))?;
    }
    let result = driver
        .finish()
        .map_err(|err| format!("finish greentic-qa-lib wizard: {err}"))?;
    result
        .answer_set
        .answers
        .as_object()
        .cloned()
        .ok_or_else(|| "greentic-qa-lib answer set must be a JSON object".to_owned())
}

fn prompt_question(question: &Value, locale: &str) -> Result<Value, String> {
    let id = question
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("field");
    let title = question.get("title").and_then(Value::as_str).unwrap_or(id);
    let kind = question
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("string");
    let required = question
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match kind {
        "enum" => prompt_enum(locale, title, question, required),
        _ => prompt_string(locale, id, title, question, required),
    }
}

fn prompt_enum(locale: &str, title: &str, question: &Value, required: bool) -> Result<Value, String> {
    let question_id = question.get("id").and_then(Value::as_str).unwrap_or_default();
    let choices = question
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| "qa enum question missing choices".to_owned())?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let choice_labels = question
        .get("choice_labels")
        .and_then(Value::as_object);
    let choice_descriptions = question
        .get("choice_descriptions")
        .and_then(Value::as_object);
    let default = question
        .get("default")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut stdout = io::stdout();
    writeln!(stdout, "{title}").map_err(|err| format!("write prompt failed: {err}"))?;
    for (idx, choice) in choices.iter().enumerate() {
        let rendered = enum_choice_label(locale, question_id, choice, choice_labels)
            .unwrap_or_else(|| choice.clone());
        writeln!(stdout, "{}. {}", idx + 1, rendered)
            .map_err(|err| format!("write prompt failed: {err}"))?;
        if let Some(description) =
            enum_choice_description(locale, question_id, choice, choice_descriptions)
        {
            writeln!(stdout, "   {}", description)
                .map_err(|err| format!("write prompt failed: {err}"))?;
        }
    }
    if question_id == "workflow" {
        writeln!(stdout, "0. {}", tx(locale, "wizard.qa.exit", "Exit wizard"))
            .map_err(|err| format!("write prompt failed: {err}"))?;
    }
    loop {
        if let Some(default) = default.as_deref() {
            write!(stdout, "> [{default}] ")
                .map_err(|err| format!("write prompt failed: {err}"))?;
        } else {
            write!(stdout, "> ").map_err(|err| format!("write prompt failed: {err}"))?;
        }
        stdout
            .flush()
            .map_err(|err| format!("flush prompt failed: {err}"))?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|err| format!("read prompt failed: {err}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let Some(default) = default.as_ref() {
                return Ok(Value::String(default.clone()));
            }
            if required {
                continue;
            }
            return Ok(Value::Null);
        }
        if question_id == "workflow" && (trimmed == "0" || trimmed.eq_ignore_ascii_case("q")) {
            return Ok(Value::String("__exit__".to_owned()));
        }
        if let Ok(index) = trimmed.parse::<usize>()
            && index > 0
            && index <= choices.len()
        {
            return Ok(Value::String(choices[index - 1].clone()));
        }
        if choices.iter().any(|choice| choice == trimmed) {
            return Ok(Value::String(trimmed.to_owned()));
        }
    }
}

fn enum_choice_label(
    locale: &str,
    question_id: &str,
    choice: &str,
    choice_labels: Option<&serde_json::Map<String, Value>>,
) -> Option<String> {
    choice_labels
        .and_then(|labels| labels.get(choice))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| match question_id {
            "workflow" => Some(tr(locale, &format!("wizard.qa.workflow.{choice}.label"))),
            "mode" => Some(tr(locale, &format!("wizard.qa.bundle_mode.{choice}.label"))),
            "latest_policy" => Some(tr(locale, &format!("wizard.qa.latest_policy.{choice}.label"))),
            _ => None,
        })
        .filter(|label| {
            label != &format!("wizard.qa.workflow.{choice}.label")
                && label != &format!("wizard.qa.bundle_mode.{choice}.label")
                && label != &format!("wizard.qa.latest_policy.{choice}.label")
        })
}

fn enum_choice_description(
    locale: &str,
    question_id: &str,
    choice: &str,
    choice_descriptions: Option<&serde_json::Map<String, Value>>,
) -> Option<String> {
    choice_descriptions
        .and_then(|descriptions| descriptions.get(choice))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| match question_id {
            "workflow" => Some(tr(locale, &format!("wizard.qa.workflow.{choice}.description"))),
            "mode" => Some(tr(locale, &format!("wizard.qa.bundle_mode.{choice}.description"))),
            "latest_policy" => {
                Some(tr(locale, &format!("wizard.qa.latest_policy.{choice}.description")))
            }
            _ => None,
        })
        .filter(|description| {
            description != &format!("wizard.qa.workflow.{choice}.description")
                && description != &format!("wizard.qa.bundle_mode.{choice}.description")
                && description != &format!("wizard.qa.latest_policy.{choice}.description")
        })
}

fn prompt_string(
    locale: &str,
    question_id: &str,
    title: &str,
    question: &Value,
    required: bool,
) -> Result<Value, String> {
    let default = question
        .get("default")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut stdout = io::stdout();
    loop {
        if let Some(description) = string_question_description(locale, question_id, question) {
            writeln!(stdout, "{title}").map_err(|err| format!("write prompt failed: {err}"))?;
            writeln!(stdout, "{}", description)
                .map_err(|err| format!("write prompt failed: {err}"))?;
            if let Some(default) = default.as_deref() {
                write!(stdout, "> [{default}] ")
                    .map_err(|err| format!("write prompt failed: {err}"))?;
            } else {
                write!(stdout, "> ").map_err(|err| format!("write prompt failed: {err}"))?;
            }
        } else if let Some(default) = default.as_deref() {
            write!(stdout, "{title} [{default}]: ")
                .map_err(|err| format!("write prompt failed: {err}"))?;
        } else {
            write!(stdout, "{title}: ").map_err(|err| format!("write prompt failed: {err}"))?;
        }
        stdout
            .flush()
            .map_err(|err| format!("flush prompt failed: {err}"))?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|err| format!("read prompt failed: {err}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let Some(default) = default.as_ref() {
                return Ok(Value::String(default.clone()));
            }
            if required {
                continue;
            }
            return Ok(Value::Null);
        }
        return Ok(Value::String(trimmed.to_owned()));
    }
}

fn string_question_description(locale: &str, question_id: &str, question: &Value) -> Option<String> {
    question
        .get("description")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            let key = format!("wizard.qa.{question_id}.description");
            let translated = tr(locale, &key);
            (translated != key).then_some(translated)
        })
}

fn merge_answers(target: &mut Map<String, Value>, source: Map<String, Value>) {
    for (key, value) in source {
        if value.is_null() {
            continue;
        }
        target.insert(key, value);
    }
}

fn normalize_qa_field_shapes(answers: &mut Map<String, Value>) {
    if let Some(Value::String(raw)) = answers.get("provider_categories") {
        let parsed = raw
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| Value::String(entry.to_owned()))
            .collect::<Vec<_>>();
        answers.insert("provider_categories".to_owned(), Value::Array(parsed));
    }
}

fn workflow_spec(locale: &str) -> Value {
    json!({
        "id": "gx-wizard-workflow",
        "title": tx(locale, "wizard.qa.title", "GX Wizard"),
        "version": "1.0.0",
        "presentation": { "default_locale": locale },
        "questions": [{
            "id": "workflow",
            "type": "enum",
            "title": tx(locale, "wizard.qa.select_workflow", "Select workflow"),
            "required": true,
            "default": "assistant_bundle",
            "choice_labels": {
                "assistant_bundle": tx(locale, "wizard.qa.workflow.assistant_bundle.label", "Assistant bundle"),
                "assistant_template_create": tx(locale, "wizard.qa.workflow.assistant_template_create.label", "Create assistant template"),
                "assistant_template_update": tx(locale, "wizard.qa.workflow.assistant_template_update.label", "Update assistant template"),
                "domain_template_create": tx(locale, "wizard.qa.workflow.domain_template_create.label", "Create domain template"),
                "domain_template_update": tx(locale, "wizard.qa.workflow.domain_template_update.label", "Update domain template")
            },
            "choice_descriptions": {
                "assistant_bundle": tx(locale, "wizard.qa.workflow.assistant_bundle.description", "Build bundle handoff inputs that combine assistant, domain, deployment, and provider settings."),
                "assistant_template_create": tx(locale, "wizard.qa.workflow.assistant_template_create.description", "Generate a new assistant template JSON file from a source reference."),
                "assistant_template_update": tx(locale, "wizard.qa.workflow.assistant_template_update.description", "Refresh an existing assistant template output from a source reference."),
                "domain_template_create": tx(locale, "wizard.qa.workflow.domain_template_create.description", "Generate a new domain template JSON file from a source reference."),
                "domain_template_update": tx(locale, "wizard.qa.workflow.domain_template_update.description", "Refresh an existing domain template output from a source reference.")
            },
            "choices": [
                "assistant_bundle",
                "assistant_template_create",
                "assistant_template_update",
                "domain_template_create",
                "domain_template_update"
            ]
        }]
    })
}

fn detail_spec(workflow: &str, locale: &str) -> Value {
    let questions = match workflow {
        "assistant_template_create" | "assistant_template_update" => vec![
            json!({"id": "template_source", "type": "string", "title": tx(locale, "wizard.qa.template_source", "Where to load the template from"), "description": tx(locale, "wizard.qa.template_source.description", "Template source: use an absolute path, relative path, oci://, repo://, or store:// reference."), "required": true, "default": "templates/assistant/default.json"}),
            json!({"id": "template_output_path", "type": "string", "title": tx(locale, "wizard.qa.template_output_path", "Template output path"), "description": tx(locale, "wizard.qa.template_output_path.description", "Where the generated assistant template JSON should be written."), "required": true, "default": if workflow.ends_with("create") { "templates/assistant/new.json" } else { "templates/assistant/updated.json" }}),
            json!({"id": "latest_policy", "type": "enum", "title": tx(locale, "wizard.qa.latest_policy", "How to handle :latest references"), "required": false, "choice_labels": latest_policy_choice_labels(locale), "choice_descriptions": latest_policy_choice_descriptions(locale), "choices": ["pin", "keep_latest"], "default": "pin"}),
        ],
        "domain_template_create" | "domain_template_update" => vec![
            json!({"id": "template_source", "type": "string", "title": tx(locale, "wizard.qa.template_source", "Where to load the template from"), "description": tx(locale, "wizard.qa.template_source.description", "Template source: use an absolute path, relative path, oci://, repo://, or store:// reference."), "required": true, "default": "templates/domain/default.json"}),
            json!({"id": "template_output_path", "type": "string", "title": tx(locale, "wizard.qa.template_output_path", "Template output path"), "description": tx(locale, "wizard.qa.template_output_path.description", "Where the generated domain template JSON should be written."), "required": true, "default": if workflow.ends_with("create") { "templates/domain/new.json" } else { "templates/domain/updated.json" }}),
            json!({"id": "latest_policy", "type": "enum", "title": tx(locale, "wizard.qa.latest_policy", "How to handle :latest references"), "required": false, "choice_labels": latest_policy_choice_labels(locale), "choice_descriptions": latest_policy_choice_descriptions(locale), "choices": ["pin", "keep_latest"], "default": "pin"}),
        ],
        _ => vec![
            json!({"id": "mode", "type": "enum", "title": tx(locale, "wizard.qa.bundle_mode", "What do you want to do with this bundle?"), "required": true, "choice_labels": bundle_mode_choice_labels(locale), "choice_descriptions": bundle_mode_choice_descriptions(locale), "default": "create", "choices": ["create", "update", "doctor"]}),
            json!({"id": "bundle_name", "type": "string", "title": tx(locale, "wizard.qa.bundle_name", "Bundle display name"), "description": tx(locale, "wizard.qa.bundle_name.description", "Human-readable name used when presenting the generated bundle."), "required": true, "default": "GX Bundle"}),
            json!({"id": "bundle_id", "type": "string", "title": tx(locale, "wizard.qa.bundle_id", "Bundle id (machine-readable)"), "description": tx(locale, "wizard.qa.bundle_id.description", "Stable machine-readable identifier, typically lowercase with hyphens."), "required": true, "default": "gx-bundle"}),
            json!({"id": "output_dir", "type": "string", "title": tx(locale, "wizard.qa.output_dir", "Where to write generated bundle files"), "description": tx(locale, "wizard.qa.output_dir.description", "Directory where generated bundle artifacts should be written. The final bundle file will be written to ./dist/<bundle-id>.gtbundle inside this directory."), "required": true, "default": "dist/bundle"}),
            json!({"id": "assistant_template_source", "type": "string", "title": tx(locale, "wizard.qa.assistant_template_source", "Where to load the assistant template from"), "description": tx(locale, "wizard.qa.assistant_template_source.description", "Assistant template source: use an absolute path, relative path, oci://, repo://, or store:// reference."), "required": true, "default": "templates/assistant/default.json"}),
            json!({"id": "domain_template_source", "type": "string", "title": tx(locale, "wizard.qa.domain_template_source", "Where to load the domain template from"), "description": tx(locale, "wizard.qa.domain_template_source.description", "Domain template source: use an absolute path, relative path, oci://, repo://, or store:// reference."), "required": true, "default": "templates/domain/default.json"}),
            json!({"id": "provider_categories", "type": "string", "title": tx(locale, "wizard.qa.provider_categories", "Provider categories"), "description": tx(locale, "wizard.qa.provider_categories.description", "Comma-separated provider categories such as llm, search, or storage."), "required": true, "default": "llm"}),
            json!({"id": "latest_policy", "type": "enum", "title": tx(locale, "wizard.qa.latest_policy", "How to handle :latest references"), "required": false, "choice_labels": latest_policy_choice_labels(locale), "choice_descriptions": latest_policy_choice_descriptions(locale), "choices": ["pin", "keep_latest"], "default": "pin"}),
        ],
    };
    json!({
        "id": format!("gx-wizard-{workflow}"),
        "title": tx(locale, "wizard.qa.title", "GX Wizard"),
        "version": "1.0.0",
        "presentation": { "default_locale": locale },
        "questions": questions
    })
}

fn bundle_mode_choice_labels(locale: &str) -> Value {
    json!({
        "create": tx(locale, "wizard.qa.bundle_mode.create.label", "Create bundle"),
        "update": tx(locale, "wizard.qa.bundle_mode.update.label", "Update bundle"),
        "doctor": tx(locale, "wizard.qa.bundle_mode.doctor.label", "Doctor bundle")
    })
}

fn bundle_mode_choice_descriptions(locale: &str) -> Value {
    json!({
        "create": tx(locale, "wizard.qa.bundle_mode.create.description", "Create a fresh bundle configuration and handoff output."),
        "update": tx(locale, "wizard.qa.bundle_mode.update.description", "Refresh bundle inputs and outputs for an existing bundle setup."),
        "doctor": tx(locale, "wizard.qa.bundle_mode.doctor.description", "Inspect and repair bundle configuration issues without changing the workflow type.")
    })
}

fn latest_policy_choice_labels(locale: &str) -> Value {
    json!({
        "pin": tx(locale, "wizard.qa.latest_policy.pin.label", "Pin resolved digest"),
        "keep_latest": tx(locale, "wizard.qa.latest_policy.keep_latest.label", "Keep :latest reference")
    })
}

fn latest_policy_choice_descriptions(locale: &str) -> Value {
    json!({
        "pin": tx(locale, "wizard.qa.latest_policy.pin.description", "Resolve :latest now and store the concrete digest for deterministic replay."),
        "keep_latest": tx(locale, "wizard.qa.latest_policy.keep_latest.description", "Preserve the moving :latest tag and re-resolve it in future runs.")
    })
}

fn tx(locale: &str, key: &str, fallback: &str) -> String {
    let translated = tr(locale, key);
    if translated == key {
        fallback.to_owned()
    } else {
        translated
    }
}
