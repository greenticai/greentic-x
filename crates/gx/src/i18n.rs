use std::collections::BTreeMap;
use std::sync::OnceLock;

#[allow(dead_code)]
type Catalog = BTreeMap<String, String>;
#[allow(dead_code)]
type CatalogsByLocale = BTreeMap<String, Catalog>;

#[allow(dead_code)]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_i18n.rs"));
}

#[allow(dead_code)]
fn catalogs() -> &'static CatalogsByLocale {
    static CATALOGS: OnceLock<CatalogsByLocale> = OnceLock::new();
    CATALOGS.get_or_init(embedded::load_embedded_catalogs)
}

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

#[allow(dead_code)]
pub fn tr(locale: &str, key: &str) -> String {
    let normalized = normalize_locale(locale);
    catalogs()
        .get(&normalized)
        .and_then(|catalog| catalog.get(key))
        .or_else(|| catalogs().get("en").and_then(|catalog| catalog.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
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

#[cfg(test)]
mod tests {
    use super::{normalize_env_locale_candidate, normalize_locale, resolve_locale, tr};

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

    #[test]
    fn translations_load_from_embedded_catalogs() {
        assert_eq!(tr("en", "wizard.qa.title"), "GX Wizard");
        assert_eq!(tr("nl", "wizard.qa.title"), "GX Wizard");
    }

    #[test]
    fn translations_fallback_to_english_and_key() {
        assert_eq!(
            tr("en", "wizard.err.template_output_ext"),
            "template_output_path must end with .json"
        );
        assert_eq!(tr("nl", "missing.key"), "missing.key");
    }
}
