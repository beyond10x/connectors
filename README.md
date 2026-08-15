# b10x/connectors

A unified integration platform for agent automation: a text-declared connector catalog compiled
to a canonical data artifact, plus a deployable service owning identity, connections,
credentials, grants, invocation, and event delivery. Clients authenticate once, hold one token,
and do everything their grants admit. Three deployment postures — personal, org, saas — with an
identical feature set.

**Status: pre-v1, foundation phase.** The catalog family and personal-local runtime build. A
bounded hosted operation API is available for Identity-authenticated, deployment-owned
Integrations; it does not yet claim the full SaaS or satellite surface.

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

`connectors serve-hosted --config PATH` serves `POST /v0alpha1/operations` and verifies each opaque
session with Identity for the exact `b10x.connectors` audience. The initial Kubernetes
Integration is intentionally one operation: `kubernetes.deployment.status`. It performs one
`GET /apis/apps/v1/namespaces/{namespace}/deployments/{name}` and projects the Deployment status;
the deployment must grant only `get` on Deployments in the configured namespaces. This API shape
and its status fields were checked against the target cluster's `apps/v1` discovery and OpenAPI
surface (EKS Kubernetes v1.34.9 at release time).

Hosted configuration is strict TOML and rejects unknown fields or inconsistent enablement:

```toml
tenant_id = "tenant-dev"

[server]
listen = "0.0.0.0:8080"

[identity]
origin = "https://identity.code.dev.babelforce.com"

[storage]
state_root = "/var/lib/b10x-connectors"

[kubernetes]
enabled = true
namespaces = ["b10x"]

[sip]
enabled = false
listen = "0.0.0.0:5060"
```

Kubernetes is enabled if and only if its namespace set is non-empty. SIP is enabled if and only if
both `listen` and `deployment_config` are present; that deployment config uses the existing strict
personal voice schema, and every SIP target must bind the configured listen address. See
[`crates/connectors-cli/examples/hosted-dev.example.toml`](crates/connectors-cli/examples/hosted-dev.example.toml).
