---
format: aep.planning-md/1
id: story:reconcile-connectors-domain-language
kind: story
status: active
title: Reconcile Connectors domain language and target identity across the codebase
summary: Analyze naming collisions, adopt one endpoint identity, and reconcile all contracts, configuration, execution and inbound behavior inside Connectors.
relations:
- decomposes: epic:local-product
- informed_by: story:connectors-ess-domain
- informed_by: epic:beyond-http
- informed_by: epic:native-voice
scope:
- confidence: inferred
  path: catalog
- confidence: cited
  path: contracts
- confidence: inferred
  path: crates/connector-address
- confidence: inferred
  path: crates/connector-oauth
- confidence: inferred
  path: crates/connector-resolve
- confidence: inferred
  path: crates/connector-secrets
- confidence: inferred
  path: crates/connector-spec
- confidence: inferred
  path: crates/connector-state
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
  path: crates/driver-audio
- confidence: cited
  path: crates/driver-cdp
- confidence: cited
  path: crates/driver-sip
- confidence: cited
  path: crates/driver-speech
- confidence: cited
  path: crates/driver-sql
- confidence: inferred
  path: crates/hosted-secrets
- confidence: inferred
  path: crates/hosted-state
- confidence: inferred
  path: crates/hosted-vault
- confidence: cited
  path: crates/integration-catalog
- confidence: cited
  path: crates/integration-gitlab
- confidence: cited
  path: crates/integration-jira
- confidence: cited
  path: crates/integration-kubernetes
- confidence: cited
  path: crates/integration-mcp
- confidence: cited
  path: crates/integration-monitoring
- confidence: cited
  path: crates/integration-platform
- confidence: cited
  path: crates/integration-sip
- confidence: cited
  path: crates/integration-slack
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: crates/rtvbp-voice-endpoint
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
- confidence: inferred
  path: crates/state-sqlite
- confidence: inferred
  path: crates/subscription-custody
- confidence: cited
  path: crates/voice-local-audio
- confidence: cited
  path: crates/voice-runtime
- confidence: cited
  path: docs
- confidence: cited
  path: ess/generated
- confidence: cited
  path: ess/system
- confidence: inferred
  path: providers
revision: 9
---
## Acceptance

Starting from the current mixed Connection/Endpoint/Instance interfaces, the repository's migrated installation passes the scenario matrix below using one documented meaning for every public domain term.

## User instruction and sequence

The operator requested exactly two stories: first analyze the naming and reconcile the whole Connectors codebase, then implement the original discovery and local developer experience request. Present both stories before implementation. This is an interactive review; no implementation is authorized by creation or critic approval of these drafts.

This story owns the complete naming/identity migration in Connectors, including configured providers and existing discovery implementations. The dependent story owns discovery expansion and its released developer workflow. Work sequentially. Do not migrate consumer repositories or add DevCenter UI.

## Naming analysis grounded in the current code

The problem is overlapping identity and overloaded terminology, not merely spelling:
- operation v4 accepts connection_ref or endpoint_ref for the same target-selection role: crates/protocol/src/operation/v4.rs:46.
- Slack constructs connection:slack:{instance_id}; it does not expose an Endpoint: crates/integration-slack/src/backend/connection_runtime.rs:388.
- Endpoint currently embeds Kubernetes-specific resource and credential details in its generic shape: crates/domain/src/endpoint.rs:10.
- Channel means supervised event intake in docs/design/01-domain-model.md:353, a provider conversation in Slack payloads, and a voice reference in crates/domain/src/voice.rs.
- Subscription names upstream registration (crates/connector-spec/src/inbound.rs:400), downstream event consumption, an interaction shape, and paid-service credential custody.
- VoiceApplicationRoute is existing voice destination configuration, not an inbound handler: crates/service/src/voice.rs:19.
- Listener is currently a runtime/configuration responsibility, not an independently managed generic domain entity.
- The current native SIP driver establishes outbound calls: crates/driver-sip/src/lib.rs:78. A declaration or a listening SIP socket does not prove incoming-call support.

## Replacement vocabulary and disposition

The following is the proposed destination model for this story, not a claim about the current implementation. Preserve distinct responsibilities; remove duplicate target identities.

| Existing vocabulary | Decision and precise meaning |
| --- | --- |
| Provider | Keep: catalog definition of supported operations, event declarations, authentication requirements, and setup profiles. |
| Connector | Use Connectors for the product and provider definition for an authored/compiled catalog document; avoid Connector as another runtime entity. |
| Service | Qualify catalog service, Kubernetes Service, and generated service deployment; they are different concepts. |
| Catalog, Specification, Overlay, Provenance | Keep their catalog-build meanings and permanently published provider/operation identities. |
| Driver, Adapter, Backend | Keep as implementation roles: protocol execution, boundary translation, runtime implementation. They are not user target identities. |
| Integration | Keep only the configured enablement of a provider, including shared application registration and deployment policy; not an individual external identity. |
| Connection, Endpoint, Instance | Merge the durable target role into Endpoint and endpoint_ref. An Endpoint is one configured or discovered service interface or external identity exposing provider capabilities. Instance is not a second public identity. Connection is reserved for actual transport connections internally. |
| Credential, CredentialRef, Profile | Keep separate from Endpoint identity: authentication material, secret lookup reference, and declared authentication/setup mode. Rotating a credential does not create a new Endpoint. |
| DiscoveryObservation, ConnectionCandidate | Retain as internal evidence where needed; remove mandatory candidate activation/materialization as another public target-registration step. Story 2 replaces the discovery workflow. |
| Source, Context, Host, Resource | A discovery source identifies where evidence came from; Kubernetes context and resource metadata remain qualified provenance. A host/resource can expose several Endpoints. No independent generic host registry or address scan is introduced. |
| Binding, Route, Address | Qualify as endpoint resolution, access route, and network address. They explain how an Endpoint is reached and authenticated, not another competing invocation target. |
| Datasource, DatasourceBinding | Keep datasource_ref for the data contract; endpoint_ref selects its configured service. Binding/schema/scope details may remain descriptive data but binding_ref cannot remain an alternate service selector. |
| ConnectSession | Rename the user-facing temporary authentication process to SetupSession; preserve its purpose, owner authentication, expiry, and one-time completion semantics. |
| Principal, Tenant, Owner, Subject, Actor, Audience | Preserve the distinct caller, ownership, upstream identity, and authorization meanings; explicitly qualify them in docs and types. |
| Grant, Approval, Redemption, Audit | Preserve permission, per-action approval, consumption, and audit responsibilities; bind target authority to Endpoint without widening access during migration. |
| Invocation, Execution, Result | Invocation is an operation request/attempt; execution machinery and its outcome remain distinct from a long-lived Session returned by it. |
| ChannelBinding, Channel | Rename the event-intake meaning to EventReceiver declaration/runtime. This replaces the existing supervised Channel responsibility, including its lifecycle and partition identity; it does not add a second entity beside it. |
| Webhook, Socket, Poll, Transport | Describe how an EventReceiver obtains events. Separate event direction from which party opens the network connection. Session is an interaction shape, not a transport. |
| Listener | Keep as local network-acceptor configuration with bind address/protocol/lifecycle. Do not invent a public listener_ref without an independent management requirement. |
| Handler | Code handling a received request/event/session; no new public Handler entity or handler_ref. |
| Event, Cursor, Replay | Keep as received normalized fact, consumer position, and reread of retained facts; distinguish provider events from platform lifecycle events. |
| Subscription | Qualify EventSubscription for consumer event selection, WebhookRegistration for upstream registration, and paid-service subscription for account entitlement. Use CredentialLease for bounded credential access. |
| Delivery | Keep an event-delivery attempt/queue with its configured destination, retry and signing behavior; do not confuse it with provider acknowledgement or a business reply. |
| Session, Lease, Termination | Keep the live interaction and its lifetime; distinguish this from SetupSession and CredentialLease. |
| Call, voice Channel, Participant, Media, Signal | Keep protocol-native call-leg identifiers at protocol boundaries; expose the neutral live interaction as Session. Qualify participant and media meanings; audio channel count remains a media property. |
| VoiceApplicationRoute, SIP TargetAlias | Name the former voice session destination in configuration. Model separately addressable configured trunks as Endpoints; dialed numbers and remote participants are operation/session data. No generic application-routing entity is introduced. |
| Target, Deployment, Placement, Daemon, Supervisor | Qualify CLI deployment target versus operation Endpoint. Preserve local/hosted execution placement and process supervision as hosting responsibilities. |
| ServiceBundle, Module, Factory | Keep generated-service packaging and construction internal and qualified; no competing endpoint identity. |
| StateStore, EventStore, Journal, Console, Client, CLI, HTTP/MCP frontend | Keep persistence and presentation responsibilities separate from public domain identities. |

## Reconciliation rules

1. Publish one glossary and old-to-new migration matrix covering the inventory above, every current domain, public reference, and owning crate. Apply it to Rust types and modules where they encode domain meaning, config, CLI/help/errors, wire contracts, SDKs inside this repo, generated projections, docs, examples, and tests. Do not mechanically rename vendor fields, native protocols, historical records, or third-party source bytes.
2. Endpoint works for manual setup and discovery, inbound and outbound. Two Slack identities produce distinct endpoint_ref values; one identity using HTTP operations plus Socket Mode remains one Endpoint. Multiple credentials can serve one identity. A source Kubernetes Endpoint and each discovered service interface remain individually addressable.
3. Separate generic Endpoint identity/capabilities/authority from typed source-specific resource and credential-resolution details. Specify ownership, lifecycle and relations before updating generated contracts; do not use an untyped metadata bag to evade the model.
4. Perform a clean versioned public contract/config cutover. Current public operations use endpoint_ref, not connection_ref or instance_id as alternate selectors. Stored identities, labels, credentials and grants migrate deterministically and idempotently; retain a migration mapping and fail ambiguous mappings explicitly rather than guessing. Historical versioned contracts and migration readers remain bounded compatibility surfaces, not hidden execution fallbacks.
5. Keep provider-native operation IDs and payload schemas stable. Compatibility documentation must name old/new versions, migration command, backup/recovery procedure, and breaking changes. Check Identity-issued external scope vocabulary before changing any authority string; if external coordination is required, record it rather than changing another repository.
6. Reconcile every inbound path: endpoint attribution, receiver lifecycle, network listener, authentication, normalization, acknowledgement, event persistence, consumer selection, delivery, and session handling. Preserve source partitions and deduplication. A Slack channel ID is provider data; it is never a Connectors receiver ID.
7. Within this story, deliver the previously requested missing catalog event intake and native incoming SIP under the reconciled model: current Slack and ARI socket declarations, Slack/Twilio webhook declarations, and bounded polling support. No new AMI or FastAGI driver. List declared versus implemented versus fixture-verified support separately.
8. Resolve the currently unanswered SIP answer policy in the naming/design phase before incoming-call implementation: an offered call, acceptance/rejection, timeout, routing to local audio or a configured voice destination, and termination need explicit behavior. The operator did not select automatic answering; do not infer it from a route being configured.

## Typed homes and unresolved semantics

The baseline typed homes were inspected at source commit 62b1014551cd2b75f89df946b63e62b41a084199. No ESS or generated contract edits are authorized in the current analysis-only task.

The [standalone naming review](../../../docs/design/23-domain-language-review.md) now supplies proposed answers to the questions originally left UNMAPPED here:
- D1-D2 and the relationship table resolve the proposed Endpoint identity/ownership model and the immediate mediated-parent cardinality from source.
- D3 distinguishes event receiver declarations from incoming-session declarations; a blanket ChannelBinding-to-EventReceiver rename is rejected.
- D4 defines the proposed receiver, consumer subscription and delivery responsibilities/lifecycles, while marking the runtime capabilities that are only declared today.
- D5 preserves the namespace/project scope carried by existing datasource bindings alongside endpoint_ref.
- D6 reconciles neutral Session versus native call/channel identity and voice destination configuration.
- D7 recommends explicit acceptance by an authorized application/CLI for incoming offers. Declarative auto-answer is a separate later product choice, not a prerequisite to naming review or an approved feature.
- D8 separates lifecycle/readiness/authorization and records compatibility boundaries, including frozen external signed fields.

These are source-grounded recommendations for operator and peer review. They are not accepted design decisions, implementation evidence, or a claim that the existing uncommitted ESS/code changes conform. Record subsequent operator decisions explicitly before any implementation is separately authorized.

## Scenario matrix and verification

The naming migration is demonstrated by a single conformance report covering:
- two configured Slack identities independently invoked by endpoint_ref, with the same Endpoint attributed through Socket Mode and Events API webhook intake;
- configured SQL, Loki and ARI endpoints plus existing Kubernetes-discovered endpoints, all using the same target contract;
- direct and mediated routes, multiple authentication profiles, refresh/rotation, restart, and interrupted/repeated migration without lost or broadened grants;
- SIP dial and incoming offer/answer/reject/timeout/remote hangup with explicit Endpoint, Session and participant identities; both local-audio and application-destination compositions;
- webhook verification/attribution, socket reconnect and persistence-before-ack, polling cursor recovery, replay, denied access, deduplication and business reply separated from acknowledgement;
- browser, audio/speech, Git fetch, MCP ingress and egress, generated native services, datasource reads, custody-only providers, and credential leases under qualified vocabulary.

Run affected contract/conformance tests, ESS validation/projection drift checks, and the complete repository gate; retain the report, exact tested commit, migration fixtures and actual failures. Audit stale names with an explicit allowance for native/vendor fields, immutable historical contracts, and migration code. A changed glossary alone cannot satisfy acceptance.

## Scope

All changes are within Connectors. Shared paths are sequential with story:local-endpoint-discovery-and-resolution, not parallel work.

| Surface | Confidence | Evidence |
| --- | --- | --- |
| crates/domain, crates/protocol, crates/service, crates/server, contracts, ess/system | cited | operation/v4.rs, domain/endpoint.rs, service/voice.rs and existing ESS entities above |
| crates/connectors-{config,cli,console,client,runtime} | cited | existing generic-endpoint-resolution scope and configuration/CLI responsibilities |
| crates/integration-*, crates/driver-*, crates/voice-*, crates/rtvbp-voice-endpoint | cited | current integration/driver manifests and voice contracts |
| crates/connector-{spec,address,resolve,oauth,state,secrets}, custody/storage adapters | inferred | naming references and authority/state migration; change only where the completed reference audit finds affected semantics |
| docs, tests, generated projections, examples, release metadata | cited | docs/design/01-domain-model.md and scripts/gate.sh |

## Relationship and delivery boundary

Both new stories decompose epic:local-product; this story is additionally informed by the existing domain, beyond-HTTP and native-voice work. The old generic-endpoint-resolution story contains the earlier model and rejected consumer migration scope; it is historical input, not execution authority for this story. Preserve useful WIP and evidence for deliberate reconciliation, not blind continuation.

Story 1 completes the repository-wide model and behavioral reconciliation on a verified Connectors commit; it does not depend on Story 2's new discovery workflow to prove its configured-endpoint scenarios. Story 2 then builds on that model and owns the combined downloadable release, live discovery acceptance, and publication evidence. No story is marked implemented merely because a plan or schema validates.

## Planning review and handoff

This is an interactive run. The naming analysis and disposition matrix above were completed during planning; no runtime, schema or consumer implementation was performed in this drafting turn. Both new stories remain draft for presentation to the operator.

Review profiles: aep-plan:plan-critic-acceptance, aep-plan:plan-critic-design, aep-plan:plan-critic-scope, and aep-plan:plan-critic-parallel-safety. All four returned approve with zero findings in round one; their verbatim immutable records are review-result:domain-stories-{acceptance,design,scope,parallel-safety}-round-one. No findings required outcomes or another round. The tool's concurrency limit prevented a four-at-once panel; the perspectives ran in smaller batches without sharing findings. Interrupted scope/safety reviews produced no received verdict and were resumed; completed acceptance/design verdicts were retained. The profiles' Sonnet pin was unavailable, so the review agents used the inherited model. These are procedural deviations, not claims of the prescribed four-simultaneous-model setup.

Existing typed homes validated; the new names and unresolved semantics remain a proposal until this story updates their ESS representation. Incoming SIP answer policy remains with the operator before that implementation begins. No approval or implementation evidence has been fabricated.

Retained in managed worktree wt-87a5e72befe7 at /home/timo/.local/state/worktree/trees/b10x/connectors/wt-87a5e72befe7, branch feat/generic-endpoint-discovery, source HEAD 62b1014551cd2b75f89df946b63e62b41a084199. The story/review drafts and journal are local and uncommitted; this turn published no branch or commit. Existing implementation WIP remains preserved. Next owner is the operator for review of these two drafts, followed by the implementing agent in the explicit Story 1 -> Story 2 order. Release the drafting session lease at handoff; retain the tree for review rather than attempting cleanup.

## Current authorization and naming review

2026-09-07 operator correction: this session is authorized for naming analysis and a review document only. The latest instruction, "Implement the plan", refers to the analysis-only plan presented in the conversation. It does not authorize code, configuration, contract, ESS, migration, release, consumer or generated-output changes.

The standalone review deliverable is [Connectors domain language: proposal for review](../../../docs/design/23-domain-language-review.md). It analyzes source commit 62b1014551cd2b75f89df946b63e62b41a084199, accounts for the domains and implementation roles, and gives source-backed recommendations for all previously open naming questions. Its D1-D8 sections supersede the initial table where they differ, particularly event versus incoming-session declarations and datasource scope. The recommendations await operator/peer review; they are not an approved design.

The prior evidence entry for "good, do the first one now" was incorrectly interpreted as implementation approval. The operator explicitly clarified that it authorized analysis only. Preserve that historical entry, but do not use it as implementation authorization. Earlier automated planning reviews assessed story drafts; they did not approve unresolved naming decisions on the operator's behalf.

Implementation is stopped. Existing uncommitted code remains preserved and is excluded as evidence that the proposed design is correct. Story 2 remains unstarted. This story's active lifecycle state covers the current analysis/review work; it is not evidence of an approved implementation. Do not mark it implemented upon completion of this document or upon a planning validation pass.

## Independent naming review

An independent reviewer agent, given the document, requirements and source baseline without the author's conversation history, reviewed docs/design/23-domain-language-review.md at SHA256 5132420037a9f6e5b9d0533f07e075ab2cf0432b17178ac509e0c5d80fa1b82e against source commit 62b1014551cd2b75f89df946b63e62b41a084199.

Verdict: revise before acceptance. The verbatim immutable report is [review-result:domain-language-independent-review-round-one](../review-result/domain-language-independent-review-round-one.md).

Three findings remain open: NIR-001 (high), logical consumer stream membership after cross-receiver event deduplication; NIR-002 (medium), controlling principal for incoming offered Sessions; NIR-003 (medium), the omitted operational-event family. The reviewer supports the main target, datasource, receiver/session and legacy-boundary distinctions, but the completeness claims need correction along with these findings.

This turn records review evidence only. The reviewed proposal has not been edited or treated as approved, and no implementation has resumed. Operator review and any requested document revisions remain the next actions; this result is not permission for code changes.
