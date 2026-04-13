# PR-GX-TOOLCHAIN-07
End-to-End Compatibility Validation

## Goal

Prove that GX can operate standalone as a composition engine while producing
artifacts that downstream Greentic tools can consume.

## Test Matrix

1. `answers.json -> gx wizard validate`
2. `answers.json -> gx wizard run`
3. emitted `solution.json` validates against schema
4. emitted `toolchain-handoff.json` validates against schema
5. emitted `bundle.answers.json` is replay-safe
6. emitted `launcher.answers.json` matches current `greentic-dev` launcher
   expectations
7. emitted `pack.input.json` is structurally valid for the documented GX pack
   compatibility contract

## Optional Environment-Gated Integration Tests

When sibling tools are available in `PATH`:

- replay `bundle.answers.json` through `greentic-bundle wizard apply --answers`
  as a deprecated compatibility bridge
- inspect or validate launcher compatibility against current `greentic-dev`
  schema output

These tests must be gated so the `greentic-x` repo remains independently
testable.

## Deliverables

- new integration tests under `crates/gx/tests/` or equivalent
- fixture answers under `tests/fixtures/` or `crates/gx/tests/fixtures/`
- documented test gating in repo docs

## Acceptance Criteria

- GX can be validated in isolation.
- Compatibility artifacts are covered by automated tests.
- External tool replay tests are optional but meaningful when enabled.
