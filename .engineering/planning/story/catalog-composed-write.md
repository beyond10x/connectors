---
format: aep.planning-md/3
id: story:catalog-composed-write
kind: story
status: draft
title: A catalog write composed of ordered steps
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A catalog selection may declare a composed write: an ordered list of steps, each an upstream
operation or a raw upload to a URL a previous step answered, where a later step's inputs are
read from an earlier step's answer by JSON pointer; the whole runs under one approval, one
idempotency key and one audit entry, and stops at the first refusal with the step named. The
Slack file upload needs it (`files.getUploadURLExternal`, then the bytes to the answered URL,
then `files.completeUploadExternal`; story:parity-slack-files), and the catalog engine sends no
binary request body today (`docs/fluxplane-plugin-parity.md`, `slack.file.upload` row).

## Surface

`adapters/catalog/spec/ess/domains/selection.yaml`: a `steps` member on a write selection, each
step with its operation or upload target, its input bindings (fixed values, caller inputs, or a
pointer into a previous step's answer) and the one binary step allowed, bounded in size by the
selection. `contracts/catalog/v1alpha1/semantics.md`, "Writes": what a composed write promises
(all-or-nothing is not promised; a refusal after a completed step reports `partial` naming the
steps that ran). The engine in `adapters/catalog/src/lib.rs` executes steps in order; the host's
approval and audit treat the composed write as one operation.

## Acceptance

- Spec first: the member, its bindings and the `partial` outcome are modelled in
  `adapters/catalog/spec/ess/domains/selection.yaml`, validated with the pinned `ess`, regenerated,
  and the contract page updated, before the engine changes.
- A three-step fixture (get URL, upload bytes from a caller file, complete) runs through
  `connectors operations invoke` under one approval; the fixture receives the three requests in
  order with the URL and file id carried over; one audit entry names the operation.
- A refusal at step three answers `partial` naming steps one and two as done, with nothing retried.
- A file above the selection's byte bound is refused as `invalid_input` naming the bound before
  step one is sent.
- The 15 shipped selections still lower byte for byte (`connectors-build definition` fixtures).

## Scope

Primary: `adapters/catalog/spec/ess/domains/selection.yaml`, `adapters/catalog/src/lib.rs`,
`adapters/catalog/tests/` (a new composed-write test), `contracts/catalog/v1alpha1/semantics.md`,
`crates/connectors-build/src/` (lowerer admits `steps`). Collides with any story changing the
selection model or `lib.rs` (the Jira write stories if they need a wrapper member): runs in a
later wave than P2.
