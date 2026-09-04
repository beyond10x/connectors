# 15 — A zero-configuration endpoint plane

Status: accepted 2026-08-24. Backs the `endpoint-plane` epic (S-058..S-061).
Timo's goal, verbatim intent: a person logs in with Google SSO and can immediately use the
platform's endpoints — no local configuration — through the local connectors CLI or the MCP
entry point. The hosted deployment is the pre-configured plane; the person brings only their
identity.

## What exists to build on (measured, 2026-08-24)

The dev deployment's Kubernetes integration is wired to the product namespace (`latest`), which
carries the whole endpoint inventory as Crossplane resources: 24
`database.mysql.sql.crossplane.io` and 23 `database.postgresql.sql.crossplane.io` managed
resources plus 35 connection secrets, alongside 78 SQL grants. The monitoring backend already
reaches the central Grafana on the infra cluster (origin, service-account token in Vault, and
fifteen Prometheus/Loki/Alertmanager targets across dev/staging/prod/infra are deployed
configuration today). The MCP transport (design 14) projects a role-filtered toolset; identity
issues self-service scoped tokens without operator (S-055).

## The three pieces

1. **Kubernetes as endpoint discoverer (S-059).** The hosted Kubernetes integration gains a
   read-only discovery datasource over the admitted namespaces' Crossplane surface: list the
   database managed resources, derive endpoint descriptors (engine, host, port, database name,
   secret reference — never credential values), and expose them as datasource bindings. The
   catalog discipline applies: discovery is read-only, effects declared, namespace-gated by the
   same `read_groups` as everything else.
2. **A generic SQL surface (S-058).** MySQL and PostgreSQL become connectors with read-only
   operations first (bounded query/list/describe-schema), executed by a SQL protocol driver in
   the mold of the existing non-HTTP drivers (`driver-sip`, `driver-cdp`); credentials arrive
   through the platform's custody (Kubernetes connection secrets via S-059's references, or
   Vault), never through user configuration. Effect posture: `read_only` ops ride the read
   path; anything mutating is a later, grant-gated story.
3. **The projected surface (S-060, S-061).** The MCP toolset grows the monitoring tools
   (Grafana/Prometheus/Loki/Alertmanager — the infra-cluster Grafana through the dev-cluster
   connectors) and, once S-058/S-059 land, the discovered-database tools. The local connectors
   CLI learns the zero-config hosted mode: `connectors session login <connectors-base>` reads the public
   discovery document served by that Connectors deployment, drives the neutral Identity loopback
   flow, and every subsequent hosted command uses short-lived tokens transparently (S-056's stdio
   bridge folds into this). This direction is deliberate: Connectors names its trusted Identity
   origin and audience; Identity does not acquire relying-party-specific endpoint metadata.

## Implemented amendment — native login and refresh (2026-09-02)

The projected client surface is now complete for existing Operation, Connection and Event APIs and
the inbound MCP transport. The CLI stores the opaque Identity session only in the OS keyring and
keeps only non-secret deployment/account selection under XDG state. Requests exchange that session
for the exact required Connector scope, reuse the access token in memory, renew within 30 seconds
of its five-minute expiry, and retry once with fresh authority after an authentication refusal.
Explicit local `--config` or `--state-root` flags continue to select the personal-local placement.

## Rules carried over

Every new surface funnels through the existing admission seam (design 13/14); discovery
publishes requirements and references, never secret bytes; the invariant families (rules 15-17)
extend to any new seam rather than being bypassed. Provider identities for `mysql` and
`postgresql` are chosen as permanent, per the adding-a-connector discipline.
