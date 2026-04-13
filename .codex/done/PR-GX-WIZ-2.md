PR Update — Simplify gx wizard UX and provider presets

Summary

This PR update simplifies the `gx wizard` interaction model and aligns it with
the intended architecture:

- `gx wizard` becomes a composition wizard only
- it collects solution choices
- resolves them against GX catalogs
- emits composition artifacts
- automatically delegates bundling to `greentic-bundle`

The wizard must now:

- use clear channel choices
- use OCI provider presets
- remove unnecessary intermediate screens
- generate outputs automatically
- return to the main menu loop after completion
- support update mode with pre-filled values

The internal architecture proposed earlier remains valid:

- `WizardAnswerDocument`
- `CompositionRequest`
- `SolutionManifest`
- `BundlePlan`
- `BundleAnswers`
- `SetupAnswers`

## Provider Preset Naming Change

Provider presets must use OCI pack references, not abstract preset names.

Example format:

- `ghcr.io/greenticai/packs/messaging/messaging-teams:latest`
- `ghcr.io/greenticai/packs/messaging/messaging-webchat:latest`
- `ghcr.io/greenticai/packs/messaging/messaging-webex:latest`
- `ghcr.io/greenticai/packs/messaging/messaging-slack:latest`

The wizard UI should show friendly names, but internally map to these OCI
references.

Mapping example:

- `Webchat` -> `ghcr.io/greenticai/packs/messaging/messaging-webchat:latest`
- `Teams` -> `ghcr.io/greenticai/packs/messaging/messaging-teams:latest`
- `WebEx` -> `ghcr.io/greenticai/packs/messaging/messaging-webex:latest`
- `Slack` -> `ghcr.io/greenticai/packs/messaging/messaging-slack:latest`
- `All of the above` -> list of all provider presets
- `Other catalog preset` -> catalog provider preset selection
- `Advanced manual provider override` -> manual OCI ref entry

## Catalog Resolution Model

Catalog entries may originate from:

- checked-in catalog JSON
- remote OCI catalog JSON

OCI catalogs must be fetched using `greentic-distributor-client`.

Example catalog source:

- `oci://ghcr.io/greenticai/catalogs/zain-x/catalog.json:latest`

Resolution flow:

```text
gx wizard
    -> load local catalogs
    -> fetch remote catalogs via greentic-distributor-client
    -> merge catalogs
    -> present unified selection list
```

Catalog entries should retain provenance metadata:

- `source_type: local | oci`
- `source_ref: <ref>`
- `resolved_digest: <digest if pinned>`

Default resolution policy:

- `update_then_pin`

Meaning:

- check if newer compatible version exists
- update to newest suitable version
- emit pinned references in generated artifacts

## Wizard UX Redesign

### Remove Screens

The following screens are removed entirely:

- Screen 5 - Review outputs
- Screen 6 - Bundle generation
- Screen 7 - Final review

Outputs will always be generated automatically.

### Generated Outputs

After composition, the wizard always generates:

- `dist/<solution-id>.solution.json`
- `dist/<solution-id>.bundle-plan.json`
- `dist/<solution-id>.bundle.answers.json`
- `dist/<solution-id>.setup.answers.json`
- `dist/<solution-id>.README.generated.md`
- `dist/<solution-id>.gtbundle`

Bundle generation is automatically delegated:

```bash
greentic-bundle wizard apply --answers dist/<solution-id>.bundle.answers.json
```

### Bundle Naming

Bundle name is automatically derived:

- `<solution-id>.gtbundle`

No user question is required.

### Wizard Menu Loop

The wizard now runs in a persistent menu loop.

Navigation rules:

- `M` -> return to main menu
- `0` -> go back or exit

Main menu example:

```text
GX Wizard

1) Create new solution
2) Update existing solution
3) Advanced options

M) Main menu
0) Exit
```

## Create Solution Flow

### Step 1 - Template Selection

Prompt:

```text
Which solution template should this start from?

1) Choose from catalog templates
2) Start from a basic empty solution
3) Advanced manual template reference

M) Main menu
0) Back
```

Catalog entries should come from merged local + OCI catalogs.

### Step 2 - Solution Identity

Prompt user for:

- `Solution name`
- `Solution id` (default derived from name)
- `Short description`
- `Output directory`

Example:

```text
Solution name: Network Assistant
Solution id [network-assistant]:
Short description:
Output directory [./dist]:
```

### Step 3 - Access Channel

Prompt:

```text
How should users access this solution?

1) Webchat
2) Teams
3) WebEx
4) Slack
5) All of the above
6) Other catalog preset
7) Advanced manual provider override

M) Main menu
0) Back
```

Internally these choices map to OCI pack refs.

### Step 4 - Generate Solution

The wizard now always:

- builds `solution.json`
- builds `bundle-plan.json`
- builds `bundle.answers.json`
- builds `setup.answers.json`
- builds `README.generated.md`
- calls `greentic-bundle` to generate `.gtbundle`

Example output:

```text
Generating solution artifacts...

✓ solution.json
✓ bundle-plan.json
✓ bundle.answers.json
✓ setup.answers.json
✓ README.generated.md

Delegating bundle generation...

✓ network-assistant.gtbundle created
```

### Step 5 - Return To Menu

After completion:

```text
Solution created successfully.

M) Main menu
0) Exit
```

## Update Solution Flow

Selecting:

- `2) Update existing solution`

Wizard should detect existing artifacts:

- `dist/<solution-id>.solution.json`

It must load and prefill values.

Example prompts:

```text
Solution name [Network Assistant]:
Solution id [network-assistant]:
Short description [Automates network diagnostics]:
Output directory [./dist]:
```

User can:

- press Enter to keep existing values
- modify fields if needed

Provider selection should also be prefilled:

```text
Current provider: Teams
Change provider?

1) Keep Teams
2) Change provider
```

After edits, regenerate artifacts.

## Internal Data Model

The existing internal model should remain with minor simplification.

- `WizardAnswerDocument`
- `CompositionRequest`
- `SolutionManifest`
- `BundlePlan`
- `BundleAnswers`
- `SetupAnswers`

Recommended simplifications:

- derive bundle filename from `solution_id`
- remove separate bundle review screens
- use provider preset OCI refs as the canonical provider representation
- preserve provenance metadata on all resolved catalog selections

## Delegation Boundaries

### `greentic-bundle`

Responsible for:

- bundle assembly
- bundle manifest packing
- archive creation
- writing `.gtbundle`

### `greentic-pack`

Responsible for:

- pack authoring
- pack scaffolding
- pack building

### `greentic-component`

Responsible for:

- component scaffolding
- provider implementation setup

### `gx`

Responsible for:

- catalog resolution
- solution composition
- mapping templates/presets into bundle plans
- generating handoff artifacts
- human-readable summaries

`gx` must not implement bundle assembly.

## Acceptance Criteria

Codex must update the PR so that:

### Wizard UX

- uses the simplified flow described above
- removes unnecessary intermediate screens
- automatically generates outputs
- loops back to the main menu

### Provider Presets

- map UI options to OCI pack references
- support `All of the above`
- support catalog presets
- support manual overrides

### Catalog Loading

- supports local catalog JSON
- supports OCI catalog JSON via `greentic-distributor-client`
- merges catalog entries

### Update Flow

- loads existing `solution.json`
- pre-fills wizard values
- allows user to accept defaults or modify them

### Outputs

Always generate:

- `solution.json`
- `bundle-plan.json`
- `bundle.answers.json`
- `setup.answers.json`
- `README.generated.md`
- `<solution-id>.gtbundle`

### Delegation

Bundle generation must still call:

```bash
greentic-bundle wizard apply --answers <bundle.answers.json>
```

No local bundle assembly must be implemented.

## Tests Required

Codex should add tests for:

- catalog loading (local + OCI)
- template resolution
- provider preset mapping
- solution artifact generation
- bundle answers delegation
- update-mode prefilled prompts
- menu navigation (`M` and `0`)

## Final Instruction To Codex

Update `PR-GX-WIZ-2` with the design changes described above.

Focus on:

- simplified wizard flow
- OCI provider preset mapping
- automatic artifact generation
- persistent menu navigation
- update mode with prefilled values
- strict delegation to `greentic-bundle`

Do not reintroduce complex template or bundle configuration screens into the
main flow.
