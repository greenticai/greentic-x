# Audit: `greentic-bundle`

## Current Responsibilities

- Owns the bundle `AnswerDocument` contract.
- Owns request normalization for bundle/setup flows.
- Uses `greentic-qa-lib` for interactive request and setup forms.
- Resolves catalogs and references needed for bundle authoring.
- Writes bundle workspace/state outputs.
- Produces `.gtbundle` artifacts.

## Current Inputs

- bundle `AnswerDocument`
- interactive request/setup answers
- app pack entries
- extension provider entries
- setup answers and setup specs
- remote catalog references
- execution intent encoded in answer locks

## Current Outputs

- normalized bundle request state
- resolved files under bundle output roots
- generated setup outputs
- final `.gtbundle`
- emitted bundle answer documents for replay

## Extension Points

- catalog resolution
- provider registries
- setup specs and setup answers
- replayable answer documents

## Gaps Vs Desired GX / DW Model

- `greentic-bundle` already owns bundle execution semantics.
- GX should not own or fork this lifecycle.
- The missing piece is a stable compatibility layer from GX composition outputs
  into bundle-consumable artifacts and, optionally, `greentic-dev` launcher
  envelopes.

## Implication For `greentic-x`

- GX should treat bundle generation as downstream execution.
- GX may keep a compatibility bridge that emits bundle answers.
- GX should not add new bundle runtime behavior in this repo.
