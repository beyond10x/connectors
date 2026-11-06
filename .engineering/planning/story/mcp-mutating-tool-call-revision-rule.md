---
format: aep.planning-md/3
id: story:mcp-mutating-tool-call-revision-rule
kind: story
status: implemented
title: A mutating MCP tool call has one stated rule for an absent revision
tags:
- from-finding
relations:
- derived_from: review-result:connectors-wave-2026-11-04-u1-adversary
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/mutation-cases.json
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/mutations.md
- confidence: cited
  path: adapters/mcp/contracts/server/v1alpha1/projection-cases.json
- confidence: inferred
  path: adapters/mcp/contracts/server/v1alpha1/projection.md
- confidence: cited
  path: apps/connectors/src/mcp/projection.rs
- confidence: cited
  path: crates/connectors-build/tests/mcp_inbound_capability_projection.rs
- confidence: inferred
  path: crates/connectors-build/tests/mcp_inbound_mutation_replay.rs
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T09:37:30Z", actor: "human:operator", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-05T17:29:39Z", actor: "human:operator", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-06T05:34:08Z", actor: "human:operator", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

The adversary pass on the MCP revision rule (review-result:connectors-wave-2026-11-04-u1-adversary,
finding 1) found two contracts that disagree. `adapters/mcp/contracts/server/v1alpha1/projection.md`
now binds a `tools/call` without a revision to the advertised revision. 
`adapters/mcp/contracts/server/v1alpha1/mutations.md:23-25` still says a mutating tool request carries
its exact current descriptor revision. Mutating tools are not advertised today, so no call reaches
either rule, but the first mutating tool will need one.

## Acceptance

- One contract text states which rule a mutating `tools/call` follows when it carries no revision, and
  the other refers to it.
- A case in `adapters/mcp/contracts/server/v1alpha1/mutation-cases.json` (or the projection case file)
  holds the chosen outcome for a mutating tool call without a revision, and the checker enforces it.

## Open question

Which rule: mutating tools require the revision (a client must have seen the description it acts on),
or they bind like reads. This is an operator decision.

## Decision (2026-11-05): option A

Decided by the directing session `b10x-chef-orchestrator` under its grant:

- A mutating `tools/call` requires its descriptor revision; `adapters/mcp/contracts/server/v1alpha1/mutations.md:23-25`
  stands.
- `adapters/mcp/contracts/server/v1alpha1/projection.md` limits the absent-revision binding to read tools
  and refers to `mutations.md`.
- A case row for an absent revision on a mutating call: `-32602`, no provider request; its checker holds it.
- Reason: it keeps the documented write contract and narrows a rule that no write reaches today, since no
  mutating tool is projected.
- Not taken: option B (mutating calls bind like reads) breaks a documented contract, which is the
  operator's alone; option C (leave both texts) leaves them disagreeing.

Accepted by the directing session for the next connectors wave (its acceptance); it does not join the
2026-11-05 wave.

## Coordinator decision during implementation (2026-11-05)

The implementor found at runtime that `bind_revision` (`apps/connectors/src/mcp/projection.rs:724`) binds an absent revision for every projected tool, without telling a read from a mutating tool. Today only the projection filter (`projection.rs:214-231`), which projects reads only, keeps a mutating call from binding without its revision. The directing session left the choice between fixing it in this unit and filing a draft to the coordinator. The coordinator fixes it in this unit, because it is this story's rule: `bind_revision` itself refuses an absent revision for a tool that is not a read, with -32602, so the rule no longer depends on what the projection filter lets through.

## Orchestrator decision on the adversary's blocker (2026-11-06)

The adversary found that the unit's runtime change answers `-32602` ("Missing projection revision") when an unlisted Write operation is called by name without a revision. The delivered story:mcp-write-operation-call-by-name-is-refused promises `not_found` with no provider request "with and without a revision", and its test (`apps/connectors/tests/local_mcp_server_write_refusal.rs:642`) went red.

The directing session decided that the delivered story stands:
- Projection visibility is decided before the revision check. A tool that is not projected answers `not_found`, whatever shape its revision has.
- The absent-revision `-32602` applies only to projected mutating tools. None is projected today, so the rule waits for the story that projects one.
- This also closes the adversary's note that the `-32602` told a granted caller that an unlisted operation is mutating.

One correction in this unit:
- At runtime, an unprojected name answers `not_found` before `bind_revision`'s mutating check; the read flag stays.
- The admission text in `mutations.md` states that the revision position comes after projection visibility, as well as after policy and scope.
- The adversary's two cases stay, with `not_found` expected for the unlisted write. One case pins the mutating refusal for a projected mutating tool, or the report notes why the test rig cannot project one.
- `docs/local-mcp-stdio.md:87` says the absent-revision binding covers reads only. If a mutating field in `projection-cases.json` is small it is added; otherwise it is filed as a draft.
- The coordinator keeps the `Unknown` arm of the classification, which no V2 or V3 bootstrap reaches today: it refuses rather than binds if a later bootstrap admits an unknown effect.

## Delivered (wave 2026-11-05b, v0.68.0)

The unit commit be6f693e4 and the correction 7ee45f9e6 were merged in 513325bf8 on `wave/2026-11-05b-connectors`. The local staged gate is green at cd51cd062.

As built:
- A listed mutating `tools/call` without a revision is refused `-32602` at the revision position, after policy, scope and projection visibility.
- A read, or a name the server does not list, binds an absent revision to the advertised one, so an unlisted write called by name stays `not_found`.
- A malformed revision stays `-32602`, and a stale one stays `stale_description`.
- `bind_revision` (`apps/connectors/src/mcp/projection.rs`) takes its read flag from `reads_only` or from the name being unlisted.
- No mutating tool can be projected today (`crates/connectors-host/src/local/runtime.rs:718-719`), so the refusal is held by the unit test `a_mutating_call_without_a_revision_is_invalid_params`.
- The read or mutating field for the projection case rows is filed as story:mcp-projection-cases-carry-read-or-mutating.

Targeted tests:

| Test | Passed |
|---|---|
| `local_mcp_server_write_refusal` | 8 |
| `--bin connectors` | 55 |
| `local_mcp_server_revision_adversary` | 7 |
| `mcp_inbound_mutation_replay` | 11 |
| `mcp_inbound_capability_projection` | 11 |

`mcp-bindings --check` is current.

Scope as landed:

| path | result |
|---|---|
| `adapters/mcp/contracts/server/v1alpha1/mutation-cases.json` | held |
| `adapters/mcp/contracts/server/v1alpha1/projection.md` | held |
| `adapters/mcp/contracts/server/v1alpha1/mutations.md` | held |
| `crates/connectors-build/tests/mcp_inbound_mutation_replay.rs` | held |
| `adapters/mcp/contracts/server/v1alpha1/projection-cases.json` | added (a renamed row) |
| `crates/connectors-build/tests/mcp_inbound_capability_projection.rs` | added |
| `apps/connectors/src/mcp/projection.rs` | added (`bind_revision`, `reads_only`, `lists`) |
| `apps/connectors/tests/local_mcp_server_write_refusal.rs` | added (the adversary's two cases) |
| `docs/local-mcp-stdio.md` | added |
