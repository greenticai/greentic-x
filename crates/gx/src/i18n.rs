pub fn normalize_locale(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered.starts_with("nl") {
        "nl".to_owned()
    } else {
        "en".to_owned()
    }
}

pub fn locale_from_env() -> Option<String> {
    for key in [
        "GX_LOCALE",
        "GREENTIC_LOCALE",
        "LC_ALL",
        "LC_MESSAGES",
        "LANG",
    ] {
        let Ok(raw) = std::env::var(key) else {
            continue;
        };
        if let Some(locale) = normalize_env_locale_candidate(&raw) {
            return Some(locale);
        }
    }
    None
}

pub fn resolve_locale(cli_locale: Option<&str>, doc_locale: Option<&str>) -> String {
    if let Some(locale) = cli_locale.filter(|value| !value.trim().is_empty()) {
        return normalize_locale(locale);
    }
    if let Some(locale) = doc_locale.filter(|value| !value.trim().is_empty()) {
        return normalize_locale(locale);
    }
    locale_from_env().unwrap_or_else(|| "en".to_owned())
}

pub fn tr(locale: &str, key: &str) -> String {
    match normalize_locale(locale).as_str() {
        "nl" => tr_nl(key),
        _ => tr_en(key),
    }
}

fn normalize_env_locale_candidate(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let base = trimmed.split(['.', '@']).next().unwrap_or_default().trim();
    if base.is_empty() || base.eq_ignore_ascii_case("C") || base.eq_ignore_ascii_case("POSIX") {
        return None;
    }
    Some(normalize_locale(base))
}

fn tr_en(key: &str) -> String {
    match key {
        "wizard.step.collect_input" => "Collect wizard inputs and load answer state".to_owned(),
        "wizard.step.normalize_request" => {
            "Normalize request into deterministic wizard input".to_owned()
        }
        "wizard.step.validate_plan" => "Validate wizard plan before execution".to_owned(),
        "wizard.step.execute_plan" => "Execute wizard side effects".to_owned(),
        "wizard.warn.latest_refs" => "remote refs use :latest".to_owned(),
        "wizard.err.invalid_schema_version" => {
            "invalid schema version; expected semantic version like 1.0.0".to_owned()
        }
        "wizard.err.answers_missing_metadata" => {
            "answers file is missing wizard metadata; rerun with --migrate to import legacy answers"
                .to_owned()
        }
        "wizard.err.unsupported_workflow" => "unsupported mode/workflow".to_owned(),
        "wizard.err.latest_policy_required" => {
            "wizard answers include :latest refs; set latest_policy to keep_latest or pin"
                .to_owned()
        }
        "wizard.err.latest_policy_invalid" => {
            "latest_policy must be keep_latest or pin when :latest refs are used".to_owned()
        }
        "wizard.err.latest_policy_pin_failed" => {
            "could not pin :latest reference to a resolved digest".to_owned()
        }
        "wizard.err.bundle_output_ext" => "bundle_output_path must end with .gtbundle".to_owned(),
        "wizard.err.template_output_ext" => "template_output_path must end with .json".to_owned(),
        "wizard.err.unsupported_source_ref_scheme" => "unsupported source ref scheme".to_owned(),
        "wizard.prompt.latest_policy_choice" => {
            "Select policy [`keep_latest`/`pin`] (default: `pin`):".to_owned()
        }
        "wizard.qa.title" => "GX Wizard".to_owned(),
        "wizard.qa.select_workflow" => "Select workflow".to_owned(),
        "wizard.qa.template_source" => "Template source reference".to_owned(),
        "wizard.qa.template_output_path" => "Template output path".to_owned(),
        "wizard.qa.latest_policy" => "Latest policy".to_owned(),
        "wizard.qa.bundle_mode" => "Bundle mode".to_owned(),
        "wizard.qa.bundle_name" => "Bundle name".to_owned(),
        "wizard.qa.bundle_id" => "Bundle id".to_owned(),
        "wizard.qa.output_dir" => "Bundle output directory".to_owned(),
        "wizard.qa.assistant_template_source" => "Assistant template source".to_owned(),
        "wizard.qa.domain_template_source" => "Domain template source".to_owned(),
        "wizard.qa.deployment_profile" => "Deployment profile".to_owned(),
        "wizard.qa.deployment_target" => "Deployment target".to_owned(),
        "wizard.qa.provider_categories" => "Provider categories (csv)".to_owned(),
        "wizard.qa.bundle_output_path" => "Bundle output path".to_owned(),
        _ => key.to_owned(),
    }
}

fn tr_nl(key: &str) -> String {
    match key {
        "wizard.step.collect_input" => "Wizard-invoer verzamelen en antwoordstatus laden".to_owned(),
        "wizard.step.normalize_request" => {
            "Verzoek normaliseren naar deterministische wizard-invoer".to_owned()
        }
        "wizard.step.validate_plan" => "Wizard-plan valideren voor uitvoering".to_owned(),
        "wizard.step.execute_plan" => "Wizard-neveneffecten uitvoeren".to_owned(),
        "wizard.warn.latest_refs" => "remote refs gebruiken :latest".to_owned(),
        "wizard.err.invalid_schema_version" => {
            "ongeldige schema-versie; verwacht semver zoals 1.0.0".to_owned()
        }
        "wizard.err.answers_missing_metadata" => {
            "antwoordbestand mist wizard-metadata; herhaal met --migrate om legacy-antwoorden te importeren"
                .to_owned()
        }
        "wizard.err.unsupported_workflow" => "niet-ondersteunde modus/workflow".to_owned(),
        "wizard.err.latest_policy_required" => {
            "wizard-antwoorden bevatten :latest refs; zet latest_policy op keep_latest of pin"
                .to_owned()
        }
        "wizard.err.latest_policy_invalid" => {
            "latest_policy moet keep_latest of pin zijn als :latest refs gebruikt worden".to_owned()
        }
        "wizard.err.latest_policy_pin_failed" => {
            "kon :latest-referentie niet vastzetten op een opgeloste digest".to_owned()
        }
        "wizard.err.bundle_output_ext" => {
            "bundle_output_path moet eindigen op .gtbundle".to_owned()
        }
        "wizard.err.template_output_ext" => {
            "template_output_path moet eindigen op .json".to_owned()
        }
        "wizard.err.unsupported_source_ref_scheme" => {
            "niet-ondersteund bronref-schema".to_owned()
        }
        "wizard.prompt.latest_policy_choice" => {
            "Kies beleid [`keep_latest`/`pin`] (standaard: `pin`):".to_owned()
        }
        "wizard.qa.title" => "GX Wizard".to_owned(),
        "wizard.qa.select_workflow" => "Kies workflow".to_owned(),
        "wizard.qa.template_source" => "Template-bronreferentie".to_owned(),
        "wizard.qa.template_output_path" => "Template-uitvoerpad".to_owned(),
        "wizard.qa.latest_policy" => "Latest-beleid".to_owned(),
        "wizard.qa.bundle_mode" => "Bundle-modus".to_owned(),
        "wizard.qa.bundle_name" => "Bundle-naam".to_owned(),
        "wizard.qa.bundle_id" => "Bundle-id".to_owned(),
        "wizard.qa.output_dir" => "Bundle-uitvoermap".to_owned(),
        "wizard.qa.assistant_template_source" => "Assistant-templatebron".to_owned(),
        "wizard.qa.domain_template_source" => "Domain-templatebron".to_owned(),
        "wizard.qa.deployment_profile" => "Deployment-profiel".to_owned(),
        "wizard.qa.deployment_target" => "Deployment-doel".to_owned(),
        "wizard.qa.provider_categories" => "Provider-categorieen (csv)".to_owned(),
        "wizard.qa.bundle_output_path" => "Bundle-uitvoerpad".to_owned(),
        _ => tr_en(key),
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_env_locale_candidate, normalize_locale, resolve_locale};

    #[test]
    fn normalize_locale_maps_supported_prefixes() {
        assert_eq!(normalize_locale("nl-NL"), "nl");
        assert_eq!(normalize_locale("en_US"), "en");
    }

    #[test]
    fn normalize_env_candidate_strips_encoding_and_variant() {
        assert_eq!(
            normalize_env_locale_candidate("nl_NL.UTF-8@euro"),
            Some("nl".to_owned())
        );
        assert_eq!(
            normalize_env_locale_candidate("en_US.UTF-8"),
            Some("en".to_owned())
        );
    }

    #[test]
    fn normalize_env_candidate_ignores_posix() {
        assert_eq!(normalize_env_locale_candidate("C"), None);
        assert_eq!(normalize_env_locale_candidate("POSIX"), None);
    }

    #[test]
    fn resolve_locale_prefers_cli_then_doc() {
        assert_eq!(resolve_locale(Some("nl-NL"), Some("en")), "nl");
        assert_eq!(resolve_locale(None, Some("nl")), "nl");
    }
}
