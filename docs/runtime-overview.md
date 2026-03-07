# Runtime Overview

Greentic-X currently centers on [`greentic-x-runtime`](/projects/ai/greentic-ng/greentic-x/crates/greentic-x-runtime), which composes:

- contract installation and activation
- resource create/get/list/patch/append/transition
- typed links between resources
- operation registration and invocation
- resolver registration and invocation
- runtime event emission

The runtime is intentionally storage-agnostic. The current repo ships in-memory
adapters for resources and event recording so the model can be exercised without
external infrastructure.

Above that layer, [`greentic-x-flow`](/projects/ai/greentic-ng/greentic-x/crates/greentic-x-flow)
now provides:

- flow definitions and execution records
- split/join semantics
- pluggable evidence storage
- pluggable neutral view rendering

This keeps the core runtime focused on lifecycle primitives while still giving
GX a concrete execution model.
