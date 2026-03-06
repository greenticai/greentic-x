use greentic_x_contracts::ContractManifest;
use greentic_x_ops::OperationManifest;
use greentic_x_runtime::{
    CreateResourceRequest, InMemoryResourceStore, OperationHandler, RecordingEventSink, Runtime,
};
use greentic_x_types::{
    ActorRef, ContractId, Provenance, ResourceId, ResourcePatch, ResourceTypeId, TransitionRequest,
};
use serde_json::{Value, json};
use std::sync::Arc;

fn main() {
    let mut runtime = Runtime::new(
        InMemoryResourceStore::default(),
        RecordingEventSink::default(),
    );
    let provenance = provenance();

    let case_manifest = load_contract("../../contracts/case/contract.json");
    let playbook_manifest = load_contract("../../contracts/playbook/contract.json");
    let outcome_manifest = load_contract("../../contracts/outcome/contract.json");
    runtime
        .install_contract(case_manifest.clone(), provenance.clone())
        .expect("case contract install should succeed");
    runtime
        .install_contract(playbook_manifest.clone(), provenance.clone())
        .expect("playbook contract install should succeed");
    runtime
        .install_contract(outcome_manifest.clone(), provenance.clone())
        .expect("outcome contract install should succeed");
    runtime
        .activate_contract(
            &case_manifest.contract_id,
            &case_manifest.version,
            provenance.clone(),
        )
        .expect("case contract activation should succeed");
    runtime
        .activate_contract(
            &playbook_manifest.contract_id,
            &playbook_manifest.version,
            provenance.clone(),
        )
        .expect("playbook contract activation should succeed");
    runtime
        .activate_contract(
            &outcome_manifest.contract_id,
            &outcome_manifest.version,
            provenance.clone(),
        )
        .expect("outcome contract activation should succeed");

    let selector_manifest = load_operation("../../ops/playbook-select/op.json");
    runtime
        .install_operation(
            selector_manifest,
            Arc::new(PlaybookSelectHandler),
            provenance.clone(),
        )
        .expect("playbook selector install should succeed");

    runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.playbook")
                .expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("playbook")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("playbook-standard-triage")
                .expect("static resource id should be valid"),
            document: json!({
                "playbook_id": "playbook-standard-triage",
                "title": "Standard triage",
                "summary": "Default deterministic playbook"
            }),
            provenance: provenance.clone(),
        })
        .expect("playbook creation should succeed");

    let selection = runtime
        .invoke_operation(
            &greentic_x_types::OperationId::new("playbook-select")
                .expect("static op id should be valid"),
            "invoke-select-1",
            json!({"severity": "high", "signal_type": "ingress"}),
            provenance.clone(),
        )
        .expect("playbook selection should succeed");

    let run = runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.playbook")
                .expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("playbook-run")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("playbook-run-3")
                .expect("static resource id should be valid"),
            document: load_json("../../contracts/playbook/examples/playbook-run.created.json"),
            provenance: provenance.clone(),
        })
        .expect("playbook-run creation should succeed");

    let run = runtime
        .patch_resource(ResourcePatch {
            contract_id: run.contract_id.clone(),
            resource_type: run.resource_type.clone(),
            resource_id: run.resource_id.clone(),
            base_revision: run.revision,
            operations: vec![
                greentic_x_types::PatchOperation::replace(
                    "/selected_variant",
                    selection["route"].clone(),
                ),
                greentic_x_types::PatchOperation::replace("/status", json!("running")),
            ],
            provenance: provenance.clone(),
        })
        .expect("playbook-run patch should succeed");

    let run = runtime
        .transition_resource(
            TransitionRequest::new(
                run.contract_id.clone(),
                run.resource_type.clone(),
                run.resource_id.clone(),
                run.revision,
                "running",
                provenance.clone(),
            )
            .expect("transition request should be valid"),
        )
        .expect("playbook-run transition should succeed");

    let run = runtime
        .append_resource(
            greentic_x_types::AppendRequest::new(
                run.contract_id.clone(),
                run.resource_type.clone(),
                run.resource_id.clone(),
                run.revision,
                "step_results",
                json!({"step": "route", "status": "ok", "detail": "selected standard triage"}),
                provenance.clone(),
            )
            .expect("append request should be valid"),
        )
        .expect("step result append should succeed");

    let outcome = runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.outcome").expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("outcome")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("outcome-9").expect("static resource id should be valid"),
            document: load_json("../../contracts/outcome/examples/outcome.created.json"),
            provenance: provenance.clone(),
        })
        .expect("outcome creation should succeed");

    let outcome = runtime
        .patch_resource(ResourcePatch {
            contract_id: outcome.contract_id.clone(),
            resource_type: outcome.resource_type.clone(),
            resource_id: outcome.resource_id.clone(),
            base_revision: outcome.revision,
            operations: vec![greentic_x_types::PatchOperation::replace(
                "/decision",
                json!("executed"),
            )],
            provenance: provenance.clone(),
        })
        .expect("outcome patch should succeed");

    let outcome = runtime
        .transition_resource(
            TransitionRequest::new(
                outcome.contract_id,
                outcome.resource_type,
                outcome.resource_id,
                outcome.revision,
                "executed",
                provenance.clone(),
            )
            .expect("transition request should be valid"),
        )
        .expect("outcome transition should succeed");

    let run = runtime
        .transition_resource(
            TransitionRequest::new(
                run.contract_id,
                run.resource_type,
                run.resource_id,
                run.revision,
                "completed",
                provenance,
            )
            .expect("transition request should be valid"),
        )
        .expect("playbook-run completion should succeed");

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "selected_playbook": selection,
            "playbook_run": run.document,
            "outcome": outcome.document
        }))
        .expect("final output should serialize")
    );
}

struct PlaybookSelectHandler;

impl OperationHandler for PlaybookSelectHandler {
    fn invoke(&self, input: Value) -> Result<Value, String> {
        let severity = input
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("normal");
        let playbook_id = if severity == "high" {
            "playbook-standard-triage"
        } else {
            "playbook-lightweight-triage"
        };
        Ok(json!({
            "playbook_id": playbook_id,
            "route": "default"
        }))
    }
}

fn load_contract(path: &str) -> ContractManifest {
    serde_json::from_str(&read_repo_file(path)).expect("embedded contract manifest should parse")
}

fn load_operation(path: &str) -> OperationManifest {
    serde_json::from_str(&read_repo_file(path)).expect("embedded op manifest should parse")
}

fn load_json(path: &str) -> Value {
    serde_json::from_str(&read_repo_file(path)).expect("embedded json payload should parse")
}

fn provenance() -> Provenance {
    Provenance::new(
        ActorRef::service("simple-playbook-app").expect("static actor id should be valid"),
    )
}

fn read_repo_file(path: &str) -> String {
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("failed to read {}", full_path.display()))
}
