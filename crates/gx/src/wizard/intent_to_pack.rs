use serde_json::Value;

use crate::{
    PackCapabilityMapping, PackInputDocument, PackProviderHint, PackTemplateSelection,
    ResolvedSolutionIntent,
};

const SCHEMA_VERSION: &str = "1.0.0";

pub(crate) fn map_solution_intent_to_pack_input(
    intent: &ResolvedSolutionIntent,
    solution_intent_ref: &str,
) -> PackInputDocument {
    let provider_hints = intent
        .provider_presets
        .iter()
        .map(|preset| PackProviderHint {
            entry_id: preset
                .get("entry_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            display_name: preset
                .get("display_name")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            provider_refs: preset
                .get("provider_refs")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    let provider_refs = provider_hints
        .iter()
        .flat_map(|hint| hint.provider_refs.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let template_selection = PackTemplateSelection {
        entry_id: intent
            .template
            .get("entry_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        display_name: intent
            .template
            .get("display_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        assistant_template_ref: intent
            .template
            .get("assistant_template_ref")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        domain_template_ref: intent
            .template
            .get("domain_template_ref")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    };

    let greentic_cap_mapping = intent
        .required_capabilities
        .iter()
        .map(|requirement| PackCapabilityMapping {
            gx_requirement: requirement.clone(),
            greentic_cap_concept: "required capability offer".to_owned(),
            status: "partial_compatibility_mapping".to_owned(),
        })
        .collect::<Vec<_>>();

    PackInputDocument {
        schema_id: "gx.pack.input".to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        solution_id: intent.solution_id.clone(),
        solution_intent_ref: solution_intent_ref.to_owned(),
        provider_refs,
        required_capability_offers: intent.required_capabilities.clone(),
        required_contracts: intent.required_contracts.clone(),
        suggested_flows: intent.suggested_flows.clone(),
        provider_hints,
        template_selection,
        defaults: intent.defaults.clone(),
        unresolved_downstream_work: vec![
            "Choose or scaffold the target pack root and pack identifiers.".to_owned(),
            "Translate pack input into greentic-pack wizard answers or pack.yaml state."
                .to_owned(),
            "Resolve capability offers and extension/component authoring details through greentic-pack."
                .to_owned(),
            "Run pack doctor, resolve, build, sign, and any manifest synchronization in greentic-pack."
                .to_owned(),
        ],
        greentic_cap_mapping,
        notes: vec![
            "This document is a compatibility input for greentic-pack, not a pack manifest."
                .to_owned(),
            "Capability requirements are carried as GX requirements mapped onto greentic-cap-style required capability offers."
                .to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_solution_intent_into_pack_input_document() {
        let intent = ResolvedSolutionIntent {
            schema_id: "gx.solution.intent".to_owned(),
            schema_version: "1.0.0".to_owned(),
            solution_id: "demo".to_owned(),
            solution_name: "Demo".to_owned(),
            description: String::new(),
            output_dir: "dist".to_owned(),
            solution_kind: "assistant".to_owned(),
            template: json!({
                "entry_id": "assistant.demo",
                "display_name": "Demo Template",
                "assistant_template_ref": "oci://example/template:latest"
            }),
            provider_presets: vec![json!({
                "entry_id": "builtin.teams",
                "display_name": "Teams",
                "provider_refs": ["oci://example/provider:latest"]
            })],
            overlay: None,
            catalog_refs: Vec::new(),
            catalog_sources: Vec::new(),
            required_capabilities: vec!["messaging.send".to_owned()],
            required_contracts: vec!["gx.customer.case".to_owned()],
            suggested_flows: vec!["customer.triage".to_owned()],
            defaults: json!({"provider_selection": "teams"}),
            notes: Vec::new(),
        };

        let pack_input = map_solution_intent_to_pack_input(&intent, "dist/demo.solution.json");

        assert_eq!(pack_input.schema_id, "gx.pack.input");
        assert_eq!(
            pack_input.provider_refs,
            vec!["oci://example/provider:latest".to_owned()]
        );
        assert_eq!(
            pack_input.required_capability_offers,
            vec!["messaging.send".to_owned()]
        );
        assert_eq!(
            pack_input.required_contracts,
            vec!["gx.customer.case".to_owned()]
        );
        assert_eq!(
            pack_input.suggested_flows,
            vec!["customer.triage".to_owned()]
        );
        assert_eq!(
            pack_input.template_selection.entry_id.as_deref(),
            Some("assistant.demo")
        );
        assert_eq!(pack_input.greentic_cap_mapping.len(), 1);
        assert!(!pack_input.unresolved_downstream_work.is_empty());
    }
}
