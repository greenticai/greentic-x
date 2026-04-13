# PR-GX-TOOLCHAIN-06
Deprecate Legacy GX Packaging Paths

## Goal

Remove or formally deprecate the remaining old GX packaging assumptions so this
repo does not continue to drift into duplicate pack/bundle behavior.

## Deliverables

- Deprecation doc:
  - `docs/migration/gx-pack-bundle-deprecation.md`
- CLI/help/doc updates removing old phrasing that implies GX owns pack/bundle
  generation
- Test updates reflecting the new boundary
- Temporary compatibility warnings where useful

## Remove Or Deprecate

- Old docs that present GX as a bundle generator
- Any internal naming that implies pack/bundle ownership by GX
- Any future-facing plan text that asks GX to reimplement pack creation
- Any tests whose primary assertion is that GX is a packaging tool rather than a
  composition tool

## Keep

- composition outputs
- handoff artifacts
- launcher compatibility output
- optional compatibility bridge to `greentic-bundle` until `greentic-dev`
  integration exists

## Acceptance Criteria

- Repository docs are consistent about ownership.
- No active PR plan in `.codex/` asks GX to own pack or bundle generation.
- Contributors can tell clearly where GX stops and sibling tools begin.
