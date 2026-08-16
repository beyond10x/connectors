# b10x/connectors

A unified integration platform for agent automation: a text-declared connector catalog compiled
to a canonical data artifact, plus a deployable service owning Connector-local connections,
credentials, grants, invocation, and event delivery. General identity remains Identity-owned;
hosted clients exchange login continuity at Identity for short-lived, exact-audience access
authority. Personal, organization, and SaaS are target postures, not claims of identical current
runtime maturity.

**Status: pre-v1, foundation phase.** The catalog family and personal-local runtime build. Bounded
hosted Operation, Connection, and Event APIs are available for Identity-authenticated,
deployment-owned Integrations. Receiver-owned operator-group, exact-scope, Integration, Connection,
and Grant admission gates hosted Slack setup, invocation, and events. The repository does not claim
the full SaaS or satellite surface.

- [docs/VISION.md](docs/VISION.md) — what this is, why, principles, non-goals.
- [docs/design/01-domain-model.md](docs/design/01-domain-model.md) — the nouns and their
  invariants.
- [docs/design/02-architecture.md](docs/design/02-architecture.md) — repository layers, ports,
  postures, and build order.
- [docs/design/03-beyond-http.md](docs/design/03-beyond-http.md) — interaction shapes, closed
  drivers, placement, and the direct byte-plane split.
- [docs/design/04-the-callers-contract.md](docs/design/04-the-callers-contract.md) — measured proof
  of the catalog document as the caller contract.
- [docs/guides/connect-slack.md](docs/guides/connect-slack.md) — the public-facing personal-local
  Slack connection flow.
- [docs/guides/connect-gitlab.md](docs/guides/connect-gitlab.md) — connect GitLab as yourself or as
  a bounded automation identity.
- [docs/stories/README.md](docs/stories/README.md) — sequenced design and implementation backlog.
- [docs/research/](docs/research/) — the platform-category survey and mined catalog-as-text
  precedents (with vendored primary sources under `docs/research/vendor/`).

This repository consolidates and succeeds `flux-connectors` and `flux-exchange`. The catalog and
ingestion pipeline migrate here largely as-is; the platform is a fresh design informed by what
those codebases proved and what they got wrong.

## Hosted server

`connectors serve-hosted --config PATH` serves the provider-credential-free Operation, Connection,
and Event contracts below `/api/connectors/v1` with
the documented development base path. It accepts only five-minute Identity access tokens, resolves
their complete validated authority envelope for the exact `urn:b10x:connectors` audience,
and derives backend owner/audit context from that envelope rather than caller-written fields. Login
sessions never enter this service. Each route requires its exact Identity-issued scope and a
deployment-admitted operator group; effect-bearing invocation additionally requires the
Connector-owned Grant join. The initial Kubernetes
Integration is intentionally one operation: `kubernetes.deployment.status`. It performs one
`GET /apis/apps/v1/namespaces/{namespace}/deployments/{name}` and projects the Deployment status;
the deployment must grant only `get` on Deployments in the configured namespaces. This API shape
and its static `Development cluster (read-only)` Connection in callable lifecycle state are
available through the two
contracts. Its status fields were checked against the target cluster's `apps/v1` discovery and
OpenAPI surface (EKS Kubernetes v1.34.9 at release time).

Hosted configuration is strict TOML and rejects unknown fields or inconsistent enablement:

```toml
tenant_id = "tenant-dev"

[server]
listen = "0.0.0.0:8080"
base_path = "/api/connectors/v1"

[identity]
origin = "https://identity.dev.b10x.example"

[authority]
operator_groups = ["operator"]

[storage]
state_root = "/var/lib/b10x-connectors"

[kubernetes]
enabled = true
namespaces = ["b10x"]

[vault]
enabled = true
address = "https://b10x-vault.b10x.svc:8200"
role = "b10x-connectors"
token_file = "/var/run/secrets/kubernetes.io/serviceaccount/token"
ca_file = "/etc/b10x-vault-ca/ca.crt"

[sip]
enabled = false
listen = "0.0.0.0:5060"

[slack]
public_origin = "https://code.example.test/api/connectors/v1"
grant_ref = "grant:slack:workspace-companion"
initiation = "provider"
allowed_events = ["app_mention"]
connect_session_ttl_seconds = 300
```

Kubernetes is enabled if and only if its namespace set is non-empty. SIP is enabled if and only if
both `listen` and `deployment_config` are present; that deployment config uses the existing strict
personal voice schema, and every SIP target must bind the configured listen address. See
[`crates/connectors-config/examples/hosted-dev.example.toml`](crates/connectors-config/examples/hosted-dev.example.toml).

Hosted Slack requires the Vault SecretStore. Its prepared transaction stages all three submitted
credentials in Vault, durably commits them, and only then publishes value-free Connection metadata.
The app-level `xapp` credential belongs to the Slack Integration and its Socket Mode supervisor;
each workspace Connection binds its `xoxb` bot credential and delegated `xoxp` user credential.
Configuration, the Connector state PVC, logs, audit, Agent, and model-facing contracts never store
or return those values. Kubernetes Deployment status still needs no provider secret and uses its
bounded ServiceAccount RBAC directly.

A local Zwirn client can use the hosted placement after Identity login:

```bash
zwirn connect hosted
zwirn connect hosted --connection "Development cluster (read-only)"
```

Identity publishes the exact trusted HTTPS base during login; an optional `--url` must match it
exactly before the keyring is opened. The client stores only that base, an account-binding digest,
and opaque Connection refs. For each request it sends the keyring-backed login session only to
Identity, exchanges it for a five-minute exact-scope access token, and sends only that access token
to Connectors. Provider credentials remain Connector/Vault-owned and never enter Operation,
Connection, or Event contracts. Selecting catalog metadata does not enable hosted invocation;
receiver-owned admission remains independent.

`/livez` reports process liveness. `/readyz` and the compatibility `/healthz` route perform a
bounded Identity readiness request and return `503` while opaque-token authority cannot be
resolved.
