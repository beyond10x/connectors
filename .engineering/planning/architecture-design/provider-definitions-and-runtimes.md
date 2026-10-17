---
format: aep.planning-md/3
id: architecture-design:provider-definitions-and-runtimes
kind: architecture-design
status: draft
title: Connector definitions repository, lowerers, install, runtime roles and the security floor
relations:
- designs: architecture-decision-record:provider-definitions-and-runtimes
- informed_by: architecture-decision-record:declarative-http-provider-runtime
revision: 5
---
## 1. Purpose

Realizes `architecture-decision-record:provider-definitions-and-runtimes`: declarative connector
definitions in their own repository, one intermediate representation (IR) that any runtime can
execute, an explicit install step in `connectors`, profile-driven OAuth, runtime roles proven by
conformance, a transport binding separate from operation semantics, a page per provider usable with
no runtime, and a simplification that keeps every security guarantee.

Facts in § 2 and § 9 come from a read-only survey of `origin/main` at `74d8d857d9` (release 0.52.0)
on 2026-10-16; the claims marked *checked* were re-read in code for this design. Everything from
§ 3 on is proposal.

## 2. Current state

| Layer | Today | Source |
|---|---|---|
| Sources | Pinned OpenAPI 3.x, Swagger 2.0, Google Discovery, local TOML action | `crates/connectors-catalog/src/{swagger,discovery,authored}.rs` |
| Selection | `adapters/catalog/providers/<id>/operations.json`, `connectors-catalog-operations/1`, 13 providers | e.g. `adapters/catalog/providers/hubspot/operations.json` |
| Lowering | `connectors-build catalog`, offline, no `build.rs` anywhere | `crates/connectors-build/src/main.rs:98-108,237-263` |
| IR | `<id>.bundle.json` with `provider`, `source`, `inventory`, `auth_profile`; full inventory, gitlab 980 KB | *checked*: `adapters/catalog/generated/bundles/hubspot.bundle.json` |
| Runtime | one `Engine`, no per-operation Rust | `adapters/catalog/src/lib.rs:1-4,793-831` |
| Distribution | absolute `bundle_directory` path; release ships no bundles | `adapters/catalog/src/local.rs:174,196`; `.github/workflows/release-binaries.yml:107,209-213` |
| Auth in definition | profile name only | *checked*: `"auth_profile": "hubspot.private-app"` |
| OAuth consent | Google-shaped: `openid` added, PKCE S256, `access_type=offline`, `prompt=consent` | *checked*: `crates/connectors-host/src/local/oauth.rs:203-227` |
| Native connectors | grafana, kubernetes, loki, prometheus, sql, tavily: Rust per operation | `Cargo.toml:4` |

## 3. Target layers

```
sources                frontends (lowering)        IR                        runtimes
---------------------  --------------------------  ------------------------  -------------------------
authored YAML       \                              connectors-provider/1     Rust host + executor
OpenAPI 3.x          >  Rust, in connectors,    ->  one JSON per provider  -> Go/Node executor
Swagger 2.0         /   run by definitions CI      + index.json (digests)     (under a host)
Google Discovery   /                                        |
                                                            +-> generated page per provider
                                                                (auth steps + curl, no runtime)
```

Frontends and runtimes meet only at the IR. A runtime has no frontend unless it chooses to.

## 4. The definitions repository

```
README.md               humans: what is here, how to add a provider, link to Connectors
AGENTS.md               agents: commands, the no-secrets rule, the gate
Taskfile.yml            lower, validate, pages, check
connectors.lock         the released connectors version the gate lowers with, by digest
connectors/<id>.yaml     authored: upstream pin, base URL, auth profiles, selection, overrides
upstream/<id>.json      url, sha256, bytes; vendor bytes only where the license allows
```

The authored format `connectors-definition/1` is owned by the definitions repository: its ESS source
is `spec/domains/definition.yaml` there, projected to JSON Schema, and `task check` already validates
every `providers/*.yaml` against it with `ess` and `yq` alone
(https://github.com/beyond10x/connectors-definitions/pull/1). Once the frontend exists, `task check`
also downloads the pinned `connectors` release, lowers every provider, validates each IR document
against `connectors-provider/1`, regenerates the pages and fails on drift. No Rust toolchain is
installed.

Authored file, illustrative (field names are fixed with the schema, not here):

```yaml
format: connectors-definition/1
provider: jira
title: Jira Cloud
upstream:
  kind: openapi
  url: https://developer.atlassian.com/cloud/jira/platform/swagger-v3.v3.json
  sha256: "<hex>"
base_url: https://api.atlassian.com/ex/jira/{cloud_id}
auth:
  - id: jira.user_oauth
    purpose: delegated_user
    scheme: http_bearer
    acquisition:
      flow: oauth2_authorization_code
      authorize_url: https://auth.atlassian.com/authorize
      token_url: https://auth.atlassian.com/oauth/token
      pkce: none
      authorize_params: {audience: api.atlassian.com, prompt: consent}
    scopes: {requestable: [read:jira-work, offline_access], minimum: [read:jira-work]}
    sources: [https://developer.atlassian.com/cloud/jira/platform/oauth-2-3lo-apps/]
operations:
  - {id: issue.get, operation_id: getIssue, effect: read, description: "Read one issue by key"}
```

A provider with no vendor document lists `method`, `path` and `parameters` per operation instead of
`upstream` and `operation_id`; this is the TOML action of the prior ADR in YAML.

## 5. `connectors-provider/1`

One canonical JSON document per provider (sorted keys, no insignificant whitespace), so its SHA-256
is stable.

| Field | Content |
|---|---|
| `format`, `provider`, `version` | identity |
| `provenance` | upstream URL and SHA-256, authored file SHA-256, source commit, frontend name and version |
| `requires` | features a runtime must support: `http`, `oauth2_authorization_code`, `oauth2_client_credentials`, `feed`, `binary`, `duplex`, … |
| `base_url` | template with named variables supplied by connection configuration |
| `auth[]` | `auth.profile/v1alpha1` declarations, non-secret |
| `operations[]` | selected operations only, resolved: `id`, `effect`, `description`, `method`, `path`, `parameters[{name, location, type, required, repeated}]`, body template, response handling, guard, paging or feed, binary, `binding` (default `http`) |
| `inventory` | counts only (`selected`, `unsupported`); the full inventory stays a provenance artifact of the definitions release |

Schema, semantics and scenarios: `contracts/catalog/` in this repository, next to the existing
`generic-http` profile. The current bundle format remains readable during migration (§ 11).

## 6. Distribution and install

| Item | Proposal |
|---|---|
| Release | tag `vX.Y.Z` → `<id>.provider.json`, `<id>.md`, `index.json` (per-file SHA-256, source commit, frontend version) |
| Commands | `connectors definitions install --source <git url> --ref <tag>`, `… list`, `… verify`, `… update --ref <tag>` |
| Lock | `definitions.lock` in the connectors config directory: source, tag, commit, index SHA-256 |
| Cache | `<data dir>/connectors/definitions/<index sha256>/`, read-only after verify |
| Configuration | an instance names `provider: jira` and resolves it through the lock; no absolute path |
| Refusals by name | digest mismatch; unknown `format`; an unmet `requires`; a tag that moved to another commit |
| Grants | none. Install does not create a connection, admit an operation, start OAuth or enable writes |
| Re-consent | a definition update that changes a profile's hosts, token URL or requested scopes marks the connections using it for re-consent before use |

## 7. Profile-driven OAuth

`oauth.rs` reads from the selected profile: `authorize_url`, `token_url`, `pkce` (`none` or
`S256`), `grants`, `authorize_params`, requested scopes, and the client registration kind. It stops
adding `openid`, `access_type` and `prompt` itself; the Google definitions declare them. Client
registration accepts a generic `{client_id, client_secret?}` entry into custody besides the Google
installed-client JSON. Refresh, custody and revalidation keep their current owners
(`architecture-decision-record:oauth-material-as-static-entry`). Every existing OAuth scenario
must pass unchanged with the Google parameters moved into data.

## 8. Runtime roles, features, matrix, transports

Roles are in the ADR (D7). A runtime's manifest:

```json
{"format": "connectors-runtime/1", "name": "connectors", "language": "rust", "version": "0.53.0",
 "role": "host", "formats": ["connectors-provider/1"],
 "features": ["http", "oauth2_authorization_code", "oauth2_client_credentials", "feed", "binary"],
 "frontends": ["openapi-3.0", "openapi-3.1", "swagger-2.0", "google-discovery", "connectors-definition/1"]}
```

The matrix page lists, per runtime and feature, the conformance report that proves it; a manifest
claim with no passing report shows as unproven. An executor in another language runs under the Rust
host the way native adapters do today: a pinned binary by SHA-256, empty environment, fd 3 only,
killed with its parent (`crates/connectors-host/src/local/runtime/process.rs:47-99`).

Transport binding: `http` maps the request to one HTTP exchange. `duplex` (reserved) maps it to a
request message with a correlation id on a channel the host opened and admitted, and the response
to the message carrying the same id, with the same deadline, size bound and audit anchor as `http`.
Executor code dispatches by operation; the binding is chosen by the definition, not by the
executor. The host's WebSocket support already frames over a connection it admitted
(`crates/connectors-host/src/http.rs:559-653`).

## 9. Security floor

Guarantees that every stage keeps, from the survey:

| Guarantee | Source |
|---|---|
| Adapter children: ELF, SHA-256 pinned, run from the verified file, empty env, null stdio, fd 3, die with parent, 10 s spawn limit | `crates/connectors-host/src/local/runtime/process.rs:47-99`; `runtime/artifact.rs:46,102` |
| Kubernetes exec credential plugin: pinned path and SHA-256, no shell, env allowlist and `LD_`/`BASH_ENV` refusals, 10 s, 64 KiB, Linux only | `adapters/kubernetes/src/exec_credential.rs:7-8,43-44,95-136,295-300,620-643` |
| HTTP egress: HTTPS unless allowed, no userinfo, header denylist, no proxy, no redirects, no retries, 5 s / 15 s | `crates/connectors-host/src/http.rs:140-199` |
| Admission: not exposed → `not_found`, not granted → `forbidden`, stale description refused | `crates/connectors-host/src/local/owner.rs:409-440` |
| Local writes: Ed25519 approval proof, one-time spend, re-resolution before dispatch | `local/owner/mutation/execution.rs:97-169`; `approvals/spend.rs:11-25` |
| Audit anchor and final observation; capacity refuses, never evicts | `local/audit/types.rs:117-181`; *checked*: `local/service.rs:26` |
| Custody: four OS backends, no file or env fallback, 64 KiB cap, zeroize | `crates/connectors-custody/src/lib.rs:1-55` |
| Local IPC: same uid, mode `&0o077 == 0`, peer credential | `local/owner/transport.rs:776-781` |
| Federation: one hop, loop detection, refresh on stale description, no replay | `crates/connectors-host/src/federation.rs:125-196` |

Gaps that exist today. The split does not widen any of them, and each is filed separately:

| Gap | Source |
|---|---|
| HTTP serving has one shared bearer; every caller is the principal `static-bearer` | *checked*: `local/service.rs:25`, `server.rs:88-103` |
| HTTP-host writes record `not_required` approval | *checked*: `local/service.rs:186-200` |
| Adapter children receive the credential document on each call | *checked*: `local/runtime/process.rs:499-515` |
| Local CLI reads write no audit anchor | survey: no anchor found |
| Audit store is not tamper-evident | survey: no chain or signature in `local/audit*` |
| No destination address filtering on egress | survey: not found in `http.rs` |
| Service mode reads credentials from env or file | `crates/connectors-host/src/credentials.rs:17-28` |

New risks from the split, and their answer:

| Risk | Answer |
|---|---|
| A definition redirects calls to another host | hosts are explicit in the reviewed YAML diff and the installed index; a change marks connections for re-consent (§ 6) |
| A definition carries code | the IR has no field for a command, script, binary or credential; schema validation refuses unknown fields |
| A tampered or moved release | digests from `index.json`, the lock pins tag and commit, a moved tag is refused |
| A second-language executor weakens guarantees | it runs as a confined child of the Rust host, or under a host that passes the host suite |
| The no-runtime page (ADR D10) bypasses the host | it is outside Connectors, says so in its first lines, and uses env-var references only |

## 10. Simplification

| Area | Proposal |
|---|---|
| Public noun | "connector": a connector definition plus the runtime that runs it. "declarative connector" for IR-driven ones, "native connector" for Rust ones. "adapter" stays as the Rust term inside this repository. "catalog provider" is retired as a user-facing name |
| Overloaded words | "provider" means the upstream vendor only; "owner" means the local supervisor process only; "plugin" means the Kubernetes exec credential plugin only |
| Duplicated types | shared ESS has 642 named types in 24 domains; `LocalHost` is declared 12 times, `ServiceHost` 3, and `NextAction`, `AttemptReference`, `Provenance`, `Failure`, `ErrorCode` twice each: consolidate |
| Documentation | README states only what a test holds; support claims come from `crates/connectors-docs` status data |
| Repository shape | after migration: `crates/` (frontends, engine, host, custody, client), `adapters/` (native only), `contracts/`, `ess/`; provider data leaves `adapters/catalog/providers` and `generated/bundles` |

Documentation drift to correct, from the survey:

| Doc | Says | Code |
|---|---|---|
| `README.md:104` | three decisions open | all three `cleared` |
| `README.md:141-142` | only process execution is `pods.exec` | exec plugin and adapter children spawn processes |
| `README.md:34` | `ess/15` | `ess/system.yaml:1` `ess/18` |
| `README.md:8` vs `:247` | live site linked; site "not yet deployed" | contradicts itself |
| `VISION.md:80-83` | newer runtime not implemented | approvals, audit, ledger implemented |
| `contracts/README.md:16-39`, `contracts/operations/v1alpha1/semantics.md:3` | cli, logs, custody, catalog, mutation runtime proposed or not implemented | shipped |
| `adapters/README.md:67` | catalog is a design | implemented; Slack, Google, Homer missing from owners |
| `website/docs/index.md:46,55` | custody is the Secret Service keyring | four backends |
| `docs/design.md:847-850` | `connectors serve` is future | `apps/connectors/src/legacy.rs:157` |
| `docs/design.md:843` | OS matrix open | macOS and Windows CI |
| `docs/design.md:984` | § 31 stores not implemented | implemented |
| `docs/design.md:242-250` | package table | lacks custody, catalog, build, docs crates |

## 11. Migration

| Stage | Outcome | Done when |
|---|---|---|
| M1 | `connectors-provider/1` schema, semantics, scenarios; frontend emits it beside the bundle | the engine runs every provider from either format with identical results |
| M2 | profile-driven OAuth (§ 7) | Google scenarios pass with parameters from data; a second provider runs authorization code |
| M3 | `connectors definitions install/list/verify/update` | install from a local directory and from a git tag; every refusal in § 6 has a test |
| M4 | the definitions repository (operator creates it), 14 providers moved, `task check` green, first tag | `connectors` on the release installs that tag and runs one read per provider in fixtures |
| M5 | generated pages per provider (ADR D10) | page regenerated in `task check`, drift fails it; one page followed by hand end to end |
| M6 | provider data removed here; docs corrected (§ 10) | gate green; drift table empty |
| later | runtime manifest and matrix; a Go executor; `duplex`; build-time work in this repository | each its own artifact |

## 12. Not covered

Native connectors stay as they are. Closing the § 9 gaps is separate work. Session and media
contracts are unchanged. Consumers outside this repository are migrated by their owners; the
bundle format stays readable until they have moved.

## 13. Revision 2026-10-17: two layers

On 2026-10-17 the ADR (revision 4) replaced the single lowered document of §§ 3–5 with two
layers. A definition, `schema: urn:beyond10x:connectors-definition:1`, names a vendored source and
selects operations by `operationId`; a lowerer per target language builds that runtime's internal
representation. In Rust that is today's bundle plus selection.

The vertical slice, `story:definition-lowerer-slice`:

| Check | Result |
|---|---|
| `crates/connectors-catalog/tests/definition.rs` | 18 passed |
| Jira definition (3 reads) lowered vs `adapters/catalog/generated/bundles/jira.bundle.json` | SHA-256 `96a7508c…` on both |
| GitLab definition (7 reads, YAML source, 4 amendments) lowered vs `gitlab.bundle.json` | SHA-256 `79ce6c95…` on both |
| Catalog provider serves `issue.get` from the lowered Jira output | `adapters/catalog/tests/jira.rs` |

Not yet carried by the definition, so lost on lowering today: `bounds`, guards, bodies, paging,
curated descriptions (the OpenAPI `summary` is used), and `base_url`, `auth.scheme` and `config`
defaults, which the bundle has no field for. Schema v2 adds them section by section; its inputs are
`docs/drafts/schema-v2-inputs.md` in the definitions repository.
