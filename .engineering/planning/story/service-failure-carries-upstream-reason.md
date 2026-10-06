---
format: aep.planning-md/3
id: story:service-failure-carries-upstream-reason
kind: story
status: active
title: A dispatch service failure carries the upstream reason, not only its code
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:connector-probe-20261006
- depends_on: story:dependency-refresh-20261006
scope:
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: inferred
  path: adapters/catalog/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: inferred
  path: apps/connectors/src/local/operations.rs
- confidence: cited
  path: contracts/cli/v1alpha1/private-adapter.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-core/src/lib.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner/mutation.rs
- confidence: inferred
  path: crates/connectors-host/src/local/runtime
- confidence: cited
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: ess/domains/cli.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T10:22:52Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-06T10:22:53Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
---
## Observed

2026-10-01, connectors 0.20.0, Confluence through the Atlassian API gateway with a service-account token
(`atlassian.basic`): `pages.changed`, `space.pages` and `page.get` answer `service_failure` with `service_code:
unauthorized` at `dispatch`. The CLI output names no reason, so the consumer could not tell a wrong path from a
missing token scope. A direct request to the same URL (`/ex/confluence/<cloud id>/wiki/api/v2/pages?limit=1`) returned
`401 {"code":401,"message":"Unauthorized; scope does not match"}`; the v1 route `/wiki/rest/api/space` returned 200
with the same token. The cause was the token's scopes.

## Acceptance

- A refused dispatch carries a bounded, redacted upstream reason (for example the provider's `message`, at most
  256 bytes, never headers or credential material) beside `service_code`, in the CLI's JSON output.
- A fixture test shows a 401 whose body names a scope reaching the caller, and a body with a secret-shaped value
  being withheld.
- `contracts/cli/v1alpha1/semantics.md` states what the field may contain.

## Ordering

Added 2026-10-06. Edits `apps/connectors-cli-contract/binding.json` and `ess/domains/cli.yaml`,
which `story:dependency-refresh-20261006` regenerates, so it lands after it (`depends_on`). Shares
`adapters/catalog/src/lib.rs` (`read_body`) with `story:catalog-honours-retry-after`, and
`apps/connectors/src/local.rs`, `crates/connectors-host/src/local/owner.rs` and the CLI contract with
`story:expired-evidence-invoke-advises-revalidate` and `story:cli-json-answers-as-json`; all three
land after it.
