---
format: aep.planning-md/3
id: story:service-failure-carries-upstream-reason
kind: story
status: draft
title: A dispatch service failure carries the upstream reason, not only its code
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
