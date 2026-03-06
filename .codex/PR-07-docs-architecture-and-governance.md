# PR-07 Add Architecture, Spec Notes, and Governance Docs

## Depends on

- `PR-01-repo-bootstrap.md`

## Goal

Document the conceptual model of Greentic-X inside the repo so the implementation remains aligned over time.

## Docs to add

### 1. Architecture overview
Explain:
- Greentic core vs Greentic-X
- contracts
- runtime
- ops
- examples/apps
- what belongs where

### 2. Contract model doc
Explain:
- contract structure
- mutation rules
- transitions
- versioning and migration intent
- append-only vs patchable fields

### 3. Ops model doc
Explain:
- op descriptors
- compatibility
- input/output schemas
- extension philosophy

### 4. Runtime model doc
Explain:
- resource lifecycle
- revisions
- audit/events
- storage/policy abstraction boundaries

### 5. Governance/proposal notes
Create a lightweight starting point for:
- future GXEP-style proposals
- compatibility guarantees
- versioning rules

Keep this lightweight in the first cut.

## Success criteria

Future work in the repo can point to clear in-repo docs instead of re-litigating the model repeatedly.
