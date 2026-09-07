---
format: aep.planning-md/1
id: story:local-endpoint-discovery-and-resolution
kind: story
status: draft
title: Deliver multi-context endpoint discovery and on-demand resolution from the CLI
summary: Use existing kubeconfig contexts to discover generic service interfaces, resolve current routes and credentials at invocation time, and verify the downloadable local and hosted Connectors release.
relations:
- decomposes: epic:local-product
- depends_on: story:reconcile-connectors-domain-language
- supersedes: story:generic-endpoint-resolution
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
  path: crates/integration-monitoring
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
- confidence: cited
  path: docs
- confidence: cited
  path: ess/generated
- confidence: cited
  path: ess/system
- confidence: cited
  path: scripts
revision: 3
---
## Acceptance

Starting with a downloaded Connectors binary and an existing multi-context kubeconfig, a developer can discover the fixture's supported service interfaces and invoke their admitted operations by endpoint_ref with credentials and connectivity resolved at call time through the documented CLI workflow.

## User instruction and sequence

This is the original local developer experience request, expanded by the operator from databases to all addressable Kubernetes service interfaces. It depends on story:reconcile-connectors-domain-language. Run only after that story's model and code reconciliation are verified. Present both drafts before implementation.

This story supersedes story:generic-endpoint-resolution as the delivery owner. Earlier work and its evidence are inputs to inspect, not proof of completion. The earlier consumer migration scope is explicitly withdrawn. All implementation and release work here is in Connectors; DevCenter UI and consumer repository migrations are excluded.

## Required behavior

1. A downloaded binary uses the developer's existing kubeconfig and authentication mechanisms. Standard kubeconfig merge rules and named contexts apply. All configured contexts are independently addressable; current-context is a convenience default, not an admission boundary or a requirement to switch a singleton backend. A failing or forbidden context does not hide successful contexts.
2. Local setup starts or connects to the Connectors daemon and exposes the endpoint workflow without hand-editing Connectors configuration for every service. Setup, endpoint list/search/describe, refresh, readiness/diagnosis, operation describe/invoke and event/session use share the vocabulary established by Story 1.
3. Discover all visible Kubernetes Service interfaces across admitted namespaces, with all pages, ports, port names and transport metadata. Inspect relevant referenced Crossplane managed resources and connection-secret metadata, and retain meaningful resource/host provenance without creating a parallel host registry or doing network scans. Show unknown and unsupported interfaces with an honest reason; do not invent an operation or execute arbitrary TCP traffic for them.
4. Map supported interfaces to reviewed provider operations and existing drivers through declared metadata and conservative detection. Cover SQL, Loki and Asterisk ARI concretely; include multi-interface Asterisk and unsupported AMI/FastAGI as discoverable metadata. A service can expose multiple endpoint_ref values for different interfaces. Discovery must not be limited to a database-kind allowlist.
5. Return canonical, collision-free endpoint_ref values identifying source context, namespace/resource and interface. Support a context-qualified reference for every interface and the requested k8s/<namespace>/<crossplane-ref> shorthand when it has exactly one matching interface across admitted contexts; ambiguity lists canonical choices and never picks whichever context was visited first. Current-context cannot silently disambiguate an explicit resource reference. Resource UID replacement must not let an old resolved target silently address a different object.
6. Invocation takes endpoint_ref for all supported providers; sql.query is an example of the generic behavior. Resolution rechecks source/target authority, current resource identity, supported operation/interface, route and current credentials for each call. Operation metadata discovery and listing do not read secret values. Secret data is obtained only when an admitted operation or configured event/session establishment needs it; caches must not defeat rotation, revocation or identity checks.
7. Use native Kubernetes port-forward connectivity for local private services and an explicitly admitted direct route where appropriate in hosted execution. Preserve TLS server identity and database/service parameters. Bound and cancel tunnels and sessions, propagate actionable access/route/driver failures, and keep secrets out of output and logs.
8. Reconcile credential-free discovery state on explicit refresh and a bounded background interval, consume pagination, retain successful observations on partial scans, and mark staleness/deletion honestly. Persist durable nonsecret configuration and sufficient provenance for restart, then revalidate routes and credentials before use. Discovery does not itself grant permission; respect Kubernetes RBAC, Connectors ownership/grants and operation approval policy.
9. Carry the same lazy endpoint resolution into the existing live datasource read surface. A datasource identifies the data contract; endpoint_ref selects the backing service. Discovery and reads remain live, with no indexing product or Flux runtime dependency.
10. Enable the Kubernetes integration in the hosted Connectors server with the same domain and resolver behavior, using server-side configured context/workload authority rather than expecting a hosted process to read the developer's laptop kubeconfig. Exercise hosted invocation and live datasource reads with tenant isolation; leave DevCenter presentation for later.
11. Route newly discovered supported event/session interfaces through Story 1's EventReceiver/Session model. This story adds discovery and resolution wiring, not another listener/channel domain. Verify at least one discovered ARI event stream so this is not only an outbound SQL/HTTP example.
12. Retire the old mandatory candidate -> observation -> materialization onboarding and single-selected-context runtime restriction. Existing explicitly configured endpoints remain addressable. Migrate useful stored source/configuration records through Story 1's versioned migration; document intentional removals and preserve a recoverable upgrade path.

## Current evidence and specific gaps

- crates/integration-kubernetes/src/local_endpoints.rs:59 restores only policy.selected_context; current activation is insufficient for independent contexts.
- crates/domain/src/endpoint.rs:10 and crates/protocol/src/operation/v4.rs:46 expose the current discovered Endpoint shape and competing target selectors; Story 1 replaces these.
- crates/protocol/src/datasource.rs:86 currently selects datasource_ref plus binding_ref; Story 1 reconciles selection, this story supplies lazy discovered sources.
- docs/design/10-local-kubernetes-context-and-resource-discovery.md and docs/design/15-a-zero-configuration-endpoint-plane.md contain the prior approach; docs/design/08-discovery-observations-and-mediated-connections.md owns candidate/materialization history.
- The read-only Flux reference is /home/timo/projects/flux: plugins/kubernetes/src/main.rs and crates/flux-capabilities/src/endpoint/broker.rs. Borrow the all-context and endpoint-resolution experience, not Flux runtime dependencies or integration CLI use.
- Existing task-owned fixture evidence is /home/timo/.cache/connectors-endpoint-kubernetes-fixture/acceptance-summary.md. Earlier PostgreSQL and ARI unary calls passed; MariaDB query setup failed on unsupported max_execution_time, while Loki invocation, credential rotation, resource replacement and restart acceptance were not established. Reverify the actual checkout and record updated observations before crediting any case.
- No release of this change has been published or validated. A green isolated unit test is not binary acceptance.

## Typed home

Use the reconciled Endpoint, credential/authority, EventReceiver and Session model delivered by Story 1; current source homes are ess/system/domains/{endpoint,connection,runtime,event,inventory}.yaml and crates/protocol/src/datasource.rs. This story introduces no independent Connection, host, source-registration or datasource-target identity. Put discovered-source-specific resolution and identity relations into the reconciled ESS model before projecting changed contracts; do not infer unknown ownership/cardinality from a ref string.

## Validation and release

Produce one reproducible developer-journey report against the actual release binary:
- clean local installation, first setup, help/diagnosis, daemon start/restart and upgrade from preserved old configuration;
- two contexts with the same namespace/resource names, independent selection, ambiguous shorthand refusal, one unavailable context, kubeconfig refresh and context credential helper behavior;
- multiport Services, pagination, cross-namespace resources, Crossplane references/secret-key mappings, unknown providers and unsupported TCP/UDP interfaces;
- actual read-only PostgreSQL and MySQL/MariaDB queries, Loki query and ARI HTTP operation plus ARI socket events through admitted native port-forward routes;
- secret rotation between calls, missing/denied credentials, revoked grants, namespace denial, partial refresh, Service UID replacement, ready-Pod changes, TLS identity and cancellation cleanup;
- live datasource description/read locally and through a hosted fixture, with source isolation and per-call credential resolution;
- the same endpoint_ref remains the service selector across operation invocation, event receipt and session establishment where that interface supports those interactions.

Resolve the observed SQL initialization failure or publish an explicit accurate supported-server constraint demonstrated by tests; do not claim a MariaDB pass from a PostgreSQL result. Use isolated fixtures, not mutations to operator clusters or messages to real Slack users.

Run bash scripts/gate.sh and the required secret/release/deterministic projection checks for the exact tested commit. Publish the Connectors release through the existing governed release process, download the produced artifact, verify version/checksum and repeat the local happy paths with retained command/output evidence. Documentation examples must use the shipped CLI syntax and published supported behavior. An ordinary source release is complete only after verifying this repository's exact tag, required checks, GitHub Release and required artifacts; a pushed tag with unfinished checks is queued. Publish validated documentation source and manifest together, then let the passive bundle producer and Atlas reconciler deliver them asynchronously. Report documentation as pending unless publication is actually verified; a background documentation failure does not invalidate the source release. Do not wait for Atlas/Website, update Website locks or snapshots, promote consumer pins, release Docs System, or redeploy facades. Full Website/Atlas gates apply only to shared delivery-control changes or an explicit request for end-to-end publication.

## Scope and ordering

| Surface | Confidence | Evidence |
| --- | --- | --- |
| crates/integration-kubernetes | cited | local_endpoints.rs and discovery implementation |
| crates/connectors-{cli,console,client,config,runtime}, crates/server, crates/service | cited | existing generic-endpoint-resolution story and daemon/operation interfaces |
| crates/integration-catalog, crates/integration-monitoring, crates/driver-sql | cited | HTTP/monitoring/SQL execution and fixture gaps |
| crates/domain, crates/protocol, contracts, ess/system | cited | Endpoint and datasource contracts above; build on Story 1's completed shape |
| docs, release configuration, test fixtures | cited | scripts/gate.sh and existing design/fixture references above |

This pair deliberately runs sequentially because the surfaces overlap. Story 1 owns vocabulary, target identity, configured-endpoint migration, all inbound semantics and missing intake implementation; this story owns multi-context discovery, discovered-target lazy resolution, datasource exposure, CLI journey and combined release. Do not recreate the former model to avoid the dependency, and do not start either implementation during this story-presentation turn.
