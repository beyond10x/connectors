---
format: aep.planning-md/3
id: architecture-decision-record:provider-definitions-and-runtimes
kind: architecture-decision-record
status: proposed
title: Declarative provider definitions live in their own repository and reach runtimes as one IR
relations:
- informed_by: architecture-decision-record:declarative-http-provider-runtime
- serves: vision:independent-contract-adapters
revision: 3
---
## Context

The operator finds Connectors slow to extend and harder to read than its function requires
(conversation of 2026-10-16). A read-only survey of the tree on that date, with claims spot-checked
in code, found:

- Ordinary HTTP providers already need no Rust. Pinned OpenAPI, Swagger 2.0 or Google Discovery
  sources plus a selection file (`adapters/catalog/providers/<id>/operations.json`, format
  `connectors-catalog-operations/1`) are lowered by `connectors-build catalog`
  (`crates/connectors-build/src/main.rs:98-108`) into `adapters/catalog/generated/bundles/<id>.bundle.json`,
  and one engine executes them (`adapters/catalog/src/lib.rs:1-4,793-831`). 14 bundles exist
  (`adapters/catalog/generated/bundles/index.json`).
- Bundles are read from disk at runtime through absolute paths in instance configuration
  (`adapters/catalog/src/local.rs:174,196,800-817`), but release archives ship no bundles
  (`.github/workflows/release-binaries.yml:107,209-213`). Users point configuration at a checkout
  (`docs/catalog-hubspot.md:108`). There is no install or fetch step.
- A bundle names its auth profile only (`"auth_profile": "hubspot.private-app"`); OAuth URLs and
  scopes sit in operator configuration (`docs/catalog-google-drive.md:208-228`). The provider-owned
  profile `auth.profile/v1alpha1` is "proposed, not implemented"
  (`contracts/auth/profile/v1alpha1/semantics.md:3`). The consent flow hard-codes Google's shape:
  `openid`, PKCE S256, `access_type=offline`, `prompt=consent`
  (`crates/connectors-host/src/local/oauth.rs:203-227`).
- Adding a provider is a pull request to this repository, whose gate builds and tests the whole
  workspace (`crates/connectors-build/src/gate.rs:149-150`): about 240k lines of Rust and 222
  integration test binaries, with `incremental = false` in the dev and test profiles
  (`Cargo.toml:62-74`).
- 17 consumers outside this repository use it as crates, CLI or data; two already keep provider
  definitions as files outside the binary, one pinned by SHA-256. Three pin crate names this tree
  no longer has (`connectors-cli`, `connectors-runtime`, `protocol`, `service`).
- Documentation contradicts the code in at least 13 places (listed in the companion design), and
  one deployable has four names: adapter, adapter service, composition, catalog provider.

[architecture-decision-record:declarative-http-provider-runtime](declarative-http-provider-runtime.md)
already decided declarative templates, provider-owned auth profiles, digest-verified bundles and an
optional catalog service. This record decides where those definitions live, what format other
runtimes implement, how they reach a host, and what a simplification must keep. It refines that
record and supersedes nothing in it.

## Decision

**D1. Provider definitions move to their own repository.** The repository is
`beyond10x/connectors-definitions`, created on 2026-10-16 at the operator's choice
(https://github.com/beyond10x/connectors-definitions). It holds one authored
YAML file per declarative provider, the pinned upstream references with their digests, a README, an
AGENTS.md and a Taskfile. It holds no Rust and no build of Rust. Native connectors (Kubernetes, SQL,
MCP, anything that starts a host process or reads host credentials) stay in this repository.

**D2. One intermediate representation, `connectors-provider/1`, is the contract between
definitions and runtimes.** It is the current bundle plus selection, reduced to the selected
operations, with the auth profiles and transport binding inline: one canonical JSON document per
provider. Its schema, semantics and conformance scenarios are owned here under `contracts/catalog/`.
A definition declares the features it requires (for example `http`, `oauth2_authorization_code`,
`feed`, `binary`); a runtime refuses by name a definition that requires a feature it lacks.

**D3. Source formats are frontends, separate from runtimes.** OpenAPI 3.x, Swagger 2.0, Google
Discovery and the authored YAML (or the existing TOML action) are lowered into
`connectors-provider/1` by frontends. The reference frontends are the existing Rust ones in
`crates/connectors-catalog`, exposed through a released `connectors` subcommand. A runtime needs no
frontend. The definitions repository's CI runs the released binary to lower, validate and publish.

**D4. Each tag of the definitions repository publishes the lowered documents.** A release carries
`<provider>.provider.json` per provider and one `index.json` naming each file's SHA-256, the source
commit and the frontend version. Authored YAML is the reviewed source; the published JSON is what
runtimes load. Lowering is deterministic: the same commit and frontend version give the same bytes.

**D5. `connectors` installs definitions explicitly and verifies them.** A command fetches a named
tag from a configured source, checks every file against `index.json`, records source, tag, commit
and index digest in a lock file, and stores the files in a local cache keyed by the index digest.
Instance configuration names a provider and the locked index instead of an absolute path. Install
and update grant nothing: no connection, no operation admission, no OAuth exchange, no write
authority. A digest mismatch, an unknown format version or an unmet feature is a refusal by name.
Nothing updates implicitly.

**D6. Auth profiles live in the definition and the OAuth flow becomes profile-driven.** The
definition carries `auth.profile/v1alpha1` declarations (authorize and token URLs, PKCE mode,
grants, extra authorize parameters, scopes, identity check, vendor source). The host's consent
flow reads those fields instead of the hard-coded Google parameters. Client secrets, tokens and
other credential material never appear in a definition; they stay in custody.

**D7. A runtime has a role and a feature set, and both are proven by conformance.**

| Role | Holds | Must not |
|---|---|---|
| `executor` | runs `connectors-provider/1` operations with an auth capability a host injects | hold custody, decide admission or approval, serve callers, write the audit record itself |
| `host` | custody, admission, approval, audit, child-process confinement, serving, federation | — |

The Rust implementation is the reference executor and the only host. A runtime in another language
(Go, Node) starts as an executor, either embedded under the Rust host as a pinned child process over
the existing private protocol or under a host of its own that passes the host suite. Each runtime
publishes a manifest (`connectors-runtime/1`: language, version, role, formats, features, frontends);
the support matrix is generated from conformance reports, never from the manifest alone.

**D8. Operation semantics are separate from transport.** An operation is a request and a response.
Its binding names the transport: `http` (implemented), or `duplex`, which carries the same request
and response as correlated messages on a channel the host opened, such as a WebSocket envelope.
`duplex` is reserved until a provider needs it. Long-lived sessions and media keep their own
contracts (`contracts/sessions`, `contracts/media`); they are not request/response.

**D9. Simplification has a floor.** Vocabulary, documentation and structure are simplified; no
security, confinement, audit or monitoring guarantee is removed or weakened, and no new path
reaches a provider with fewer guarantees than the current host path. The floor and the existing
gaps are listed in the companion design. A gap found during this work is filed as its own
artifact; the split does not wait on closing it and does not widen it.

**D10. Every definition is usable with no runtime at all.** The definitions repository publishes,
per provider, a generated Markdown page lowered from the same `connectors-provider/1` document:
how to authenticate (each auth profile as numbered steps with `curl`, for example the authorize
URL, the code exchange and the refresh) and how to call each operation (method, URL template,
parameters, one `curl` line, the response shape). Credentials appear only as environment-variable
references such as `$JIRA_TOKEN`. A developer or an agent with no runtime in its language can act
from the page alone, and an agent can turn it into a skill. The page is generated, never authored,
so it cannot drift from the definition. This path is outside Connectors: it carries none of the
host's custody, admission, approval or audit, and each page says so in its first lines and names
`connectors` as the governed way to make the same call. D9's floor applies to paths through
Connectors; this path is equivalent to reading the vendor's documentation.

## Consequences

- Adding or changing a declarative provider is a change to the definitions repository only. Its
  gate runs a released binary over YAML and needs no Rust toolchain.
- This repository loses the provider data and the per-provider tests that only exercise data, and
  keeps the frontends, the engine, the host and native connectors.
- A second-language runtime implements one JSON format and a feature subset, and its standing is a
  conformance report. Before one exists, the generated page (D10) is the interface.
- A definition becomes a supply-chain input. It is data, never code; it cannot name a binary, a
  shell command or a credential; a change to a provider's hosts or scopes is visible in the
  reviewed YAML diff and in the installed index, and requires a new consent before use.
- Consumers that pin crates by commit are unaffected until they move; consumers that read bundles
  from a checkout move to the installed cache.

## Not decided here

- Whether upstream vendor documents are committed or only referenced by URL and digest; a
  license-by-license call, as `decision-blocker:confluence-openapi-redistribution` was.
- Signing of releases beyond digests.
- Build-time changes in this repository (`incremental = false`, test-binary count); a separate
  milestone.

## Sources

- Companion design: `architecture-design:provider-definitions-and-runtimes`.
- Prior decision: `architecture-decision-record:declarative-http-provider-runtime`.
- Contracts: `contracts/catalog/v1alpha1/semantics.md` (bundle transport, line 320),
  `contracts/auth/profile/v1alpha1/semantics.md`.
