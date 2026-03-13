use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::WizardAction;

pub(crate) fn resolve_wizard_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

pub(crate) fn run_bundle_handoff(cwd: &Path, answers_path: &Path) -> Result<(), String> {
    let invocation = bundle_handoff_invocation(answers_path);
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

pub(crate) fn bundle_handoff_invocation(answers_path: &Path) -> Vec<OsString> {
    vec![
        OsString::from("wizard"),
        OsString::from("apply"),
        OsString::from("--answers"),
        answers_path.as_os_str().to_os_string(),
    ]
}

pub(crate) fn default_handoff_answers_path(cwd: &Path, _action: WizardAction) -> PathBuf {
    cwd.join(".gx").join("wizard").join("bundle.answers.json")
}
