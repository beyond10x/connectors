---
format: aep.planning-md/3
id: story:invoke-returns-attempt-id
kind: story
status: active
title: A successful invoke names the attempt it produced
relations:
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T10:59:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T10:59:14Z", actor: "human:timo", revision: 4}
---
## Outcome

A caller of the released `connectors-client` that invokes a write operation over HTTP learns which
attempt the host recorded for it. The host serves `POST /v1alpha2/invoke`
(`contracts/service/compatibility.md` § 2), records a `connectors.mutations.AttemptRecord` for every
admitted `external_write` invocation before provider dispatch, and answers with the § 5 extended
Response whose `mutation` is a `connectors.service_wire.MutationObservation` naming that attempt
(`AttemptReference {instance, id}`). The client returns the result value with that observation. A
downstream consumer resolves a binding's instance to a Connectors endpoint, invokes through the
client and correlates its own records with the attempt id.

The legacy `/v1/invoke` binding and its v1alpha1 envelope stay as frozen
(`compatibility.md:49`): no new member, no new route behaviour, `Client::invoke` keeps compiling.

## Source

- `Client::invoke` (`crates/connectors-client/src/lib.rs:97`) returns only the result `Value`.
- The HTTP host (`crates/connectors-host/src/server.rs:73`, `:108`) calls the adapter directly and
  records no attempt; only the local owner's mutation path does
  (`crates/connectors-host/src/local/mutations.rs:150`).
- `MutationObservation` and `AttemptReference` exist in `ess/domains/service_wire.yaml:25-37`;
  `AttemptRecord` in `ess/domains/mutations.yaml:82`. The wire types in `connectors-core` are
  hand-written serde; ESS generates the JSON Schema they are checked against.

## Units

1. `task:v1alpha2-invoke-wire-spec`: contract and ESS first.
2. `task:v1alpha2-invoke-wire-codec`: `connectors-core` types checked against the generated schema.
3. `task:http-host-mutation-ledger`: the host records attempts and serves `/v1alpha2/invoke`.
4. `task:client-invoke-v1alpha2`: the client binding, conformance and release notes.

## Acceptance

- Spec first: the v1alpha2 invoke request and Response, with `mutation`, are modelled in
  `ess/domains/service_wire.yaml`, validated with the pinned `ess`, and the codec is checked against
  the generated schema in `task check`.
- A successful `external_write` invocation through the client's v1alpha2 binding returns an attempt
  id, and a test reads back the `AttemptRecord` the host recorded under that id.
- A refusal before dispatch (admission, input, unknown operation) returns no attempt; a dispatched
  invocation whose answer is lost reports `outcome_unknown` with the attempt it recorded and is not
  retried.
- A read operation on v1alpha2 carries no `mutation`.
- `/v1/invoke` answers byte-identical to the previous release on the existing wire vectors.
- The CHANGELOG names the new route, the client entry point and the unchanged legacy binding.

## Release note

- The 0.35.0 CHANGELOG lists `connectors.mutations.AttemptRecord.connection_ref` becoming optional (absent for attempts the HTTP host records) as a breaking change for `connectors-client` callers, with its migration.

## Delivered and known limits (wave 20261008b)

- Units: wire spec, codec, vector schema check in the repository gate, audit anchor (HTTP host), attempt ledger, client `invoke_v1alpha2`, one conformance write scenario. Two adversary passes; their 7 findings are fixed with their red tests kept (`http_audit_anchor_adversary.rs`, `http_mutation_ledger_adversary.rs`).
- Retained final observations are held in memory: a host restart loses them and those audit records stay `Anchored`. When 10,000 are held the host refuses new invocations; none is evicted.
- A write whose completion cannot be stored stays `Dispatching`; its response says `applied` with cause `{unavailable, attempt_store}` and nothing retries the settlement.
- A panic inside the audit store's own `finish` loses that observation; a panic during a recovery append keeps admission closed until restart.
- Nothing checks that an HTTP host does not share its `state` directory with a local owner.
- HTTP-host attempts are stamped with the system clock under the anti-regression floor; they never carry an idempotency key.
- `contracts/service/compatibility.md` § 2 says a raw 404 is not proof of an unsupported version; the client reports a 404 without a v1alpha2 envelope as `unsupported` as this story asks. The contract does not yet say so.
- The § 5 status-to-code table exists twice, in the client and in the host.
