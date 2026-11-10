---
format: aep.planning-md/3
id: story:served-kubernetes-authenticates-in-cluster
kind: story
status: draft
title: The served Kubernetes adapter authenticates to the API server with the pod's projected service-account token
summary: In-cluster token and CA from the standard paths, rotation without restart, no static bearer file
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:served-instance-admits-a-service-account-token
revision: 1
---
Requested by Assembly (assembly-dev session), 2026-11-10. Reason: the pod holds no long-lived
credential.

## Outcome

A served Kubernetes adapter running in a pod authenticates to the API server with the pod's
projected ServiceAccount token and the cluster CA, read from the standard in-cluster paths
(`/var/run/secrets/kubernetes.io/serviceaccount/token` and `ca.crt`). `base_url` defaults to the
in-cluster API server (from `KUBERNETES_SERVICE_HOST` and `KUBERNETES_SERVICE_PORT`). No static
bearer file is configured.

Today `connectors-kubernetes --config` reads the closed v2 `ServiceConfig`
(`adapters/kubernetes/src/main.rs:30-36`, `crates/connectors-host/src/server.rs:28-44`). Its
upstream credential is `HttpConfig.credential` (`crates/connectors-host/src/http.rs:18-31`): a
static file that `read_file` opens `O_NOFOLLOW`, owner-only and at most 8192 bytes
(`crates/connectors-host/src/credentials.rs:136-185`). A projected token is a symlink into a
rotating `..data` directory and is refreshed by the kubelet, so that rule refuses it.

## Acceptance

- With in-cluster credentials selected and no `base_url`, the adapter sends
  `Authorization: Bearer <token>` to the in-cluster API server and verifies it with the projected
  CA. A test pins the request and the CA used.
- Token rotation is picked up without a restart. The token is re-read before its expiry (from the
  JWT `exp`) and once on a 401, and the request is retried once with the new token. A test rotates
  the fixture's `..data` link and asserts that the next request carries the new token.
- The token never appears in a log line, an error, a `describe` answer or the effective
  configuration. A test writes a known token and finds it in none of those bytes.
- A missing, unreadable or empty token or CA file refuses at start by a named reason, before
  serving. One test per case.
- The existing strict rule for static credential files (`O_NOFOLLOW`, owner-only, at most
  8192 bytes) is unchanged for every other credential. The existing credential tests stay green,
  and a test shows that a symlinked static credential file is still refused.

## Out of scope

- Caller admission (story:served-kubernetes-admits-service-account-tokens).
- Write operations.
- Exec-credential plugins and kubeconfig, which the local composition already covers.

## Ambiguities

- How in-cluster credentials are selected: by an explicit member (for example
  `credential: in-cluster`) in a new service configuration version, or by the absence of
  `credential` and `base_url`. An explicit member is the safer default.
- The fixed in-cluster paths: whether they are fixed, or overridable for tests only.
