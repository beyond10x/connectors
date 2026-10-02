---
format: aep.planning-md/3
id: story:mcp-framing-refusal-preserves-uncertainty
kind: story
status: implemented
title: Preserve unknown effects after unreadable MCP responses
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- depends_on: story:mcp-outbound-connection-lifecycle
scope:
- confidence: cited
  path: adapters/mcp/contracts/client/v1alpha1/scenarios/unreadable-answer-is-not-the-callers-input.yaml
- confidence: cited
  path: adapters/mcp/contracts/client/v1alpha1/semantics.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T22:07:35Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T22:07:35Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-02T22:44:31Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Acceptance

After a dispatched request receives an unreadable response, the outbound lifecycle
contract and its named `mcp.outbound.framing-refused` scenario preserve the unknown
remote business effect. They report the existing upstream_protocol diagnostic
without claiming non-execution or rollback and without automatic redispatch. The
predispatch version-mismatch and invalid caller-input rules remain distinct.

## Observed contradiction and owner

At released source e1cc88088fff3cfeb07fdbce39b9aed586ab48a6,
`adapters/mcp/contracts/client/v1alpha1/semantics.md:368` says the caller's request
is not carried out after an unreadable reply. Its owned scenario
`scenarios/unreadable-answer-is-not-the-callers-input.yaml` explicitly starts from
a request already sent, then repeats that conclusion, while its final never row
forbids deriving provider effects from a decoding failure. The outbound invocation
worker found the prose conflict; coordinator inspection additionally found the same
contradiction in the scenario. These are authored statements, not observed network
behavior. Existing contracts/service/compatibility.md and mutations terminal
Indeterminate semantics own uncertainty after dispatch; no new product type is
introduced.

## Scope

Cited exact files under adapters/mcp/contracts/client/v1alpha1:
- semantics.md, framing-refused paragraph only.
- scenarios/unreadable-answer-is-not-the-callers-input.yaml, matching then row only.

Root authors this reconciliation while separate invocation/projection workers own
their three declared paths. The invocation worker's unapplied prose patch is input,
not a complete fix because it omitted the scenario. Preserve upstream_protocol,
peer attribution, both revisions and all predispatch guarantees. Run existing
lifecycle guards plus the new invocation guard at integration; fresh review reads
both changed owners. No new mirrored prose-only test is required for this bounded
text correction. Actual postdispatch runtime fault conformance remains open.

## Verified integration — 2026-10-03

The two named owning prose/scenario files now preserve unknown remote business effect after unreadable postdispatch output, retain upstream_protocol and forbid automatic redispatch. Both files were inputs to the independent outbound invocation review; the source-only follow-up verified their actual meaning and links. Existing predispatch guarantees unchanged.

Full repository gate `cargo run --locked -p connectors-build -- gate --msrv`
exited0. Runner totals:150 summaries,1227passed,0failed,65ignored. Format, Clippy,
Rust1.88 library and1.91 workspace checks, independent adapter models, descriptor
and generated-output checks and AEP validation passed. ESS synthesis emitted498
scenarios(43authored) with21retainedrefusals; metadata target289passed remains
coverage-unknown/inconclusive. No new MCP runtime conformance is claimed.
Gate log SHA256:2d2dd6c294b1746e5dcb3792b44d3385d2cbe6fa00863d9dccc66ea04b5440b5.
Records retained in task-owned mcp-next-wave/integration/gate.log and worker archives.
