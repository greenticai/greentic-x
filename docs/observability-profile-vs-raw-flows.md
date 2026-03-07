# Observability Profile Vs Raw Flows

Use `gx.observability.playbook.v1` when the flow matches the common
observability pattern:

- resolve
- query
- analyse
- present
- optional split/join

Use raw flows when:

- you need custom branching rules
- you need intermediate mapping steps the profile does not express
- you want exact control over step ids and data flow

## Profile Benefits

- much less boilerplate
- consistent step sequencing
- easy translation into a compiled normal GX flow
- good fit for workshop and downstream starter playbooks

## Raw Flow Benefits

- full control over execution structure
- no profile-level assumptions
- clearer when you are exploring new GX patterns that should not be baked into
  the profile yet

## Repo Guidance

This repo keeps both forms intentionally:

- the profile proves that a compact authoring path is viable
- the raw example proves that GX remains a normal flow model underneath

Downstream repos should start with the profile when it fits, then drop to raw
flows only when the profile would become more confusing than helpful.
