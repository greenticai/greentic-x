use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use greentic_qa_lib::{I18nConfig, QaLibError, WizardDriver, WizardFrontend, WizardRunConfig};
use serde_json::Value;

use crate::WizardAnswerDocument;
use crate::i18n::resolved_wizard_i18n;

use super::catalog::{RemoteCatalogFetcher, load_catalogs};

const GX_WIZARD_FORM: &str = include_str!("../../questions/wizard.form.json");
const GX_WIZARD_CORE_FORM: &str = include_str!("../../questions/core.json");
const GX_WIZARD_COMPOSITION_FORM: &str = include_str!("../../questions/composition.json");
const GX_WIZARD_PROVIDERS_FORM: &str = include_str!("../../questions/providers.json");
const CANCELLED_SENTINEL: &str = "__gx_wizard_cancelled__";

pub(crate) fn collect_interactive_answers(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<bool, String> {
    let spec = build_runtime_form_spec_for_document(cwd, document, fetcher)?;
    let initial_answers = serde_json::to_string(&Value::Object(document.answers.clone()))
        .map_err(|err| format!("failed to serialize initial answers: {err}"))?;
    let config = WizardRunConfig {
        spec_json: serde_json::to_string(&spec)
            .map_err(|err| format!("failed to serialize QA wizard spec: {err}"))?,
        initial_answers_json: Some(initial_answers),
        frontend: WizardFrontend::JsonUi,
        i18n: I18nConfig {
            locale: Some(document.locale.clone()),
            resolved: Some(resolved_wizard_i18n(&document.locale)),
            debug: false,
        },
        verbose: false,
    };

    let mut driver =
        WizardDriver::new(config).map_err(|err| format!("GX QA wizard failed: {err}"))?;

    loop {
        driver
            .next_payload_json()
            .map_err(|err| format!("GX QA wizard failed: {err}"))?;
        if driver.is_complete() {
            break;
        }

        let ui_raw = driver
            .last_ui_json()
            .ok_or_else(|| "GX QA wizard failed: missing last_ui_json".to_owned())?;
        let ui: Value = serde_json::from_str(ui_raw)
            .map_err(|err| format!("GX QA wizard failed: failed to parse UI payload: {err}"))?;
        let question_id = ui
            .get("next_question_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "GX QA wizard failed: missing next_question_id".to_owned())?
            .to_owned();
        let question = find_question(&ui, &question_id)
            .map_err(|err| format!("GX QA wizard failed: {err}"))?;

        loop {
            let answer = match prompt_for_question(&question_id, &question) {
                Ok(answer) => answer,
                Err(QaLibError::Component(message)) if message == CANCELLED_SENTINEL => {
                    return Ok(false);
                }
                Err(err) => return Err(format!("GX QA wizard failed: {err}")),
            };
            let patch = serde_json::json!({ question_id.clone(): answer }).to_string();
            let submit = driver
                .submit_patch_json(&patch)
                .map_err(|err| format!("GX QA wizard failed: {err}"))?;
            if submit.status != "error" {
                break;
            }

            match classify_submit_error(&submit.response_json, &question_id) {
                SubmitErrorDisposition::Retry(message) => {
                    eprintln!("{message}");
                }
                SubmitErrorDisposition::Fatal(message) => {
                    return Err(format!("GX QA wizard failed: {message}"));
                }
                SubmitErrorDisposition::Advance => break,
            }
        }
    }

    let result = driver
        .finish()
        .map_err(|err| format!("GX QA wizard failed: {err}"))?;
    apply_answer_object(document, result.answer_set.answers);
    Ok(true)
}

pub(crate) fn build_runtime_form_spec_for_document(
    cwd: &Path,
    document: &WizardAnswerDocument,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<Value, String> {
    let catalogs = load_catalogs(cwd, &catalog_refs(document), fetcher)?;
    let manifests = find_solution_manifests(cwd)?;
    build_runtime_form_spec(document, &catalogs, &manifests)
}

fn find_question(ui: &Value, question_id: &str) -> Result<Value, String> {
    ui.get("questions")
        .and_then(Value::as_array)
        .and_then(|questions| {
            questions.iter().find_map(|question| {
                (question.get("id").and_then(Value::as_str) == Some(question_id))
                    .then(|| question.clone())
            })
        })
        .ok_or_else(|| format!("question `{question_id}` missing from UI payload"))
}

enum SubmitErrorDisposition {
    Retry(String),
    Advance,
    Fatal(String),
}

fn classify_submit_error(response_json: &str, question_id: &str) -> SubmitErrorDisposition {
    let Ok(value) = serde_json::from_str::<Value>(response_json) else {
        return SubmitErrorDisposition::Fatal(format!(
            "failed to parse validation response: {response_json}"
        ));
    };
    let validation = value.get("validation").cloned().unwrap_or(Value::Null);
    let unknown_fields = validation
        .get("unknown_fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !unknown_fields.is_empty() {
        let fields = unknown_fields
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        return SubmitErrorDisposition::Fatal(format!("unknown answer fields: {fields}"));
    }

    let missing_required = validation
        .get("missing_required")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if missing_required
        .iter()
        .filter_map(Value::as_str)
        .any(|field| field == question_id)
    {
        return SubmitErrorDisposition::Retry(format!("{question_id} is required."));
    }

    let question_errors = validation
        .get("errors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|error| {
            error
                .get("question_id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == question_id)
                || error
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| path == format!("/{question_id}"))
        })
        .collect::<Vec<_>>();
    if let Some(message) = question_errors
        .iter()
        .filter_map(|error| error.get("message").and_then(Value::as_str))
        .next()
    {
        return SubmitErrorDisposition::Retry(message.to_owned());
    }

    SubmitErrorDisposition::Advance
}

fn build_runtime_form_spec(
    document: &WizardAnswerDocument,
    catalogs: &crate::WizardCatalogSet,
    manifests: &[PathBuf],
) -> Result<Value, String> {
    let mut root = serde_json::from_str::<Value>(GX_WIZARD_FORM)
        .map_err(|err| format!("failed to parse embedded GX wizard form: {err}"))?;

    let include_refs = root
        .get("includes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.get("form_ref")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();

    let mut merged_questions = Vec::new();
    for form_ref in include_refs {
        let fragment = embedded_form_fragment(&form_ref)?;
        merged_questions.extend(
            fragment
                .get("questions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        );
    }
    root["includes"] = Value::Array(Vec::new());
    root["questions"] = Value::Array(merged_questions);

    inject_runtime_defaults(&mut root, document, catalogs, manifests)?;
    Ok(root)
}

fn embedded_form_fragment(form_ref: &str) -> Result<Value, String> {
    let raw = match form_ref {
        "gx.questions.core" => GX_WIZARD_CORE_FORM,
        "gx.questions.composition" => GX_WIZARD_COMPOSITION_FORM,
        "gx.questions.providers" => GX_WIZARD_PROVIDERS_FORM,
        other => return Err(format!("unknown embedded GX QA form include `{other}`")),
    };
    serde_json::from_str(raw).map_err(|err| format!("failed to parse {form_ref}: {err}"))
}

fn inject_runtime_defaults(
    form: &mut Value,
    document: &WizardAnswerDocument,
    catalogs: &crate::WizardCatalogSet,
    manifests: &[PathBuf],
) -> Result<(), String> {
    let Some(questions) = form.get_mut("questions").and_then(Value::as_array_mut) else {
        return Err("GX wizard form missing questions array".to_owned());
    };

    for question in questions {
        let Some(id) = question
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        match id.as_str() {
            "existing_solution_path" => {
                if !manifests.is_empty() {
                    question["type"] = Value::String("enum".to_owned());
                    question["choices"] = Value::Array(
                        manifests
                            .iter()
                            .map(|path| Value::String(path.display().to_string()))
                            .collect(),
                    );
                    if let Some(first) = manifests.first() {
                        set_default_value(question, Some(first.display().to_string()));
                    }
                    append_description(
                        question,
                        &format!(
                            "Discovered solutions: {}",
                            manifests
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                }
            }
            "template_entry_id" => {
                if !catalogs.templates.is_empty() {
                    question["type"] = Value::String("enum".to_owned());
                    question["choices"] = Value::Array(
                        catalogs
                            .templates
                            .iter()
                            .map(|entry| Value::String(entry.entry_id.clone()))
                            .collect(),
                    );
                    set_default_value(
                        question,
                        catalogs
                            .templates
                            .first()
                            .map(|entry| entry.entry_id.clone()),
                    );
                    append_description(
                        question,
                        &format!(
                            "Available templates: {}",
                            catalogs
                                .templates
                                .iter()
                                .map(|entry| format!("{} ({})", entry.entry_id, entry.display_name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                }
            }
            "provider_preset_entry_id" => {
                if !catalogs.provider_presets.is_empty() {
                    question["type"] = Value::String("enum".to_owned());
                    question["choices"] = Value::Array(
                        catalogs
                            .provider_presets
                            .iter()
                            .map(|entry| Value::String(entry.entry_id.clone()))
                            .collect(),
                    );
                    set_default_value(
                        question,
                        catalogs
                            .provider_presets
                            .first()
                            .map(|entry| entry.entry_id.clone()),
                    );
                    append_description(
                        question,
                        &format!(
                            "Available provider presets: {}",
                            catalogs
                                .provider_presets
                                .iter()
                                .map(|entry| format!("{} ({})", entry.entry_id, entry.display_name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                }
            }
            _ => {}
        }

        if let Some(existing) = document.answers.get(&id) {
            match existing {
                Value::String(value) if !value.trim().is_empty() => {
                    set_default_value(question, Some(value.clone()));
                }
                Value::Array(items) if id == "catalog_oci_refs" || id == "provider_refs" => {
                    let joined = items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !joined.is_empty() {
                        set_default_value(question, Some(joined));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn append_description(question: &mut Value, extra: &str) {
    let current = question
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let merged = if current.is_empty() {
        extra.to_owned()
    } else {
        format!("{current} {extra}")
    };
    question["description"] = Value::String(merged);
}

fn set_default_value(question: &mut Value, value: Option<String>) {
    match value {
        Some(value) => question["default_value"] = Value::String(value),
        None => question["default_value"] = Value::Null,
    }
}

fn prompt_for_question(question_id: &str, question: &Value) -> Result<Value, QaLibError> {
    let title = question
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(question_id);
    let description = question.get("description").and_then(Value::as_str);
    let kind = question
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("string");
    let required = question
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let default_value = question
        .get("default_value")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let choices = question
        .get("choices")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut stdout = io::stdout();
    writeln!(stdout, "{title}").map_err(io_error)?;
    if let Some(description) = description
        && !description.is_empty()
    {
        writeln!(stdout, "{description}").map_err(io_error)?;
    }
    if kind == "enum" && !choices.is_empty() {
        for (index, choice) in choices.iter().enumerate() {
            writeln!(stdout, "{}. {}", index + 1, choice).map_err(io_error)?;
        }
    } else if !choices.is_empty() {
        writeln!(stdout, "Choices: {}", choices.join(", ")).map_err(io_error)?;
    }
    let prompt = match default_prompt_value(kind, default_value.as_deref(), &choices) {
        Some(default) if !default.is_empty() => format!("> [{default}] "),
        _ => "> ".to_owned(),
    };

    loop {
        write!(stdout, "{prompt}").map_err(io_error)?;
        stdout.flush().map_err(io_error)?;
        let mut line = String::new();
        io::stdin().read_line(&mut line).map_err(io_error)?;
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("m") || trimmed == "0" {
            return Err(QaLibError::Component(CANCELLED_SENTINEL.to_owned()));
        }

        let value = if trimmed.is_empty() {
            if let Some(default) = default_value.as_deref() {
                default.to_owned()
            } else if required {
                writeln!(stdout, "This question requires an answer.").map_err(io_error)?;
                continue;
            } else {
                String::new()
            }
        } else {
            trimmed.to_owned()
        };

        match parse_question_value(kind, &value, &choices) {
            Ok(parsed) => return Ok(parsed),
            Err(err) => {
                writeln!(stdout, "{err}").map_err(io_error)?;
            }
        }
    }
}

fn parse_question_value(kind: &str, raw: &str, choices: &[String]) -> Result<Value, String> {
    match kind {
        "boolean" => match raw.to_ascii_lowercase().as_str() {
            "y" | "yes" | "true" | "1" => Ok(Value::Bool(true)),
            "n" | "no" | "false" | "0" => Ok(Value::Bool(false)),
            _ => Err("Enter yes/no, y/n, true/false, or 1/0.".to_owned()),
        },
        "enum" => {
            if choices.is_empty() {
                Ok(Value::String(raw.to_owned()))
            } else if let Some(choice) = enum_choice_value(raw, choices) {
                Ok(Value::String(choice.to_owned()))
            } else {
                Err(format!("Choose a number between 1 and {}.", choices.len()))
            }
        }
        "integer" => raw
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| "Enter a whole number.".to_owned()),
        "number" => raw
            .parse::<f64>()
            .map(Value::from)
            .map_err(|_| "Enter a number.".to_owned()),
        _ => Ok(Value::String(raw.to_owned())),
    }
}

fn default_prompt_value<'a>(
    kind: &str,
    default_value: Option<&'a str>,
    choices: &'a [String],
) -> Option<String> {
    match (kind, default_value) {
        ("enum", Some(default)) => choices
            .iter()
            .position(|choice| choice == default)
            .map(|index| (index + 1).to_string())
            .or_else(|| Some(default.to_owned())),
        (_, Some(default)) => Some(default.to_owned()),
        _ => None,
    }
}

fn enum_choice_value<'a>(raw: &str, choices: &'a [String]) -> Option<&'a str> {
    raw.parse::<usize>()
        .ok()
        .and_then(|index| choices.get(index.saturating_sub(1)))
        .map(String::as_str)
        .or_else(|| {
            choices
                .iter()
                .find(|choice| choice.as_str() == raw)
                .map(String::as_str)
        })
}

fn apply_answer_object(document: &mut WizardAnswerDocument, answers: Value) {
    let Some(map) = answers.as_object() else {
        return;
    };
    for (key, value) in map {
        if value.is_null() {
            document.answers.remove(key);
        } else {
            document.answers.insert(key.clone(), value.clone());
        }
    }
}

fn catalog_refs(document: &WizardAnswerDocument) -> Vec<String> {
    document
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
        .unwrap_or_default()
}

fn find_solution_manifests(cwd: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    collect_solution_manifests(cwd, &mut found, 0)?;
    found.sort();
    Ok(found)
}

fn collect_solution_manifests(
    dir: &Path,
    found: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), String> {
    if depth > 4 || !dir.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_solution_manifests(&path, found, depth + 1)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".solution.json"))
        {
            found.push(path);
        }
    }
    Ok(())
}

fn io_error(err: io::Error) -> QaLibError {
    QaLibError::Component(format!("interactive prompt failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssistantTemplateCatalogEntry, CatalogProvenance, ProviderPresetCatalogEntry,
        WizardCatalogSet,
    };
    use serde_json::Map;

    #[test]
    fn build_runtime_form_spec_merges_embedded_includes() {
        let document = WizardAnswerDocument {
            wizard_id: "greentic-bundle.wizard.run".to_owned(),
            schema_id: "greentic-bundle.wizard.answers".to_owned(),
            schema_version: "1.0.0".to_owned(),
            locale: "en".to_owned(),
            answers: Map::new(),
            locks: Map::new(),
        };
        let form =
            build_runtime_form_spec(&document, &WizardCatalogSet::default(), &[]).expect("form");
        let questions = form["questions"].as_array().expect("questions");
        assert!(questions.iter().any(|question| question["id"] == "mode"));
        assert!(
            questions
                .iter()
                .any(|question| question["id"] == "template_mode")
        );
        assert!(
            questions
                .iter()
                .any(|question| question["id"] == "provider_selection")
        );
    }

    #[test]
    fn build_runtime_form_spec_injects_catalog_choices() {
        let document = WizardAnswerDocument {
            wizard_id: "greentic-bundle.wizard.run".to_owned(),
            schema_id: "greentic-bundle.wizard.answers".to_owned(),
            schema_version: "1.0.0".to_owned(),
            locale: "en".to_owned(),
            answers: Map::new(),
            locks: Map::new(),
        };
        let catalogs = WizardCatalogSet {
            templates: vec![AssistantTemplateCatalogEntry {
                entry_id: "assistant.network.phase1".to_owned(),
                kind: "assistant-template".to_owned(),
                version: "1.0.0".to_owned(),
                display_name: "Network Assistant".to_owned(),
                description: String::new(),
                assistant_template_ref: "oci://example/template:latest".to_owned(),
                domain_template_ref: None,
                bundle_ref: None,
                provenance: Some(CatalogProvenance {
                    source_type: "store".to_owned(),
                    source_ref: "store://demo/catalog".to_owned(),
                    resolved_digest: None,
                }),
            }],
            provider_presets: vec![ProviderPresetCatalogEntry {
                entry_id: "preset.webchat".to_owned(),
                kind: "provider-preset".to_owned(),
                version: "1.0.0".to_owned(),
                display_name: "Webchat".to_owned(),
                description: String::new(),
                provider_refs: vec!["oci://example/provider:latest".to_owned()],
                bundle_ref: None,
                provenance: None,
            }],
            ..WizardCatalogSet::default()
        };

        let form = build_runtime_form_spec(&document, &catalogs, &[]).expect("form");
        let questions = form["questions"].as_array().expect("questions");
        let template = questions
            .iter()
            .find(|question| question["id"] == "template_entry_id")
            .expect("template question");
        let provider = questions
            .iter()
            .find(|question| question["id"] == "provider_preset_entry_id")
            .expect("provider question");

        assert_eq!(template["type"], "enum");
        assert_eq!(provider["type"], "enum");
        assert_eq!(template["choices"][0], "assistant.network.phase1");
        assert_eq!(provider["choices"][0], "preset.webchat");
    }

    #[test]
    fn parse_question_value_supports_boolean_and_enum() {
        assert_eq!(
            parse_question_value("boolean", "yes", &[]).expect("boolean"),
            Value::Bool(true)
        );
        assert_eq!(
            parse_question_value("enum", "teams", &["teams".to_owned()]).expect("enum"),
            Value::String("teams".to_owned())
        );
        assert_eq!(
            parse_question_value(
                "enum",
                "2",
                &["webchat".to_owned(), "teams".to_owned(), "slack".to_owned()]
            )
            .expect("numeric enum"),
            Value::String("teams".to_owned())
        );
        assert!(parse_question_value("enum", "slack", &["teams".to_owned()]).is_err());
    }

    #[test]
    fn default_prompt_value_uses_choice_index_for_enums() {
        let choices = vec!["webchat".to_owned(), "teams".to_owned(), "slack".to_owned()];
        assert_eq!(
            default_prompt_value("enum", Some("teams"), &choices),
            Some("2".to_owned())
        );
        assert_eq!(
            default_prompt_value("string", Some("dist"), &choices),
            Some("dist".to_owned())
        );
    }

    #[test]
    fn classify_submit_error_retries_question_local_validation() {
        let response = serde_json::json!({
            "validation": {
                "errors": [
                    {
                        "question_id": "solution_name",
                        "path": "/solution_name",
                        "message": "qa_spec.min_len"
                    }
                ],
                "missing_required": [],
                "unknown_fields": []
            }
        });

        match classify_submit_error(&response.to_string(), "solution_name") {
            SubmitErrorDisposition::Retry(message) => assert_eq!(message, "qa_spec.min_len"),
            _ => panic!("expected retry disposition"),
        }
    }

    #[test]
    fn classify_submit_error_advances_on_incomplete_form_state() {
        let response = serde_json::json!({
            "validation": {
                "errors": [],
                "missing_required": ["solution_name", "template_mode"],
                "unknown_fields": []
            }
        });

        assert!(matches!(
            classify_submit_error(&response.to_string(), "mode"),
            SubmitErrorDisposition::Advance
        ));
    }
}
