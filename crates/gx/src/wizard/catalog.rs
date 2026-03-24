use std::fs;
use std::path::{Path, PathBuf};

use greentic_distributor_client::{
    DistClient, DistOptions, OciPackFetcher, PackFetchOptions, oci_packs::DefaultRegistryClient,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::runtime::Runtime;

use crate::{
    AssistantTemplateCatalogEntry, CatalogProvenance, OverlayCatalogEntry,
    ProviderPresetCatalogEntry, RootCatalogEntry, RootCatalogIndex, WizardCatalogSet,
    catalog_repo::load_root_catalog,
};

const BUILTIN_WEBCHAT_REF: &str = "ghcr.io/greenticai/packs/messaging/messaging-webchat:latest";
const BUILTIN_TEAMS_REF: &str = "ghcr.io/greenticai/packs/messaging/messaging-teams:latest";
const BUILTIN_WEBEX_REF: &str = "ghcr.io/greenticai/packs/messaging/messaging-webex:latest";
const BUILTIN_SLACK_REF: &str = "ghcr.io/greenticai/packs/messaging/messaging-slack:latest";

#[derive(Clone, Debug)]
pub(crate) struct FetchResult {
    pub(crate) bytes: Vec<u8>,
    pub(crate) resolved_digest: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPackArtifact {
    pub(crate) path: PathBuf,
    pub(crate) resolved_digest: String,
    pub(crate) media_type: String,
}

pub(crate) trait RemoteCatalogFetcher {
    fn fetch_json(&self, cache_root: &Path, reference: &str) -> Result<FetchResult, String>;
    fn resolve_pack_ref(&self, cache_root: &Path, reference: &str) -> Result<String, String>;
    fn fetch_pack_artifact(
        &self,
        cache_root: &Path,
        reference: &str,
    ) -> Result<ResolvedPackArtifact, String>;
}

pub(crate) struct DistributorCatalogFetcher;

impl RemoteCatalogFetcher for DistributorCatalogFetcher {
    fn fetch_json(&self, cache_root: &Path, reference: &str) -> Result<FetchResult, String> {
        let resolved = fetch_pack_artifact(cache_root, reference)?;
        let bytes = fs::read(&resolved.path).map_err(|err| {
            format!(
                "failed to read fetched catalog artifact {}: {err}",
                resolved.path.display()
            )
        })?;
        Ok(FetchResult {
            bytes,
            resolved_digest: Some(resolved.resolved_digest),
        })
    }

    fn resolve_pack_ref(&self, cache_root: &Path, reference: &str) -> Result<String, String> {
        let resolved = fetch_pack_artifact(cache_root, reference)?;
        Ok(resolved.resolved_digest)
    }

    fn fetch_pack_artifact(
        &self,
        cache_root: &Path,
        reference: &str,
    ) -> Result<ResolvedPackArtifact, String> {
        fetch_pack_artifact(cache_root, reference)
    }
}

pub(crate) fn load_catalogs(
    cwd: &Path,
    catalog_refs: &[String],
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<WizardCatalogSet, String> {
    let mut catalogs = WizardCatalogSet::default();
    load_local_templates(cwd, &mut catalogs.templates)?;
    load_local_provider_presets(cwd, &mut catalogs.provider_presets)?;
    load_local_overlays(cwd, &mut catalogs.overlays)?;
    catalogs.provider_presets.extend(builtin_provider_presets());

    for catalog_ref in catalog_refs {
        let (document, provenance) = load_explicit_catalog(cwd, catalog_ref, fetcher)?;
        catalogs
            .templates
            .extend(root_templates(&document, &provenance));
        catalogs
            .provider_presets
            .extend(root_provider_presets(&document, &provenance));
        catalogs
            .overlays
            .extend(root_overlays(&document, &provenance));
    }

    catalogs.templates = dedupe_templates(catalogs.templates);
    catalogs.provider_presets = dedupe_provider_presets(catalogs.provider_presets);
    catalogs.overlays = dedupe_overlays(catalogs.overlays);
    Ok(catalogs)
}

fn load_explicit_catalog(
    cwd: &Path,
    catalog_ref: &str,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<(RootCatalogIndex, CatalogProvenance), String> {
    if is_remote_catalog_ref(catalog_ref) {
        let fetched = fetcher.fetch_json(cwd, catalog_ref)?;
        let document: RootCatalogIndex = serde_json::from_slice(&fetched.bytes)
            .map_err(|err| format!("failed to decode remote catalog {catalog_ref}: {err}"))?;
        return Ok((
            document,
            CatalogProvenance {
                source_type: catalog_source_type(catalog_ref).to_owned(),
                source_ref: catalog_ref.to_owned(),
                resolved_digest: fetched.resolved_digest,
            },
        ));
    }
    let path = if Path::new(catalog_ref).is_absolute() {
        PathBuf::from(catalog_ref)
    } else {
        cwd.join(catalog_ref)
    };
    let catalog_path = if path.is_dir() {
        path.join("catalog.json")
    } else {
        path
    };
    let document = load_root_catalog(&catalog_path)?;
    Ok((
        document,
        CatalogProvenance {
            source_type: "local".to_owned(),
            source_ref: catalog_path.display().to_string(),
            resolved_digest: None,
        },
    ))
}

fn is_remote_catalog_ref(reference: &str) -> bool {
    reference.starts_with("oci://")
        || reference.starts_with("repo://")
        || reference.starts_with("store://")
}

fn catalog_source_type(reference: &str) -> &str {
    if reference.starts_with("store://") {
        "store"
    } else if reference.starts_with("repo://") {
        "repo"
    } else {
        "oci"
    }
}

fn root_templates(
    document: &RootCatalogIndex,
    provenance: &CatalogProvenance,
) -> Vec<AssistantTemplateCatalogEntry> {
    document
        .entries
        .iter()
        .filter(|entry| entry.kind == "assistant_template")
        .map(|entry| AssistantTemplateCatalogEntry {
            entry_id: entry.id.clone(),
            kind: "assistant-template".to_owned(),
            version: version_for_entry(entry),
            display_name: title_for_entry(entry),
            description: description_for_entry(entry),
            assistant_template_ref: entry
                .metadata
                .get("assistant_template_ref")
                .and_then(Value::as_str)
                .unwrap_or(&entry.ref_path)
                .to_owned(),
            domain_template_ref: entry
                .metadata
                .get("domain_template_ref")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            bundle_ref: entry
                .metadata
                .get("bundle_ref")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            provenance: Some(provenance.clone()),
        })
        .collect()
}

fn root_provider_presets(
    document: &RootCatalogIndex,
    provenance: &CatalogProvenance,
) -> Vec<ProviderPresetCatalogEntry> {
    document
        .entries
        .iter()
        .filter(|entry| entry.kind == "provider_preset")
        .map(|entry| ProviderPresetCatalogEntry {
            entry_id: entry.id.clone(),
            kind: "provider-preset".to_owned(),
            version: version_for_entry(entry),
            display_name: title_for_entry(entry),
            description: description_for_entry(entry),
            provider_refs: entry
                .metadata
                .get("provider_refs")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| vec![entry.ref_path.clone()]),
            bundle_ref: entry
                .metadata
                .get("bundle_ref")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            provenance: Some(provenance.clone()),
        })
        .collect()
}

fn root_overlays(
    document: &RootCatalogIndex,
    provenance: &CatalogProvenance,
) -> Vec<OverlayCatalogEntry> {
    document
        .entries
        .iter()
        .filter(|entry| entry.kind == "overlay")
        .map(|entry| OverlayCatalogEntry {
            entry_id: entry.id.clone(),
            kind: "overlay".to_owned(),
            version: version_for_entry(entry),
            display_name: title_for_entry(entry),
            description: description_for_entry(entry),
            default_locale: entry
                .metadata
                .get("default_locale")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            tenant_id: entry
                .metadata
                .get("tenant_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            branding: entry
                .metadata
                .get("branding")
                .cloned()
                .filter(|value| !value.is_null()),
            provenance: Some(provenance.clone()),
        })
        .collect()
}

fn title_for_entry(entry: &RootCatalogEntry) -> String {
    if entry.title.trim().is_empty() {
        entry.id.clone()
    } else {
        entry.title.clone()
    }
}

fn description_for_entry(entry: &RootCatalogEntry) -> String {
    entry.description.clone()
}

fn version_for_entry(entry: &RootCatalogEntry) -> String {
    if entry.version.trim().is_empty() {
        "1.0.0".to_owned()
    } else {
        entry.version.clone()
    }
}

pub(crate) fn find_template_by_id<'a>(
    catalogs: &'a WizardCatalogSet,
    entry_id: &str,
) -> Option<&'a AssistantTemplateCatalogEntry> {
    catalogs
        .templates
        .iter()
        .find(|item| item.entry_id == entry_id)
}

pub(crate) fn find_provider_preset_by_id<'a>(
    catalogs: &'a WizardCatalogSet,
    entry_id: &str,
) -> Option<&'a ProviderPresetCatalogEntry> {
    catalogs
        .provider_presets
        .iter()
        .find(|item| item.entry_id == entry_id)
}

pub(crate) fn find_overlay_by_id<'a>(
    catalogs: &'a WizardCatalogSet,
    entry_id: &str,
) -> Option<&'a OverlayCatalogEntry> {
    catalogs
        .overlays
        .iter()
        .find(|item| item.entry_id == entry_id)
}

pub(crate) fn builtin_channel_options() -> [(&'static str, &'static str); 4] {
    [
        ("webchat", "Webchat"),
        ("teams", "Teams"),
        ("webex", "WebEx"),
        ("slack", "Slack"),
    ]
}

pub(crate) fn builtin_provider_ref(label: &str) -> Option<&'static str> {
    match label {
        "webchat" => Some(BUILTIN_WEBCHAT_REF),
        "teams" => Some(BUILTIN_TEAMS_REF),
        "webex" => Some(BUILTIN_WEBEX_REF),
        "slack" => Some(BUILTIN_SLACK_REF),
        _ => None,
    }
}

pub(crate) fn normalize_pack_fetch_ref(reference: &str) -> Result<String, String> {
    if let Some(body) = reference.strip_prefix("oci://") {
        return Ok(body.to_owned());
    }
    if let Some(body) = reference.strip_prefix("repo://") {
        return map_registry_ref(body, "GREENTIC_REPO_REGISTRY_BASE", "repo");
    }
    if let Some(body) = reference.strip_prefix("store://") {
        return Ok(format!("store://{body}"));
    }
    Ok(reference.to_owned())
}

pub(crate) fn pin_reference(reference: &str, digest: &str) -> String {
    let normalized_digest = if digest.starts_with("sha256:") {
        digest.to_owned()
    } else {
        format!("sha256:{digest}")
    };
    if reference.contains('@') {
        return reference.to_owned();
    }
    if let Some(body) = reference.strip_prefix("oci://") {
        return format!("oci://{}@{}", strip_latest_tag(body), normalized_digest);
    }
    format!("{}@{}", strip_latest_tag(reference), normalized_digest)
}

fn strip_latest_tag(reference: &str) -> String {
    reference
        .strip_suffix(":latest")
        .unwrap_or(reference)
        .to_owned()
}

fn pack_fetch_options(cache_root: &Path) -> PackFetchOptions {
    PackFetchOptions {
        allow_tags: true,
        offline: false,
        cache_dir: cache_root.join(".gx").join("cache").join("pack-fetch"),
        ..PackFetchOptions::default()
    }
}

fn fetch_pack_artifact(cache_root: &Path, reference: &str) -> Result<ResolvedPackArtifact, String> {
    if reference.starts_with("store://") {
        return fetch_store_pack_artifact(cache_root, reference);
    }

    let reference = normalize_pack_fetch_ref(reference)?;
    let options = pack_fetch_options(cache_root);
    let runtime =
        Runtime::new().map_err(|err| format!("failed to start pack fetch runtime: {err}"))?;
    let resolved = runtime
        .block_on(
            OciPackFetcher::<DefaultRegistryClient>::new(options).fetch_pack_to_cache(&reference),
        )
        .map_err(|err| format!("failed to fetch pack artifact {reference}: {err}"))?;
    Ok(ResolvedPackArtifact {
        path: resolved.path,
        resolved_digest: resolved.resolved_digest,
        media_type: resolved.media_type,
    })
}

fn fetch_store_pack_artifact(
    cache_root: &Path,
    reference: &str,
) -> Result<ResolvedPackArtifact, String> {
    let options = DistOptions {
        allow_tags: true,
        offline: false,
        cache_dir: cache_root.join(".gx").join("cache").join("distributor"),
        ..DistOptions::default()
    };
    let runtime =
        Runtime::new().map_err(|err| format!("failed to start store fetch runtime: {err}"))?;
    let client = DistClient::new(options);
    let downloaded = runtime
        .block_on(client.download_store_artifact(reference))
        .map_err(|err| format!("failed to fetch store artifact {reference}: {err}"))?;
    let path = cache_store_artifact(cache_root, reference, &downloaded.digest, &downloaded.bytes)?;
    Ok(ResolvedPackArtifact {
        path,
        resolved_digest: downloaded.digest,
        media_type: downloaded.media_type,
    })
}

fn cache_store_artifact(
    cache_root: &Path,
    reference: &str,
    digest: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let file_name = reference_file_name(reference);
    let path = cache_root
        .join(".gx")
        .join("cache")
        .join("store-download")
        .join(digest.replace(':', "-"))
        .join(file_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create store download cache directory {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::write(&path, bytes).map_err(|err| {
        format!(
            "failed to write store download cache {}: {err}",
            path.display()
        )
    })?;
    Ok(path)
}

fn reference_file_name(reference: &str) -> String {
    let candidate = reference
        .rsplit('/')
        .next()
        .unwrap_or("artifact.bin")
        .split('@')
        .next()
        .unwrap_or("artifact.bin")
        .split(':')
        .next()
        .unwrap_or("artifact.bin")
        .trim();
    if candidate.is_empty() {
        "artifact.bin".to_owned()
    } else {
        candidate.to_owned()
    }
}

fn map_registry_ref(target: &str, env_var: &str, kind: &str) -> Result<String, String> {
    if looks_like_oci_reference(target) {
        return Ok(target.to_owned());
    }
    let base = std::env::var(env_var).map_err(|_| {
        format!("{kind} reference {kind}://{target} requires {env_var} or a direct OCI target")
    })?;
    Ok(format!(
        "{}/{}",
        base.trim_end_matches('/'),
        target.trim_start_matches('/')
    ))
}

fn looks_like_oci_reference(value: &str) -> bool {
    let Some(first_segment) = value.split('/').next() else {
        return false;
    };
    first_segment.contains('.')
        || first_segment.contains(':')
        || first_segment == "localhost"
        || value.contains('@')
}

fn load_local_templates(
    cwd: &Path,
    target: &mut Vec<AssistantTemplateCatalogEntry>,
) -> Result<(), String> {
    for path in json_files_in_dir(&cwd.join("catalog").join("templates"))? {
        let mut entry: AssistantTemplateCatalogEntry = read_json_file(&path)?;
        entry.provenance = Some(local_provenance(&path));
        target.push(entry);
    }
    for path in json_files_in_dir(&cwd.join("assistant_templates"))? {
        let mut entry: AssistantTemplateCatalogEntry = read_json_file(&path)?;
        entry.provenance = Some(local_provenance(&path));
        target.push(entry);
    }
    Ok(())
}

fn load_local_provider_presets(
    cwd: &Path,
    target: &mut Vec<ProviderPresetCatalogEntry>,
) -> Result<(), String> {
    for path in json_files_in_dir(&cwd.join("catalog").join("provider-presets"))? {
        let mut entry: ProviderPresetCatalogEntry = read_json_file(&path)?;
        entry.provenance = Some(local_provenance(&path));
        target.push(entry);
    }
    Ok(())
}

fn load_local_overlays(cwd: &Path, target: &mut Vec<OverlayCatalogEntry>) -> Result<(), String> {
    for path in json_files_in_dir(&cwd.join("catalog").join("overlays"))? {
        let mut entry: OverlayCatalogEntry = read_json_file(&path)?;
        entry.provenance = Some(local_provenance(&path));
        target.push(entry);
    }
    Ok(())
}

fn builtin_provider_presets() -> Vec<ProviderPresetCatalogEntry> {
    vec![
        preset("builtin.webchat", "Webchat", BUILTIN_WEBCHAT_REF),
        preset("builtin.teams", "Teams", BUILTIN_TEAMS_REF),
        preset("builtin.webex", "WebEx", BUILTIN_WEBEX_REF),
        preset("builtin.slack", "Slack", BUILTIN_SLACK_REF),
    ]
}

fn preset(entry_id: &str, display_name: &str, provider_ref: &str) -> ProviderPresetCatalogEntry {
    ProviderPresetCatalogEntry {
        entry_id: entry_id.to_owned(),
        kind: "provider-preset".to_owned(),
        version: "1.0.0".to_owned(),
        display_name: display_name.to_owned(),
        description: format!("{display_name} built-in channel preset."),
        provider_refs: vec![provider_ref.to_owned()],
        bundle_ref: None,
        provenance: Some(CatalogProvenance {
            source_type: "local".to_owned(),
            source_ref: format!("builtin:{entry_id}"),
            resolved_digest: None,
        }),
    }
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn json_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(dir)
        .map_err(|err| format!("failed to read catalog directory {}: {err}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn local_provenance(path: &Path) -> CatalogProvenance {
    CatalogProvenance {
        source_type: "local".to_owned(),
        source_ref: path.display().to_string(),
        resolved_digest: None,
    }
}

fn dedupe_templates(
    entries: Vec<AssistantTemplateCatalogEntry>,
) -> Vec<AssistantTemplateCatalogEntry> {
    dedupe_by_version(entries, |item| item.entry_id.clone())
}

fn dedupe_provider_presets(
    entries: Vec<ProviderPresetCatalogEntry>,
) -> Vec<ProviderPresetCatalogEntry> {
    dedupe_by_version(entries, |item| item.entry_id.clone())
}

fn dedupe_overlays(entries: Vec<OverlayCatalogEntry>) -> Vec<OverlayCatalogEntry> {
    dedupe_by_version(entries, |item| item.entry_id.clone())
}

fn dedupe_by_version<T>(entries: Vec<T>, key_fn: impl Fn(&T) -> String) -> Vec<T>
where
    T: Clone + HasVersion,
{
    let mut ordered = entries;
    ordered.sort_by(|left, right| {
        let left_key = key_fn(left);
        let right_key = key_fn(right);
        left_key
            .cmp(&right_key)
            .then(compare_versions(left.version(), right.version()))
    });
    let mut deduped = Vec::new();
    for entry in ordered {
        if let Some(last) = deduped.last()
            && key_fn(last) == key_fn(&entry)
        {
            let _ = deduped.pop();
        }
        deduped.push(entry);
    }
    deduped
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    parse_semver(left).cmp(&parse_semver(right))
}

fn parse_semver(raw: &str) -> (u64, u64, u64) {
    let mut parts = raw.split('.').map(|item| item.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

trait HasVersion {
    fn version(&self) -> &str;
}

impl HasVersion for AssistantTemplateCatalogEntry {
    fn version(&self) -> &str {
        &self.version
    }
}

impl HasVersion for ProviderPresetCatalogEntry {
    fn version(&self) -> &str {
        &self.version
    }
}

impl HasVersion for OverlayCatalogEntry {
    fn version(&self) -> &str {
        &self.version
    }
}

pub(crate) fn value_from_template(entry: &AssistantTemplateCatalogEntry) -> Value {
    serde_json::to_value(entry).unwrap_or(Value::Null)
}

pub(crate) fn value_from_provider(entry: &ProviderPresetCatalogEntry) -> Value {
    serde_json::to_value(entry).unwrap_or(Value::Null)
}

pub(crate) fn value_from_overlay(entry: &OverlayCatalogEntry) -> Value {
    serde_json::to_value(entry).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct StubFetcher {
        result: RefCell<Option<FetchResult>>,
    }

    impl RemoteCatalogFetcher for StubFetcher {
        fn fetch_json(&self, _cache_root: &Path, _reference: &str) -> Result<FetchResult, String> {
            self.result
                .borrow_mut()
                .take()
                .ok_or_else(|| "missing stub fetch result".to_owned())
        }

        fn resolve_pack_ref(&self, _cache_root: &Path, reference: &str) -> Result<String, String> {
            Ok(format!("sha256:resolved-{}", reference.replace('/', "-")))
        }

        fn fetch_pack_artifact(
            &self,
            _cache_root: &Path,
            reference: &str,
        ) -> Result<ResolvedPackArtifact, String> {
            Ok(ResolvedPackArtifact {
                path: PathBuf::from(reference),
                resolved_digest: format!("sha256:resolved-{}", reference.replace('/', "-")),
                media_type: "application/octet-stream".to_owned(),
            })
        }
    }

    #[test]
    fn pin_reference_rewrites_latest_tag() {
        assert_eq!(
            pin_reference("ghcr.io/demo/preset:latest", "sha256:abc"),
            "ghcr.io/demo/preset@sha256:abc"
        );
        assert_eq!(
            pin_reference("oci://ghcr.io/demo/template:latest", "abc"),
            "oci://ghcr.io/demo/template@sha256:abc"
        );
    }

    #[test]
    fn load_catalogs_merges_local_and_oci_entries() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path();
        fs::create_dir_all(root.join("assistant_templates")).expect("mkdir");
        fs::write(
            root.join("assistant_templates/local.json"),
            r#"{
              "entry_id": "local-template",
              "kind": "assistant-template",
              "version": "1.0.0",
              "display_name": "Local template",
              "assistant_template_ref": "templates/assistant/basic-empty.json"
            }"#,
        )
        .expect("write");
        let remote_json = br#"{
          "schema": "gx.catalog.index.v1",
          "id": "demo.catalog",
          "version": "1.0.0",
          "title": "Demo",
          "entries": [{
            "id": "remote-preset",
            "kind": "provider_preset",
            "ref": "provider_presets/remote.json",
            "title": "Remote preset",
            "version": "1.0.0",
            "metadata": {
              "provider_refs": ["ghcr.io/demo/preset:latest"]
            }
          }]
        }"#;
        let fetcher = StubFetcher {
            result: RefCell::new(Some(FetchResult {
                bytes: remote_json.to_vec(),
                resolved_digest: Some("sha256:remote".to_owned()),
            })),
        };
        let catalogs = load_catalogs(
            root,
            &[String::from("oci://demo/catalog.json:latest")],
            &fetcher,
        )
        .expect("catalogs");
        assert!(
            catalogs
                .templates
                .iter()
                .any(|item| item.entry_id == "local-template")
        );
        assert!(
            catalogs
                .provider_presets
                .iter()
                .any(|item| item.entry_id == "remote-preset")
        );
    }

    #[test]
    fn load_catalogs_reads_local_root_catalog() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path();
        fs::write(
            root.join("catalog.json"),
            r#"{
              "schema": "gx.catalog.index.v1",
              "id": "local.catalog",
              "version": "1.0.0",
              "title": "Local",
              "entries": [{
                "id": "local-provider",
                "kind": "provider_preset",
                "ref": "provider_presets/local.json",
                "title": "Local Provider",
                "version": "1.0.0",
                "metadata": {"provider_refs": ["ghcr.io/demo/local:latest"]}
              }]
            }"#,
        )
        .expect("write");
        let fetcher = StubFetcher {
            result: RefCell::new(None),
        };
        let catalogs =
            load_catalogs(root, &[String::from("catalog.json")], &fetcher).expect("catalogs");
        assert!(
            catalogs
                .provider_presets
                .iter()
                .any(|item| item.entry_id == "local-provider")
        );
    }

    #[test]
    fn normalize_pack_fetch_ref_maps_greentic_biz_store_refs() {
        assert_eq!(
            normalize_pack_fetch_ref(
                "store://greentic-biz/tenant-a/catalogs/zain-x/catalog.json:latest"
            )
            .expect("store refs should be preserved for store downloads"),
            "store://greentic-biz/tenant-a/catalogs/zain-x/catalog.json:latest"
        );
    }

    #[test]
    fn reference_file_name_strips_store_locator_suffixes() {
        assert_eq!(
            reference_file_name(
                "store://greentic-biz/tenant-a/catalogs/zain-x/catalog.json:latest"
            ),
            "catalog.json"
        );
    }
}
