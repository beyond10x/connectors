---
format: aep.planning-md/3
id: story:invoke-shapes-its-result
kind: story
status: implemented
title: invoke --fields and --each print only the named members of a provider result
tags:
- d240
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T11:46:44Z", actor: "human:timo", revision: 2, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
- {from: "proposed", to: "active", at: "2026-10-30T11:46:44Z", actor: "human:timo", revision: 3, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
- {from: "active", to: "implemented", at: "2026-10-30T13:57:10Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:conn-d240", correlation: "wave-d240-output-shaping"}
---
## Problem

An agent that needs four fields of one ticket reads the whole `invoke` answer: 1,340 B for
`ticket.show`, 38,522 B for one page of 40 ticket audits, 67,546 B for 100 comments (measured on
the pinned Zendesk response examples wrapped in the 0.60.0 `InvokeResult` envelope). It then
writes a helper script to extract them. About 407 B of every answer is fixed envelope
(connector, instance, operation, revision, provenance digests).

## Outcome

`connectors invoke` takes `--fields <paths>` and `--each <path>` and prints only the named members
of the provider result.

- Path grammar (decision D240 a = A, an own path list, no filter language): `name(.name)*`; `[]`
  maps over an array; `alias:path` renames; a key holding `.` or `[` is written as an RFC 6901
  pointer (`/page[after]`). Paths are comma-separated. Paths read the provider's `result`; for a
  catalog operation that is `{status, body, provenance}`, and the leading `body.` may be left out
  when no top-level member of the result has that name.
- `connectors invoke zendesk/support ticket.show --fields ticket.id,state:ticket.status` prints
  `{"ok":true,"result":{"ticket":{"id":101},"state":"open"}}` under `--output json`.
- `--each <path>` prints one NDJSON line per element of the array at `path`, each shaped by
  `--fields`, then the envelope as the last line.
- A path that matches nothing is listed in `result.missing`, so an absent field is never silent.
- `--fields status` is the status-only probe: `{"ok":true,"result":{"status":200}}`.
- A refusal is never shaped: the failure envelope stays on stderr unchanged. A catalog result
  whose `status` is not 2xx is printed whole.
- On a write, `--fields` narrows only the provider result; `request_id`, `mutation` and
  `source_audit` are always printed, so `outcome_unknown` cannot be projected away. `--each` is
  refused on a write as `invalid_input` naming `each`.
- Shaping runs in the CLI process after the owner, the audit record and the mutation ledger have
  the full answer. It is not part of the approval subject and is never sent to the provider.
- A malformed path is refused before any provider call as `invalid_input` naming `fields` (or
  `each`).

## Contract

- `ess/domains/cli.yaml`: `connectors.cli.InvokeInput` gains `fields` and `each`; a new
  `connectors.cli.InvokeShaped` result; the generated CLI contract regenerated.
- `contracts/cli/v1alpha1/semantics.md` gains a section "Shaping a result".
- The generated CLI reference regenerated (`connectors-docs generate`).

## Acceptance

Named tests in `apps/connectors/tests/invoke_shapes.rs` against the existing Zendesk loopback
fixture: fields and alias on a read, `[]` over an array, RFC 6901 escape, `missing`, `--each`
NDJSON with the envelope last, status-only probe, non-2xx printed whole, refusal unshaped, write
keeps `request_id`/`mutation`/`source_audit`, `--each` refused on a write, malformed path refused
before any call. Path-grammar unit tests in the shaping module.

## Out of scope

Filters, a jq or JSONPath dependency, nested caller-declared templates, shaping of `instances
list`, `operations list` and `connections list` (needs an `ess-cli/1` global; later).

## Implemented

Wave D240, merged into `wave/d240-output-shaping` at 9179b5e3d; release 0.63.0.

- `apps/connectors/src/shape.rs`: field lists, member paths, RFC 6901 pointers and aliases, applied
  after the owner, the audit record and the mutation ledger hold the full answer.
- Contract section "Shaping a result" in `contracts/cli/v1alpha1/semantics.md`; `InvokeShaped` in
  the CLI specification.
- Tests: `apps/connectors/tests/invoke_shapes.rs` (13), `invoke_shapes_adversary.rs` (3).
- Review: `review-result:adversary-d240-s1-pass-1`, fixed. The contract was narrowed: a key holding
  `,` or leading or trailing whitespace cannot be named.
