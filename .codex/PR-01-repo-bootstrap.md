# PR-01 Bootstrap — Create greentic-x Monorepo Skeleton

## Goal

Create a new `greentic-x` monorepo that becomes the home for:
- Greentic-X runtime crates
- reference contracts
- reference ops
- examples
- documentation/spec notes

This is a new repo, so no audit PR is required.

## Principles

- Greentic-X is the shared object and operations framework layer
- Greentic core stays minimal
- repo layout should support incremental growth without repo sprawl
- keep the initial structure small but clean

## Required repository layout

Codex should create an initial structure similar to:

```text
greentic-x/
├─ Cargo.toml
├─ Cargo.lock
├─ README.md
├─ docs/
├─ crates/
│  ├─ greentic-x-types/
│  ├─ greentic-x-contracts/
│  ├─ greentic-x-events/
│  ├─ greentic-x-runtime/
│  └─ greentic-x-ops/
├─ contracts/
│  ├─ case/
│  ├─ evidence/
│  ├─ outcome/
│  └─ playbook/
├─ ops/
│  ├─ approval-basic/
│  ├─ playbook-select/
│  └─ rca-basic/
└─ examples/
   ├─ simple-case-app/
   ├─ simple-playbook-app/
   └─ end-to-end-demo/
```

Refine exact names only if there is a strong code-level reason.

## Workspace requirements

- proper Cargo workspace
- workspace lints/dependency strategy consistent with your usual repo style
- minimal CI/test placeholders if standard in your repos
- clear README describing scope and non-goals

## Documentation requirements

Top-level README must explain:
- what Greentic-X is
- what belongs in this repo
- what does not belong in Greentic core
- crate/directory responsibilities
- status of runtime/contracts/ops/examples

## Non-goals

- do not fully implement the runtime yet
- do not overdesign governance machinery in code
- do not add dozens of crates prematurely

## Success criteria

The repo is ready for incremental implementation without forcing later large-scale restructuring.
