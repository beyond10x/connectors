---
format: aep.planning-md/3
id: approval-record:wave-d218-short-invocation
kind: approval-record
status: approved
title: 'Wave D218: connectors invoke <connector>[/<instance>] <operation>, accepted by the orchestrator'
relations:
- decides: story:connectors-do-short-invocation
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-29T21:13:03Z", actor: "human:timo", revision: 2}
---
## Approval

The directing session `b10x-chef-orchestrator` approved item D218 on 2026-10-29, under its grant:
"implement connectors story:connectors-do-short-invocation (draft rev 14) as one wave through
/aep:implementing, and release it per the repository's process". It settled the story's open
questions:

- "a write invoked without `--approval-file` is refused by a named code that names the approval
  steps (the approval flow itself is unchanged)."
- "An instance with two or more connections: use the single `ready` one; when none or several are
  ready, refuse by a named code listing them."
- "The breaking removal in 'Removed, not deprecated' rests on the operator's words quoted in the
  story (2026-10-22, 'i dont want to keep anything old'). Update every in-organization consumer of
  the removed forms in the same wave where it lives in this repository (docs, website/docs, skills,
  examples); list any consumer outside this repository in your final report instead of editing it."

It added acceptance from a consumer's evidence on v0.59.1:

1. "An instance with exactly one connection invoked without naming the connection resolves it.
   Today `operations invoke` without `--connection` gives `invalid_input` / stage `arguments` with
   no reason: whatever still refuses must say why."
2. "`connections revalidate --expected-revision` given the instance's configuration_revision instead
   of the connection revision gives `lifecycle_conflict` / `retry_status`; make that refusal name
   which revision was expected and where to read it."

Sequencing: the integration branch starts from `origin/main` after v0.59.2 (ESS 0.77.0). The release
is 0.60.0, a minor bump for the breaking change.

## Design taken by the coordinator

- D1 `connectors invoke <connector>[/<instance>] <operation>`: positional target and operation;
  options `--input-json`, `--input-file` (stdin when stdin is not a terminal), `--approval-file`,
  `--idempotency-key`, `--describe`, `--definition` with `--definitions-root` and
  `--credential-file`, and the optional pins `--connection`, `--schema`, `--revision`. A pin that
  differs from the instance's current admitted description is refused, never re-resolved.
- D2 With `/<instance>` left out: one instance of the connector selects it; two or more are
  `instance_ambiguous` (exit 2), listing `<connector>/<instance>`; none is `not_found`.
  `default_instance` is not consulted, since the command always names the connector.
- D3 Connection: the single `ready` connection; none or several ready is `connection_unresolved`,
  listing each connection with its state. Amended after review (coordinator): the consumer's
  acceptance "an instance with exactly one connection invoked without naming the connection
  resolves it" must hold for an idle connection, whose validation evidence lapses to `pending` as a
  normal state and is re-checked with the provider before the call. So with no `ready` connection,
  exactly one usable one (`pending` and current under the configuration) is selected; one `ready`
  beside lapsed ones selects the `ready` one; anything else is `connection_unresolved`.
- D4 A write without `--approval-file` is `approval_required` naming the three approval commands,
  before any provider call.
- D5 Terminal stdin and no input flag: `invalid_input` with `field: input`, without reading.
- D6 `--describe`: description, input schema, read or write, schema and revision; no provider call.
- D7 The invoke result names `connector` and `instance`, never `adapter`; `instances list` items name
  connector and instance only; a legacy `[adapters.<alias>]` catalog entry is named by the connector
  `setup migrate` derives for it, never `catalog`.
- D8 Every `invalid_input` of `invoke` names its argument in `field`.
- D9 Removed: `operations invoke`; the explicit service route `describe`, `invoke --endpoint`,
  `serve` (`compat.rs`, `legacy.rs`, `compatibility.json`, its completion and help lines).
- D10 `connections connect <connector>/<instance> --endpoint <url>` writes `endpoint` into the
  instance's table (creating the instance, and the connector table enabled, when absent), leaving the
  rest of the file as it was; a different endpoint on an existing instance is refused, since another
  host is another instance. No other command takes `--endpoint`.
- D11 `connections revalidate --expected-revision` that is not the connection's revision names
  `field: expected_revision`, the expected kind (`connection`), and `connections describe` as where
  to read it; given the configuration revision, it says so.

## Units

| unit | scope | depends on |
|---|---|---|
| U1 invoke | binding + ESS cli domain + `apps/connectors/src/local/invoke.rs`: D1-D8 | none |
| U2 connections | `connections connect` target and `--endpoint` (D10), revalidate refusal (D11) | none |
| U3 removal | D9, the migration of every test from `operations invoke` to `invoke`, D7's `instances list` | U1 |
| U4 docs | README, CHANGELOG 0.60.0, contracts/cli, docs/, website/docs, regenerated references | U1, U2, U3 |

Objective: O5 (the generic agent platform), through epic:fluxplane-plugin-parity.

## Commits this approval covers

One commit or more per unit on its branch, the merges into `wave/d218-short-invocation`, the store
commits (this record, the story's acceptance and moves, the close), one pull request to `main`
merged through `PUT /pulls/<n>/merge`, the annotated tag `v0.60.0` and its release page, all as
`b10x-bot[bot]`. Nothing else.
