# b10x connectors

A unified integration platform for agent automation: a text-declared connector catalog compiled
to a canonical data artifact, plus a deployable service owning Connector-local connections,
credentials, grants, invocation, and event delivery. General identity remains Identity-owned;
hosted clients exchange login continuity at Identity for short-lived, exact-audience access
authority. Personal, organization, and SaaS are target postures, not claims of identical current
runtime maturity.

**Status: pre-v1, foundation phase.** The catalog family and personal-local runtime build. Bounded
hosted Operation, Connection, and Event APIs are available for Identity-authenticated,
deployment-owned Integrations. Receiver-owned operator-group, exact-scope, Integration, Connection,
and Grant admission gates hosted Slack setup, invocation, and events. The component does not claim
the full SaaS or satellite surface.

- [docs/VISION.md](docs/VISION.md) — what this is, why, principles, non-goals.
- [docs/design/01-domain-model.md](docs/design/01-domain-model.md) — the nouns and their
  invariants.
- [docs/design/02-architecture.md](docs/design/02-architecture.md) — component layers, ports,
  postures, and build order.
- [docs/design/03-beyond-http.md](docs/design/03-beyond-http.md) — interaction shapes, closed
  drivers, placement, and the direct byte-plane split.
- [docs/design/04-the-callers-contract.md](docs/design/04-the-callers-contract.md) — measured proof
  of the catalog document as the caller contract.
- [docs/design/11-hosted-module-request-authority.md](docs/design/11-hosted-module-request-authority.md)
  — Connector-only, request-bound Work/Ontology authority and tenant/user attribution.
- [docs/guides/connect-slack.md](docs/guides/connect-slack.md) — the public-facing personal-local
  Slack connection flow.
- [docs/guides/connect-gitlab.md](docs/guides/connect-gitlab.md) — connect GitLab as yourself or as
  a bounded automation identity.
- [docs/guides/connect-jira.md](docs/guides/connect-jira.md) — deployment-owned read-only Jira and
  delegated, approval-gated user writes.
- [docs/stories/README.md](docs/stories/README.md) — sequenced design and implementation backlog.
- [docs/research/](docs/research/) — the platform-category survey and mined catalog-as-text
  precedents (with vendored primary sources under `docs/research/vendor/`).

This component consolidates and succeeds `flux-connectors` and `flux-exchange`. The catalog and
ingestion pipeline migrate here largely as-is; the platform is a fresh design informed by what
those codebases proved and what they got wrong.

## MCP directions

Connectors has two deliberately different MCP roles. The hosted `/mcp` endpoint is an inbound
transport onto already-governed Connector operations. Outbound MCP is a generated service adapter:
a reviewable profile pins the complete remote tool snapshot, maps every tool to a local operation,
and then joins the same deployment overlays, Grants, approvals, credential store, and
Connection-bound egress as other Integration adapters. It does not read Harness's local MCP
registry and never interprets server annotations as authority. See
[design 18](docs/design/18-governed-outbound-mcp-services.md).

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
module_tenant_ids = ["tenant-dev"]

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

[platform]
tenant_member_modules = ["ontology", "planner", "work"]
work_origin = "http://b10x-work:8080"
ontology_origin = "http://b10x-ontology:8080"
planner_origin = "http://b10x-planner:8080"
module_signing_key_file = "/var/run/b10x-module-auth/private.pem"
module_signing_key_id = "developer-1"
module_signing_issuer = "b10x-connectors"

[slack]
public_origin = "https://code.example.test/api/connectors/v1"
grant_ref = "grant:slack:workspace-companion"
initiation = "provider"
allowed_events = ["app_mention"]
connect_session_ttl_seconds = 300
```

Activated GitLab, Slack, and Jira OAuth registrations keep client IDs and other non-secret policy
in this configuration. Their client secrets are written once through the operator-only admin API,
directly into the configured SecretStore. See
[Administer hosted Integrations](docs/guides/administer-hosted-integrations.md); no provider secret
needs to enter the public repository, deployment values, or CI.

Kubernetes is enabled if and only if its namespace set is non-empty. SIP is enabled if and only if
both `listen` and `deployment_config` are present; that deployment config uses the existing strict
personal voice schema, and every SIP target must bind the configured listen address. See
[`crates/connectors-config/examples/hosted-dev.example.toml`](crates/connectors-config/examples/hosted-dev.example.toml).

`module_tenant_ids` is the exact sorted set this Connector deployment may project to hosted module
origins; an empty value retains the single `tenant_id` compatibility posture. Every Work and
Ontology request is signed only after the request's verified tenant is in that set and its exact
operation is admitted. The Babelforce developer config deliberately contains one tenant, while the
request signer and tenant-partitioned event store are exercised with multiple tenants. Static
Ontology bearer configuration is rejected, and signing failure never falls back to direct HTTP.

Hosted Slack requires the Vault SecretStore. Its prepared transaction stages all three submitted
credentials in Vault, durably commits them, and only then publishes value-free Connection metadata.
The app-level `xapp` credential belongs to the Slack Integration and its Socket Mode supervisor;
each workspace Connection binds its `xoxb` bot credential and delegated `xoxp` user credential.
Configuration, the Connector PostgreSQL database, logs, audit, Agent, and model-facing contracts
never store or return those values. Kubernetes Deployment status still needs no provider secret and uses its
bounded ServiceAccount RBAC directly.

A local Connectors client can select the hosted placement once and then use it without endpoint or
token flags:

```bash
connectors login https://connectors.example.test/api/connectors/v1
connectors operation search
connectors connection list
connectors mcp
connectors logout
```

`connectors completions <shell>` prints a completion script for `bash`, `zsh`, `fish`, `elvish`
or `powershell`, generated from the same command tree that parses the arguments. Write it where
the shell reads completions at start-up, for example
`connectors completions fish > ~/.config/fish/completions/connectors.fish`.

The Connectors deployment publishes an unauthenticated bootstrap document naming the neutral
Identity origin and exact Connector audience it trusts. Browser login returns an opaque session
which is stored only in the operating-system keyring; non-secret account and deployment selection
is stored beneath XDG state. Each request exchanges the session only with Identity for the smallest
required Connector scope, caches the five-minute access token in memory, and renews it before
expiry. `connectors mcp` keeps stdout exclusively for MCP messages and never exposes either kind of
credential to its caller. Passing an explicit local `--config` or `--state-root` continues to select
the personal-local placement. Provider credentials remain Connector/Vault-owned and never enter
Operation, Connection, Event, or MCP contracts; receiver-owned admission remains independent.

`/livez` reports process liveness. `/readyz` and the compatibility `/healthz` route perform a
bounded Identity readiness request and return `503` while opaque-token authority cannot be
resolved.

<!-- b10x-docs:start -->
## Documentation

[Connectors documentation](https://beyond10x.github.io/docs/connectors/) · [Start](https://beyond10x.github.io/) · [Ecosystem](https://beyond10x.github.io/ecosystem/) · [Impact](https://beyond10x.github.io/changes/) · [Releases](https://beyond10x.github.io/releases/)
<!-- b10x-docs:end -->
