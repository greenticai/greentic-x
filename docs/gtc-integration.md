# GX to GTC Integration

`gx` owns solution composition. `gtc` owns orchestration.

The boundary between them is contract-driven. `gx wizard` emits the GX-native
artifacts it already needs for pack, bundle, and launcher handoff, and also
emits generic `gtc` handoff documents that `gtc` can consume without knowing
GX-specific semantics.

## Emitted Artifacts

For a solution id like `network-assistant`, `gx wizard run` writes:

- `dist/network-assistant.solution.json`
- `dist/network-assistant.toolchain-handoff.json`
- `dist/network-assistant.launcher.answers.json`
- `dist/network-assistant.pack.input.json`
- `dist/network-assistant.bundle-plan.json`
- `dist/network-assistant.bundle.answers.json`
- `dist/network-assistant.setup.answers.json`
- `dist/network-assistant.gtc.setup.handoff.json`
- `dist/network-assistant.gtc.start.handoff.json`

When downstream `greentic-bundle` replay is used, the final bundle artifact is
still expected at:

- `dist/dist/network-assistant.gtbundle`

## Generic GTC Contracts

The `gtc`-facing artifacts use generic contracts:

- `gtc.extension.setup.handoff`
- `gtc.extension.start.handoff`

These documents intentionally stay small:

- setup handoff points `gtc setup` at the bundle ref and setup answers path
- start handoff points `gtc start` at the bundle ref

This keeps the ownership boundary explicit:

- `gx` owns solution composition
- `gtc` owns discovery and routing
- `setup` owns setup
- `start` owns readiness and launch

## Practical Flow

```bash
greentic-x wizard run --answers answers.json
gtc setup --extension-setup-handoff dist/network-assistant.gtc.setup.handoff.json
gtc start --extension-start-handoff dist/network-assistant.gtc.start.handoff.json
```

This lets GX-based extensions such as `telco-x` integrate into Greentic infra
through shared contracts instead of family-specific orchestration code.
