---
format: aep.planning-md/1
id: story:mcp-profile-selection-matrix
kind: story
status: active
title: Select the supported MCP revisions, transports and capabilities as a coverage matrix
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-specification-pin
- depends_on: story:mcp-domain-model
scope:
- confidence: inferred
  path: adapters/mcp/contracts/protocol/v1alpha1/selection.md
revision: 5
---
## Acceptance

`adapters/mcp/contracts/protocol/v1alpha1/selection.md` carries a coverage matrix
in which every protocol revision, every transport and every optional capability
named by the pinned specification appears exactly once with one of three
dispositions — supported, explicitly refused, or deferred — each carrying a reason
and a source line into the pinned revision, so that a capability absent from the
matrix is a defect in the matrix rather than a silent approximation.

## Scope

- `adapters/mcp/contracts/protocol/v1alpha1/selection.md` — new; the selection and
  the matrix
- `adapters/README.md` `## Owners` row for MCP is **not** touched here; it is
  `story:mcp-specification-pin`'s

## Domain relations

None of its own. The matrix is a disposition per protocol feature, not a statement
about which noun owns which. Where a row's disposition would require knowing an
ownership relation — a durable capability snapshot, for instance — the row cites
the `UNMAPPED:` marker `story:mcp-domain-model` carries and dispositions the
feature `deferred`, which is the honest third value the epic itself supplies:
"supported, explicitly refused or deferred, with reasons."

## What this story must establish

`epic:mcp-contracts` asks for four separable things here and this document is the
single owner of all four:

1. **The selected revisions and transports.** `initiative:complete-local-connectors`, milestone 5 (cited by its number, not its line: this citation read `:35` and the sentence is at `:33` — the identical drift `story:mcp-specification-pin` corrected in itself and nobody propagated to the siblings drafted in the same wave)
   already records the intent — "over stdio and Streamable HTTP, with
   version-specific 2026-07-28 and explicit 2025-11-25 interoperability" — and the
   epic requires that "local stdio and deployed HTTP profiles" be evaluated
   explicitly. Outbound stdio is dispositioned against
   `decision-blocker:mcp-outbound-stdio-process-ownership`, which holds whether
   this repository may execute an unpinned server binary at all; inbound stdio,
   where the client spawns `$BIN server` and Connectors is the child, is not behind
   that question and is dispositioned on its merits.
2. **Tools, resources, prompts and the optional reverse-direction capabilities,
   separately.** The epic: "Specify tools, resources, prompts and optional
   reverse-direction capabilities separately: supported, explicitly refused or
   deferred, with reasons."
3. **The rule that a listing is not support.** The epic: "Do not equate a protocol
   capability listing with implemented support." The matrix therefore separates
   *what the pinned revision defines* from *what this repository selects*, in two
   columns, and never lets the first imply the second.
4. **Refusal rather than approximation.** Acceptance criterion 4 of the epic:
   "Unsupported functionality is explicitly refused, not silently approximated."
   Each `refused` row states the refusal a caller observes, not merely that the
   feature is absent.

## Why one document may hold both directions

`epic:mcp-contracts` says outbound and inbound "are separate directions with
separate trust boundaries", and no story in this decomposition specifies behaviour
for both. This document specifies behaviour for neither: it selects protocol
versions and features, and the per-direction rows point at the direction documents
that own the semantics. The repository already has this shape —
`contracts/service/compatibility.md` owns version selection across every family
under its "Independent version axes" section while stating "Individual stories
still own family behavior. A compatibility disposition is not implementation
support." This document is that, for MCP.

## What it does not cover

How a selected transport actually frames, streams, cancels or loses a session —
`story:mcp-outbound-connection-lifecycle` and `story:mcp-inbound-local-binding`
own that per direction. What a caller may invoke — the projection story. No
runtime, no fixture server and no executable is added.
