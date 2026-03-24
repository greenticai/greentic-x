use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::{
    BundlePlan, CompositionRequest, SetupAnswers, SolutionManifest, WizardAnswerDocument,
    WizardCatalogSet,
};

use super::bundle::materialize_bundle_member;
use super::catalog::{
    RemoteCatalogFetcher, builtin_provider_ref, find_overlay_by_id, find_provider_preset_by_id,
    find_template_by_id, pin_reference, value_from_overlay, value_from_provider,
    value_from_template,
};

const BUNDLE_WIZARD_ID: &str = "greentic-bundle.wizard.run";
const BUNDLE_SCHEMA_ID: &str = "greentic-bundle.wizard.answers";
const SCHEMA_VERSION: &str = "1.0.0";

pub(crate) struct GeneratedArtifacts {
    pub(crate) solution_manifest: SolutionManifest,
    pub(crate) bundle_plan: BundlePlan,
    pub(crate) bundle_answers: WizardAnswerDocument,
    pub(crate) setup_answers: SetupAnswers,
    pub(crate) readme: String,
}

pub(crate) fn generate_artifacts(
    cwd: &Path,
    request: &CompositionRequest,
    catalogs: &WizardCatalogSet,
    locale: &str,
    execution_resolves_remote: bool,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<GeneratedArtifacts, String> {
    let template = resolve_template(cwd, request, catalogs, execution_resolves_remote, fetcher)?;
    let providers =
        resolve_provider_presets(cwd, request, catalogs, execution_resolves_remote, fetcher)?;
    let overlay = resolve_overlay(request, catalogs);
    let solution_manifest = SolutionManifest {
        schema_id: "gx.solution.manifest".to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        solution_id: request.solution_id.clone(),
        solution_name: request.solution_name.clone(),
        description: request.description.clone(),
        output_dir: request.output_dir.clone(),
        template: template.clone(),
        provider_presets: providers.clone(),
        overlay: overlay.clone(),
        catalog_sources: request.catalog_oci_refs.clone(),
    };
    let bundle_plan = BundlePlan {
        schema_id: "gx.bundle.plan".to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        solution_id: request.solution_id.clone(),
        bundle_output_path: request.bundle_output_path.clone(),
        bundle_answers_path: request.bundle_answers_path.clone(),
        steps: vec![
            json!({"kind": "emit_solution_manifest", "path": request.solution_manifest_path}),
            json!({"kind": "emit_bundle_plan", "path": request.bundle_plan_path}),
            json!({"kind": "emit_bundle_answers", "path": request.bundle_answers_path}),
            json!({"kind": "emit_setup_answers", "path": request.setup_answers_path}),
            json!({"kind": "emit_readme", "path": request.readme_path}),
            json!({"kind": "delegate_bundle_generation", "path": request.bundle_output_path}),
        ],
    };
    let provider_refs = providers
        .iter()
        .filter_map(|item| item.get("provider_refs").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let template_sources =
        materialize_template_sources(cwd, &template, execution_resolves_remote, fetcher)?;
    let bundle_answers = build_bundle_answers(
        request,
        locale,
        &template,
        template_sources.as_ref(),
        &provider_refs,
        overlay.as_ref(),
    );
    let setup_answers = SetupAnswers {
        schema_id: "gx.setup.answers".to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        solution_id: request.solution_id.clone(),
        setup_mode: "minimal".to_owned(),
        provider_refs,
        overlay,
    };
    let readme = render_readme(request, &solution_manifest, &bundle_plan);
    Ok(GeneratedArtifacts {
        solution_manifest,
        bundle_plan,
        bundle_answers,
        setup_answers,
        readme,
    })
}

pub(crate) fn write_generated_artifacts(
    cwd: &Path,
    request: &CompositionRequest,
    generated: &GeneratedArtifacts,
) -> Result<(), String> {
    write_json_file(
        cwd,
        &request.solution_manifest_path,
        &generated.solution_manifest,
    )?;
    write_json_file(cwd, &request.bundle_plan_path, &generated.bundle_plan)?;
    write_json_file(cwd, &request.bundle_answers_path, &generated.bundle_answers)?;
    write_json_file(cwd, &request.setup_answers_path, &generated.setup_answers)?;
    let readme_path = resolve_output_path(cwd, &request.readme_path);
    if let Some(parent) = readme_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create README output directory {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::write(&readme_path, &generated.readme)
        .map_err(|err| format!("failed to write {}: {err}", readme_path.display()))?;
    Ok(())
}

pub(crate) fn generated_output_paths(request: &CompositionRequest) -> Vec<String> {
    vec![
        request.solution_manifest_path.clone(),
        request.bundle_plan_path.clone(),
        request.bundle_answers_path.clone(),
        request.setup_answers_path.clone(),
        request.readme_path.clone(),
        request.bundle_output_path.clone(),
    ]
}

fn resolve_template(
    cwd: &Path,
    request: &CompositionRequest,
    catalogs: &WizardCatalogSet,
    execution_resolves_remote: bool,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<Value, String> {
    match request.template_mode.as_str() {
        "catalog" => {
            let entry_id = request.template_entry_id.as_deref().ok_or_else(|| {
                "template_entry_id is required for catalog template mode".to_owned()
            })?;
            let entry = find_template_by_id(catalogs, entry_id)
                .ok_or_else(|| format!("unknown catalog template {entry_id}"))?;
            let mut value = value_from_template(entry);
            maybe_pin_template_refs(cwd, &mut value, execution_resolves_remote, fetcher)?;
            Ok(value)
        }
        "basic_empty" => Ok(json!({
            "entry_id": "builtin.basic-empty",
            "kind": "assistant-template",
            "version": "1.0.0",
            "display_name": "Basic empty solution",
            "assistant_template_ref": "templates/assistant/basic-empty.json",
            "domain_template_ref": "templates/domain/basic-empty.json",
            "provenance": {
                "source_type": "local",
                "source_ref": "builtin:basic-empty",
                "resolved_digest": null
            }
        })),
        "manual" => Ok(json!({
            "entry_id": "manual",
            "kind": "assistant-template",
            "version": "1.0.0",
            "display_name": request
                .template_display_name
                .clone()
                .unwrap_or_else(|| "Manual template".to_owned()),
            "assistant_template_ref": request
                .assistant_template_ref
                .clone()
                .ok_or_else(|| "assistant_template_ref is required for manual template mode".to_owned())?,
            "domain_template_ref": request.domain_template_ref.clone()
        })),
        other => Err(format!("unsupported template_mode {other}")),
    }
}

fn resolve_provider_presets(
    _cwd: &Path,
    request: &CompositionRequest,
    catalogs: &WizardCatalogSet,
    _execution_resolves_remote: bool,
    _fetcher: &dyn RemoteCatalogFetcher,
) -> Result<Vec<Value>, String> {
    let presets = match request.provider_selection.as_str() {
        "webchat" | "teams" | "webex" | "slack" => {
            let provider_ref =
                builtin_provider_ref(&request.provider_selection).ok_or_else(|| {
                    format!(
                        "unsupported provider selection {}",
                        request.provider_selection
                    )
                })?;
            vec![json!({
                "entry_id": format!("builtin.{}", request.provider_selection),
                "kind": "provider-preset",
                "version": "1.0.0",
                "display_name": request
                    .provider_preset_display_name
                    .clone()
                    .unwrap_or_else(|| title_case(&request.provider_selection)),
                "provider_refs": [provider_ref]
            })]
        }
        "all" => vec![
            builtin_value("builtin.webchat", "Webchat", "webchat"),
            builtin_value("builtin.teams", "Teams", "teams"),
            builtin_value("builtin.webex", "WebEx", "webex"),
            builtin_value("builtin.slack", "Slack", "slack"),
        ],
        "catalog" => {
            let entry_id = request.provider_preset_entry_id.as_deref().ok_or_else(|| {
                "provider_preset_entry_id is required for catalog provider mode".to_owned()
            })?;
            let entry = find_provider_preset_by_id(catalogs, entry_id)
                .ok_or_else(|| format!("unknown provider preset {entry_id}"))?;
            vec![value_from_provider(entry)]
        }
        "manual" => vec![json!({
            "entry_id": "manual",
            "kind": "provider-preset",
            "version": "1.0.0",
            "display_name": request
                .provider_preset_display_name
                .clone()
                .unwrap_or_else(|| "Manual override".to_owned()),
            "provider_refs": request.provider_refs
        })],
        other => return Err(format!("unsupported provider_selection {other}")),
    };
    Ok(presets)
}

fn resolve_overlay(request: &CompositionRequest, catalogs: &WizardCatalogSet) -> Option<Value> {
    request
        .overlay_entry_id
        .as_deref()
        .and_then(|entry_id| find_overlay_by_id(catalogs, entry_id))
        .map(value_from_overlay)
        .or_else(|| {
            if request.overlay_display_name.is_some()
                || request.overlay_default_locale.is_some()
                || request.overlay_tenant_id.is_some()
            {
                Some(json!({
                    "entry_id": request.overlay_entry_id.clone().unwrap_or_else(|| "manual-overlay".to_owned()),
                    "kind": "overlay",
                    "version": "1.0.0",
                    "display_name": request.overlay_display_name.clone().unwrap_or_else(|| "Manual overlay".to_owned()),
                    "default_locale": request.overlay_default_locale,
                    "tenant_id": request.overlay_tenant_id
                }))
            } else {
                None
            }
        })
}

#[derive(Clone, Debug)]
struct MaterializedTemplateSources {
    assistant_template_source: String,
    domain_template_source: String,
}

fn materialize_template_sources(
    cwd: &Path,
    template: &Value,
    execution_resolves_remote: bool,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<Option<MaterializedTemplateSources>, String> {
    if !execution_resolves_remote {
        return Ok(None);
    }
    let Some(bundle_ref) = template.get("bundle_ref").and_then(Value::as_str) else {
        return Ok(None);
    };
    let bundle_fetch_ref = inherited_bundle_fetch_ref(template, bundle_ref);
    let assistant_ref = template
        .get("assistant_template_ref")
        .and_then(Value::as_str)
        .unwrap_or("templates/assistant/basic-empty.json");
    let domain_ref = template
        .get("domain_template_ref")
        .and_then(Value::as_str)
        .unwrap_or(assistant_ref);
    let assistant_template_source =
        materialize_bundle_member(cwd, &bundle_fetch_ref, assistant_ref, fetcher)?;
    let domain_template_source =
        materialize_bundle_member(cwd, &bundle_fetch_ref, domain_ref, fetcher)?;
    Ok(Some(MaterializedTemplateSources {
        assistant_template_source: assistant_template_source.display().to_string(),
        domain_template_source: domain_template_source.display().to_string(),
    }))
}

fn inherited_bundle_fetch_ref(template: &Value, bundle_ref: &str) -> String {
    let Some(path) = bundle_ref.strip_prefix("oci://ghcr.io/greentic-biz/") else {
        return bundle_ref.to_owned();
    };
    let Some(provenance_ref) = template
        .get("provenance")
        .and_then(|value| value.get("source_ref"))
        .and_then(Value::as_str)
    else {
        return bundle_ref.to_owned();
    };
    let Some(tenant_and_path) = provenance_ref.strip_prefix("store://greentic-biz/") else {
        return bundle_ref.to_owned();
    };
    let Some((tenant, _catalog_path)) = tenant_and_path.split_once('/') else {
        return bundle_ref.to_owned();
    };
    if tenant.trim().is_empty() {
        return bundle_ref.to_owned();
    }
    format!("store://greentic-biz/{tenant}/{path}")
}

fn maybe_pin_template_refs(
    cwd: &Path,
    template: &mut Value,
    execution_resolves_remote: bool,
    fetcher: &dyn RemoteCatalogFetcher,
) -> Result<(), String> {
    if !execution_resolves_remote {
        return Ok(());
    }
    for key in ["assistant_template_ref", "domain_template_ref"] {
        if let Some(reference) = template
            .get(key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            && reference.contains(":latest")
        {
            let digest = fetcher.resolve_pack_ref(cwd, &reference)?;
            if let Some(object) = template.as_object_mut() {
                object.insert(
                    key.to_owned(),
                    Value::String(pin_reference(&reference, &digest)),
                );
            }
        }
    }
    Ok(())
}

fn build_bundle_answers(
    request: &CompositionRequest,
    locale: &str,
    template: &Value,
    template_sources: Option<&MaterializedTemplateSources>,
    provider_refs: &[String],
    overlay: Option<&Value>,
) -> WizardAnswerDocument {
    let assistant_template_source = template_sources
        .map(|item| item.assistant_template_source.as_str())
        .or_else(|| {
            template
                .get("assistant_template_ref")
                .and_then(Value::as_str)
        })
        .unwrap_or("templates/assistant/basic-empty.json");
    let domain_template_source = template_sources
        .map(|item| item.domain_template_source.as_str())
        .or_else(|| template.get("domain_template_ref").and_then(Value::as_str))
        .unwrap_or(assistant_template_source);
    let mut answers = serde_json::Map::from_iter([
        ("mode".to_owned(), Value::String(request.mode.clone())),
        (
            "bundle_name".to_owned(),
            Value::String(request.solution_name.clone()),
        ),
        (
            "bundle_id".to_owned(),
            Value::String(request.solution_id.clone()),
        ),
        (
            "output_dir".to_owned(),
            Value::String(request.output_dir.clone()),
        ),
        (
            "bundle_output_path".to_owned(),
            Value::String(request.bundle_output_path.clone()),
        ),
        (
            "assistant_template_source".to_owned(),
            Value::String(assistant_template_source.to_owned()),
        ),
        (
            "domain_template_source".to_owned(),
            Value::String(domain_template_source.to_owned()),
        ),
        (
            "provider_preset_refs".to_owned(),
            Value::Array(provider_refs.iter().cloned().map(Value::String).collect()),
        ),
        ("export_intent".to_owned(), Value::Bool(true)),
    ]);
    if let Some(overlay) = overlay {
        answers.insert("overlay".to_owned(), overlay.clone());
    }
    WizardAnswerDocument {
        wizard_id: BUNDLE_WIZARD_ID.to_owned(),
        schema_id: BUNDLE_SCHEMA_ID.to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        locale: locale.to_owned(),
        answers,
        locks: serde_json::Map::from_iter([(
            "execution".to_owned(),
            Value::String("execute".to_owned()),
        )]),
    }
}

fn render_readme(
    request: &CompositionRequest,
    solution_manifest: &SolutionManifest,
    bundle_plan: &BundlePlan,
) -> String {
    format!(
        "# {}\n\n{}\n\n## Generated Files\n\n- `{}`\n- `{}`\n- `{}`\n- `{}`\n- `{}`\n- `{}`\n\n## Delegated Bundle Generation\n\n`greentic-bundle wizard apply --answers {}`\n",
        solution_manifest.solution_name,
        if solution_manifest.description.is_empty() {
            "Generated by gx wizard.".to_owned()
        } else {
            solution_manifest.description.clone()
        },
        request.solution_manifest_path,
        request.bundle_plan_path,
        request.bundle_answers_path,
        request.setup_answers_path,
        request.readme_path,
        bundle_plan.bundle_output_path,
        request.bundle_answers_path
    )
}

fn write_json_file(
    path_root: &Path,
    relative_path: &str,
    value: &impl serde::Serialize,
) -> Result<(), String> {
    let path = resolve_output_path(path_root, relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize {}: {err}", path.display()))?;
    fs::write(&path, format!("{rendered}\n"))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn resolve_output_path(path_root: &Path, relative_path: &str) -> std::path::PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        path_root.join(path)
    }
}

fn builtin_value(entry_id: &str, display_name: &str, key: &str) -> Value {
    json!({
        "entry_id": entry_id,
        "kind": "provider-preset",
        "version": "1.0.0",
        "display_name": display_name,
        "provider_refs": [builtin_provider_ref(key).unwrap_or_default()]
    })
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wizard::catalog::ResolvedPackArtifact;
    use crate::{CatalogProvenance, WizardCatalogSet};
    use std::cell::RefCell;
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct StubFetcher {
        digests: RefCell<Vec<String>>,
    }

    impl RemoteCatalogFetcher for StubFetcher {
        fn fetch_json(
            &self,
            _cache_root: &Path,
            _reference: &str,
        ) -> Result<super::super::catalog::FetchResult, String> {
            Err("unused".to_owned())
        }

        fn resolve_pack_ref(&self, _cache_root: &Path, reference: &str) -> Result<String, String> {
            self.digests.borrow_mut().push(reference.to_owned());
            Ok("sha256:abc123".to_owned())
        }

        fn fetch_pack_artifact(
            &self,
            _cache_root: &Path,
            reference: &str,
        ) -> Result<ResolvedPackArtifact, String> {
            Ok(ResolvedPackArtifact {
                path: PathBuf::from(reference),
                resolved_digest: "sha256:abc123".to_owned(),
                media_type: "application/octet-stream".to_owned(),
            })
        }
    }

    #[test]
    fn provider_mapping_supports_all_of_the_above() {
        let request = CompositionRequest {
            mode: "create".to_owned(),
            template_mode: "basic_empty".to_owned(),
            template_entry_id: None,
            template_display_name: None,
            assistant_template_ref: None,
            domain_template_ref: None,
            solution_name: "Demo".to_owned(),
            solution_id: "demo".to_owned(),
            description: String::new(),
            output_dir: "dist".to_owned(),
            provider_selection: "all".to_owned(),
            provider_preset_entry_id: None,
            provider_preset_display_name: None,
            provider_refs: Vec::new(),
            overlay_entry_id: None,
            overlay_display_name: None,
            overlay_default_locale: None,
            overlay_tenant_id: None,
            catalog_oci_refs: Vec::new(),
            catalog_resolution_policy: "update_then_pin".to_owned(),
            bundle_output_path: "dist/demo.gtbundle".to_owned(),
            solution_manifest_path: "dist/demo.solution.json".to_owned(),
            bundle_plan_path: "dist/demo.bundle-plan.json".to_owned(),
            bundle_answers_path: "dist/demo.bundle.answers.json".to_owned(),
            setup_answers_path: "dist/demo.setup.answers.json".to_owned(),
            readme_path: "dist/demo.README.generated.md".to_owned(),
            existing_solution_path: None,
        };
        let presets = resolve_provider_presets(
            Path::new("."),
            &request,
            &WizardCatalogSet::default(),
            false,
            &StubFetcher {
                digests: RefCell::new(Vec::new()),
            },
        )
        .expect("presets");
        assert_eq!(presets.len(), 4);
    }

    #[test]
    fn template_resolution_uses_catalog_entry() {
        let request = CompositionRequest {
            mode: "create".to_owned(),
            template_mode: "catalog".to_owned(),
            template_entry_id: Some("assistant.network.phase1".to_owned()),
            template_display_name: None,
            assistant_template_ref: None,
            domain_template_ref: None,
            solution_name: "Demo".to_owned(),
            solution_id: "demo".to_owned(),
            description: String::new(),
            output_dir: "dist".to_owned(),
            provider_selection: "webchat".to_owned(),
            provider_preset_entry_id: None,
            provider_preset_display_name: None,
            provider_refs: Vec::new(),
            overlay_entry_id: None,
            overlay_display_name: None,
            overlay_default_locale: None,
            overlay_tenant_id: None,
            catalog_oci_refs: Vec::new(),
            catalog_resolution_policy: "update_then_pin".to_owned(),
            bundle_output_path: "dist/demo.gtbundle".to_owned(),
            solution_manifest_path: "dist/demo.solution.json".to_owned(),
            bundle_plan_path: "dist/demo.bundle-plan.json".to_owned(),
            bundle_answers_path: "dist/demo.bundle.answers.json".to_owned(),
            setup_answers_path: "dist/demo.setup.answers.json".to_owned(),
            readme_path: "dist/demo.README.generated.md".to_owned(),
            existing_solution_path: None,
        };
        let catalogs = WizardCatalogSet {
            templates: vec![crate::AssistantTemplateCatalogEntry {
                entry_id: "assistant.network.phase1".to_owned(),
                kind: "assistant-template".to_owned(),
                version: "1.0.0".to_owned(),
                display_name: "Network Assistant".to_owned(),
                description: "Network Assistant template".to_owned(),
                assistant_template_ref:
                    "oci://ghcr.io/greenticai/greentic-x/templates/assistant/network-phase1:latest"
                        .to_owned(),
                domain_template_ref: Some(
                    "oci://ghcr.io/greenticai/greentic-x/templates/domain/network-phase1:latest"
                        .to_owned(),
                ),
                bundle_ref: None,
                provenance: Some(CatalogProvenance {
                    source_type: "local".to_owned(),
                    source_ref: "catalog/templates/assistant.network.phase1.json".to_owned(),
                    resolved_digest: None,
                }),
            }],
            ..WizardCatalogSet::default()
        };
        let generated = generate_artifacts(
            Path::new("."),
            &request,
            &catalogs,
            "en",
            false,
            &StubFetcher {
                digests: RefCell::new(Vec::new()),
            },
        )
        .expect("artifacts");
        assert_eq!(
            generated.solution_manifest.template["entry_id"],
            "assistant.network.phase1"
        );
        assert_eq!(
            generated.bundle_answers.answers["assistant_template_source"],
            "oci://ghcr.io/greenticai/greentic-x/templates/assistant/network-phase1:latest"
        );
        assert_eq!(generated.bundle_answers.answers["export_intent"], true);
        assert_eq!(generated.bundle_answers.locks["execution"], "execute");
    }

    #[test]
    fn bundle_fetch_ref_inherits_store_tenant_from_catalog_provenance() {
        let template = json!({
            "entry_id": "zx.network.phase1",
            "assistant_template_ref": "assistant_templates/network-assistant.phase1.json",
            "domain_template_ref": "assistant_templates/network-assistant.phase1.json",
            "bundle_ref": "oci://ghcr.io/greentic-biz/zain-x-bundle:latest",
            "provenance": {
                "source_type": "store",
                "source_ref": "store://greentic-biz/3point/catalogs/zain-x/catalog.json:latest"
            }
        });

        assert_eq!(
            inherited_bundle_fetch_ref(
                &template,
                "oci://ghcr.io/greentic-biz/zain-x-bundle:latest"
            ),
            "store://greentic-biz/3point/zain-x-bundle:latest"
        );
    }

    #[test]
    fn writes_all_solution_artifacts() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let request = CompositionRequest {
            mode: "create".to_owned(),
            template_mode: "basic_empty".to_owned(),
            template_entry_id: None,
            template_display_name: None,
            assistant_template_ref: None,
            domain_template_ref: None,
            solution_name: "Network Assistant".to_owned(),
            solution_id: "network-assistant".to_owned(),
            description: "Automates network diagnostics".to_owned(),
            output_dir: "dist".to_owned(),
            provider_selection: "teams".to_owned(),
            provider_preset_entry_id: None,
            provider_preset_display_name: Some("Teams".to_owned()),
            provider_refs: Vec::new(),
            overlay_entry_id: None,
            overlay_display_name: None,
            overlay_default_locale: None,
            overlay_tenant_id: None,
            catalog_oci_refs: Vec::new(),
            catalog_resolution_policy: "update_then_pin".to_owned(),
            bundle_output_path: "dist/network-assistant.gtbundle".to_owned(),
            solution_manifest_path: "dist/network-assistant.solution.json".to_owned(),
            bundle_plan_path: "dist/network-assistant.bundle-plan.json".to_owned(),
            bundle_answers_path: "dist/network-assistant.bundle.answers.json".to_owned(),
            setup_answers_path: "dist/network-assistant.setup.answers.json".to_owned(),
            readme_path: "dist/network-assistant.README.generated.md".to_owned(),
            existing_solution_path: None,
        };
        let generated = generate_artifacts(
            temp.path(),
            &request,
            &WizardCatalogSet::default(),
            "en",
            false,
            &StubFetcher {
                digests: RefCell::new(Vec::new()),
            },
        )?;
        write_generated_artifacts(temp.path(), &request, &generated)?;
        assert!(
            temp.path()
                .join("dist/network-assistant.solution.json")
                .exists()
        );
        assert!(
            temp.path()
                .join("dist/network-assistant.bundle-plan.json")
                .exists()
        );
        assert!(
            temp.path()
                .join("dist/network-assistant.bundle.answers.json")
                .exists()
        );
        assert!(
            temp.path()
                .join("dist/network-assistant.setup.answers.json")
                .exists()
        );
        assert!(
            temp.path()
                .join("dist/network-assistant.README.generated.md")
                .exists()
        );
        Ok(())
    }
}
