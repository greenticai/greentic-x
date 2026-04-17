use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use jsonschema::validator_for;
use serde_json::Value;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn workspace_root() -> PathBuf {
    repo_root().parent().expect("workspace root").to_path_buf()
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_greentic-x"))
}

fn run_cli(args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(binary_path())
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("failed to run greentic-x: {err}"))?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|err| format!("stdout was not utf-8: {err}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "greentic-x failed with status {}.\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ))
    }
}

fn load_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read json")).expect("parse json")
}

fn assert_matches_schema(schema_path: &Path, value_path: &Path) {
    let schema = load_json(schema_path);
    let validator = validator_for(&schema).expect("validator");
    let value = load_json(value_path);
    validator.validate(&value).expect("schema validation");
}

fn copy_fixture_answers(cwd: &Path) -> PathBuf {
    let path = cwd.join("answers.json");
    fs::copy(fixture_path("network-assistant.answers.json"), &path).expect("copy fixture");
    path
}

fn copy_fixture_named(cwd: &Path, fixture_name: &str) -> PathBuf {
    let path = cwd.join("answers.json");
    fs::copy(fixture_path(fixture_name), &path).expect("copy fixture");
    path
}

#[test]
fn fixture_answers_validate_and_run_emit_compatible_outputs() {
    let temp = TempDir::new().expect("tempdir");
    let cwd = temp.path();
    copy_fixture_answers(cwd);

    let validate_output =
        run_cli(&["wizard", "validate", "--answers", "answers.json"], cwd).expect("validate");
    let validate_plan: Value = serde_json::from_str(&validate_output).expect("validate plan");
    assert_eq!(validate_plan["requested_action"], "validate");
    assert_eq!(validate_plan["metadata"]["execution"], "dry_run");

    let run_output = run_cli(&["wizard", "run", "--answers", "answers.json"], cwd).expect("run");
    let run_plan: Value = serde_json::from_str(&run_output).expect("run plan");
    assert_eq!(run_plan["requested_action"], "run");
    assert_eq!(run_plan["metadata"]["execution"], "execute");

    let root = repo_root();
    assert_matches_schema(
        &root.join("schemas/solution-intent.schema.json"),
        &cwd.join("dist/network-assistant.solution.json"),
    );
    assert_matches_schema(
        &root.join("schemas/toolchain-handoff.schema.json"),
        &cwd.join("dist/network-assistant.toolchain-handoff.json"),
    );
    assert_matches_schema(
        &root.join("schemas/pack-input.schema.json"),
        &cwd.join("dist/network-assistant.pack.input.json"),
    );

    let bundle_answers = load_json(&cwd.join("dist/network-assistant.bundle.answers.json"));
    assert_eq!(bundle_answers["wizard_id"], "greentic-bundle.wizard.run");
    assert_eq!(
        bundle_answers["schema_id"],
        "greentic-bundle.wizard.answers"
    );

    let launcher_answers = load_json(&cwd.join("dist/network-assistant.launcher.answers.json"));
    assert_eq!(
        launcher_answers["wizard_id"],
        "greentic-dev.wizard.launcher.main"
    );
    assert_eq!(launcher_answers["schema_id"], "greentic-dev.launcher.main");
    assert_eq!(launcher_answers["answers"]["selected_action"], "bundle");

    let pack_input = load_json(&cwd.join("dist/network-assistant.pack.input.json"));
    assert_eq!(pack_input["schema_id"], "gx.pack.input");
    assert_eq!(pack_input["solution_id"], "network-assistant");
    assert!(pack_input["unresolved_downstream_work"].is_array());

    let gtc_setup_handoff = load_json(&cwd.join("dist/network-assistant.gtc.setup.handoff.json"));
    assert_eq!(
        gtc_setup_handoff["schema_id"],
        "gtc.extension.setup.handoff"
    );
    assert_eq!(
        gtc_setup_handoff["bundle_ref"],
        "dist/dist/network-assistant.gtbundle"
    );
    assert_eq!(
        gtc_setup_handoff["answers_path"],
        "dist/network-assistant.setup.answers.json"
    );

    let gtc_start_handoff = load_json(&cwd.join("dist/network-assistant.gtc.start.handoff.json"));
    assert_eq!(
        gtc_start_handoff["schema_id"],
        "gtc.extension.start.handoff"
    );
    assert_eq!(
        gtc_start_handoff["bundle_ref"],
        "dist/dist/network-assistant.gtbundle"
    );
}

#[test]
fn telco_catalog_fixture_emits_generic_gtc_handoffs() {
    let temp = TempDir::new().expect("tempdir");
    let cwd = temp.path();
    copy_fixture_named(cwd, "telco-network-assistant.answers.json");

    let telco_catalog = workspace_root().join("telco-x/catalog.json");
    let run_output = run_cli(
        &[
            "wizard",
            "run",
            "--answers",
            "answers.json",
            "--catalog",
            telco_catalog.to_str().expect("catalog path"),
        ],
        cwd,
    )
    .expect("run");
    let run_plan: Value = serde_json::from_str(&run_output).expect("run plan");
    assert_eq!(run_plan["requested_action"], "run");
    assert_eq!(run_plan["metadata"]["execution"], "execute");

    let solution = load_json(&cwd.join("dist/telco-network-assistant.solution.json"));
    assert_eq!(solution["solution_id"], "telco-network-assistant");
    assert_eq!(
        solution["template"]["entry_id"],
        "tx.assistant-template.network-assistant.phase1"
    );

    let toolchain_handoff =
        load_json(&cwd.join("dist/telco-network-assistant.toolchain-handoff.json"));
    assert_eq!(toolchain_handoff["gtc_handoff"]["tool"], "gtc");
    assert_eq!(
        toolchain_handoff["gtc_handoff"]["setup_handoff_path"],
        "dist/telco-network-assistant.gtc.setup.handoff.json"
    );
    assert_eq!(
        toolchain_handoff["gtc_handoff"]["start_handoff_path"],
        "dist/telco-network-assistant.gtc.start.handoff.json"
    );

    let gtc_setup_handoff =
        load_json(&cwd.join("dist/telco-network-assistant.gtc.setup.handoff.json"));
    assert_eq!(
        gtc_setup_handoff["schema_id"],
        "gtc.extension.setup.handoff"
    );
    assert_eq!(
        gtc_setup_handoff["bundle_ref"],
        "dist/dist/telco-network-assistant.gtbundle"
    );
    assert_eq!(
        gtc_setup_handoff["answers_path"],
        "dist/telco-network-assistant.setup.answers.json"
    );

    let gtc_start_handoff =
        load_json(&cwd.join("dist/telco-network-assistant.gtc.start.handoff.json"));
    assert_eq!(
        gtc_start_handoff["schema_id"],
        "gtc.extension.start.handoff"
    );
    assert_eq!(
        gtc_start_handoff["bundle_ref"],
        "dist/dist/telco-network-assistant.gtbundle"
    );
}

#[test]
fn optional_external_toolchain_replay_checks() {
    if std::env::var_os("GX_TEST_EXTERNAL_TOOLCHAIN").is_none() {
        return;
    }

    let temp = TempDir::new().expect("tempdir");
    let cwd = temp.path();
    copy_fixture_answers(cwd);
    run_cli(&["wizard", "run", "--answers", "answers.json"], cwd).expect("run");

    if command_available("greentic-bundle") {
        let status = Command::new("greentic-bundle")
            .args([
                "wizard",
                "apply",
                "--answers",
                "dist/network-assistant.bundle.answers.json",
            ])
            .current_dir(cwd)
            .status()
            .expect("run greentic-bundle");
        assert!(status.success(), "greentic-bundle replay failed: {status}");
    }

    if command_available("greentic-dev") {
        let output = Command::new("greentic-dev")
            .args(["wizard", "--schema"])
            .current_dir(cwd)
            .output()
            .expect("run greentic-dev");
        assert!(
            output.status.success(),
            "greentic-dev --schema failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let schema: Value = serde_json::from_slice(&output.stdout).expect("launcher schema");
        let validator = validator_for(&schema).expect("launcher schema validator");
        let launcher_answers = load_json(&cwd.join("dist/network-assistant.launcher.answers.json"));
        validator
            .validate(&launcher_answers)
            .expect("launcher answers accepted by greentic-dev schema");
    }
}

fn command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
