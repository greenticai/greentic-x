use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::{WizardAction, WizardAnswerDocument};

use super::wizard_action_name;

pub(crate) fn write_wizard_answers_at(
    path: &Path,
    document: &WizardAnswerDocument,
) -> Result<(), String> {
    let resolved = path.to_path_buf();
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create answers parent directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let rendered = serde_json::to_string_pretty(document)
        .map_err(|err| format!("failed to serialize answer document: {err}"))?;
    fs::write(&resolved, format!("{rendered}\n")).map_err(|err| {
        format!(
            "failed to write answer document {}: {err}",
            resolved.display()
        )
    })
}

pub(crate) fn resolve_wizard_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

pub(crate) fn default_handoff_answers_path(cwd: &Path, action: WizardAction) -> PathBuf {
    cwd.join(".gx")
        .join("wizard")
        .join(format!("{}.answers.json", wizard_action_name(action)))
}

pub(crate) fn run_bundle_handoff(
    cwd: &Path,
    action: WizardAction,
    answers_path: &Path,
) -> Result<(), String> {
    let invocation = bundle_handoff_invocation(action, answers_path);
    let mut command = ProcessCommand::new("greentic-bundle");
    command.current_dir(cwd);
    for arg in &invocation {
        command.arg(arg);
    }
    let status = command.status().map_err(|err| {
        format!(
            "failed to run greentic-bundle wizard handoff (greentic-bundle {}): {err}",
            invocation
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        )
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "greentic-bundle wizard handoff failed for {} with status {}",
            answers_path.display(),
            status
        ))
    }
}

pub(crate) fn bundle_handoff_invocation(
    action: WizardAction,
    answers_path: &Path,
) -> Vec<OsString> {
    vec![
        OsString::from("wizard"),
        OsString::from(wizard_action_name(action)),
        OsString::from("--answers"),
        answers_path.as_os_str().to_os_string(),
    ]
}
