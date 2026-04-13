# Network Assistant

A network support assistant

## GX Outputs

- `dist/network-assistant.solution.json`
- `dist/network-assistant.toolchain-handoff.json`
- `dist/network-assistant.launcher.answers.json`
- `dist/network-assistant.pack.input.json`
- `dist/network-assistant.bundle-plan.json`
- `dist/network-assistant.bundle.answers.json`
- `dist/network-assistant.setup.answers.json`
- `dist/network-assistant.README.generated.md`

## Downstream Toolchain Handoff

- pack compatibility input: `dist/network-assistant.pack.input.json`
- expected downstream bundle output: `dist/dist/network-assistant.gtbundle`
- direct bundle handoff command: `greentic-bundle wizard apply --answers dist/network-assistant.bundle.answers.json`
- launcher compatibility file: `dist/network-assistant.launcher.answers.json`
- launcher target: `greentic-dev.wizard.launcher.main` / `greentic-dev.launcher.main`
