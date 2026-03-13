use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::WizardAnswerDocument;
use crate::i18n::tr;

use super::catalog::{
    RemoteCatalogFetcher, builtin_channel_options, find_provider_preset_by_id, load_catalogs,
};

pub(crate) enum Navigation {
    MainMenu,
    Back,
    Exit,
    Value(String),
}

pub(crate) fn collect_interactive_answers(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<bool, String> {
    let locale = document.locale.clone();
    loop {
        match prompt_menu(
            &tr(&locale, "wizard.menu.title"),
            &[
                &menu_option(&locale, 1, "wizard.menu.create"),
                &menu_option(&locale, 2, "wizard.menu.update"),
                &menu_option(&locale, 3, "wizard.menu.advanced"),
                "",
                &main_menu_option(&locale),
                &back_or_exit_option(&locale, true),
            ],
        )? {
            Navigation::Exit => return Ok(false),
            Navigation::MainMenu => continue,
            Navigation::Back => return Ok(false),
            Navigation::Value(value) if value == "1" => {
                run_create_flow(cwd, document, fetcher, &locale)?;
                return Ok(true);
            }
            Navigation::Value(value) if value == "2" => {
                run_update_flow(cwd, document, fetcher, &locale)?;
                return Ok(true);
            }
            Navigation::Value(value) if value == "3" => {
                run_advanced_options(document, &locale)?;
            }
            _ => {}
        }
    }
}

pub(crate) fn parse_navigation(input: &str, allow_main_menu: bool) -> Navigation {
    let trimmed = input.trim();
    if trimmed == "0" {
        return Navigation::Exit;
    }
    if allow_main_menu && trimmed.eq_ignore_ascii_case("m") {
        return Navigation::MainMenu;
    }
    Navigation::Value(trimmed.to_owned())
}

fn run_create_flow(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    fetcher: &dyn RemoteCatalogFetcher,
    locale: &str,
) -> Result<(), String> {
    let catalogs = load_catalogs(cwd, &catalog_refs(document), fetcher)?;
    document.answers.insert(
        "mode".to_owned(),
        serde_json::Value::String("create".to_owned()),
    );

    match prompt_menu(
        &tr(locale, "wizard.create.template.title"),
        &[
            &menu_option(locale, 1, "wizard.create.template.catalog"),
            &menu_option(locale, 2, "wizard.create.template.basic"),
            &menu_option(locale, 3, "wizard.create.template.manual"),
            "",
            &main_menu_option(locale),
            &back_or_exit_option(locale, false),
        ],
    )? {
        Navigation::MainMenu | Navigation::Exit => return Err(cancelled(locale)),
        Navigation::Back => return Err(cancelled(locale)),
        Navigation::Value(value) if value == "1" => {
            choose_catalog_template(document, &catalogs, locale)?
        }
        Navigation::Value(value) if value == "2" => {
            document.answers.insert(
                "template_mode".to_owned(),
                serde_json::Value::String("basic_empty".to_owned()),
            );
        }
        Navigation::Value(value) if value == "3" => choose_manual_template(document, locale)?,
        _ => return Err(tr(locale, "wizard.error.invalid_template_selection")),
    }

    let solution_name = prompt_text(locale, "wizard.field.solution_name", None)?;
    let default_solution_id = slugify(&solution_name);
    let solution_id = prompt_text(
        locale,
        "wizard.field.solution_id",
        Some(&default_solution_id),
    )?;
    let description = prompt_text(locale, "wizard.field.short_description", None)?;
    let output_dir = prompt_text(locale, "wizard.field.output_dir", Some("./dist"))?;
    document.answers.insert(
        "solution_name".to_owned(),
        serde_json::Value::String(solution_name),
    );
    document.answers.insert(
        "solution_id".to_owned(),
        serde_json::Value::String(solution_id),
    );
    document.answers.insert(
        "description".to_owned(),
        serde_json::Value::String(description),
    );
    document.answers.insert(
        "output_dir".to_owned(),
        serde_json::Value::String(normalize_output_dir(&output_dir)),
    );
    choose_provider(document, &catalogs, None, locale)?;
    Ok(())
}

fn run_update_flow(
    cwd: &Path,
    document: &mut WizardAnswerDocument,
    fetcher: &dyn RemoteCatalogFetcher,
    locale: &str,
) -> Result<(), String> {
    let catalogs = load_catalogs(cwd, &catalog_refs(document), fetcher)?;
    let manifests = find_solution_manifests(cwd)?;
    let Some(path) = select_solution_manifest(&manifests)? else {
        return Err(cancelled(locale));
    };
    document.answers.insert(
        "mode".to_owned(),
        serde_json::Value::String("update".to_owned()),
    );
    document.answers.insert(
        "existing_solution_path".to_owned(),
        serde_json::Value::String(path.display().to_string()),
    );

    let raw = fs::read_to_string(&path).map_err(|err| {
        format!(
            "{} {}: {err}",
            tr(locale, "wizard.error.read_failed"),
            path.display()
        )
    })?;
    let manifest: serde_json::Value = serde_json::from_str(&raw).map_err(|err| {
        format!(
            "{} {}: {err}",
            tr(locale, "wizard.error.parse_failed"),
            path.display()
        )
    })?;
    let current_name = manifest
        .get("solution_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("GX Solution");
    let current_id = manifest
        .get("solution_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("gx-solution");
    let current_description = manifest
        .get("description")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let current_output_dir = manifest
        .get("output_dir")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("dist");

    let solution_name = prompt_text(locale, "wizard.field.solution_name", Some(current_name))?;
    let solution_id = prompt_text(locale, "wizard.field.solution_id", Some(current_id))?;
    let description = prompt_text(
        locale,
        "wizard.field.short_description",
        Some(current_description),
    )?;
    let output_dir = prompt_text(locale, "wizard.field.output_dir", Some(current_output_dir))?;
    document.answers.insert(
        "solution_name".to_owned(),
        serde_json::Value::String(solution_name),
    );
    document.answers.insert(
        "solution_id".to_owned(),
        serde_json::Value::String(solution_id),
    );
    document.answers.insert(
        "description".to_owned(),
        serde_json::Value::String(description),
    );
    document.answers.insert(
        "output_dir".to_owned(),
        serde_json::Value::String(normalize_output_dir(&output_dir)),
    );

    let current_provider = manifest
        .get("provider_presets")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("display_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Webchat");

    match prompt_menu(
        &format!(
            "{}: {current_provider}\n{}",
            tr(locale, "wizard.update.current_provider"),
            tr(locale, "wizard.update.change_provider")
        ),
        &[
            &menu_option(locale, 1, "wizard.update.keep_provider"),
            &menu_option(locale, 2, "wizard.update.change_provider_option"),
            &main_menu_option(locale),
            &back_or_exit_option(locale, false),
        ],
    )? {
        Navigation::Value(value) if value == "1" => {}
        Navigation::Value(value) if value == "2" => {
            choose_provider(document, &catalogs, Some(current_provider), locale)?
        }
        Navigation::MainMenu | Navigation::Exit | Navigation::Back => {
            return Err(cancelled(locale));
        }
        _ => {}
    }
    Ok(())
}

fn run_advanced_options(document: &mut WizardAnswerDocument, locale: &str) -> Result<(), String> {
    let current = catalog_refs(document);
    let prompt = if current.is_empty() {
        tr(locale, "wizard.advanced.catalog_source")
    } else {
        tr(locale, "wizard.advanced.catalog_sources")
    };
    let default = if current.is_empty() {
        None
    } else {
        Some(current.join(", "))
    };
    let value = prompt_text_raw(&prompt, default.as_deref(), locale)?;
    let refs = value
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_owned())
        .collect::<Vec<_>>();
    document.answers.insert(
        "catalog_oci_refs".to_owned(),
        serde_json::Value::Array(refs.into_iter().map(serde_json::Value::String).collect()),
    );
    Ok(())
}

fn choose_catalog_template(
    document: &mut WizardAnswerDocument,
    catalogs: &crate::WizardCatalogSet,
    locale: &str,
) -> Result<(), String> {
    if catalogs.templates.is_empty() {
        return Err(tr(locale, "wizard.error.no_catalog_templates"));
    }
    let mut options = catalogs
        .templates
        .iter()
        .enumerate()
        .map(|(idx, item)| format!("{}) {}", idx + 1, item.display_name))
        .collect::<Vec<_>>();
    options.push("M) Main menu".to_owned());
    options.push("0) Back".to_owned());
    let selection = prompt_menu(
        &tr(locale, "wizard.create.template.choose"),
        &options.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    let Navigation::Value(value) = selection else {
        return Err(cancelled(locale));
    };
    let index = value
        .parse::<usize>()
        .map_err(|_| tr(locale, "wizard.error.invalid_template_selection"))?;
    let Some(entry) = catalogs.templates.get(index.saturating_sub(1)) else {
        return Err(tr(locale, "wizard.error.invalid_template_selection"));
    };
    document.answers.insert(
        "template_mode".to_owned(),
        serde_json::Value::String("catalog".to_owned()),
    );
    document.answers.insert(
        "template_entry_id".to_owned(),
        serde_json::Value::String(entry.entry_id.clone()),
    );
    document.answers.insert(
        "template_display_name".to_owned(),
        serde_json::Value::String(entry.display_name.clone()),
    );
    Ok(())
}

fn choose_manual_template(document: &mut WizardAnswerDocument, locale: &str) -> Result<(), String> {
    let template_ref = prompt_text(locale, "wizard.field.template_reference", None)?;
    let domain_ref = prompt_text(
        locale,
        "wizard.field.domain_template_reference",
        Some(&template_ref),
    )?;
    document.answers.insert(
        "template_mode".to_owned(),
        serde_json::Value::String("manual".to_owned()),
    );
    document.answers.insert(
        "assistant_template_ref".to_owned(),
        serde_json::Value::String(template_ref),
    );
    document.answers.insert(
        "domain_template_ref".to_owned(),
        serde_json::Value::String(domain_ref),
    );
    Ok(())
}

fn choose_provider(
    document: &mut WizardAnswerDocument,
    catalogs: &crate::WizardCatalogSet,
    current_provider: Option<&str>,
    locale: &str,
) -> Result<(), String> {
    let prompt = tr(locale, "wizard.provider.title");
    let selection = prompt_menu(
        &prompt,
        &[
            &menu_option(locale, 1, "wizard.provider.webchat"),
            &menu_option(locale, 2, "wizard.provider.teams"),
            &menu_option(locale, 3, "wizard.provider.webex"),
            &menu_option(locale, 4, "wizard.provider.slack"),
            &menu_option(locale, 5, "wizard.provider.all"),
            &menu_option(locale, 6, "wizard.provider.catalog"),
            &menu_option(locale, 7, "wizard.provider.manual"),
            &main_menu_option(locale),
            &back_or_exit_option(locale, false),
        ],
    )?;
    match selection {
        Navigation::Value(value) if value == "1" => set_builtin_provider(document, "webchat"),
        Navigation::Value(value) if value == "2" => set_builtin_provider(document, "teams"),
        Navigation::Value(value) if value == "3" => set_builtin_provider(document, "webex"),
        Navigation::Value(value) if value == "4" => set_builtin_provider(document, "slack"),
        Navigation::Value(value) if value == "5" => {
            document.answers.insert(
                "provider_selection".to_owned(),
                serde_json::Value::String("all".to_owned()),
            );
            document.answers.insert(
                "provider_preset_display_name".to_owned(),
                serde_json::Value::String(tr(locale, "wizard.provider.all_label")),
            );
        }
        Navigation::Value(value) if value == "6" => {
            choose_catalog_provider(document, catalogs, locale)?;
        }
        Navigation::Value(value) if value == "7" => {
            let default = current_provider
                .unwrap_or("ghcr.io/greenticai/packs/messaging/messaging-webchat:latest");
            let provider_ref = prompt_text(locale, "wizard.field.provider_oci_ref", Some(default))?;
            document.answers.insert(
                "provider_selection".to_owned(),
                serde_json::Value::String("manual".to_owned()),
            );
            document.answers.insert(
                "provider_refs".to_owned(),
                serde_json::Value::Array(vec![serde_json::Value::String(provider_ref)]),
            );
            document.answers.insert(
                "provider_preset_display_name".to_owned(),
                serde_json::Value::String(tr(locale, "wizard.provider.manual_label")),
            );
        }
        _ => return Err(cancelled(locale)),
    }
    Ok(())
}

fn choose_catalog_provider(
    document: &mut WizardAnswerDocument,
    catalogs: &crate::WizardCatalogSet,
    locale: &str,
) -> Result<(), String> {
    if catalogs.provider_presets.is_empty() {
        return Err(tr(locale, "wizard.error.no_catalog_provider_presets"));
    }
    let mut options = catalogs
        .provider_presets
        .iter()
        .enumerate()
        .map(|(idx, item)| format!("{}) {}", idx + 1, item.display_name))
        .collect::<Vec<_>>();
    options.push("M) Main menu".to_owned());
    options.push("0) Back".to_owned());
    let choice = prompt_menu(
        &tr(locale, "wizard.provider.choose"),
        &options.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    let Navigation::Value(value) = choice else {
        return Err(cancelled(locale));
    };
    let index = value
        .parse::<usize>()
        .map_err(|_| tr(locale, "wizard.error.invalid_provider_preset_selection"))?;
    let entry = catalogs
        .provider_presets
        .get(index.saturating_sub(1))
        .ok_or_else(|| tr(locale, "wizard.error.invalid_provider_preset_selection"))?;
    document.answers.insert(
        "provider_selection".to_owned(),
        serde_json::Value::String("catalog".to_owned()),
    );
    document.answers.insert(
        "provider_preset_entry_id".to_owned(),
        serde_json::Value::String(entry.entry_id.clone()),
    );
    document.answers.insert(
        "provider_preset_display_name".to_owned(),
        serde_json::Value::String(entry.display_name.clone()),
    );
    if let Some(resolved) = find_provider_preset_by_id(catalogs, &entry.entry_id) {
        document.answers.insert(
            "provider_refs".to_owned(),
            serde_json::Value::Array(
                resolved
                    .provider_refs
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    Ok(())
}

fn set_builtin_provider(document: &mut WizardAnswerDocument, key: &str) {
    let display_name = builtin_channel_options()
        .into_iter()
        .find(|(value, _)| value == &key)
        .map(|(_, label)| label.to_owned())
        .unwrap_or_else(|| key.to_owned());
    document.answers.insert(
        "provider_selection".to_owned(),
        serde_json::Value::String(key.to_owned()),
    );
    document.answers.insert(
        "provider_preset_display_name".to_owned(),
        serde_json::Value::String(display_name),
    );
}

fn prompt_menu(title: &str, options: &[&str]) -> Result<Navigation, String> {
    let mut stdout = io::stdout();
    writeln!(stdout, "{title}").map_err(|err| format!("write prompt failed: {err}"))?;
    for option in options {
        writeln!(stdout, "{option}").map_err(|err| format!("write prompt failed: {err}"))?;
    }
    write!(stdout, "> ").map_err(|err| format!("write prompt failed: {err}"))?;
    stdout
        .flush()
        .map_err(|err| format!("flush prompt failed: {err}"))?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|err| format!("read prompt failed: {err}"))?;
    let nav = parse_navigation(&line, true);
    if matches!(nav, Navigation::Value(ref value) if value.is_empty()) {
        Ok(Navigation::Back)
    } else {
        Ok(nav)
    }
}

fn prompt_text(locale: &str, key: &str, default: Option<&str>) -> Result<String, String> {
    let title = tr(locale, key);
    prompt_text_raw(&title, default, locale)
}

fn prompt_text_raw(title: &str, default: Option<&str>, locale: &str) -> Result<String, String> {
    let mut stdout = io::stdout();
    loop {
        if let Some(default) = default {
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
        if trimmed.eq_ignore_ascii_case("m") {
            return Err(cancelled(locale));
        }
        if trimmed == "0" {
            return Err(cancelled(locale));
        }
        if trimmed.is_empty() {
            if let Some(default) = default {
                return Ok(default.to_owned());
            }
            continue;
        }
        return Ok(trimmed.to_owned());
    }
}

fn menu_option(locale: &str, index: usize, key: &str) -> String {
    format!("{index}) {}", tr(locale, key))
}

fn main_menu_option(locale: &str) -> String {
    format!("M) {}", tr(locale, "wizard.nav.main_menu"))
}

fn back_or_exit_option(locale: &str, exit: bool) -> String {
    let key = if exit {
        "wizard.nav.exit"
    } else {
        "wizard.nav.back"
    };
    format!("0) {}", tr(locale, key))
}

fn cancelled(locale: &str) -> String {
    tr(locale, "wizard.error.cancelled")
}

fn catalog_refs(document: &WizardAnswerDocument) -> Vec<String> {
    document
        .answers
        .get("catalog_oci_refs")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_output_dir(value: &str) -> String {
    value.strip_prefix("./").unwrap_or(value).to_owned()
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

fn select_solution_manifest(manifests: &[PathBuf]) -> Result<Option<PathBuf>, String> {
    if manifests.is_empty() {
        return Ok(None);
    }
    if manifests.len() == 1 {
        return Ok(manifests.first().cloned());
    }
    let mut options = manifests
        .iter()
        .enumerate()
        .map(|(idx, path)| format!("{}) {}", idx + 1, path.display()))
        .collect::<Vec<_>>();
    options.push("M) Main menu".to_owned());
    options.push("0) Back".to_owned());
    let choice = prompt_menu(
        "Choose existing solution",
        &options.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    let Navigation::Value(value) = choice else {
        return Ok(None);
    };
    let index = value
        .parse::<usize>()
        .map_err(|_| "invalid solution selection".to_owned())?;
    Ok(manifests.get(index.saturating_sub(1)).cloned())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_supports_main_menu_and_exit() {
        assert!(matches!(parse_navigation("M", true), Navigation::MainMenu));
        assert!(matches!(parse_navigation("m", true), Navigation::MainMenu));
        assert!(matches!(parse_navigation("0", true), Navigation::Exit));
    }
}
