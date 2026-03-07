# Reference Examples

Greentic-X now includes four flow/package examples under `examples/` that prove
the intended GX authoring shapes.

## Profile-Driven Examples

- `examples/top-contributors-generic/`
  - pattern: resolve -> query -> query linked -> analyse -> present
  - source: `profile.json`
  - demonstrates: sequential multi-step analysis from the observability profile

- `examples/entity-utilisation-generic/`
  - pattern: resolve -> query -> threshold analysis -> present
  - source: `profile.json`
  - demonstrates: compact alert-style analysis with minimal boilerplate

- `examples/root-cause-split-join-generic/`
  - pattern: resolve -> split -> join -> present
  - source: `profile.json`
  - demonstrates: profile-driven split/join compilation with two generic branches

## Raw Flow Example

- `examples/change-correlation-generic/`
  - pattern: resolve -> query -> correlate -> present
  - source: `flow.json`
  - demonstrates: direct GX flow authoring without the profile layer

## Shared Package Shape

Each example package includes:

- `manifest.json`
- `flow.json`
- optional `profile.json`
- `input.json`
- `stubs.json`
- `expected.evidence.json`
- `expected.view.json`
- `README.md`

This keeps the examples directly runnable through `gx simulate` and directly
checkable in tests.
