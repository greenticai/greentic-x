# Specs Overview

The `specs/` directory is the formal standards layer for Greentic-X.

It sits above the current Rust implementation and gives downstream repos a
stable place to align on:

- resource envelopes
- resolver results
- operation descriptors and call/result envelopes
- flow run state
- evidence items
- neutral views
- the optional observability authoring profile

The key rule is: specs should map back to existing GX concepts, not invent a
parallel model that contradicts the runtime and manifests already in this repo.
