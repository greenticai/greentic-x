# Evidence and View Separation

`PR-GX-03` keeps evidence capture and view rendering separate on purpose.

## Evidence

`EvidenceItem` represents what was observed or produced during flow execution.
The executor can collect evidence emitted by operations and persist it through
the `EvidenceStore` trait.

`FlowRunRecord` stores evidence references rather than embedding every evidence
payload directly. That keeps run records compact and makes evidence storage a
replaceable concern.

## Views

`ViewModel` represents neutral presentation output:

- a stable `view_type`
- a title and summary
- optional body content
- references to primary data

Views are produced through the `ViewRenderer` trait. The executor builds a
render payload from the final run state and lets the renderer decide how to
shape presentation output.

## Why Keep Them Separate

Evidence answers "what happened or what was produced?".

Views answer "how should a downstream consumer see the result?".

Those concerns overlap, but they are not the same:

- one flow can emit multiple evidence items but only one summary view
- different renderers can produce different neutral views from the same run
- evidence should stay auditable even if presentation shapes evolve

This split matches the GX direction without forcing a UI or channel-specific
rendering model into the runtime.
