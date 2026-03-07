# Greentic-X Docs

This directory contains the in-repo architecture and specification notes for Greentic-X.

- `architecture.md`: repo boundaries and how the major pieces fit together
- `specs-overview.md`: how the new `specs/` layer maps to current GX concepts
- `core-catalog.md`: overview of the minimal reusable GX catalog
- `authoring-profile-observability.md`: optional profile for compact observability-style authoring
- `contracts.md`: contract manifest model and reference contract philosophy
- `ops.md`: operation manifest model and reference op philosophy
- `runtime.md`: runtime lifecycle, revision handling, event emission, and extension boundaries
- `runtime-overview.md`: current runtime responsibilities and composition
- `runtime-boundary.md`: what the runtime owns versus what later GX layers add
- `flow-executor.md`: the current GX flow execution layer built on top of the runtime
- `parallelism-and-join-semantics.md`: the current split/join model and why it stays deterministic for now
- `evidence-and-view-separation.md`: why evidence capture and neutral views are distinct concerns
- `tooling-overview.md`: current `gx` CLI commands and how they fit with `greentic-pack`
- `how-to-build-a-downstream-solution.md`: downstream repo workflow and reuse guidance
- `simulation-workflow.md`: how to iterate on flow packages with stubbed simulation
- `reference-examples.md`: the current flow/profile-driven reference examples and what they demonstrate
- `observability-profile-vs-raw-flows.md`: when the compact profile helps and when raw flows are clearer
- `why-six-primitives.md`: rationale for the current GX primitive set
- `governance.md`: lightweight compatibility, versioning, and proposal notes
- `examples.md`: walkthrough of the runnable example applications

Pack-specific notes live alongside the source packs in `packs/README.md`.
