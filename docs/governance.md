# Governance Notes

## Purpose

These notes are a lightweight starting point for keeping the Greentic-X model coherent as the repo grows.

This is not a full governance framework yet. It is a small set of guardrails for future work.

## Proposal Pattern

Future substantial changes should be written down before implementation, especially when they affect:

- shared types
- event shapes
- contract manifest format
- op manifest format
- runtime lifecycle semantics
- compatibility guarantees

A lightweight proposal should explain:

- the problem
- the change
- compatibility impact
- migration implications
- why existing crates or models are insufficient

## Compatibility Expectations

The repo should prefer additive evolution where possible.

In practice that means:

- avoid breaking identifier or event shape changes casually
- add versioned contract/op artifacts when semantics materially change
- make compatibility expectations explicit in manifests
- prefer new versions over silent behavior changes

## Versioning Rules

Current working assumptions:

- workspace version tracks coordinated repo releases
- contract manifests carry their own contract versions
- op manifests carry their own op versions
- runtime behavior changes that affect lifecycle semantics should be called out explicitly in docs and changelogs

Future work may refine this into stronger versioning policy, but the repo should already avoid ambiguous compatibility expectations.

## Reuse-First Rule

Before introducing new shared concepts, check whether they already belong in:

- existing Greentic shared crates
- current Greentic-X shared crates

Do not duplicate cross-cutting models casually. If a new shared concept is necessary, document why.

## Packaging and Tooling

Contract and op artifacts are currently local JSON assets, not generated `.gtpack` bundles.

Tooling integration with `greentic-pack` wizard should happen only after:

- the manifest shapes are stable enough
- the packaging story is clear
- the repo is ready to support generated artifacts without churn

## Current Governance Gap

The repo still needs:

- a stronger proposal template if change volume grows
- clearer policy for schema validation and compatibility guarantees
- a more explicit release / publication policy once crates become publishable
