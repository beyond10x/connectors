---
format: aep.planning-md/1
id: story:broker-read-only-git-fetch-sessions
kind: story
status: implemented
title: Broker read-only Git fetch sessions
summary: Keep GitLab credentials in Connectors while Substrate materializes one exact governed checkout.
relations:
- decomposes: epic:substrate-integration
scope:
- confidence: cited
  path: crates/connectors-client
- confidence: cited
  path: crates/connectors-config
- confidence: cited
  path: crates/connectors-runtime
- confidence: cited
  path: crates/integration-gitlab
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
- confidence: cited
  path: docs/design
- confidence: cited
  path: ess/domains/git.yaml
- confidence: cited
  path: ess/system.yaml
revision: 6
---
# Story: Broker read-only Git fetch sessions

## Outcome

Workspace can establish a bounded Smart Git upload-pack session for one currently authorized GitLab project and exact commit without releasing the provider credential.

## Acceptance

- Creation revalidates Identity, Connection, Grant, project, reference, and commit.
- A stable locator and transient authority admit only upload-pack discovery and fetch.
- Provider credentials remain in Connectors and no Git byte-plane route is model- or public-ingress exposed.
- Expiry, revocation, request and byte exhaustion, restart, and OAuth refresh produce typed outcomes.
- The ESS domain in `ess/domains/git.yaml`, hosted server, official client, and tests agree.

## Scope

Connectors GitLab integration, hosted runtime, byte-plane listener, official client, ESS declaration, and deployment-facing configuration.
