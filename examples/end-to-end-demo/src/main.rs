use greentic_x_contracts::ContractManifest;
use greentic_x_ops::OperationManifest;
use greentic_x_runtime::{
    CreateResourceRequest, InMemoryResourceStore, OperationHandler, RecordingEventSink, Runtime,
};
use greentic_x_types::{
    ActorRef, AppendRequest, ContractId, OperationId, Provenance, ResourceId, ResourcePatch,
    ResourceTypeId, TransitionRequest,
};
use serde_json::{Value, json};
use std::sync::Arc;

fn main() {
    let mut runtime = Runtime::new(
        InMemoryResourceStore::default(),
        RecordingEventSink::default(),
    );
    let provenance = provenance();

    for path in [
        "../../contracts/case/contract.json",
        "../../contracts/evidence/contract.json",
        "../../contracts/outcome/contract.json",
        "../../contracts/playbook/contract.json",
    ] {
        let manifest = load_contract(path);
        runtime
            .install_contract(manifest.clone(), provenance.clone())
            .expect("contract install should succeed");
        runtime
            .activate_contract(&manifest.contract_id, &manifest.version, provenance.clone())
            .expect("contract activation should succeed");
    }

    runtime
        .install_operation(
            load_operation("../../ops/playbook-select/op.json"),
            Arc::new(PlaybookSelectHandler),
            provenance.clone(),
        )
        .expect("playbook-select install should succeed");
    runtime
        .install_operation(
            load_operation("../../ops/rca-basic/op.json"),
            Arc::new(RcaBasicHandler),
            provenance.clone(),
        )
        .expect("rca-basic install should succeed");
    runtime
        .install_operation(
            load_operation("../../ops/approval-basic/op.json"),
            Arc::new(ApprovalBasicHandler),
            provenance.clone(),
        )
        .expect("approval-basic install should succeed");

    let case = runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.case").expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("case")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("case-42").expect("static resource id should be valid"),
            document: load_json("../../contracts/case/examples/case.created.json"),
            provenance: provenance.clone(),
        })
        .expect("case creation should succeed");

    let selection = runtime
        .invoke_operation(
            &OperationId::new("playbook-select").expect("static op id should be valid"),
            "invoke-select-1",
            json!({"severity": "high", "signal_type": "ingress"}),
            provenance.clone(),
        )
        .expect("playbook select should succeed");

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
        .expect("playbook run creation should succeed");
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
        .expect("playbook run patch should succeed");
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
        .expect("playbook run start should succeed");

    let evidence = runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.evidence")
                .expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("evidence")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("evidence-17")
                .expect("static resource id should be valid"),
            document: load_json("../../contracts/evidence/examples/evidence.created.json"),
            provenance: provenance.clone(),
        })
        .expect("evidence creation should succeed");
    let evidence = runtime
        .append_resource(
            AppendRequest::new(
                evidence.contract_id.clone(),
                evidence.resource_type.clone(),
                evidence.resource_id.clone(),
                evidence.revision,
                "observations",
                json!({"observed_at": "2026-03-06T12:00:00Z", "detail": "ingress drop confirmed", "source": "checker-a"}),
                provenance.clone(),
            )
            .expect("append request should be valid"),
        )
        .expect("evidence append should succeed");

    let case = runtime
        .append_resource(
            AppendRequest::new(
                case.contract_id.clone(),
                case.resource_type.clone(),
                case.resource_id.clone(),
                case.revision,
                "evidence",
                json!({"kind": "evidence-link", "uri": format!("local://{}", evidence.resource_id.as_str()), "captured_by": "end-to-end-demo"}),
                provenance.clone(),
            )
            .expect("append request should be valid"),
        )
        .expect("case evidence append should succeed");

    let rca = runtime
        .invoke_operation(
            &OperationId::new("rca-basic").expect("static op id should be valid"),
            "invoke-rca-1",
            json!({"signals": ["ingress_drop"], "evidence_count": 1}),
            provenance.clone(),
        )
        .expect("rca operation should succeed");

    let outcome = runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.outcome").expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("outcome")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("outcome-9").expect("static resource id should be valid"),
            document: json!({
                "outcome_id": "outcome-9",
                "summary": rca["summary"].clone(),
                "decision": "proposed",
                "state": "proposed"
            }),
            provenance: provenance.clone(),
        })
        .expect("outcome creation should succeed");

    let approval = runtime
        .invoke_operation(
            &OperationId::new("approval-basic").expect("static op id should be valid"),
            "invoke-approval-1",
            json!({"risk_score": 0.2, "requested_action": "reroute"}),
            provenance.clone(),
        )
        .expect("approval operation should succeed");

    let outcome = runtime
        .patch_resource(ResourcePatch {
            contract_id: outcome.contract_id.clone(),
            resource_type: outcome.resource_type.clone(),
            resource_id: outcome.resource_id.clone(),
            base_revision: outcome.revision,
            operations: vec![greentic_x_types::PatchOperation::replace(
                "/decision",
                Value::String(
                    if approval["approved"].as_bool() == Some(true) {
                        "approved"
                    } else {
                        "manual-review"
                    }
                    .to_owned(),
                ),
            )],
            provenance: provenance.clone(),
        })
        .expect("outcome patch should succeed");
    let outcome = runtime
        .transition_resource(
            TransitionRequest::new(
                outcome.contract_id.clone(),
                outcome.resource_type.clone(),
                outcome.resource_id.clone(),
                outcome.revision,
                "approved",
                provenance.clone(),
            )
            .expect("transition request should be valid"),
        )
        .expect("outcome approval should succeed");
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
        .expect("outcome execution should succeed");

    let run = runtime
        .append_resource(
            AppendRequest::new(
                run.contract_id.clone(),
                run.resource_type.clone(),
                run.resource_id.clone(),
                run.revision,
                "step_results",
                json!({"step": "approval", "status": "ok", "detail": "approval completed"}),
                provenance.clone(),
            )
            .expect("append request should be valid"),
        )
        .expect("step result append should succeed");
    let run = runtime
        .transition_resource(
            TransitionRequest::new(
                run.contract_id,
                run.resource_type,
                run.resource_id,
                run.revision,
                "completed",
                provenance.clone(),
            )
            .expect("transition request should be valid"),
        )
        .expect("playbook run completion should succeed");

    let case = runtime
        .transition_resource(
            TransitionRequest::new(
                case.contract_id,
                case.resource_type,
                case.resource_id,
                case.revision,
                "triaged",
                provenance.clone(),
            )
            .expect("transition request should be valid"),
        )
        .expect("case triage should succeed");
    let case = runtime
        .transition_resource(
            TransitionRequest::new(
                case.contract_id,
                case.resource_type,
                case.resource_id,
                case.revision,
                "investigating",
                provenance.clone(),
            )
            .expect("transition request should be valid"),
        )
        .expect("case investigation should succeed");
    let case = runtime
        .transition_resource(
            TransitionRequest::new(
                case.contract_id,
                case.resource_type,
                case.resource_id,
                case.revision,
                "resolved",
                provenance,
            )
            .expect("transition request should be valid"),
        )
        .expect("case resolution should succeed");

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "selected_playbook": selection,
            "evidence": evidence.document,
            "rca": rca,
            "outcome": outcome.document,
            "playbook_run": run.document,
            "case": case.document
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
        Ok(json!({"playbook_id": playbook_id, "route": "default"}))
    }
}

struct RcaBasicHandler;

impl OperationHandler for RcaBasicHandler {
    fn invoke(&self, input: Value) -> Result<Value, String> {
        let evidence_count = input
            .get("evidence_count")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let confidence = if evidence_count > 0 { "medium" } else { "low" };
        Ok(json!({
            "summary": "Ingress capacity degradation is the primary hypothesis",
            "confidence": confidence
        }))
    }
}

struct ApprovalBasicHandler;

impl OperationHandler for ApprovalBasicHandler {
    fn invoke(&self, input: Value) -> Result<Value, String> {
        let risk_score = input
            .get("risk_score")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let approved = risk_score <= 0.5;
        Ok(json!({
            "approved": approved,
            "reason": if approved { "risk score within threshold" } else { "requires manual review" }
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
    Provenance::new(ActorRef::service("end-to-end-demo").expect("static actor id should be valid"))
}

fn read_repo_file(path: &str) -> String {
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("failed to read {}", full_path.display()))
}
