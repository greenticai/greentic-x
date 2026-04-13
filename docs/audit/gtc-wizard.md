# Audit: `gtc wizard`

## Current Responsibilities

- Exposes `wizard` as a top-level `gtc` subcommand.
- Detects locale and forwards it to the delegated tool.
- Routes `wizard` to `greentic-dev`, not to an internal wizard engine.
- Provides doctor/install visibility for the delegated binaries that the higher
  level toolchain depends on.

## Current Inputs

- CLI args after `gtc wizard`
- global `--locale`
- `--debug-router` and similar router/debug flags
- local environment and `PATH`

## Current Outputs

- delegated process execution of `greentic-dev wizard ...`
- passthrough stdout/stderr from the delegated process
- router/debug output when enabled

## Extension Points

- Very limited today.
- The practical extension point is indirect: change what `greentic-dev wizard`
  accepts or does.
- `gtc` itself currently adds routing and locale forwarding, not composition or
  packaging logic.

## Gaps Vs Desired GX / DW Model

- `gtc wizard` is not a true extension host yet.
- It does not dynamically discover GX or industry extensions.
- It cannot consume a GX composition contract directly today.
- Any future GX integration should target `greentic-dev` launcher compatibility
  first, then later plug that into `gtc`.

## Implication For `greentic-x`

- Do not design GX as if `gtc` is already the orchestrator contract.
- Treat `gtc` as the outer entrypoint and `greentic-dev` as the current host.
