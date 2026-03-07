# PR-GX-05 — Observability Profile and Reference Examples

## Title

Ship the first reference examples and the optional observability playbook
profile to prove Greentic-X covers real multi-step analysis use cases

## Goal

Demonstrate that Greentic-X can express common observability and troubleshooting
scenarios with minimal boilerplate, while keeping the core generic.

This PR should provide:

- optional observability authoring profile
- a set of generic examples
- example flows showing both sequential and parallel analysis patterns

## Alignment With Current Repo

The repo already has Rust examples that manually drive the current runtime. Keep
them as useful smoke/examples, but add a second layer of **flow/profile-driven**
reference examples so the GX architecture is proven in the intended shape.

Do not delete the current examples just because they are not yet executor-based.

## Why

The strongest proof that GX is correctly designed is not abstract documentation
but working examples that show:

- a simple resolve → query → analyse → present flow
- a correlation flow
- a split/join synthesis flow

These should remain neutral and not include Zain/customer-specific details.

## Deliverables

### 1. Observability profile implementation/compilation path

Take `gx.observability.playbook.v1` from the earlier specs PR and make it
usable:

- parse/load profile
- compile to a normal GX flow model
- validate step structure
- support compact patterns such as:
  - resolver
  - query op
  - analysis op
  - present op
  - optional split/join

### 2. Reference examples

Create examples such as:

#### `examples/top-contributors-generic/`

A generic “top contributors over time” example.

#### `examples/entity-utilisation-generic/`

A generic threshold-based utilisation analysis example.

#### `examples/change-correlation-generic/`

A generic event ranking-by-proximity example.

#### `examples/root-cause-split-join-generic/`

A generic split/join correlation example with two branches:

- health telemetry
- attribution analysis

These should mirror workshop-style structural patterns without using
customer/vendor/domain nouns.

### 3. Example expected outputs

For each example include:

- input payload
- expected evidence items
- expected view model
- short explanation of which execution pattern it demonstrates

## Important constraints

- Keep examples generic
- Do not introduce network-specific package names
- Do not couple the profile to telecom semantics
- Make it obvious that downstream repos can translate these into their own
  domain language

## Docs to add

- `docs/reference-examples.md`
- `docs/observability-profile-vs-raw-flows.md`

Explain:

- when to use the compact profile
- when to use raw flows
- how downstream repos can build domain-specific playbooks on top

## Tests

Add:

- profile compile tests
- example validation tests
- end-to-end simulation for all examples

## Acceptance criteria

- At least four reference examples exist
- The optional observability profile compiles into executable GX flows
- Docs explain how downstream customer repos can use these examples as templates
- The current Rust examples and the new flow/profile examples tell a consistent
  story rather than competing with each other

## Codex instruction

Use these examples as a proving ground for the GX architecture. The goal is not
generic toy docs but realistic, reusable reference patterns that downstream
solutions can translate into their own concrete use cases.
