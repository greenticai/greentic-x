use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::{WizardAction, WizardExecutionMode, WizardNormalizedAnswers, WizardTemplateAnswers};

pub(crate) fn should_materialize_template(
    action: WizardAction,
    execution: WizardExecutionMode,
    normalized_answers: &WizardNormalizedAnswers,
) -> bool {
    matches!(normalized_answers, WizardNormalizedAnswers::Template(_))
        && matches!(action, WizardAction::Run | WizardAction::Apply)
        && matches!(execution, WizardExecutionMode::Execute)
}

pub(crate) fn materialize_template(
    cwd: &Path,
    template_answers: &WizardTemplateAnswers,
) -> Result<(), String> {
    let output_path = if Path::new(&template_answers.template_output_path).is_absolute() {
        Path::new(&template_answers.template_output_path).to_path_buf()
    } else {
        cwd.join(&template_answers.template_output_path)
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create template output dir {}: {err}",
                parent.display()
            )
        })?;
    }
    let payload = rendered_template_payload(template_answers);
    fs::write(
        &output_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&payload)
                .map_err(|err| format!("failed to serialize template payload: {err}"))?
        ),
    )
    .map_err(|err| {
        format!(
            "failed to write template output {}: {err}",
            output_path.display()
        )
    })
}

fn rendered_template_payload(template_answers: &WizardTemplateAnswers) -> Value {
    json!({
        "schema_version": "1.0.0",
        "template_kind": template_answers.template_kind,
        "template_action": template_answers.template_action,
        "template_source": template_answers.template_source,
        "generated_by": "gx.wizard",
    })
}
