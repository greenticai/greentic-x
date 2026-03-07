# Observability Authoring Profile

`gx.observability.playbook.v1` is an optional compact profile for common
observability-style flows.

It is not a new primitive and it should not become a second runtime model.
Instead, it exists to reduce authoring boilerplate for patterns like:

- resolve
- query
- analyse/correlate
- present

Future GX executor work should compile this profile into ordinary GX flow
definitions.
