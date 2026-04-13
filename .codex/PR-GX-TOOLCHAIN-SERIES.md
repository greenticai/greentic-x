# GX Toolchain Realignment PR Series

This is the new implementation sequence for wizard/toolchain work in
`greentic-x`, based on the cross-repo audit.

## Why This Series Exists

The audit showed that:

- `gtc wizard` routes into `greentic-dev`
- `greentic-dev` owns the current launcher flow
- `greentic-pack` owns pack workflows
- `greentic-bundle` owns bundle workflows
- `gx` should stop evolving as a parallel packaging orchestrator

The correct direction is:

- GX owns solution composition and compatibility artifacts
- sibling Greentic tools own pack/bundle execution
- later integration should connect GX into `greentic-dev`, not duplicate it

## PR Order

1. `PR-GX-TOOLCHAIN-00.md`
   Architecture lock and audit docs
2. `PR-GX-TOOLCHAIN-01.md`
   Refactor GX into composition-only core
3. `PR-GX-TOOLCHAIN-02.md`
   Stable GX handoff contracts
4. `PR-GX-TOOLCHAIN-03.md`
   `greentic-dev` launcher compatibility
5. `PR-GX-TOOLCHAIN-04.md`
   Pack compatibility mapping without pack execution
6. `PR-GX-TOOLCHAIN-05.md`
   QA-spec alignment for GX interactive collection
7. `PR-GX-TOOLCHAIN-06.md`
   Deprecate legacy GX packaging paths
8. `PR-GX-TOOLCHAIN-07.md`
   End-to-end compatibility validation

## Non-Goals Of This Series

- No direct integration into `gtc` yet
- No reimplementation of `greentic-pack`
- No reimplementation of `greentic-bundle`
- No sibling-repo code changes from this repo

## Intended End State

- `gx` is a clean composition engine
- `gx` emits stable compatibility artifacts
- `greentic-dev` integration becomes straightforward later
- pack and bundle generation are fully reused from existing Greentic tools
