# Audit: `greentic-pack`

## Current Responsibilities

- Owns pack-oriented wizard flows and side effects.
- Scaffolds new packs.
- Replays delegated flow and component wizard answers.
- Updates `pack.yaml` and syncs pack manifest state.
- Runs doctor, resolve, build, and sign steps.
- Maintains extension and capability-related authoring helpers.
- Uses `greentic-qa-lib` for interactive QA-driven parts of the wizard.

## Current Inputs

- pack `AnswerDocument`
- interactive menu selections
- nested flow/component wizard answers
- pack root path and pack identifiers
- extension/catalog selections
- optional sign key path

## Current Outputs

- pack directory scaffolds
- updated `pack.yaml`
- generated or updated pack assets
- resolved pack lock data
- built `.gtpack` outputs
- signatures when signing is requested

## Extension Points

- Extension catalogs and capability offer editing paths
- nested delegation to `greentic-flow` and `greentic-component`
- answer replay and schema emission

## Gaps Vs Desired GX / DW Model

- `greentic-pack` is already a workflow owner, not just a file-format tool.
- There is no stable GX-to-pack compatibility contract yet.
- A GX integration path should not duplicate any of:
  - pack scaffolding
  - pack update
  - pack resolve/build/sign
- Capability modelling exists, but the integration boundary between GX solution
  intent and pack inputs is not formalized.

## Implication For `greentic-x`

- GX must not reimplement pack generation here.
- GX should emit pack-oriented compatibility artifacts only.
- Any later integration should let `greentic-pack` keep full control over pack
  execution.
