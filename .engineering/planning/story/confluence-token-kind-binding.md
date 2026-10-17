---
format: aep.planning-md/3
id: story:confluence-token-kind-binding
kind: story
status: active
title: A Confluence connection binds v1 or v2 by probing its token once
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:confluence-v1-authored-contract
- depends_on: story:connection-lease-holds-unattended
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T08:46:55Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-15T08:46:56Z", actor: "human:timo", revision: 3}
---
## Outcome

A Confluence connection serves both token kinds without the caller choosing. On connect and on
revalidate, the provider probes the token once:
- a v2 read (`/wiki/api/v2/spaces`, limit 1);
- if it answers 401 with Atlassian's scope-mismatch reason, a v1 read
  (`/wiki/rest/api/space`, limit 1).

It binds the connection to the API that answered, records the binding in the connection's
evidence, and from then on routes every operation id to that API's operation. Callers name
operation ids only. `operations list` and `operations describe` show the operations of the
bound API.

## Rules

- The probe runs at connect, repair and revalidate only, never on an ordinary read.
- A token that both APIs refuse does not connect, and the refusal names both answers
  (`insufficient_scope`, the stage, and which API answered what). A token that v2 refuses for a
  reason other than scope mismatch is refused as today; it does not fall back to v1.
- The binding is part of the connection's validated evidence. A revalidation that finds the
  other API answering now rebinds, and says so in its result.
- The mechanism is generic in the catalog provider: a provider's configuration declares
  alternative bindings, each with its own bundle, selection set and access probe, plus the
  answer that moves to the next binding. Confluence is its first user. No Confluence code
  goes into the engine.
- The Confluence abstraction, the Atlassian documents contract and the shared datasource
  contracts it feeds, states the binding: which reads each binding serves, and what an id
  missing from the bound binding answers (a named refusal, never a silent empty result).

## Scope

The catalog provider engine and its config format (`adapters/catalog/src`), the catalog spec
(`adapters/catalog/spec/ess`), `contracts/catalog` semantics, the Atlassian documents contract
and its ESS domain, `docs/catalog-confluence.md` and `docs/local-catalog-provider.md`.

## Acceptance

- A recorded token that is scope-refused on v2 and answered on v1 connects bound to v1, and
  `page.get` returns the v1 page through `connectors operations invoke`.
- A recorded token answered on v2 connects bound to v2, exactly as before, with no v1 request.
- A token refused by both does not connect, and the refusal names both answers.
- A non-scope v2 refusal (for example 401 for a bad password) does not fall back.
- A revalidation that sees the other API answering rebinds and says so.
- An id the bound binding lacks answers a named refusal.
- The release notes give the exact configuration block and commands that move a local
  Confluence install to the bound form.

## Delivery

The provider child keeps no state between validate and invoke, so the binding is carried by
the host (decision-blocker:confluence-token-kind-binding-delivery, option a). Surfaces, spec
first in the shared ESS and the contracts:

- an optional `binding` in the private validation baseline (`ess/domains/cli.yaml`) and the
  stored evidence (`ess/domains/credential_evidence.yaml`), omitted when absent so stored
  bytes do not change;
- optional `bindings` in the private bootstrap, one descriptor and requirements per binding;
- an optional `binding` on invoke, taken from the evidence the read captures;
- a reason beside `insufficient_scope` on a failed validate, so a refusal names both answers;
- a rebind field in the revalidate result;
- `--connection` on `operations list` and `operations describe`.

It starts after story:connection-lease-holds-unattended merges, which edits the same host
files. Until it ships, the v1 reads are their own provider configuration, `confluence-v1`,
selected by the operator's configuration block.
