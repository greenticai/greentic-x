use std::io::{self, Write};

use greentic_qa_lib::{I18nConfig, WizardDriver, WizardFrontend, WizardRunConfig};
use serde_json::{Map, Value, json};

use crate::WizardAnswerDocument;
use crate::i18n::tr;

pub(crate) fn collect_interactive_answers(
    document: &mut WizardAnswerDocument,
    cli_mode: Option<&str>,
    locale: &str,
) -> Result<(), String> {
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
    document
        .answers
        .insert("workflow".to_owned(), Value::String(workflow.clone()));

    let detail_answers = run_qa_form(detail_spec(&workflow, locale), locale)?;
    merge_answers(&mut document.answers, detail_answers);
    normalize_qa_field_shapes(&mut document.answers);
    Ok(())
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
        let answer = prompt_question(question)?;
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

fn prompt_question(question: &Value) -> Result<Value, String> {
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
        "enum" => prompt_enum(title, question, required),
        _ => prompt_string(title, question, required),
    }
}

fn prompt_enum(title: &str, question: &Value, required: bool) -> Result<Value, String> {
    let choices = question
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| "qa enum question missing choices".to_owned())?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let default = question
        .get("default")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut stdout = io::stdout();
    writeln!(stdout, "{title}").map_err(|err| format!("write prompt failed: {err}"))?;
    for (idx, choice) in choices.iter().enumerate() {
        writeln!(stdout, "{}. {}", idx + 1, choice)
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

fn prompt_string(title: &str, question: &Value, required: bool) -> Result<Value, String> {
    let default = question
        .get("default")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut stdout = io::stdout();
    loop {
        if let Some(default) = default.as_deref() {
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
            json!({"id": "template_source", "type": "string", "title": tx(locale, "wizard.qa.template_source", "Template source reference"), "required": true, "default": "local://templates/assistant/default"}),
            json!({"id": "template_output_path", "type": "string", "title": tx(locale, "wizard.qa.template_output_path", "Template output path"), "required": true, "default": if workflow.ends_with("create") { "templates/assistant/new.json" } else { "templates/assistant/updated.json" }}),
            json!({"id": "latest_policy", "type": "enum", "title": tx(locale, "wizard.qa.latest_policy", "Latest policy"), "required": false, "choices": ["pin", "keep_latest"], "default": "pin"}),
        ],
        "domain_template_create" | "domain_template_update" => vec![
            json!({"id": "template_source", "type": "string", "title": tx(locale, "wizard.qa.template_source", "Template source reference"), "required": true, "default": "local://templates/domain/default"}),
            json!({"id": "template_output_path", "type": "string", "title": tx(locale, "wizard.qa.template_output_path", "Template output path"), "required": true, "default": if workflow.ends_with("create") { "templates/domain/new.json" } else { "templates/domain/updated.json" }}),
            json!({"id": "latest_policy", "type": "enum", "title": tx(locale, "wizard.qa.latest_policy", "Latest policy"), "required": false, "choices": ["pin", "keep_latest"], "default": "pin"}),
        ],
        _ => vec![
            json!({"id": "mode", "type": "enum", "title": tx(locale, "wizard.qa.bundle_mode", "Bundle mode"), "required": true, "default": "create", "choices": ["create", "update", "doctor"]}),
            json!({"id": "bundle_name", "type": "string", "title": tx(locale, "wizard.qa.bundle_name", "Bundle name"), "required": true, "default": "GX Bundle"}),
            json!({"id": "bundle_id", "type": "string", "title": tx(locale, "wizard.qa.bundle_id", "Bundle id"), "required": true, "default": "gx-bundle"}),
            json!({"id": "output_dir", "type": "string", "title": tx(locale, "wizard.qa.output_dir", "Bundle output directory"), "required": true, "default": "dist/bundle"}),
            json!({"id": "assistant_template_source", "type": "string", "title": tx(locale, "wizard.qa.assistant_template_source", "Assistant template source"), "required": true, "default": "local://templates/assistant/default"}),
            json!({"id": "domain_template_source", "type": "string", "title": tx(locale, "wizard.qa.domain_template_source", "Domain template source"), "required": true, "default": "local://templates/domain/default"}),
            json!({"id": "deployment_profile", "type": "string", "title": tx(locale, "wizard.qa.deployment_profile", "Deployment profile"), "required": true, "default": "default"}),
            json!({"id": "deployment_target", "type": "string", "title": tx(locale, "wizard.qa.deployment_target", "Deployment target"), "required": true, "default": "local"}),
            json!({"id": "provider_categories", "type": "string", "title": tx(locale, "wizard.qa.provider_categories", "Provider categories (csv)"), "required": true, "default": "llm"}),
            json!({"id": "bundle_output_path", "type": "string", "title": tx(locale, "wizard.qa.bundle_output_path", "Bundle output path"), "required": true, "default": "dist/app.gtbundle"}),
            json!({"id": "latest_policy", "type": "enum", "title": tx(locale, "wizard.qa.latest_policy", "Latest policy"), "required": false, "choices": ["pin", "keep_latest"], "default": "pin"}),
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

fn tx(locale: &str, key: &str, fallback: &str) -> String {
    let translated = tr(locale, key);
    if translated == key {
        fallback.to_owned()
    } else {
        translated
    }
}
