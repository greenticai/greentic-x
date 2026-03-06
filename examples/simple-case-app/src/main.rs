use greentic_x_contracts::ContractManifest;
use greentic_x_runtime::{
    CreateResourceRequest, InMemoryResourceStore, RecordingEventSink, Runtime,
};
use greentic_x_types::{
    ActorRef, AppendRequest, ContractId, Provenance, ResourceId, ResourcePatch, ResourceTypeId,
    TransitionRequest,
};
use serde_json::{Value, json};

fn main() {
    let mut runtime = Runtime::new(
        InMemoryResourceStore::default(),
        RecordingEventSink::default(),
    );
    let case_manifest = load_contract("../../contracts/case/contract.json");
    let provenance = provenance();

    runtime
        .install_contract(case_manifest.clone(), provenance.clone())
        .expect("case contract install should succeed");
    runtime
        .activate_contract(
            &case_manifest.contract_id,
            &case_manifest.version,
            provenance.clone(),
        )
        .expect("case contract activation should succeed");

    let initial_case = load_json("../../contracts/case/examples/case.created.json");
    let created = runtime
        .create_resource(CreateResourceRequest {
            contract_id: ContractId::new("gx.case").expect("static contract id should be valid"),
            resource_type: ResourceTypeId::new("case")
                .expect("static resource type should be valid"),
            resource_id: ResourceId::new("case-42").expect("static resource id should be valid"),
            document: initial_case,
            provenance: provenance.clone(),
        })
        .expect("case creation should succeed");

    let patched = runtime
        .patch_resource(ResourcePatch {
            contract_id: created.contract_id.clone(),
            resource_type: created.resource_type.clone(),
            resource_id: created.resource_id.clone(),
            base_revision: created.revision,
            operations: vec![
                greentic_x_types::PatchOperation::replace(
                    "/summary",
                    json!("Ingress volume drop affects the primary path"),
                ),
                greentic_x_types::PatchOperation::replace("/owner", json!("case-coordinator")),
            ],
            provenance: provenance.clone(),
        })
        .expect("case patch should succeed");

    let appended = runtime
        .append_resource(
            AppendRequest::new(
                patched.contract_id.clone(),
                patched.resource_type.clone(),
                patched.resource_id.clone(),
                patched.revision,
                "evidence",
                json!({
                    "kind": "log",
                    "uri": "s3://demo/case-42/ingress-log.json",
                    "captured_by": "simple-case-app"
                }),
                provenance.clone(),
            )
            .expect("append request should be valid"),
        )
        .expect("case append should succeed");

    let transitioned = runtime
        .transition_resource(
            TransitionRequest::new(
                appended.contract_id,
                appended.resource_type,
                appended.resource_id,
                appended.revision,
                "triaged",
                provenance,
            )
            .expect("transition request should be valid"),
        )
        .expect("case transition should succeed");

    println!(
        "{}",
        serde_json::to_string_pretty(&transitioned.document)
            .expect("final case document should serialize")
    );
}

fn load_contract(path: &str) -> ContractManifest {
    serde_json::from_str(&read_repo_file(path)).expect("embedded contract manifest should parse")
}

fn load_json(path: &str) -> Value {
    serde_json::from_str(&read_repo_file(path)).expect("embedded json payload should parse")
}

fn provenance() -> Provenance {
    Provenance::new(ActorRef::service("simple-case-app").expect("static actor id should be valid"))
}

fn read_repo_file(path: &str) -> String {
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("failed to read {}", full_path.display()))
}
