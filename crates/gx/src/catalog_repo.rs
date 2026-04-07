use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::validator_for;
use serde_json::{Value, json};

use crate::{RootCatalogEntry, RootCatalogIndex};

const CATALOG_INDEX_SCHEMA: &str = include_str!("../schemas/catalog-index.schema.json");
const ASSISTANT_TEMPLATE_SCHEMA: &str = include_str!("../schemas/assistant-template.schema.json");
const PROVIDER_PRESET_SCHEMA: &str = include_str!("../schemas/provider-preset.schema.json");
const OVERLAY_SCHEMA: &str = include_str!("../schemas/overlay.schema.json");
const SETUP_PROFILE_SCHEMA: &str = include_str!("../schemas/setup-profile.schema.json");

pub(crate) fn init_catalog_repo(
    path: &Path,
    repo_name: &str,
    title: Option<String>,
    description: Option<String>,
    include_examples: bool,
    include_publish_workflow: bool,
) -> Result<String, String> {
    if path.exists() {
        let mut entries = fs::read_dir(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "{} already exists and is not empty",
                path.display()
            ));
        }
    }
    fs::create_dir_all(path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    for dir in [
        "assistant_templates",
        "provider_presets",
        "bundles",
        "views",
        "overlays",
        "setup_profiles",
        "contracts",
        "resolvers",
        "adapters",
        "analysis",
        "playbooks",
        "packs",
    ] {
        fs::create_dir_all(path.join(dir))
            .map_err(|err| format!("failed to create {}: {err}", path.join(dir).display()))?;
        fs::write(
            path.join(dir).join("README.md"),
            format!("# {}\n\nAdd catalog assets here.\n", dir.replace('_', " ")),
        )
        .map_err(|err| format!("failed to write {}: {err}", path.join(dir).display()))?;
    }
    if include_examples {
        write_json(
            &path
                .join("assistant_templates")
                .join("example-template.json"),
            &json!({
                "entry_id": format!("{repo_name}.assistant-template.example"),
                "kind": "assistant-template",
                "version": "1.0.0",
                "display_name": "Example Assistant Template",
                "description": "Example assistant template for a solution catalog repo.",
                "assistant_template_ref": "assistant_templates/example-template.json",
                "domain_template_ref": "assistant_templates/example-template.json"
            }),
        )?;
        write_json(
            &path.join("bundles").join("example-bundle.json"),
            &json!({
                "bundle_id": format!("{repo_name}.bundle.example"),
                "title": "Example bundle"
            }),
        )?;
        write_json(
            &path.join("views").join("example-view.json"),
            &json!({
                "view_id": format!("{repo_name}.view.example"),
                "title": "Example view"
            }),
        )?;
        write_json(
            &path.join("overlays").join("default.json"),
            &json!({
                "entry_id": format!("{repo_name}.overlay.default"),
                "kind": "overlay",
                "version": "1.0.0",
                "display_name": "Default Overlay",
                "description": "Default branding and locale overlay.",
                "default_locale": "en",
                "tenant_id": repo_name
            }),
        )?;
        write_json(
            &path.join("setup_profiles").join("default.json"),
            &json!({
                "entry_id": format!("{repo_name}.setup.default"),
                "kind": "setup-profile",
                "version": "1.0.0",
                "display_name": "Default Setup Profile",
                "settings": {}
            }),
        )?;
    }
    if include_publish_workflow {
        let ci_dir = path.join("ci");
        fs::create_dir_all(&ci_dir)
            .map_err(|err| format!("failed to create {}: {err}", ci_dir.display()))?;
        fs::write(
            ci_dir.join("create_bundle_archive.sh"),
            create_bundle_archive_script(repo_name),
        )
        .map_err(|err| format!("failed to write bundle archive script: {err}"))?;
        fs::write(
            ci_dir.join("render_remote_catalog.py"),
            render_remote_catalog_script(),
        )
        .map_err(|err| format!("failed to write remote catalog renderer: {err}"))?;
        let workflow_dir = path.join(".github").join("workflows");
        fs::create_dir_all(&workflow_dir)
            .map_err(|err| format!("failed to create {}: {err}", workflow_dir.display()))?;
        fs::write(workflow_dir.join("ci.yml"), ci_workflow(repo_name))
            .map_err(|err| format!("failed to write CI workflow: {err}"))?;
        fs::write(
            workflow_dir.join("publish.yml"),
            publish_workflow(repo_name),
        )
        .map_err(|err| format!("failed to write publish workflow: {err}"))?;
    }
    fs::write(
        path.join("README.md"),
        format!(
            "# {}\n\n{}\n",
            title.clone().unwrap_or_else(|| repo_name.to_owned()),
            description
                .clone()
                .unwrap_or_else(|| "Catalog-driven GX solution repo.".to_owned())
        ),
    )
    .map_err(|err| format!("failed to write README: {err}"))?;
    fs::write(
        path.join("Cargo.toml"),
        scaffold_cargo_toml(repo_name, title.as_deref()),
    )
    .map_err(|err| format!("failed to write Cargo.toml: {err}"))?;
    let catalog = build_catalog_index(path, title.as_deref(), description.as_deref())?;
    write_root_catalog(&path.join("catalog.json"), &catalog)?;
    Ok(format!("initialized catalog repo at {}", path.display()))
}

pub(crate) fn build_catalog_repo(repo: &Path, check: bool) -> Result<String, String> {
    let existing = repo.join("catalog.json");
    let catalog = build_catalog_index(repo, None, None)?;
    let rendered = render_root_catalog(&catalog)?;
    if check {
        let current = fs::read_to_string(&existing)
            .map_err(|err| format!("failed to read {}: {err}", existing.display()))?;
        if normalize_json_text(&current)? == normalize_json_text(&rendered)? {
            return Ok(format!("catalog.json is up to date in {}", repo.display()));
        }
        return Err(format!("catalog.json is out of date in {}", repo.display()));
    }
    fs::write(&existing, rendered)
        .map_err(|err| format!("failed to write {}: {err}", existing.display()))?;
    Ok(format!("built {}", existing.display()))
}

pub(crate) fn validate_catalog_repo(repo: &Path) -> Result<String, String> {
    let root = load_root_catalog(&repo.join("catalog.json"))?;
    validate_root_catalog_schema(&root)?;
    validate_root_catalog_contents(repo, &root)?;
    Ok(format!("catalog validation passed for {}", repo.display()))
}

pub(crate) fn load_root_catalog(path: &Path) -> Result<RootCatalogIndex, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    validate_json_against_schema(&value, CATALOG_INDEX_SCHEMA, path)?;
    serde_json::from_value(value)
        .map_err(|err| format!("failed to decode {}: {err}", path.display()))
}

pub(crate) fn render_root_catalog(catalog: &RootCatalogIndex) -> Result<String, String> {
    let rendered = serde_json::to_string_pretty(catalog)
        .map_err(|err| format!("failed to serialize catalog: {err}"))?;
    Ok(format!("{rendered}\n"))
}

fn write_root_catalog(path: &Path, catalog: &RootCatalogIndex) -> Result<(), String> {
    fs::write(path, render_root_catalog(catalog)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn build_catalog_index(
    repo: &Path,
    title_override: Option<&str>,
    description_override: Option<&str>,
) -> Result<RootCatalogIndex, String> {
    let repo_name = repo
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("catalog-repo");
    let readme = fs::read_to_string(repo.join("README.md")).unwrap_or_default();
    let mut entries = Vec::new();
    entries.extend(discover_catalog_entries(
        repo,
        "assistant_templates",
        "assistant_template",
    )?);
    entries.extend(discover_catalog_entries(
        repo,
        "provider_presets",
        "provider_preset",
    )?);
    entries.extend(discover_catalog_entries(repo, "bundles", "bundle")?);
    entries.extend(discover_catalog_entries(repo, "overlays", "overlay")?);
    entries.extend(discover_catalog_entries(
        repo,
        "setup_profiles",
        "setup_profile",
    )?);
    entries.extend(discover_manifest_entries(
        repo,
        "contracts",
        "contract",
        "contract.json",
        &["contract_id", "id"],
    )?);
    entries.extend(discover_manifest_entries(
        repo,
        "resolvers",
        "resolver",
        "manifest.json",
        &["resolver_id", "id"],
    )?);
    entries.extend(discover_manifest_entries(
        repo,
        "adapters",
        "adapter",
        "manifest.json",
        &["adapter_id", "id"],
    )?);
    entries.extend(discover_manifest_entries(
        repo,
        "analysis",
        "analysis_op",
        "manifest.json",
        &["operation_id", "id"],
    )?);
    entries.extend(discover_manifest_entries(
        repo,
        "playbooks",
        "playbook",
        "manifest.json",
        &["flow_id", "id"],
    )?);
    entries.extend(discover_view_entries(repo)?);
    entries.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.id.cmp(&right.id)));
    fail_on_duplicate_ids(&entries)?;
    Ok(RootCatalogIndex {
        schema: "gx.catalog.index.v1".to_owned(),
        id: format!("{}.catalog", repo_name),
        version: "1.0.0".to_owned(),
        title: title_override
            .map(ToOwned::to_owned)
            .or_else(|| first_readme_heading(&readme))
            .unwrap_or_else(|| repo_name.to_owned()),
        description: description_override
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "Generated GX catalog index.".to_owned()),
        entries,
    })
}

fn discover_catalog_entries(
    repo: &Path,
    dir_name: &str,
    kind: &str,
) -> Result<Vec<RootCatalogEntry>, String> {
    let dir = repo.join(dir_name);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    let files = collect_json_files(&dir)?;
    for path in files {
        let relative = relative_path(repo, &path)?;
        let value = read_json_value(&path)?;
        let id = entry_id_for_kind(kind, &value, &relative);
        let title = entry_title(&value, &id);
        let description = entry_description(&value);
        let metadata = match kind {
            "assistant_template" => json!({
                "assistant_template_ref": relative,
                "domain_template_ref": value.get("domain_template_ref").and_then(Value::as_str),
                "bundle_ref": value.get("bundle_ref").and_then(Value::as_str)
            }),
            "provider_preset" => json!({
                "provider_refs": value.get("provider_refs").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
                "bundle_ref": value.get("bundle_ref").and_then(Value::as_str)
            }),
            "overlay" => json!({
                "default_locale": value.get("default_locale").cloned().unwrap_or(Value::Null),
                "tenant_id": value.get("tenant_id").cloned().unwrap_or(Value::Null),
                "branding": value.get("branding").cloned().unwrap_or(Value::Null)
            }),
            _ => json!({}),
        };
        entries.push(RootCatalogEntry {
            id,
            kind: kind.to_owned(),
            ref_path: relative,
            title,
            description,
            tags: Vec::new(),
            version: value
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("1.0.0")
                .to_owned(),
            source: String::new(),
            metadata,
        });
    }
    Ok(entries)
}

fn discover_manifest_entries(
    repo: &Path,
    dir_name: &str,
    kind: &str,
    marker: &str,
    id_keys: &[&str],
) -> Result<Vec<RootCatalogEntry>, String> {
    let base = repo.join(dir_name);
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for marker_path in collect_marker_files(&base, marker)? {
        let relative = relative_path(repo, &marker_path)?;
        let value = read_json_value(&marker_path)?;
        let id = id_keys
            .iter()
            .find_map(|key| value.get(*key).and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| fallback_id(&relative));
        entries.push(RootCatalogEntry {
            id: id.clone(),
            kind: kind.to_owned(),
            ref_path: relative,
            title: entry_title(&value, &id),
            description: entry_description(&value),
            tags: Vec::new(),
            version: value
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("1.0.0")
                .to_owned(),
            source: String::new(),
            metadata: json!({}),
        });
    }
    Ok(entries)
}

fn discover_view_entries(repo: &Path) -> Result<Vec<RootCatalogEntry>, String> {
    let views = repo.join("views");
    if !views.exists() {
        return Ok(Vec::new());
    }
    let mut candidates = collect_json_files(&views)?;
    candidates.extend(collect_marker_files(&views, "view.json")?);
    candidates.sort();
    candidates.dedup();
    let mut entries = Vec::new();
    for path in candidates {
        let relative = relative_path(repo, &path)?;
        let value = read_json_value(&path)?;
        let id = value
            .get("view_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| fallback_id(&relative));
        entries.push(RootCatalogEntry {
            id: id.clone(),
            kind: "view".to_owned(),
            ref_path: relative,
            title: entry_title(&value, &id),
            description: entry_description(&value),
            tags: Vec::new(),
            version: value
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("1.0.0")
                .to_owned(),
            source: String::new(),
            metadata: json!({}),
        });
    }
    Ok(entries)
}

fn validate_root_catalog_schema(catalog: &RootCatalogIndex) -> Result<(), String> {
    let value =
        serde_json::to_value(catalog).map_err(|err| format!("failed to encode catalog: {err}"))?;
    validate_json_against_schema(&value, CATALOG_INDEX_SCHEMA, Path::new("catalog.json"))
}

fn validate_root_catalog_contents(repo: &Path, catalog: &RootCatalogIndex) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for entry in &catalog.entries {
        if !known_kind(&entry.kind) {
            return Err(format!("catalog.json: unknown entry kind {}", entry.kind));
        }
        if !ids.insert(entry.id.clone()) {
            return Err(format!("catalog.json: duplicate entry id {}", entry.id));
        }
        let path = repo.join(&entry.ref_path);
        if !path.exists() {
            return Err(format!(
                "catalog.json: referenced file {} does not exist",
                path.display()
            ));
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            let value = read_json_value(&path)?;
            match entry.kind.as_str() {
                "assistant_template" => {
                    validate_json_against_schema(&value, ASSISTANT_TEMPLATE_SCHEMA, &path)?
                }
                "provider_preset" => {
                    validate_json_against_schema(&value, PROVIDER_PRESET_SCHEMA, &path)?
                }
                "overlay" => validate_json_against_schema(&value, OVERLAY_SCHEMA, &path)?,
                "setup_profile" => {
                    validate_json_against_schema(&value, SETUP_PROFILE_SCHEMA, &path)?
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn validate_json_against_schema(
    value: &Value,
    schema_source: &str,
    path: &Path,
) -> Result<(), String> {
    let schema: Value = serde_json::from_str(schema_source).map_err(|err| {
        format!(
            "failed to parse embedded schema for {}: {err}",
            path.display()
        )
    })?;
    let validator = validator_for(&schema).map_err(|err| {
        format!(
            "failed to prepare schema validator for {}: {err}",
            path.display()
        )
    })?;
    if let Err(_first) = validator.validate(value) {
        let mut messages = validator
            .iter_errors(value)
            .map(|err| err.to_string())
            .collect::<Vec<_>>();
        messages.sort();
        return Err(format!(
            "{} failed schema validation: {}",
            path.display(),
            messages.join("; ")
        ));
    }
    Ok(())
}

fn known_kind(kind: &str) -> bool {
    matches!(
        kind,
        "assistant_template"
            | "bundle"
            | "view"
            | "overlay"
            | "setup_profile"
            | "provider_preset"
            | "contract"
            | "resolver"
            | "adapter"
            | "analysis_op"
            | "playbook"
    )
}

fn fail_on_duplicate_ids(entries: &[RootCatalogEntry]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for entry in entries {
        if !ids.insert(entry.id.clone()) {
            return Err(format!("duplicate catalog entry id {}", entry.id));
        }
    }
    Ok(())
}

fn collect_json_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = fs::read_dir(dir)
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn collect_marker_files(dir: &Path, marker: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    visit_dirs(dir, &mut |path| {
        let candidate = path.join(marker);
        if candidate.exists() {
            files.push(candidate);
        }
    })?;
    files.sort();
    Ok(files)
}

fn visit_dirs(dir: &Path, visit: &mut dyn FnMut(&Path)) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            visit(&path);
            visit_dirs(&path, visit)?;
        }
    }
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|err| {
            format!(
                "failed to relativize {} against {}: {err}",
                path.display(),
                root.display()
            )
        })
        .map(|value| value.to_string_lossy().replace('\\', "/"))
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn entry_id_for_kind(kind: &str, value: &Value, relative: &str) -> String {
    match kind {
        "assistant_template" => value.get("entry_id"),
        "provider_preset" => value.get("entry_id"),
        "overlay" => value.get("entry_id"),
        "setup_profile" => value
            .get("entry_id")
            .or_else(|| value.get("profile_id"))
            .or_else(|| value.get("id")),
        "bundle" => value.get("bundle_id").or_else(|| value.get("id")),
        _ => value.get("id"),
    }
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .unwrap_or_else(|| fallback_id(relative))
}

fn entry_title(value: &Value, fallback: &str) -> String {
    value
        .get("display_name")
        .or_else(|| value.get("title"))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

fn entry_description(value: &Value) -> String {
    value
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn fallback_id(relative: &str) -> String {
    relative.trim_end_matches(".json").replace(['/', '\\'], ".")
}

fn first_readme_heading(readme: &str) -> Option<String> {
    readme
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_owned())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let rendered = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize {}: {err}", path.display()))?;
    fs::write(path, format!("{rendered}\n"))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn normalize_json_text(raw: &str) -> Result<String, String> {
    let value: Value =
        serde_json::from_str(raw).map_err(|err| format!("failed to parse json: {err}"))?;
    serde_json::to_string(&value).map_err(|err| format!("failed to normalize json: {err}"))
}

fn scaffold_cargo_toml(repo_name: &str, title: Option<&str>) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\ndescription = \"{}\"\nlicense = \"MIT\"\n\n[dev-dependencies]\ngreentic-x-contracts = \"0.4\"\ngreentic-x-flow = \"0.4\"\ngreentic-x-ops = \"0.4\"\ngreentic-x-runtime = \"0.4\"\ngreentic-x-types = \"0.4\"\nserde_json = \"1\"\n",
        repo_name,
        title.unwrap_or(repo_name)
    )
}

fn ci_workflow(repo_name: &str) -> String {
    CI_WORKFLOW_TEMPLATE.replace("__REPO_NAME__", repo_name)
}

fn publish_workflow(repo_name: &str) -> String {
    PUBLISH_WORKFLOW_TEMPLATE.replace("__REPO_NAME__", repo_name)
}

fn create_bundle_archive_script(repo_name: &str) -> String {
    CREATE_BUNDLE_ARCHIVE_TEMPLATE.replace("__REPO_NAME__", repo_name)
}

fn render_remote_catalog_script() -> String {
    RENDER_REMOTE_CATALOG_TEMPLATE.to_owned()
}

const CI_WORKFLOW_TEMPLATE: &str = r#"name: CI

on:
  pull_request:
  push:
    branches:
      - master
      - main

permissions:
  contents: read
  packages: write

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  BUNDLE_OCI_REF: ghcr.io/${{ github.repository_owner }}/bundles/__REPO_NAME__-bundle
  CATALOG_OCI_REF: ghcr.io/${{ github.repository_owner }}/catalogs/__REPO_NAME__
  TEMPLATE_OCI_PREFIX: ghcr.io/greenticai/greentic-x/templates
  CATALOG_ARTIFACT_TYPE: application/vnd.greentic.catalog.v1+json
  CATALOG_LAYER_MEDIA_TYPE: application/vnd.greentic.catalog.root.v1+json
  BUNDLE_LAYER_MEDIA_TYPE: application/vnd.greentic.catalog-bundle.v1+tar+gzip
  TEMPLATE_ARTIFACT_TYPE: application/vnd.greentic.template.v1+json
  TEMPLATE_LAYER_MEDIA_TYPE: application/json

jobs:
  catalog-validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-binstall
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/greentic-x
            ~/.cargo/.crates.toml
            ~/.cargo/.crates2.json
          key: ${{ runner.os }}-greentic-x-bin-${{ hashFiles('.github/workflows/ci.yml', '.github/workflows/publish.yml') }}
      - name: Add cargo bin to PATH
        shell: bash
        run: echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"
      - name: Install greentic-x CLI
        shell: bash
        run: |
          set -euo pipefail
          export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
          if ! command -v greentic-x >/dev/null 2>&1; then
            cargo binstall --no-confirm greentic-x
          fi
          if ! command -v greentic-x >/dev/null 2>&1; then
            cargo binstall --force --no-confirm greentic-x
          fi
          greentic-x --version
      - name: Build canonical catalog
        run: greentic-x catalog build --repo .
      - name: Validate canonical catalog
        run: greentic-x catalog validate --repo .

  publish-ghcr:
    runs-on: ubuntu-latest
    needs:
      - catalog-validate
    if: github.event_name == 'push' && (github.ref_name == 'master' || github.ref_name == 'main')
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-binstall
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/greentic-x
            ~/.cargo/.crates.toml
            ~/.cargo/.crates2.json
          key: ${{ runner.os }}-greentic-x-bin-${{ hashFiles('.github/workflows/ci.yml', '.github/workflows/publish.yml') }}
      - uses: oras-project/setup-oras@v1
      - name: Add cargo bin to PATH
        shell: bash
        run: echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"
      - name: Determine repo version
        id: version
        shell: bash
        run: |
          set -euo pipefail
          VERSION="$(sed -n '/^\[package\]/,/^\[/ s/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
          if [ -z "$VERSION" ]; then
            echo "Unable to determine version from Cargo.toml" >&2
            exit 1
          fi
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
      - name: Install greentic-x CLI
        shell: bash
        run: |
          set -euo pipefail
          export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
          if ! command -v greentic-x >/dev/null 2>&1; then
            cargo binstall --no-confirm greentic-x
          fi
          if ! command -v greentic-x >/dev/null 2>&1; then
            cargo binstall --force --no-confirm greentic-x
          fi
          greentic-x --version
      - name: Build canonical catalog
        run: greentic-x catalog build --repo .
      - name: Validate canonical catalog
        run: greentic-x catalog validate --repo .
      - name: Render remote catalog variants
        shell: bash
        run: |
          set -euo pipefail
          mkdir -p target
          python3 ci/render_remote_catalog.py \
            --input catalog.json \
            --output target/catalog.remote.json \
            --bundle-ref "oci://${BUNDLE_OCI_REF}:${{ steps.version.outputs.version }}"
          python3 ci/render_remote_catalog.py \
            --input catalog.json \
            --output target/catalog.remote.latest.json \
            --bundle-ref "oci://${BUNDLE_OCI_REF}:latest"
      - name: Build bundle archive
        shell: bash
        run: |
          set -euo pipefail
          ARCHIVE="target/__REPO_NAME__-bundle-${{ steps.version.outputs.version }}.tar.gz"
          bash ci/create_bundle_archive.sh "${{ steps.version.outputs.version }}" "$ARCHIVE"
          ls -l "$ARCHIVE"
      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GHCR_TOKEN || github.token }}
      - name: Push remote catalog
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ steps.version.outputs.version }}"
          oras push \
            --artifact-type "${CATALOG_ARTIFACT_TYPE}" \
            "${CATALOG_OCI_REF}:${VERSION}" \
            "target/catalog.remote.json:${CATALOG_LAYER_MEDIA_TYPE}"
          oras push \
            --artifact-type "${CATALOG_ARTIFACT_TYPE}" \
            "${CATALOG_OCI_REF}:latest" \
            "target/catalog.remote.latest.json:${CATALOG_LAYER_MEDIA_TYPE}"
      - name: Push bundle archive
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ steps.version.outputs.version }}"
          ARCHIVE="target/__REPO_NAME__-bundle-${VERSION}.tar.gz"
          oras push "${BUNDLE_OCI_REF}:${VERSION}" \
            "$ARCHIVE:${BUNDLE_LAYER_MEDIA_TYPE}"
          oras push "${BUNDLE_OCI_REF}:latest" \
            "$ARCHIVE:${BUNDLE_LAYER_MEDIA_TYPE}"
      - name: Push template artifacts
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ steps.version.outputs.version }}"
          shopt -s nullglob
          for template_file in templates/assistant/*.json templates/domain/*.json; do
            template_group="$(basename "$(dirname "$template_file")")"
            template_name="$(basename "$template_file" .json)"
            template_ref="${TEMPLATE_OCI_PREFIX}/${template_group}/${template_name}"
            oras push \
              --artifact-type "${TEMPLATE_ARTIFACT_TYPE}" \
              "${template_ref}:${VERSION}" \
              "${template_file}:${TEMPLATE_LAYER_MEDIA_TYPE}"
            oras push \
              --artifact-type "${TEMPLATE_ARTIFACT_TYPE}" \
              "${template_ref}:latest" \
              "${template_file}:${TEMPLATE_LAYER_MEDIA_TYPE}"
          done
"#;

const PUBLISH_WORKFLOW_TEMPLATE: &str = r#"name: Publish

on:
  workflow_dispatch:

permissions:
  contents: read
  packages: write

concurrency:
  group: publish-manual-${{ github.ref }}
  cancel-in-progress: false

env:
  BUNDLE_OCI_REF: ghcr.io/${{ github.repository_owner }}/bundles/__REPO_NAME__-bundle
  CATALOG_OCI_REF: ghcr.io/${{ github.repository_owner }}/catalogs/__REPO_NAME__
  TEMPLATE_OCI_PREFIX: ghcr.io/greenticai/greentic-x/templates
  CATALOG_ARTIFACT_TYPE: application/vnd.greentic.catalog.v1+json
  CATALOG_LAYER_MEDIA_TYPE: application/vnd.greentic.catalog.root.v1+json
  BUNDLE_LAYER_MEDIA_TYPE: application/vnd.greentic.catalog-bundle.v1+tar+gzip
  TEMPLATE_ARTIFACT_TYPE: application/vnd.greentic.template.v1+json
  TEMPLATE_LAYER_MEDIA_TYPE: application/json

jobs:
  publish-ghcr:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-binstall
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/greentic-x
            ~/.cargo/.crates.toml
            ~/.cargo/.crates2.json
          key: ${{ runner.os }}-greentic-x-bin-${{ hashFiles('.github/workflows/ci.yml', '.github/workflows/publish.yml') }}
      - uses: oras-project/setup-oras@v1
      - name: Add cargo bin to PATH
        shell: bash
        run: echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"
      - name: Determine repo version
        id: version
        shell: bash
        run: |
          set -euo pipefail
          VERSION="$(sed -n '/^\[package\]/,/^\[/ s/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
          if [ -z "$VERSION" ]; then
            echo "Unable to determine version from Cargo.toml" >&2
            exit 1
          fi
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
      - name: Install greentic-x CLI
        shell: bash
        run: |
          set -euo pipefail
          export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
          if ! command -v greentic-x >/dev/null 2>&1; then
            cargo binstall --no-confirm greentic-x
          fi
          if ! command -v greentic-x >/dev/null 2>&1; then
            cargo binstall --force --no-confirm greentic-x
          fi
          greentic-x --version
      - name: Build canonical catalog
        run: greentic-x catalog build --repo .
      - name: Validate canonical catalog
        run: greentic-x catalog validate --repo .
      - name: Render remote catalog variants
        shell: bash
        run: |
          set -euo pipefail
          mkdir -p target
          python3 ci/render_remote_catalog.py \
            --input catalog.json \
            --output target/catalog.remote.json \
            --bundle-ref "oci://${BUNDLE_OCI_REF}:${{ steps.version.outputs.version }}"
          python3 ci/render_remote_catalog.py \
            --input catalog.json \
            --output target/catalog.remote.latest.json \
            --bundle-ref "oci://${BUNDLE_OCI_REF}:latest"
      - name: Build bundle archive
        shell: bash
        run: |
          set -euo pipefail
          ARCHIVE="target/__REPO_NAME__-bundle-${{ steps.version.outputs.version }}.tar.gz"
          bash ci/create_bundle_archive.sh "${{ steps.version.outputs.version }}" "$ARCHIVE"
          ls -l "$ARCHIVE"
      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GHCR_TOKEN || github.token }}
      - name: Push remote catalog
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ steps.version.outputs.version }}"
          oras push \
            --artifact-type "${CATALOG_ARTIFACT_TYPE}" \
            "${CATALOG_OCI_REF}:${VERSION}" \
            "target/catalog.remote.json:${CATALOG_LAYER_MEDIA_TYPE}"
          oras push \
            --artifact-type "${CATALOG_ARTIFACT_TYPE}" \
            "${CATALOG_OCI_REF}:latest" \
            "target/catalog.remote.latest.json:${CATALOG_LAYER_MEDIA_TYPE}"
      - name: Push bundle archive
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ steps.version.outputs.version }}"
          ARCHIVE="target/__REPO_NAME__-bundle-${VERSION}.tar.gz"
          oras push "${BUNDLE_OCI_REF}:${VERSION}" \
            "$ARCHIVE:${BUNDLE_LAYER_MEDIA_TYPE}"
          oras push "${BUNDLE_OCI_REF}:latest" \
            "$ARCHIVE:${BUNDLE_LAYER_MEDIA_TYPE}"
      - name: Push template artifacts
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ steps.version.outputs.version }}"
          shopt -s nullglob
          for template_file in templates/assistant/*.json templates/domain/*.json; do
            template_group="$(basename "$(dirname "$template_file")")"
            template_name="$(basename "$template_file" .json)"
            template_ref="${TEMPLATE_OCI_PREFIX}/${template_group}/${template_name}"
            oras push \
              --artifact-type "${TEMPLATE_ARTIFACT_TYPE}" \
              "${template_ref}:${VERSION}" \
              "${template_file}:${TEMPLATE_LAYER_MEDIA_TYPE}"
            oras push \
              --artifact-type "${TEMPLATE_ARTIFACT_TYPE}" \
              "${template_ref}:latest" \
              "${template_file}:${TEMPLATE_LAYER_MEDIA_TYPE}"
          done
"#;

const CREATE_BUNDLE_ARCHIVE_TEMPLATE: &str = r#"#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <version> <output-path>" >&2
  exit 1
fi

VERSION="$1"
OUTPUT_PATH="$2"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGING_DIR="$ROOT_DIR/target/bundle-archive"
ARCHIVE_ROOT="__REPO_NAME__-bundle-${VERSION}"

rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/$ARCHIVE_ROOT"
mkdir -p "$(dirname "$OUTPUT_PATH")"

cp "$ROOT_DIR/catalog.json" "$STAGING_DIR/$ARCHIVE_ROOT/"
cp "$ROOT_DIR/README.md" "$STAGING_DIR/$ARCHIVE_ROOT/"
cp "$ROOT_DIR/Cargo.toml" "$STAGING_DIR/$ARCHIVE_ROOT/"

for path in \
  assistant_templates \
  provider_presets \
  bundles \
  views \
  overlays \
  setup_profiles \
  contracts \
  resolvers \
  adapters \
  analysis \
  playbooks \
  packs
do
  if [ -e "$ROOT_DIR/$path" ]; then
    cp -R "$ROOT_DIR/$path" "$STAGING_DIR/$ARCHIVE_ROOT/"
  fi
done

tar -C "$STAGING_DIR" -czf "$OUTPUT_PATH" "$ARCHIVE_ROOT"
echo "$OUTPUT_PATH"
"#;

const RENDER_REMOTE_CATALOG_TEMPLATE: &str = r#"#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


def rewrite_catalog(value: dict, bundle_ref: str) -> dict:
    entries = value.get("entries")
    if not isinstance(entries, list):
        raise SystemExit("catalog root is missing entries")

    for entry in entries:
        if not isinstance(entry, dict):
            raise SystemExit("catalog entry is not an object")
        reference = entry.get("ref")
        if not isinstance(reference, str) or not reference:
            raise SystemExit("catalog entry is missing ref")
        metadata = entry.setdefault("metadata", {})
        if not isinstance(metadata, dict):
            raise SystemExit("catalog entry metadata must be an object")
        metadata["bundle_ref"] = bundle_ref
        if entry.get("kind") == "assistant_template":
            metadata.setdefault("assistant_template_ref", reference)
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--bundle-ref", required=True)
    args = parser.parse_args()

    source = Path(args.input)
    target = Path(args.output)
    value = json.loads(source.read_text())
    rendered = json.dumps(rewrite_catalog(value, args.bundle_ref), indent=2) + "\n"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_valid_catalog_repo() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let repo = temp.path().join("zain-x");
        init_catalog_repo(&repo, "zain-x", None, None, true, true)?;
        assert!(repo.join("catalog.json").exists());
        assert!(repo.join("Cargo.toml").exists());
        assert!(repo.join("provider_presets").exists());
        assert!(repo.join("ci/create_bundle_archive.sh").exists());
        assert!(repo.join("ci/render_remote_catalog.py").exists());
        assert!(repo.join(".github/workflows/ci.yml").exists());
        assert!(repo.join(".github/workflows/publish.yml").exists());
        let cargo_toml = fs::read_to_string(repo.join("Cargo.toml"))?;
        assert!(cargo_toml.contains("greentic-x-contracts = \"0.4\""));
        assert!(cargo_toml.contains("greentic-x-flow = \"0.4\""));
        let workflow = fs::read_to_string(repo.join(".github/workflows/ci.yml"))?;
        assert!(workflow.contains("cargo binstall --no-confirm greentic-x"));
        assert!(workflow.contains("ghcr.io/${{ github.repository_owner }}/catalogs/zain-x"));
        assert!(workflow.contains("ghcr.io/${{ github.repository_owner }}/bundles/zain-x-bundle"));
        assert!(workflow.contains("python3 ci/render_remote_catalog.py"));
        validate_catalog_repo(&repo)?;
        Ok(())
    }

    #[test]
    fn build_generates_deterministic_catalog_order() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let repo = temp.path();
        fs::write(repo.join("README.md"), "# Demo\n")?;
        fs::create_dir_all(repo.join("overlays"))?;
        fs::create_dir_all(repo.join("assistant_templates"))?;
        write_json(
            &repo.join("overlays/default.json"),
            &json!({"entry_id":"demo.overlay","kind":"overlay","version":"1.0.0","display_name":"Overlay"}),
        )?;
        write_json(
            &repo.join("assistant_templates/example.json"),
            &json!({"entry_id":"demo.template","kind":"assistant-template","version":"1.0.0","display_name":"Template","assistant_template_ref":"assistant_templates/example.json"}),
        )?;
        build_catalog_repo(repo, false)?;
        let first = fs::read_to_string(repo.join("catalog.json"))?;
        build_catalog_repo(repo, false)?;
        let second = fs::read_to_string(repo.join("catalog.json"))?;
        assert_eq!(first, second);
        Ok(())
    }

    #[test]
    fn validate_fails_on_broken_refs() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        fs::write(
            temp.path().join("catalog.json"),
            serde_json::to_string_pretty(&json!({
                "schema":"gx.catalog.index.v1",
                "id":"demo.catalog",
                "version":"1.0.0",
                "title":"Demo",
                "entries":[{"id":"missing","kind":"assistant_template","ref":"assistant_templates/missing.json"}]
            }))?,
        )?;
        let err = validate_catalog_repo(temp.path()).expect_err("expected validation failure");
        assert!(err.contains("does not exist"));
        Ok(())
    }
}
