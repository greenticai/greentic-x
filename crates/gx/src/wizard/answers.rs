use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::{
    CompositionRequest, GX_WIZARD_ID, GX_WIZARD_SCHEMA_ID, ResolvedSolutionIntent,
    WizardAnswerDocument, WizardCommonArgs, WizardNormalizedAnswers,
};

pub(crate) fn load_wizard_answers(
    cwd: &Path,
    args: &WizardCommonArgs,
    target_schema_version: &str,
    _locale: &str,
) -> Result<WizardAnswerDocument, String> {
    let mut document = WizardAnswerDocument {
        wizard_id: GX_WIZARD_ID.to_owned(),
        schema_id: GX_WIZARD_SCHEMA_ID.to_owned(),
        schema_version: target_schema_version.to_owned(),
        locale: "en".to_owned(),
        answers: serde_json::Map::new(),
        locks: serde_json::Map::new(),
    };
    let Some(path) = args.answers.as_ref() else {
        return Ok(document);
    };
    let resolved = if path.is_absolute() {
        path.clone()
    } else {
        cwd.join(path)
    };
    let raw = fs::read_to_string(&resolved)
        .map_err(|err| format!("failed to read answers file {}: {err}", resolved.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse answers file {}: {err}", resolved.display()))?;
    let parsed: WizardAnswerDocument = serde_json::from_value(value)
        .map_err(|err| format!("failed to decode answer document: {err}"))?;
    if parsed.schema_id != GX_WIZARD_SCHEMA_ID {
        return Err(format!(
            "answers document schema_id mismatch: expected {GX_WIZARD_SCHEMA_ID}, got {}",
            parsed.schema_id
        ));
    }
    let source_schema_version = normalize_schema_version(&parsed.schema_version, "en")?;
    if source_schema_version != target_schema_version && !args.migrate {
        return Err(format!(
            "answers document schema_version {source_schema_version} differs from target {target_schema_version}; rerun with --migrate"
        ));
    }
    document = parsed;
    document.schema_version = target_schema_version.to_owned();
    Ok(document)
}

pub(crate) fn normalize_schema_version(raw: &str, _locale: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.parse::<u64>().is_err()) {
        return Err(format!(
            "invalid schema version; expected semantic version like 1.0.0 ({trimmed})"
        ));
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn normalize_wizard_answers(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    _cli_mode: Option<&str>,
    _locale: &str,
    _resolve_remote: bool,
) -> Result<WizardNormalizedAnswers, String> {
    let mut request = normalize_composition_request(document);
    if request.mode == "update" {
        prefill_from_existing_solution(cwd, document, &mut request)?;
    }
    upsert_computed_answers(document, &request);
    Ok(WizardNormalizedAnswers::Composition(request))
}

fn normalize_composition_request(document: &mut WizardAnswerDocument) -> CompositionRequest {
    let solution_name = upsert_string_answer(document, "solution_name", "GX Solution");
    let default_solution_id = slugify(&solution_name);
    let solution_id = upsert_string_answer(document, "solution_id", &default_solution_id);
    let output_dir = upsert_string_answer(document, "output_dir", "dist");
    let bundle_output_path = bundle_output_path(&output_dir, &solution_id);
    CompositionRequest {
        mode: upsert_string_answer(document, "mode", "create"),
        template_mode: upsert_string_answer(document, "template_mode", "basic_empty"),
        template_entry_id: optional_string_answer(document, "template_entry_id"),
        template_display_name: optional_string_answer(document, "template_display_name"),
        assistant_template_ref: optional_string_answer(document, "assistant_template_ref"),
        domain_template_ref: optional_string_answer(document, "domain_template_ref"),
        solution_name,
        solution_id: solution_id.clone(),
        description: upsert_string_answer(document, "description", ""),
        output_dir: output_dir.clone(),
        provider_selection: upsert_string_answer(document, "provider_selection", "webchat"),
        provider_preset_entry_id: optional_string_answer(document, "provider_preset_entry_id"),
        provider_preset_display_name: optional_string_answer(
            document,
            "provider_preset_display_name",
        ),
        provider_refs: upsert_string_array_answer(document, "provider_refs", &[]),
        overlay_entry_id: optional_string_answer(document, "overlay_entry_id"),
        overlay_display_name: optional_string_answer(document, "overlay_display_name"),
        overlay_default_locale: optional_string_answer(document, "overlay_default_locale"),
        overlay_tenant_id: optional_string_answer(document, "overlay_tenant_id"),
        catalog_oci_refs: upsert_string_array_answer(document, "catalog_oci_refs", &[]),
        catalog_resolution_policy: upsert_string_answer(
            document,
            "catalog_resolution_policy",
            "update_then_pin",
        ),
        bundle_output_path: bundle_output_path.clone(),
        solution_manifest_path: format!("{output_dir}/{solution_id}.solution.json"),
        toolchain_handoff_path: format!("{output_dir}/{solution_id}.toolchain-handoff.json"),
        launcher_answers_path: format!("{output_dir}/{solution_id}.launcher.answers.json"),
        pack_input_path: format!("{output_dir}/{solution_id}.pack.input.json"),
        bundle_plan_path: format!("{output_dir}/{solution_id}.bundle-plan.json"),
        bundle_answers_path: format!("{output_dir}/{solution_id}.bundle.answers.json"),
        setup_answers_path: format!("{output_dir}/{solution_id}.setup.answers.json"),
        gtc_setup_handoff_path: format!("{output_dir}/{solution_id}.gtc.setup.handoff.json"),
        gtc_start_handoff_path: format!("{output_dir}/{solution_id}.gtc.start.handoff.json"),
        readme_path: format!("{output_dir}/{solution_id}.README.generated.md"),
        existing_solution_path: optional_string_answer(document, "existing_solution_path"),
    }
}

fn prefill_from_existing_solution(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    request: &mut CompositionRequest,
) -> Result<(), String> {
    let path = request
        .existing_solution_path
        .clone()
        .unwrap_or_else(|| request.solution_manifest_path.clone());
    let resolved = if Path::new(&path).is_absolute() {
        Path::new(&path).to_path_buf()
    } else {
        cwd.join(&path)
    };
    if !resolved.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&resolved).map_err(|err| {
        format!(
            "failed to read existing solution {}: {err}",
            resolved.display()
        )
    })?;
    let manifest: ResolvedSolutionIntent = serde_json::from_str(&raw).map_err(|err| {
        format!(
            "failed to parse existing solution {}: {err}",
            resolved.display()
        )
    })?;

    request.solution_name = prefer_existing(&request.solution_name, &manifest.solution_name);
    request.solution_id = prefer_existing(&request.solution_id, &manifest.solution_id);
    request.description = prefer_existing(&request.description, &manifest.description);
    request.output_dir = prefer_existing(&request.output_dir, &manifest.output_dir);
    request.bundle_output_path = bundle_output_path(&request.output_dir, &request.solution_id);
    request.solution_manifest_path = format!(
        "{}/{}.solution.json",
        request.output_dir, request.solution_id
    );
    request.toolchain_handoff_path = format!(
        "{}/{}.toolchain-handoff.json",
        request.output_dir, request.solution_id
    );
    request.launcher_answers_path = format!(
        "{}/{}.launcher.answers.json",
        request.output_dir, request.solution_id
    );
    request.pack_input_path = format!(
        "{}/{}.pack.input.json",
        request.output_dir, request.solution_id
    );
    request.bundle_plan_path = format!(
        "{}/{}.bundle-plan.json",
        request.output_dir, request.solution_id
    );
    request.bundle_answers_path = format!(
        "{}/{}.bundle.answers.json",
        request.output_dir, request.solution_id
    );
    request.setup_answers_path = format!(
        "{}/{}.setup.answers.json",
        request.output_dir, request.solution_id
    );
    request.gtc_setup_handoff_path = format!(
        "{}/{}.gtc.setup.handoff.json",
        request.output_dir, request.solution_id
    );
    request.gtc_start_handoff_path = format!(
        "{}/{}.gtc.start.handoff.json",
        request.output_dir, request.solution_id
    );
    request.readme_path = format!(
        "{}/{}.README.generated.md",
        request.output_dir, request.solution_id
    );

    if let Some(entry_id) = manifest
        .template
        .get("entry_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    {
        request.template_entry_id = Some(entry_id);
    }
    if let Some(display_name) = manifest
        .template
        .get("display_name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    {
        request.template_display_name = Some(display_name);
    }
    if let Some(reference) = manifest
        .template
        .get("assistant_template_ref")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    {
        request.assistant_template_ref = Some(reference);
    }
    if let Some(reference) = manifest
        .template
        .get("domain_template_ref")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    {
        request.domain_template_ref = Some(reference);
    }

    if (request.provider_selection.is_empty() || request.provider_selection == "webchat")
        && let Some(provider) = manifest.provider_presets.first()
    {
        request.provider_preset_entry_id = provider
            .get("entry_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        request.provider_preset_display_name = provider
            .get("display_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        request.provider_selection = provider_selection_from_entry(provider);
        request.provider_refs = provider
            .get("provider_refs")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    document.answers.insert(
        "solution_name".to_owned(),
        Value::String(request.solution_name.clone()),
    );
    document.answers.insert(
        "solution_id".to_owned(),
        Value::String(request.solution_id.clone()),
    );
    document.answers.insert(
        "description".to_owned(),
        Value::String(request.description.clone()),
    );
    document.answers.insert(
        "output_dir".to_owned(),
        Value::String(request.output_dir.clone()),
    );
    Ok(())
}

fn bundle_output_path(output_dir: &str, solution_id: &str) -> String {
    format!("{output_dir}/dist/{solution_id}.gtbundle")
}

fn upsert_computed_answers(document: &mut WizardAnswerDocument, request: &CompositionRequest) {
    document.answers.insert(
        "bundle_output_path".to_owned(),
        Value::String(request.bundle_output_path.clone()),
    );
    document.answers.insert(
        "solution_manifest_path".to_owned(),
        Value::String(request.solution_manifest_path.clone()),
    );
    document.answers.insert(
        "toolchain_handoff_path".to_owned(),
        Value::String(request.toolchain_handoff_path.clone()),
    );
    document.answers.insert(
        "launcher_answers_path".to_owned(),
        Value::String(request.launcher_answers_path.clone()),
    );
    document.answers.insert(
        "pack_input_path".to_owned(),
        Value::String(request.pack_input_path.clone()),
    );
    document.answers.insert(
        "bundle_plan_path".to_owned(),
        Value::String(request.bundle_plan_path.clone()),
    );
    document.answers.insert(
        "bundle_answers_path".to_owned(),
        Value::String(request.bundle_answers_path.clone()),
    );
    document.answers.insert(
        "setup_answers_path".to_owned(),
        Value::String(request.setup_answers_path.clone()),
    );
    document.answers.insert(
        "readme_path".to_owned(),
        Value::String(request.readme_path.clone()),
    );
    document.answers.insert(
        "workflow".to_owned(),
        Value::String("compose_solution".to_owned()),
    );
}

fn prefer_existing(current: &str, existing: &str) -> String {
    if current.trim().is_empty()
        || current == "GX Solution"
        || current == "gx-solution"
        || current == "dist"
    {
        existing.to_owned()
    } else {
        current.to_owned()
    }
}

fn upsert_string_answer(document: &mut WizardAnswerDocument, key: &str, default: &str) -> String {
    let value = match document.answers.get(key) {
        Some(Value::String(existing)) => existing.to_owned(),
        _ => default.to_owned(),
    };
    document
        .answers
        .insert(key.to_owned(), Value::String(value.clone()));
    value
}

fn upsert_string_array_answer(
    document: &mut WizardAnswerDocument,
    key: &str,
    default: &[&str],
) -> Vec<String> {
    let values = match document.answers.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
        Some(Value::String(value)) if !value.trim().is_empty() => value
            .split(',')
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>(),
        _ => default.iter().map(|item| (*item).to_owned()).collect(),
    };
    document.answers.insert(
        key.to_owned(),
        Value::Array(values.iter().cloned().map(Value::String).collect()),
    );
    values
}

fn optional_string_answer(document: &WizardAnswerDocument, key: &str) -> Option<String> {
    document
        .answers
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_owned()
}

fn provider_selection_from_entry(provider: &Value) -> String {
    match provider.get("entry_id").and_then(Value::as_str) {
        Some("builtin.webchat") => "webchat".to_owned(),
        Some("builtin.teams") => "teams".to_owned(),
        Some("builtin.webex") => "webex".to_owned(),
        Some("builtin.slack") => "slack".to_owned(),
        Some(_) | None => "catalog".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn update_mode_prefills_from_existing_solution() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let cwd = temp.path();
        fs::create_dir_all(cwd.join("dist")).expect("mkdir");
        fs::write(
            cwd.join("dist/demo.solution.json"),
            serde_json::to_string_pretty(&json!({
                "schema_id": "gx.solution.intent",
                "schema_version": "1.0.0",
                "solution_id": "demo",
                "solution_name": "Demo Solution",
                "description": "Existing description",
                "output_dir": "dist",
                "template": {
                    "entry_id": "assistant.network.phase1",
                    "display_name": "Network Assistant"
                },
                "provider_presets": [{
                    "entry_id": "builtin.teams",
                    "display_name": "Teams",
                    "provider_refs": ["oci://ghcr.io/greenticai/packs/messaging/messaging-teams:stable"]
                }]
            }))
            .expect("serialize"),
        )
        .expect("write");

        let mut document = WizardAnswerDocument {
            wizard_id: GX_WIZARD_ID.to_owned(),
            schema_id: GX_WIZARD_SCHEMA_ID.to_owned(),
            schema_version: "1.0.0".to_owned(),
            locale: "en".to_owned(),
            answers: serde_json::Map::from_iter([
                ("mode".to_owned(), Value::String("update".to_owned())),
                (
                    "existing_solution_path".to_owned(),
                    Value::String("dist/demo.solution.json".to_owned()),
                ),
            ]),
            locks: serde_json::Map::new(),
        };
        let normalized =
            normalize_wizard_answers(cwd, &mut document, None, "en", false).expect("normalized");
        let WizardNormalizedAnswers::Composition(request) = normalized;
        assert_eq!(request.solution_name, "Demo Solution");
        assert_eq!(request.description, "Existing description");
        assert_eq!(request.provider_selection, "teams");
        assert_eq!(
            request.toolchain_handoff_path,
            "dist/demo.toolchain-handoff.json"
        );
        assert_eq!(
            request.launcher_answers_path,
            "dist/demo.launcher.answers.json"
        );
        assert_eq!(request.pack_input_path, "dist/demo.pack.input.json");
        assert_eq!(
            request.provider_preset_entry_id.as_deref(),
            Some("builtin.teams")
        );
        assert_eq!(
            request.provider_refs,
            vec!["oci://ghcr.io/greenticai/packs/messaging/messaging-teams:stable".to_owned()]
        );
    }
}
