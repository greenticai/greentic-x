# Telco to GTC E2E Smoke

This is a repeatable local smoke flow for the shared extension boundary:

- `telco-x` provides the catalog
- `greentic-x` composes the solution and emits generic `gtc` handoff artifacts
- `greentic-bundle` builds the downstream bundle
- `gtc` consumes the generic setup/start handoff contracts

The goal is to verify the contract boundary, not to test Telco business logic.

## Prerequisites

- built local `greentic-x` binary
- built local `gtc` binary
- `greentic-bundle`, `greentic-setup`, and `greentic-start` available in `PATH`
- local repo checkout containing:
  - `greentic-x`
  - `telco-x`
  - `greentic`

Useful checks:

```bash
which greentic-bundle
which greentic-setup
which greentic-start
ls /home/vgrishkyan/greentic/greentic-x/target/debug/greentic-x
ls /home/vgrishkyan/greentic/greentic/target/debug/gtc
```

## 1. Prepare a temp workspace

```bash
mkdir -p /tmp/telco-gtc-e2e
cp /home/vgrishkyan/greentic/greentic-x/crates/gx/tests/fixtures/telco-network-assistant.answers.json /tmp/telco-gtc-e2e/answers.json
cd /home/vgrishkyan/greentic
```

## 2. Compose the Telco solution through GX

```bash
/home/vgrishkyan/greentic/greentic-x/target/debug/greentic-x wizard run \
  --answers /tmp/telco-gtc-e2e/answers.json \
  --catalog /home/vgrishkyan/greentic/telco-x/catalog.json
```

Expected generated files:

- `dist/telco-network-assistant.solution.json`
- `dist/telco-network-assistant.toolchain-handoff.json`
- `dist/telco-network-assistant.bundle.answers.json`
- `dist/telco-network-assistant.setup.answers.json`
- `dist/telco-network-assistant.gtc.setup.handoff.json`
- `dist/telco-network-assistant.gtc.start.handoff.json`

## 3. Build the downstream bundle

```bash
greentic-bundle wizard apply --answers dist/telco-network-assistant.bundle.answers.json
```

Important:

- the downstream bundle artifact is currently expected at `dist/dist/telco-network-assistant.gtbundle`
- this `dist/dist` shape is part of the current bundle toolchain contract

## 4. Run GTC setup via the generic handoff

```bash
/home/vgrishkyan/greentic/greentic/target/debug/gtc setup \
  --extension-setup-handoff dist/telco-network-assistant.gtc.setup.handoff.json \
  --dry-run
```

Expected result:

- bundle is extracted
- setup answers are loaded
- local setup UI starts

## 5. Run GTC start via the generic handoff

```bash
/home/vgrishkyan/greentic/greentic/target/debug/gtc start \
  --extension-start-handoff dist/telco-network-assistant.gtc.start.handoff.json
```

Expected result:

- `gtc` resolves the extracted bundle
- `greentic-start` launches the local runtime
- output shows local HTTP endpoint and ready state

## 6. Stop the runtime

If the foreground session is still attached, stop it with `Ctrl+C`.

If needed, terminate the temporary processes manually:

```bash
ps -ef | rg 'greentic-start|gtc start'
kill <gtc-pid> <greentic-start-pid>
```

## What This Smoke Confirms

- `telco-x` works as a real catalog-driven extension input
- `greentic-x` emits generic `gtc` setup/start handoff artifacts
- `gtc` consumes the shared contracts without knowing Telco-specific semantics
- the same contract shape can later be reused by `greentic-dw`
