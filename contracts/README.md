# Contracts

`contracts/` is the semantic source for every shared contract: the textual description first, then schemas, Rust bindings and fixtures when a contract is implemented (`docs/design.md:296-313`). Each contract has one document at `contracts/<family>/…/<version>/semantics.md`. Status is stated at the top of each document.

## Index

| Contract id | Document | Status | Defines |
|---|---|---|---|
| `operations/v1alpha1` | [service/v1alpha1](service/v1alpha1/semantics.md) | implemented | describe/invoke wire boundary, error envelope, limits |
| proposed governed service binding | [service/v1alpha2](service/v1alpha2/semantics.md) | proposed | verified caller context, admission, policy, audit and [one-hop delegated approval](service/delegation.md); proposed v1alpha2 routes and codecs per the [compatibility owner](service/compatibility.md), not implemented |
| `operations/v1alpha1` `mutation` profile | [operations/v1alpha1](operations/v1alpha1/semantics.md) | proposed | effects, idempotency, approval binding, durable attempt record, outcome-unknown |
| `datasource.records/v1alpha1` | [service/v1alpha1](service/v1alpha1/semantics.md) | implemented | bounded pages, cursors, provenance, completeness |
| `datasource.records/v1alpha1` `document` profile | [datasources/records/v1alpha1](datasources/records/v1alpha1/semantics.md) | proposed | single-item content bodies with representation, version, byte truncation |
| `datasource.relational/v1alpha1` | [service/v1alpha1](service/v1alpha1/semantics.md) | implemented | PostgreSQL schema discovery and bounded reads |
| `datasource.logs/v1alpha1` | [datasources/logs/v1alpha1](datasources/logs/v1alpha1/semantics.md) | proposed | log lines with stream identity, native query, window and byte bounds |
| `datasource.series/v1alpha1` | [datasources/series/v1alpha1](datasources/series/v1alpha1/semantics.md) | proposed | labeled time series with step, PromQL, partial results |
| `endpoint_discovery/v1alpha1` | [service/v1alpha1](service/v1alpha1/semantics.md) | implemented | address/port observations and candidates, no dial |
| `host_discovery/v1alpha1` | [service/v1alpha1](service/v1alpha1/semantics.md) | implemented | node/host observations |
| `resource_discovery/v1alpha1` | [discovery/resources/v1alpha1](discovery/resources/v1alpha1/semantics.md) | proposed | opaque-locator observations with target-adapter candidates, generations, withdrawal |
| `route.mediated_http/v1alpha1` | [discovery/mediated_route/v1alpha1](discovery/mediated_route/v1alpha1/semantics.md) | proposed | one-hop forwarding of a child adapter's HTTP through a parent connection's admitted binding |
| `auth.connection/v1alpha1` | [auth/connection/v1alpha1](auth/connection/v1alpha1/semantics.md) | proposed | stable connection refs, identities, scope/actor, route, safe status |
| `auth.profile/v1alpha1` | [auth/profile/v1alpha1](auth/profile/v1alpha1/semantics.md) | proposed | provider-declared credential purposes, schemes, flows, scopes |
| `auth.acquisition/v1alpha1` | [auth/acquisition/v1alpha1](auth/acquisition/v1alpha1/semantics.md) | proposed | begin/complete/refresh/revoke/repair flows: OAuth2 code, client credentials, static entry |
| `auth.custody/v1alpha1` | [auth/custody/v1alpha1](auth/custody/v1alpha1/semantics.md) | proposed (extends `SecretStore`) | immutable secret versions, credential sets, CAS active reference |
| `auth.capability/v1alpha1` | [auth/capability/v1alpha1](auth/capability/v1alpha1/semantics.md) | proposed (extends `AuthenticatedHttp`) | connection-bound runtime capabilities: HTTP bearer/basic/signing, mTLS, socket peer, exec plugin, SIP lease, session authority, inbound verifier, mediated HTTP |
| `auth.evidence/v1alpha1` | [auth/evidence/v1alpha1](auth/evidence/v1alpha1/semantics.md) | proposed | value-free readiness and provider-side authorization checks |
| `sessions/v1alpha1` | [sessions/v1alpha1](sessions/v1alpha1/semantics.md) | proposed | bidirectional sessions: offer, establishment, correlation, lease, revocation, terminal races, duplex transport |
| `media/v1alpha1` | [media/v1alpha1](media/v1alpha1/semantics.md) | proposed | negotiated tracks, duplex frames, readiness, loss, overload, DTMF, interrupt |
| `catalog/v1alpha1` | [catalog/v1alpha1](catalog/v1alpha1/semantics.md) | proposed | list/describe/locate pre-compiled adapter bundles with provenance and coverage; optional; grants nothing |
| `operations/v1alpha1` `generic-http` profile (and `datasource.records` `generic-http-page`) | [catalog/v1alpha1](catalog/v1alpha1/semantics.md) §3.2 | proposed | an operation realized from a bundle's request mapping by one generic engine: status, JSON passthrough, provenance, status-to-code table |

Inventory on 2026-09-08: 17 semantic documents (1 implemented service document and
16 proposed documents). The table has 5 implemented rows and 17 proposed rows;
multiple rows can share one document. An index entry records a proposal's presence,
not implementation support. [Service compatibility](service/compatibility.md) selects the proposed extended wire binding and records every family’s disposition, independently of semantic-family and adapter-kind versions.

Deferred families with no document yet: `execution`, `events`, `resources`,
`configuration` (`docs/design.md:174-186`).

## Which adapter needs which contract

| Adapter document | Contracts |
|---|---|
| [Atlassian](../docs/adapters/atlassian.md) | operations + mutation, records + document, auth.connection, auth.profile, auth.acquisition, auth.custody, auth.capability, auth.evidence |
| [Kubernetes](../docs/adapters/kubernetes.md) | records, endpoint_discovery, host_discovery (implemented) + logs, mutation, resource_discovery, route.mediated_http, auth.profile, auth.capability, auth.evidence |
| [Docker](../docs/adapters/docker.md) | operations + mutation, records, logs, endpoint_discovery, auth.profile, auth.capability, auth.evidence |
| [Grafana, Loki, Prometheus, Alertmanager](../docs/adapters/grafana.md) | records, logs, series, resource_discovery, route.mediated_http, auth.connection, auth.profile, auth.acquisition, auth.capability, auth.evidence |
| [Media session: SIP, RTVBP, bridge, local audio](../docs/adapters/media-session.md) | operations + mutation, sessions, media, auth.profile, auth.capability, auth.custody, auth.evidence, auth.connection |
| [Catalog and pre-compiled third-party specs](../docs/adapters/catalog.md) | catalog, operations `generic-http` + mutation, records `generic-http-page`, auth.profile, auth.acquisition, auth.capability (+ `http-header`, `http-query`), auth.evidence, auth.connection, auth.custody |

## Provider authentication at a glance

| Provider | Mechanisms (old source) | Profiles | Acquisition | Capability | Evidence |
|---|---|---|---|---|---|
| Jira | basic email+token; user OAuth2 code+refresh; service client_credentials; service bearer (`../connectors/providers/jira.toml:140-297`) | 4 | oauth2 code, client credentials, static entry | http-basic, http-bearer | scope, identity |
| Confluence | basic email+token; service bearer (`providers/confluence.toml`) | 2 | static entry | http-basic, http-bearer | scope |
| Kubernetes | bearer/token file, client certificate, exec plugin gated (`crates/integration-kubernetes/src/local.rs:311-315,1076-1079`) | 3 | static config | http-bearer, mtls, exec plugin | SelfSubjectReview, SelfSubjectAccessReview |
| Docker | none old; socket peer or TLS client certificate (to verify) | 2 | static config | socket peer, mtls | ping |
| Grafana | service-account bearer via connect session (`providers/grafana.toml`) | 1 | static entry | http-bearer | verify operation |
| Loki / Prometheus / Alertmanager | none declared; direct optional bearer/basic + tenant header; mediated via parent (`docs/design/08:150-160` old) | optional | static config | http-bearer/basic or mediated-http | verify operation |
| SIP | trunk username+password to sipx (`crates/driver-sip/src/lib.rs:104-114`) | 1 | static entry | sip-credential-lease | custody reachable |
| RTVBP | host-issued proof-bound authority + DPoP (`docs/design/05:297-305` old) | 1 (no vendor credential) | host issued | session-authority, inbound-verifier | redemption ledger |

## Document template

Every proposed contract document has ten sections: identity; old evidence and disposition; types; rules; ordering and limits; conformance scenarios; compatibility; SDK and host obligations; ESS entities; open decisions with defaults. Numeric limits marked "first-profile default" are starting values to be measured, not frozen (`docs/design.md:1146`).
