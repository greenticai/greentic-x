use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use crate::i18n::tr;
use crate::{
    GX_WIZARD_ID, GX_WIZARD_SCHEMA_ID, WizardAnswerDocument, WizardBundleAnswers, WizardCommonArgs,
    WizardNormalizedAnswers, WizardTemplateAnswers,
};
use serde_json::Value;

use super::remote::{
    DistributorRemoteRefResolver, RemoteRefResolver, is_resolvable_remote_source_ref,
    pin_reference_to_digest,
};

pub(crate) fn load_wizard_answers(
    cwd: &Path,
    args: &WizardCommonArgs,
    target_schema_version: &str,
    locale: &str,
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
    let object = value
        .as_object()
        .ok_or_else(|| format!("answers file {} must be a JSON object", resolved.display()))?
        .clone();
    let has_metadata = object.contains_key("wizard_id")
        || object.contains_key("schema_id")
        || object.contains_key("schema_version")
        || object.contains_key("locale");
    if has_metadata {
        let parsed: WizardAnswerDocument = serde_json::from_value(Value::Object(object))
            .map_err(|err| format!("failed to decode answer document: {err}"))?;
        if parsed.wizard_id.trim().is_empty() {
            return Err("answers document wizard_id must not be empty".to_owned());
        }
        if parsed.schema_id.trim().is_empty() {
            return Err("answers document schema_id must not be empty".to_owned());
        }
        if parsed.locale.trim().is_empty() {
            return Err("answers document locale must not be empty".to_owned());
        }
        if parsed.schema_id != GX_WIZARD_SCHEMA_ID {
            return Err(format!(
                "answers document schema_id mismatch: expected {GX_WIZARD_SCHEMA_ID}, got {}",
                parsed.schema_id
            ));
        }
        let source_schema_version = normalize_schema_version(&parsed.schema_version, locale)?;
        if source_schema_version != target_schema_version && !args.migrate {
            return Err(format!(
                "answers document schema_version {source_schema_version} differs from target {target_schema_version}; rerun with --migrate"
            ));
        }
        document = parsed;
        document.schema_version = target_schema_version.to_owned();
    } else if args.migrate {
        document.answers = object
            .iter()
            .filter(|(key, _)| key.as_str() != "locks")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if let Some(Value::Object(locks)) = object.get("locks") {
            document.locks = locks.clone();
        }
    } else {
        return Err(tr(locale, "wizard.err.answers_missing_metadata"));
    }
    Ok(document)
}

pub(crate) fn normalize_schema_version(raw: &str, locale: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "{} ({trimmed})",
            tr(locale, "wizard.err.invalid_schema_version")
        ));
    }
    for part in &parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(format!(
                "{} ({trimmed})",
                tr(locale, "wizard.err.invalid_schema_version")
            ));
        }
    }
    Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
}

pub(crate) fn normalize_wizard_answers(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    cli_mode: Option<&str>,
    locale: &str,
    resolve_remote: bool,
) -> Result<WizardNormalizedAnswers, String> {
    let workflow = cli_mode
        .map(|mode| mode.trim().to_owned())
        .filter(|mode| !mode.is_empty())
        .unwrap_or_else(|| upsert_string_answer(document, "workflow", "assistant_bundle"));
    document
        .answers
        .insert("workflow".to_owned(), Value::String(workflow.clone()));
    let normalized = match workflow.as_str() {
        "assistant_bundle" => {
            normalize_bundle_answers(document, locale).map(WizardNormalizedAnswers::AssistantBundle)
        }
        "assistant_template_create" => {
            normalize_template_answers(document, "assistant", "create", locale)
                .map(WizardNormalizedAnswers::Template)
        }
        "assistant_template_update" => {
            normalize_template_answers(document, "assistant", "update", locale)
                .map(WizardNormalizedAnswers::Template)
        }
        "domain_template_create" => {
            normalize_template_answers(document, "domain", "create", locale)
                .map(WizardNormalizedAnswers::Template)
        }
        "domain_template_update" => {
            normalize_template_answers(document, "domain", "update", locale)
                .map(WizardNormalizedAnswers::Template)
        }
        other => Err(format!(
            "{} {other}; expected one of assistant_bundle, assistant_template_create, assistant_template_update, domain_template_create, domain_template_update",
            tr(locale, "wizard.err.unsupported_workflow")
        )),
    }?;
    if resolve_remote {
        apply_remote_resolution(cwd, document, &normalized, locale)?;
    }
    Ok(normalized)
}

fn normalize_bundle_answers(
    document: &mut WizardAnswerDocument,
    locale: &str,
) -> Result<WizardBundleAnswers, String> {
    let workflow = upsert_string_answer(document, "workflow", "assistant_bundle");
    if workflow != "assistant_bundle" {
        return Err(format!(
            "{} {workflow}; expected assistant_bundle",
            tr(locale, "wizard.err.unsupported_workflow")
        ));
    }
    document.answers.remove("deployment_profile");
    document.answers.remove("deployment_target");
    document.answers.remove("bundle_output_path");
    document.locks.remove("deployment_profile");
    document.locks.remove("deployment_target");
    document.locks.remove("bundle_output_path");
    let assistant_template_source = upsert_string_answer(
        document,
        "assistant_template_source",
        "templates/assistant/default.json",
    );
    validate_supported_source_ref(
        "assistant_template_source",
        &assistant_template_source,
        locale,
    )?;
    let domain_template_source = upsert_string_answer(
        document,
        "domain_template_source",
        "templates/domain/default.json",
    );
    validate_supported_source_ref("domain_template_source", &domain_template_source, locale)?;
    let provider_categories =
        upsert_string_array_answer(document, "provider_categories", &["llm"])?;
    let bundle_name = upsert_string_answer(document, "bundle_name", "GX Bundle");
    let bundle_id = upsert_string_answer(document, "bundle_id", "gx-bundle");
    let output_dir = upsert_string_answer(document, "output_dir", "dist/bundle");
    let bundle_output_path = format!("{output_dir}/dist/{bundle_id}.gtbundle");
    let latest_policy = resolve_latest_policy(
        document,
        locale,
        [
            assistant_template_source.as_str(),
            domain_template_source.as_str(),
        ],
    )?;
    let latest_refs = find_latest_refs([
        assistant_template_source.as_str(),
        domain_template_source.as_str(),
    ]);
    document.answers.insert(
        "bundle_output_path".to_owned(),
        Value::String(bundle_output_path.clone()),
    );
    let mode = upsert_string_answer(document, "mode", "create");
    if mode != "create" && mode != "update" && mode != "doctor" {
        return Err(format!(
            "answers.mode must be create, update, or doctor; got {mode}"
        ));
    }
    Ok(WizardBundleAnswers {
        workflow,
        bundle_mode: mode,
        bundle_name,
        bundle_id,
        output_dir,
        assistant_template_source,
        domain_template_source,
        provider_categories,
        bundle_output_path,
        latest_policy,
        latest_refs,
    })
}

fn normalize_template_answers(
    document: &mut WizardAnswerDocument,
    template_kind: &str,
    template_action: &str,
    locale: &str,
) -> Result<WizardTemplateAnswers, String> {
    let workflow = upsert_string_answer(
        document,
        "workflow",
        &format!("{template_kind}_template_{template_action}"),
    );
    let template_source = upsert_string_answer(
        document,
        "template_source",
        &format!("templates/{template_kind}/default.json"),
    );
    validate_supported_source_ref("template_source", &template_source, locale)?;
    let template_output_path = upsert_string_answer(
        document,
        "template_output_path",
        &format!(
            "templates/{template_kind}/{}.json",
            if template_action == "create" {
                "new"
            } else {
                "updated"
            }
        ),
    );
    if !template_output_path.ends_with(".json") {
        return Err(format!(
            "{}, got {template_output_path}",
            tr(locale, "wizard.err.template_output_ext")
        ));
    }
    let latest_policy = resolve_latest_policy(document, locale, [template_source.as_str()])?;
    let latest_refs = find_latest_refs([template_source.as_str()]);
    Ok(WizardTemplateAnswers {
        workflow,
        template_kind: template_kind.to_owned(),
        template_action: template_action.to_owned(),
        template_source,
        template_output_path,
        latest_policy,
        latest_refs,
    })
}

fn optional_string_answer(
    document: &WizardAnswerDocument,
    key: &str,
) -> Result<Option<String>, String> {
    match document.answers.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(format!("answers.{key} must not be empty"))
            } else {
                Ok(Some(trimmed.to_owned()))
            }
        }
        Some(_) => Err(format!("answers.{key} must be a string")),
    }
}

fn resolve_latest_policy<'a>(
    document: &mut WizardAnswerDocument,
    locale: &str,
    refs: impl IntoIterator<Item = &'a str>,
) -> Result<Option<String>, String> {
    let refs = refs.into_iter().collect::<Vec<_>>();
    let latest_refs = find_latest_refs(refs.iter().copied());
    let latest_policy = optional_string_answer(document, "latest_policy")?;
    if latest_refs.is_empty() {
        return Ok(latest_policy);
    }
    if let Some(policy) = latest_policy.as_deref() {
        validate_latest_policy(policy, locale)?;
        return Ok(Some(policy.to_owned()));
    }

    if !is_automated_context() && io::stdin().is_terminal() && io::stdout().is_terminal() {
        let selected = prompt_latest_policy(locale, &latest_refs)?;
        document
            .answers
            .insert("latest_policy".to_owned(), Value::String(selected.clone()));
        Ok(Some(selected))
    } else {
        Err(tr(locale, "wizard.err.latest_policy_required"))
    }
}

fn is_automated_context() -> bool {
    cfg!(test)
        || std::env::var_os("RUST_TEST_THREADS").is_some()
        || std::env::var_os("CI").is_some()
        || std::env::var_os("GX_WIZARD_NON_INTERACTIVE").is_some()
}

fn apply_remote_resolution(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    normalized: &WizardNormalizedAnswers,
    locale: &str,
) -> Result<(), String> {
    let resolver = DistributorRemoteRefResolver;
    match normalized {
        WizardNormalizedAnswers::AssistantBundle(bundle) => resolve_remote_sources(
            cwd,
            document,
            locale,
            &resolver,
            &bundle.latest_policy,
            [
                (
                    "assistant_template_source",
                    bundle.assistant_template_source.as_str(),
                ),
                (
                    "domain_template_source",
                    bundle.domain_template_source.as_str(),
                ),
            ],
        ),
        WizardNormalizedAnswers::Template(template) => resolve_remote_sources(
            cwd,
            document,
            locale,
            &resolver,
            &template.latest_policy,
            [("template_source", template.template_source.as_str())],
        ),
    }
}

fn resolve_remote_sources<'a>(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    locale: &str,
    resolver: &dyn RemoteRefResolver,
    latest_policy: &Option<String>,
    refs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), String> {
    let pin_latest = latest_policy.as_deref() == Some("pin");
    let mut lock_entries = Vec::new();
    for (field, source_ref) in refs {
        if !is_resolvable_remote_source_ref(source_ref) {
            continue;
        }
        let resolved = resolver.resolve(cwd, source_ref)?;
        let pinned_ref = pin_reference_to_digest(source_ref, &resolved.resolved_digest);
        if pin_latest && source_ref.contains(":latest") {
            let pinned = pinned_ref.clone().ok_or_else(|| {
                format!(
                    "{} {source_ref}",
                    tr(locale, "wizard.err.latest_policy_pin_failed")
                )
            })?;
            document
                .answers
                .insert(field.to_owned(), Value::String(pinned.clone()));
        }
        lock_entries.push(Value::Object(
            [
                ("field".to_owned(), Value::String(field.to_owned())),
                (
                    "requested_ref".to_owned(),
                    Value::String(source_ref.to_owned()),
                ),
                (
                    "resolved_digest".to_owned(),
                    Value::String(resolved.resolved_digest),
                ),
                (
                    "pinned_ref".to_owned(),
                    pinned_ref.map_or(Value::Null, Value::String),
                ),
            ]
            .into_iter()
            .collect(),
        ));
    }
    if !lock_entries.is_empty() {
        document.locks.insert(
            "resolved_source_refs".to_owned(),
            Value::Array(lock_entries),
        );
    }
    Ok(())
}

fn validate_latest_policy(policy: &str, locale: &str) -> Result<(), String> {
    match policy {
        "keep_latest" | "pin" => Ok(()),
        other => Err(format!(
            "{}, got {other}",
            tr(locale, "wizard.err.latest_policy_invalid")
        )),
    }
}

fn prompt_latest_policy(locale: &str, latest_refs: &[String]) -> Result<String, String> {
    let mut stdout = io::stdout();
    writeln!(
        stdout,
        "{}: {}",
        tr(locale, "wizard.warn.latest_refs"),
        latest_refs.join(", ")
    )
    .map_err(|err| format!("failed to write prompt: {err}"))?;
    writeln!(
        stdout,
        "{}",
        tr(locale, "wizard.prompt.latest_policy_choice")
    )
    .map_err(|err| format!("failed to write prompt: {err}"))?;
    stdout
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read policy input: {err}"))?;
    let normalized = input.trim().to_ascii_lowercase();
    let selected = match normalized.as_str() {
        "" | "pin" | "p" => "pin",
        "keep_latest" | "k" | "keep" | "latest" => "keep_latest",
        other => {
            return Err(format!(
                "{}, got {other}",
                tr(locale, "wizard.err.latest_policy_invalid")
            ));
        }
    };
    Ok(selected.to_owned())
}

fn upsert_string_answer(document: &mut WizardAnswerDocument, key: &str, default: &str) -> String {
    let value = match document.answers.get(key) {
        Some(Value::String(existing)) if !existing.trim().is_empty() => existing.trim().to_owned(),
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
) -> Result<Vec<String>, String> {
    let values = match document.answers.get(key) {
        Some(Value::Array(items)) => {
            let mut parsed = Vec::new();
            for item in items {
                let Some(value) = item.as_str() else {
                    return Err(format!("answers.{key} must contain only strings"));
                };
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(format!("answers.{key} entries must not be empty"));
                }
                parsed.push(trimmed.to_owned());
            }
            if parsed.is_empty() {
                default.iter().map(|value| (*value).to_owned()).collect()
            } else {
                parsed
            }
        }
        Some(Value::Null) | None => default.iter().map(|value| (*value).to_owned()).collect(),
        Some(_) => return Err(format!("answers.{key} must be an array of strings")),
    };
    document.answers.insert(
        key.to_owned(),
        Value::Array(values.iter().cloned().map(Value::String).collect()),
    );
    Ok(values)
}

fn find_latest_refs<'a>(refs: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    refs.into_iter()
        .filter(|value| is_remote_source_ref(value) && value.contains(":latest"))
        .map(|value| value.to_owned())
        .collect()
}

fn is_remote_source_ref(value: &str) -> bool {
    value.starts_with("oci://") || value.starts_with("repo://") || value.starts_with("store://")
}

fn validate_supported_source_ref(field: &str, value: &str, locale: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("answers.{field} must not be empty"));
    }
    if trimmed.contains("://") {
        let allowed = trimmed.starts_with("oci://")
            || trimmed.starts_with("repo://")
            || trimmed.starts_with("store://");
        if !allowed {
            return Err(format!(
                "answers.{field} {} in {trimmed}; expected an absolute/relative path, oci://, repo://, or store://",
                tr(locale, "wizard.err.unsupported_source_ref_scheme")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::remote::ResolvedRemoteRef;
    use super::*;

    struct StubResolver;

    impl RemoteRefResolver for StubResolver {
        fn resolve(
            &self,
            _cache_root: &Path,
            _reference: &str,
        ) -> Result<ResolvedRemoteRef, String> {
            Ok(ResolvedRemoteRef {
                resolved_digest: "sha256:abc123".to_owned(),
            })
        }
    }

    fn empty_doc() -> WizardAnswerDocument {
        WizardAnswerDocument {
            wizard_id: GX_WIZARD_ID.to_owned(),
            schema_id: GX_WIZARD_SCHEMA_ID.to_owned(),
            schema_version: "1.0.0".to_owned(),
            locale: "en".to_owned(),
            answers: serde_json::Map::new(),
            locks: serde_json::Map::new(),
        }
    }

    #[test]
    fn remote_resolution_pins_latest_when_policy_is_pin() -> Result<(), String> {
        let mut document = empty_doc();
        document.answers.insert(
            "assistant_template_source".to_owned(),
            Value::String("oci://ghcr.io/example/assistant:latest".to_owned()),
        );
        let resolver = StubResolver;
        resolve_remote_sources(
            Path::new("."),
            &mut document,
            "en",
            &resolver,
            &Some("pin".to_owned()),
            [(
                "assistant_template_source",
                "oci://ghcr.io/example/assistant:latest",
            )],
        )?;
        assert_eq!(
            document.answers.get("assistant_template_source"),
            Some(&Value::String(
                "oci://ghcr.io/example/assistant@sha256:abc123".to_owned()
            ))
        );
        let lock_count = document
            .locks
            .get("resolved_source_refs")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        assert_eq!(lock_count, 1);
        let pinned_ref = document
            .locks
            .get("resolved_source_refs")
            .and_then(Value::as_array)
            .and_then(|entries| entries.first())
            .and_then(|entry| entry.get("pinned_ref"))
            .and_then(Value::as_str);
        assert_eq!(
            pinned_ref,
            Some("oci://ghcr.io/example/assistant@sha256:abc123")
        );
        Ok(())
    }

    #[test]
    fn remote_resolution_skips_pin_for_keep_latest() -> Result<(), String> {
        let mut document = empty_doc();
        document.answers.insert(
            "assistant_template_source".to_owned(),
            Value::String("oci://ghcr.io/example/assistant:latest".to_owned()),
        );
        let resolver = StubResolver;
        resolve_remote_sources(
            Path::new("."),
            &mut document,
            "en",
            &resolver,
            &Some("keep_latest".to_owned()),
            [(
                "assistant_template_source",
                "oci://ghcr.io/example/assistant:latest",
            )],
        )?;
        assert_eq!(
            document.answers.get("assistant_template_source"),
            Some(&Value::String(
                "oci://ghcr.io/example/assistant:latest".to_owned()
            ))
        );
        let lock_count = document
            .locks
            .get("resolved_source_refs")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        assert_eq!(lock_count, 1);
        let pinned_ref = document
            .locks
            .get("resolved_source_refs")
            .and_then(Value::as_array)
            .and_then(|entries| entries.first())
            .and_then(|entry| entry.get("pinned_ref"));
        assert_eq!(
            pinned_ref,
            Some(&Value::String(
                "oci://ghcr.io/example/assistant@sha256:abc123".to_owned()
            ))
        );
        Ok(())
    }
}
