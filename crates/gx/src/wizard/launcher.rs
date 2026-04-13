use serde_json::Value;

use crate::WizardAnswerDocument;

pub(crate) const GREENTIC_DEV_LAUNCHER_WIZARD_ID: &str = "greentic-dev.wizard.launcher.main";
pub(crate) const GREENTIC_DEV_LAUNCHER_SCHEMA_ID: &str = "greentic-dev.launcher.main";
pub(crate) const GREENTIC_DEV_LAUNCHER_SELECTED_ACTION_BUNDLE: &str = "bundle";

pub(crate) fn build_bundle_launcher_document(
    locale: &str,
    schema_version: &str,
    delegated_answer_document: &WizardAnswerDocument,
) -> Result<WizardAnswerDocument, String> {
    let delegated = serde_json::to_value(delegated_answer_document)
        .map_err(|err| format!("failed to encode delegated answer document: {err}"))?;
    let answers = serde_json::Map::from_iter([
        (
            "selected_action".to_owned(),
            Value::String(GREENTIC_DEV_LAUNCHER_SELECTED_ACTION_BUNDLE.to_owned()),
        ),
        ("delegate_answer_document".to_owned(), delegated),
    ]);
    Ok(WizardAnswerDocument {
        wizard_id: GREENTIC_DEV_LAUNCHER_WIZARD_ID.to_owned(),
        schema_id: GREENTIC_DEV_LAUNCHER_SCHEMA_ID.to_owned(),
        schema_version: schema_version.to_owned(),
        locale: locale.to_owned(),
        answers,
        locks: serde_json::Map::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_greentic_dev_launcher_document_for_bundle_delegate() {
        let delegated = WizardAnswerDocument {
            wizard_id: "greentic-bundle.wizard.run".to_owned(),
            schema_id: "greentic-bundle.wizard.answers".to_owned(),
            schema_version: "1.0.0".to_owned(),
            locale: "en".to_owned(),
            answers: serde_json::Map::from_iter([(
                "solution_id".to_owned(),
                Value::String("demo".to_owned()),
            )]),
            locks: serde_json::Map::new(),
        };

        let launcher = build_bundle_launcher_document("en", "1.0.0", &delegated).expect("launcher");

        assert_eq!(launcher.wizard_id, GREENTIC_DEV_LAUNCHER_WIZARD_ID);
        assert_eq!(launcher.schema_id, GREENTIC_DEV_LAUNCHER_SCHEMA_ID);
        assert_eq!(
            launcher.answers["selected_action"],
            GREENTIC_DEV_LAUNCHER_SELECTED_ACTION_BUNDLE
        );
        assert_eq!(
            launcher.answers["delegate_answer_document"],
            json!({
                "wizard_id": "greentic-bundle.wizard.run",
                "schema_id": "greentic-bundle.wizard.answers",
                "schema_version": "1.0.0",
                "locale": "en",
                "answers": {
                    "solution_id": "demo"
                },
                "locks": {}
            })
        );
    }
}
