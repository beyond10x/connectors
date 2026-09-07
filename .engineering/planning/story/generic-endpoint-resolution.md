---
format: aep.planning-md/1
id: story:generic-endpoint-resolution
kind: story
status: active
title: Discover service interfaces and resolve admitted operations on demand
summary: Superseded by the sequential domain-reconciliation and local-discovery stories; earlier consumer migration scope is withdrawn.
relations:
- decomposes: epic:local-product
- informed_by: story:kubernetes-joins-the-catalog
scope:
- confidence: cited
  path: contracts
- confidence: cited
  path: crates/connectors-cli
- confidence: cited
  path: crates/connectors-client
- confidence: cited
  path: crates/connectors-config
- confidence: cited
  path: crates/connectors-console
- confidence: cited
  path: crates/connectors-runtime
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/driver-sql
- confidence: cited
  path: crates/integration-catalog
- confidence: cited
  path: crates/integration-kubernetes
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
- confidence: cited
  path: docs
- confidence: cited
  path: ess/system
revision: 7
---
## Superseded scope — 2026-09-07

The operator withdrew the consumer-migration scope recorded below and requested two sequential Connectors-only stories: story:reconcile-connectors-domain-language, followed by story:local-endpoint-discovery-and-resolution. The second story supersedes this one as the delivery owner; the first owns the preceding naming/model and whole-codebase reconciliation. Both new drafts must be presented before any implementation resumes.

The historical heading "Approved implementation" below must not be read as authorization to migrate Zwirn, agent-platform, platform/runtime consumers, or any other consumer repository. That expansion was the agent's mistake. This record and existing WIP are retained as history/evidence; its active status is not renewed execution approval and no work is to be scheduled from it while the replacement stories are under review.

## Acceptance

A fresh Connectors installation with a valid Kubernetes context can start its daemon through setup, discover all Service interfaces, and invoke an admitted existing HTTP, WebSocket or SQL operation by endpoint reference with current credentials and an approved route, locally and hosted, without per-endpoint materialization.

## Approved implementation

The operator approved the complete generic endpoint discovery plan on 2026-09-07. Preserve Connections, grants, catalogs and operation identities; replace candidate/observation/materialize onboarding, in-memory discovery, restricted monitoring execution and daemonless provider I/O. New endpoint and operation protocol versions coordinate CLI, SDK, Zwirn and agent consumers. Endpoint identity binds source, resource UID and interface; unknown and unsupported interfaces remain visible. Namespace/provider read policy is admitted once; write approval remains independent. Secret values never enter discovery. Local routes use native Kubernetes port-forward streams; hosted and explicit external routes use approved direct transport. TLS preserves logical authority. Reconcile durable nonsecret inventory every 60 seconds and explicitly, consuming all pages and retaining unseen records on partial scans.

Typed home: `ess/system/domains/endpoint.yaml`, including Endpoint-to-Connection references. Installed ESS 0.9.2 refuses the baseline components.yaml command_line variant; verification uses the repository-pinned ESS 0.18.0. This work serves organization objectives O1 and O5 through this repository's AGENTS; no corresponding local vision artifacts exist, so no dangling serves edge is asserted.

## Scope

Contract implementor owns `crates/domain`, `crates/protocol`, `crates/service`, `crates/server`, `crates/connectors-client` and `contracts`, except the CLI implementor's separately named daemon lifecycle modules. Kubernetes implementor owns `crates/integration-kubernetes` and shared dynamic HTTP invocation in `crates/integration-catalog`. CLI implementor owns `crates/connectors-cli`, `crates/connectors-console`, daemon lifecycle modules in protocol/client/server, and ESS CLI component declarations. Parent owns `crates/connectors-runtime`, `crates/connectors-config`, `crates/driver-sql`, ESS endpoint declaration, final protocol projections, docs and cross-repository consumers. Entry-point merges are sequentially reviewed by parent; concurrent implementors exchange shared API definitions before integration. All are one coordinated integration story, not independently releasable fragments.

## Verification

Test cold setup, daemon restart, complete multiport paginated discovery, namespace denial/partial refresh, current credentials and rotation, exact Service UID and ready-Pod target validation, protocol/input admission before secret access, tunnel cancellation, TLS validation, query bounds, consumer wire contracts and old direct Connections through the daemon. Run the full authoritative repository gate with bounded reproducible build output. Real cluster tests use isolated fixtures only; never mutate operator clusters to obtain evidence.

## Delivery

Implement in managed worktrees, use bot-authored published commits and coordinated reviewable branches. Source release tags and deployment are separate from implementation. No AMI driver, host scan or Devcenter UI is included. Record actual verification and remaining limitations before any completion claim. The four-critic decomposition panel is skipped because this is one integration artifact rather than a multi-artifact decomposition; code receives independent review and whole-path tests.
