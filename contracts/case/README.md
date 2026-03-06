# Case Contract

Generic shared operational case record.

Contents:
- `contract.json`: contract manifest
- `schemas/`: resource and append-entry schemas
- `examples/`: sample case payload

The contract models a case lifecycle with patchable metadata, append-only evidence entries, and a small state machine from `new` through `closed`.
