# Toolchain Audit Summary

## Current Ownership

| Concern | Current owner |
| --- | --- |
| Outer CLI entrypoint | `gtc` |
| Wizard launcher/orchestration | `greentic-dev` |
| QA runtime and forms | `greentic-qa-lib` / `qa-spec` |
| Pack workflows | `greentic-pack` |
| Bundle workflows | `greentic-bundle` |
| Capability schemas and resolution types | `greentic-cap` |
| GX solution composition | `greentic-x` |

## Where Wizard Orchestration Really Lives

- `gtc wizard` routes into `greentic-dev wizard`.
- `greentic-dev` owns the current launcher contract and chooses between pack and
  bundle delegated flows.
- `greentic-pack` and `greentic-bundle` each own their own answer contracts and
  execution semantics.
- `greentic-x` currently runs a parallel composition wizard rather than
  participating in the `greentic-dev` launcher model.

## How Answers Are Passed Between Steps

- `greentic-dev` launcher answers use a launcher envelope.
- The launcher envelope records `selected_action`.
- Delegated pack or bundle answer documents are nested under
  `delegate_answer_document`.
- `greentic-pack` and `greentic-bundle` each use their own native
  `AnswerDocument` with:
  - `wizard_id`
  - `schema_id`
  - `schema_version`
  - `locale`
  - `answers`
  - `locks`

## How Pack And Bundle Are Invoked Today

- `gtc wizard` invokes `greentic-dev wizard`.
- `greentic-dev wizard` plans and runs either:
  - `greentic-pack wizard ...`
  - `greentic-bundle wizard ...`
- `gx wizard` currently bypasses `greentic-dev` and can directly call
  `greentic-bundle wizard apply --answers ...`

## What Is Reusable

- `greentic-qa-lib` / `qa-spec` for form-driven question flows
- `greentic-dev` launcher envelope as the near-term host contract
- `greentic-pack` and `greentic-bundle` answer-document styles
- `greentic-distributor-client`-based catalog/reference resolution
- `greentic-cap` capability concepts and bundle/setup artifact types

## What Is Currently Duplicated

- Top-level wizard orchestration between `greentic-dev` and `gx`
- Interactive prompting patterns between bespoke GX prompting and QA-spec-driven
  flows in sibling tools
- Toolchain boundary assumptions in GX docs that still imply packaging ownership

## Required Post-Audit Direction

1. GX should become composition-only.
2. GX should emit stable compatibility and handoff artifacts.
3. GX should not reimplement pack generation.
4. GX should not reimplement bundle generation.
5. GX should target `greentic-dev` launcher compatibility for future
   integration.
6. GX should reuse `greentic-qa-lib` for interactive collection where practical.

## Gaps To Close In Later PRs

- formal GX handoff schemas
- `greentic-dev` launcher-compatible output from GX
- pack-oriented compatibility output from GX without pack execution
- replacement of bespoke GX prompting with QA-spec-driven forms
- deprecation of stale GX docs that imply package-generation ownership
