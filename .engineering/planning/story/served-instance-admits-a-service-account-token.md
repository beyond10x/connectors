---
format: aep.planning-md/3
id: story:served-instance-admits-a-service-account-token
kind: story
status: active
title: A served catalog instance admits a projected service-account token for one configured principal
summary: A v1alpha2 admission profile checked with TokenReview, and catalog providers on the served host
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
---
## Outcome

A served instance admits its caller by a projected Kubernetes service-account token instead of a
shared static bearer. The instance is configured with one audience and one principal
(`system:serviceaccount:<namespace>:<name>`); on each request it asks the cluster API with a
TokenReview whether the presented token is authenticated for that audience, and admits the call
only when the review names that principal. Nothing is cached across requests, and an unreachable
or failing review refuses the call (fail closed). The anchor's principal is the service account.

The served host also serves catalog providers: a catalog instance (a built-in connector or a
lowered definition such as connectors-definitions `github.yaml`) runs behind
`/v1/describe` and `/v1alpha2/invoke`, with its credential read from a mounted file. Today only
the Kubernetes, SQL and federation hosts are served (`docs/running-services.md`).

Wave D243-C1, unit C2. Design section 2 (a), decision B1 (TokenReview), and section 1 row 2.

## Decided

- B1: a projected service-account token, checked with TokenReview (orchestrator, 2026-10-30).
- One instance admits one principal: the cleared option A of
  `decision-blocker:mcp-caller-connection-assignment`.

## Scope

Contract and model first, in this order: the new admission profile `service-account-token` in
`contracts/service/compatibility.md` (section 2.1) and `contracts/service/v1alpha2/semantics.md`,
the service configuration in ESS (`connectors.declarations.ServiceHostConfiguration`), then the
code: `crates/connectors-host/src/server.rs` admission, a TokenReview client, the catalog provider's
served mode (`adapters/catalog/src/main.rs`), `docs/running-services.md`, `CHANGELOG.md`.

Configuration: `admission: service-account-token`, `audience`, `principal`, and the TokenReview
endpoint with its CA and the instance's own credential. `static-bearer` stays the default, and an
existing service configuration reads unchanged.

## Acceptance (named tests)

`crates/connectors-host`, against a loopback TokenReview test double:
- `a_reviewed_token_of_the_configured_principal_is_admitted`
- `the_review_names_the_configured_audience`
- `a_token_of_another_service_account_is_refused_unauthorized_with_no_adapter_call`
- `a_token_the_review_does_not_authenticate_is_refused_with_no_adapter_call`
- `a_review_for_another_audience_is_refused_with_no_adapter_call`
- `a_failing_or_unreachable_review_is_unavailable_with_no_adapter_call`
- `a_missing_bearer_is_unauthorized_before_any_review`
- `the_anchor_principal_is_the_service_account`
- `static_bearer_admission_is_unchanged`
- `a_v2_service_configuration_reads_unchanged`

`adapters/catalog`, against a loopback provider:
- `a_served_catalog_instance_describes_and_reads`
- `a_served_catalog_instance_reads_its_credential_from_the_mounted_file`
- `a_served_definition_instance_describes_its_operations`

Conformance scenarios named in `contracts/service/compatibility.md`:
`v1alpha2-service-account-token-admitted`, `v1alpha2-service-account-token-other-principal`,
`v1alpha2-service-account-token-review-unavailable`.

## Review

A security-reviewer pass on the admission path before the wave's pull request.
