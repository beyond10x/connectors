---
format: aep.planning-md/3
id: architecture-decision-record:provider-definitions-and-runtimes
kind: architecture-decision-record
status: accepted
title: Connector definitions live in their own repository and reach each runtime through its lowerer
relations:
- informed_by: architecture-decision-record:declarative-http-provider-runtime
- serves: vision:independent-contract-adapters
revision: 11
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-17T12:17:39Z", actor: "human:timo", revision: 8}
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

Vocabulary follows `architecture-decision-record:connector-vocabulary`: a provider is the company
(Atlassian, Google), a connector one service or API it offers (Jira, Gmail), an auth profile one
way to authenticate to a connector. A connector definition is one file `connectors/<id>.yaml` in
the definitions repository. Planning ids of this record keep their original wording.

## Decision

**D1. Connector definitions move to their own repository.** The repository is
`beyond10x/connectors-definitions`, created on 2026-10-16 at the operator's choice
(https://github.com/beyond10x/connectors-definitions). It holds one YAML file per declarative
connector, the vendored upstream documents, the definition schema's ESS source, a README, an
AGENTS.md and a Taskfile. It holds no Rust and no build of Rust. Native connectors (Kubernetes, SQL,
MCP, anything that starts a host process or reads host credentials) stay in this repository.

**D2. Two layers: a short definition, and an internal representation a lowerer builds.** A
definition, `connectors/<id>.yaml`, starts with `schema: urn:beyond10x:connectors-definition:1`. It
names a vendored source and says only what that source cannot: which operations to expose under
which local key (`issue.get: {op: getIssue}`), configuration variables with defaults (`hostname`,
default `gitlab.com`), auth profiles, and the sections v2 adds. Sections are maps keyed by id, so an
operation is unique on its connector by construction. The file is written for few tokens; how a
runtime names an operation atomically (`jira.issue.get`) is the lowerer's concern. The schema's
source is ESS (`spec/domains/definition.yaml` in the definitions repository), projected to JSON
Schema with that URN as `$id`, and `task check` validates every definition with `ess` and `yq`.

The internal representation is each runtime's own. In Rust it is today's bundle plus selection, so
the catalog engine runs a lowered definition unchanged. A definition section binds to a Connectors
contract by name (`operations/v1alpha1`, `datasource.feed/v1alpha1`, …), so every runtime exposes
the same behaviour as one trait or interface.

**D3. Every target language has a lowerer; source formats are read by lowerers, not by runtimes'
callers.** A lowerer reads a definition and its vendored source (OpenAPI 3.x first; Swagger 2.0
and Google Discovery later) and builds the internal representation. The reference is the Rust
lowerer, `connectors-build definition` over `crates/connectors-catalog/src/definition.rs`. A
runtime's capability manifest lists the source kinds and definition sections its lowerer supports.

**D4. Every upstream document is vendored.** The definitions repository commits each source
gzipped under `upstream/<id>/`, with its license, retrieval date and URL beside it. The definition
pins it by SHA-256, and `task check` and the lowerer both refuse a mismatch. A vendor removing a URL
does not make a definition unreproducible (operator, 2026-10-16; follows "vendor as is",
`decision-blocker:confluence-openapi-redistribution`, 2026-09-30). Each tag publishes the
definitions, the vendored sources and one `index.json` naming each file's SHA-256 and the commit.

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
| `executor` | runs lowered definitions with an auth capability a host injects | hold custody, decide admission or approval, serve callers, write the audit record itself |
| `host` | custody, admission, approval, audit, child-process confinement, serving, federation | — |

The Rust implementation is the reference executor and the only host. A runtime in another language
(Go, Node) starts as an executor, either embedded under the Rust host as a pinned child process over
the existing private protocol or under a host of its own that passes the host suite. Each runtime
publishes a manifest (`connectors-runtime/1`: language, version, role, definition sections, source kinds, features);
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
per provider, a Markdown page generated by the reference lowerer from the definition and its source:
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

- Adding or changing a declarative connector is a change to the definitions repository only. Its
  gate runs a released binary over YAML and needs no Rust toolchain.
- This repository loses the provider data and the per-provider tests that only exercise data, and
  keeps the lowerer, the engine, the host and native connectors.
- A second-language runtime implements one JSON format and a feature subset, and its standing is a
  conformance report. Before one exists, the generated page (D10) is the interface.
- A definition becomes a supply-chain input. It is data, never code; it cannot name a binary, a
  shell command or a credential; a change to a provider's hosts or scopes is visible in the
  reviewed YAML diff and in the installed index, and requires a new consent before use.
- Consumers that pin crates by commit are unaffected until they move; consumers that read bundles
  from a checkout move to the installed cache.

## Not decided here

- Signing of releases beyond digests.
- Build-time changes in this repository (`incremental = false`, test-binary count); a separate
  milestone.

## Sources

- Companion design: `architecture-design:provider-definitions-and-runtimes`.
- Prior decision: `architecture-decision-record:declarative-http-provider-runtime`.
- Contracts: `contracts/catalog/v1alpha1/semantics.md` (bundle transport, line 320),
  `contracts/auth/profile/v1alpha1/semantics.md`.

## Decision: D11 connector declaration

**D11. The definition owns the connector declaration; `connector.json` becomes lowered output.**
`architecture-decision-record:connector-vocabulary` adds
`adapters/catalog/providers/<catalog provider>/connector.json`
(`connectors-catalog-connector/1`): connector and provider ids, default endpoint, `api_path`,
the selection and the auth profile blocks. A connector definition states the same facts. The
definition owns them: the Rust lowerer writes `connector.json`, and a check holds the committed
file byte-identical to what the definition lowers to, as it already does for the bundle. A
configured instance's revision digests the selected auth block, the endpoint, the catalog provider
and the bundle and selection digests, not the order of `profiles[]`; the lowered blocks must keep
today's values, absent optional fields included, so the `derived_instance` and
`connector_declarations` tests stay green. The interim `<provider>.auth.json` the lowerer writes
until then is dropped. Agreed with the session that owns the vocabulary change on 2026-10-17.

## Acceptance

Accepted by the operator on 2026-10-17, with D1 to D11 as written; #179 and connectors-definitions #1 merge after the connector-vocabulary landing.

## Decision: D12 credential binding and invoking a definition file

**D12. A stored credential is bound to what it authenticates against, not to the operation list;
an owner can invoke a definition file without installing or configuring it.** The operator approved
this on 2026-10-18 (option "A" of the invoke-from-a-definition design). It amends D5 only as far as
point 4 says.

1. *What a connection is bound to.* A connection stays bound to the provider authority (endpoint
   host and instance id), the auth profile declaration (scheme, scopes, subject, evidence lifetime:
   the declaration revision the registry already keeps), the validated external identity, the
   trust inputs that decide where the credential may be sent (CA file, token CA, allowed hosts,
   request prefix), and the provider executable digest. It is no longer bound to the bundle, its
   vendored source or the selection, which only say which operations exist. Today all of these are
   folded into one `configuration_revision` (`adapters/catalog/src/local.rs` `Local::configure`)
   and `Binding` (`crates/connectors-host/src/local/registry.rs`) holds only that digest; the
   executable digest is in neither. The binding therefore gains a second digest over the credential
   side and the executable digest; the operation side keeps moving `configuration_revision`.
2. *What a change does.*

   | Change | Connection | Who acts |
   |---|---|---|
   | bundle, source or selection only | not stale; follows the new revision automatically on its next use or revalidation, with no credential source and no question | host |
   | provider executable digest only | not stale; follows automatically after the new executable's validation confirms the same identity | host |
   | CA, token CA, allowed hosts or request prefix | `pending`, `stale = true`, explicit `connections revalidate` (today's rule) | operator |
   | authority or profile authentication | `lifecycle_conflict` (today's rule), except point 3 | operator |

   Rationale: operation admission is checked per call (`permissions.operations` and the admitted
   selection). Binding the token to the operation list protects nothing, because the token can do
   whatever the provider allows whichever operations the selection lists. Approvals keep their own
   binding to the full configuration revision and executable selection; D12 does not change them.
3. *Invoke from a definition file.* `connectors operations invoke --definition <file.yaml>
   --operation <key>` (input as in every other `operations invoke`) lowers a
   `connectors-definition/1` file with the reference lowerer (D3,
   `crates/connectors-catalog/src/definition.rs` `lower()`) into an owner-only cache keyed by the
   digest of the definition and its source, computes the configuration revision the instance has,
   and invokes the operation through the host path every configured instance takes: custody,
   admission, approval and audit unchanged (D9). The operator's configuration file is not
   modified. The command asks the operator once, and only when no connection exists for the
   instance or its authority or profile changed: reuse a credential already stored in connectors
   (validated by the new profile, published only when the validated identity is the stored one), or
   connect fresh (token prompt, or the OAuth consent the host supports for the profile). Otherwise
   the operation runs directly. `invoke` alone stays the legacy explicit command.
4. *How far this amends D5.* D5 says install and update grant nothing; that stays true. Installing,
   updating or lowering a definition never creates a connection, admits an operation or performs an
   OAuth exchange. The amendment: an explicit `operations invoke --definition` by the owner is
   itself the request to connect and invoke, so after the one question it may connect and invoke in
   the same command. It adds no implicit update, skips none of D5's digest checks for a definition
   taken from an installed index, and does not let a definition name a binary, a shell command or a
   credential (Consequences: a definition is data, never code).

Not decided by D12: making the host's OAuth consent flow read a definition's authorize and token
parameters (D6, no story yet); persisting a lowered ad-hoc definition into the installed cache of
D5; signing.

Delivered by `story:connection-binding-excludes-operations`, `story:invoke-definition-file` and
`story:invoke-definition-reuse-credential`.

## Acceptance of D12

Approved by the operator on 2026-10-18 as option "A"; recorded by the planning session the same day.
D1 to D11 are unchanged.

Contract change for point 2 (2026-10-18, recorded with story:connection-binding-excludes-operations):
a read command (`operations describe`, `list`, `invoke`) starts the configured, digest-pinned
adapter without any credential when no description is cached for the configuration and the
instance holds a connection current under it, replacing the CLI contract's "never implicit launch".
