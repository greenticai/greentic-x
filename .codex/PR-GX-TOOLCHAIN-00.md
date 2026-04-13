# PR-GX-TOOLCHAIN-00
Audit Lock and Ownership Reset

## Goal

Lock the post-audit architecture for `greentic-x` before more wizard work lands.
This PR does not add new runtime behavior. It updates docs and implementation
direction so future GX work stops assuming ownership that belongs elsewhere.

## Core Findings To Lock

- `gtc wizard` is a passthrough into `greentic-dev wizard`; it is not the
  primary wizard orchestrator.
- `greentic-dev` currently owns the top-level launcher flow and delegates to
  either `greentic-pack` or `greentic-bundle`.
- `greentic-pack` owns pack scaffolding/build/sign/update/doctor workflows.
- `greentic-bundle` owns bundle answer contracts, normalization, setup, and
  `.gtbundle` creation.
- `greentic-qa-lib` is a reusable QA runtime and form engine, not the current
  top-level workflow owner.
- `gx` currently overlaps with `greentic-dev` by acting as its own composition
  wizard and directly invoking `greentic-bundle`.

## Required Decisions

1. `gx` will no longer be treated as a pack/bundle generator.
2. `gx` will become a solution composition engine that emits handoff artifacts.
3. `greentic-dev` remains the future top-level launcher integration point.
4. `greentic-pack` and `greentic-bundle` remain the only owners of pack and
   bundle generation logic.
5. Any new GX compatibility layer must target existing answer contracts instead
   of inventing new pack/bundle execution semantics in this repo.

## Deliverables

- Update `.codex/repo_overview.md` if needed for the new ownership boundary.
- Add `docs/audit/` docs capturing:
  - `gtc-wizard.md`
  - `greentic-dev.md`
  - `pack.md`
  - `bundle.md`
  - `qa-lib.md`
  - `cap.md`
  - `summary.md`
- Add this PR doc as the architecture lock for the new sequence.

## Non-Goals

- No runtime integration into `gtc` yet.
- No direct edits to sibling repos.
- No new GX wizard features yet.

## Acceptance Criteria

- The repo contains a written audit that matches current code reality.
- Future PRs refer to `greentic-dev` as launcher owner.
- Future PRs refer to `greentic-pack`/`greentic-bundle` as packaging owners.
- The old assumption that GX should own bundle/pack generation is explicitly
  retired in docs.
