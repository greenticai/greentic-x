# Why Six Primitives

The aligned GX roadmap assumes six reusable primitive families:

- resources
- contracts
- operations
- resolvers
- evidence
- views

The current repo implements the first four directly and prepares the ground for
the remaining two.

Why this is enough:

- resources hold durable state
- contracts define allowed structure and lifecycle
- operations perform reusable work
- resolvers standardize lookup and disambiguation
- evidence captures what was observed or produced
- views present neutral output for downstream channels

Topology, observability, and analysis scenarios can be represented by combining
these primitives rather than introducing product-specific engines into core.
