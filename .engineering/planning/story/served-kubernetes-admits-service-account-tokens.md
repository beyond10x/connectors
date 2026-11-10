---
format: aep.planning-md/3
id: story:served-kubernetes-admits-service-account-tokens
kind: story
status: draft
title: The served Kubernetes adapter admits callers by service-account token like the catalog provider
summary: ServiceConfigV3 admission service-account-token with audience and principal allow-list; v2 static-bearer configs keep working
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:served-admission-admits-a-set-of-service-accounts
revision: 2
---
Requested by Assembly (assembly-dev session), 2026-11-10.

## Outcome

The served Kubernetes adapter admits its callers the way the served catalog provider does: a
`ServiceConfigV3` with `admission: service-account-token`, one audience, and a principal or an
allow-list of principals or service accounts, each checked with a TokenReview.

Today only the catalog provider reads `ServiceConfigV3` (`adapters/catalog/src/served.rs:60,156`;
the type is `crates/connectors-host/src/server.rs:88-161`). `connectors-kubernetes --config`
reads the closed v2 `ServiceConfig` (`adapters/kubernetes/src/main.rs:30-36`), which admits only a
static bearer. `serve` already takes `impl Into<ServiceConfigV3>` (`server.rs:552`).

## Acceptance

- A Kubernetes service configuration with `admission: service-account-token`, an audience and a
  principal list starts, and each request is admitted by a TokenReview. A test with a fixture
  TokenReview endpoint shows an allowed principal answered, and a token for another principal, a
  wrong audience or an unauthenticated review refused before any Kubernetes API request.
- Every admission member and refusal the catalog provider's v3 configuration accepts behaves the
  same here. The two share one reader, and a test pins that.
- An existing static-bearer v2 Kubernetes configuration starts and answers unchanged. The
  existing served Kubernetes tests stay green.
- The configuration schema published for the Kubernetes service names the v3 form.

## Out of scope

- How the adapter authenticates to the API server (story:served-kubernetes-authenticates-in-cluster).
- Write operations.

## Ambiguities

- Whether a v3 Kubernetes configuration may still choose `admission: static-bearer`, or whether
  static callers stay on v2 only.

## Consumer preference (Assembly, 2026-11-11)

These are the consumer's preferences, not decisions. The ambiguities above stay open for the
operator.

- A v3 configuration admits callers either by TokenReview or by a static bearer. One instance
  never admits both.
