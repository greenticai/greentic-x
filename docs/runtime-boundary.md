# Runtime Boundary

Greentic-X runtime responsibilities today:

- own installed/active contract state
- enforce resource lifecycle rules declared by contracts
- register and invoke operations against validated manifests
- register and invoke resolvers through a normalized envelope
- maintain typed resource links
- emit audit-friendly runtime events

What stays outside the runtime for now:

- customer/domain semantics
- production persistence and transport implementations
- rich policy engines
- automatic schema distribution/registration for non-local backends
- advanced schedulers, queues, and distributed execution infrastructure

`greentic-x-flow` now composes the runtime for flow orchestration, evidence
stores, and neutral views. The core runtime still stays small enough to evolve
while the GX execution layer grows separately on top.
