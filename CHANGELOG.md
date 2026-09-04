# Changelog

Notable changes to `beyond10x/connectors`, newest first. Versions follow
[semantic versioning](https://semver.org/spec/v2.0.0.html); before `1.0.0` a minor bump is where a
breaking change goes.

Nothing here publishes to a registry — every crate is an internal workspace member (design 02 §2).
The version is an **artifact identity**: `[workspace.package] version` is what
`catalog-build`'s `generator` string carries, so cutting a version rewrites every catalog document,
every `connectors.lock` row, and the wire User-Agent. Those three move together, always.

## Unreleased

Every first-level word of the shipped binary moves, and bare `connectors serve` stops starting the
local server: the breaking change the preamble puts in a minor bump.

### Changed

- **`connectors --help` lists eight words, not sixteen.** Ten first-level commands that were five
  different activities in one block are grouped: `setup` (`init`, `connect`, `completions`),
  `inspect` (`doctor`, `providers`, `auth` — which was `auth status`), `session` (`login`, `logout`)
  and `serve` (`local` — which was bare `serve` — `hosted` and `mcp`). `connection`, `event`,
  `operation` and `admin` stay where they were.
- **Every old path but one works for one more release.** `connectors doctor`,
  `connectors auth status`, `connectors serve-hosted --config …` and the rest are rewritten onto
  their new path before the arguments are parsed, produce the same output, and write one line to
  stderr naming where they went. Clap decides the rewrite: the global `-o`/`--output` may stand in
  front of the words or between them in any of its four spellings, and
  `connectors help <old word>` and `connectors auth help` answer as they did. The table that does
  it, `MOVED` in `crates/connectors-cli/src/lib.rs`, is removed in the release after this one. The
  one path not carried is the next entry.
- **`connectors serve` no longer starts the local server; `connectors serve local` does.** This is
  a break, not a deprecated path that still works: bare `connectors serve`,
  `connectors serve --help`, `connectors serve -h`, and `connectors serve` with nothing but global
  options in front of it or behind it — `connectors serve -o json`, `connectors -o json serve` —
  are the `serve` group, a `Commands:` listing as for `setup`, `inspect` and `session`, and exit 2
  without serving and without a note. A script that started the server with bare
  `connectors serve` has to say `connectors serve local` from this release on. `serve` and
  `serve -o json` are one invocation, so one of them starting a server while the other listed
  commands would be two commands under one name, which is the defect this release removed. Only
  `connectors serve --config …` and `connectors serve --state-root …` — the old leaf's own
  options, which the group refuses and `serve local` declares — are still rewritten onto
  `connectors serve local`, with the note.
- The hosted image starts on `connectors serve hosted`. `README.md`, the guides, `Taskfile.yaml`
  and the design pages name the new paths, and the fence that refuses shipped text naming an old
  one now reads all of them rather than Rust sources alone.
- The `connectors` command-line surface is declared in `ess/system/components.yaml`, and the clap
  tree projected from it is committed under `ess/generated/clap/` and held against the parser on
  every run (`docs/design/19-the-cli-surface.md`).

### Removed

- `CLI_TOTAL_LINE_LIMIT`, the cap on the thin frontend's line count in
  `crates/catalog-build/tests/main/architecture_fence.rs`. It was raised at every one of the six
  times it fired and never once moved a line out of the binary; `product_cli_is_a_thin_frontend`
  still bounds what the frontend may link and what it may declare.

## 0.5.11 — 2026-09-04

### Fixed

- Keep delegated GitLab repository reads alive after the two-hour OAuth access token expires.
  GitLab refresh responses may omit `scope`; Connectors now accepts that documented response shape
  and continues to verify the refreshed token's exact scopes through `/oauth/token/info` before
  committing the rotated access and refresh credentials.
- Make the CLI credential-store test create the owner-only state root required by the runtime,
  instead of depending on the hosted runner's `/tmp` permissions and blocking releases in CI.

## 0.5.10 — 2026-09-04

### Changed

- `connectors doctor`, `providers` and `auth status` render as a scannable report rather than a
  JSON dump. A list of records is one aligned row each, led by an ASCII severity marker that
  survives a pipe. The leading columns are laid out to a 120-column budget, spent on content
  rather than on column names, so that the last column *starts* within the first 120 terminal
  columns and everything before it is aligned there. It does not make a row fit 120 columns: the
  last column is never cut, so 66 of the 71 lines `connectors providers` prints are wider than
  that and the longest is 237. `doctor` goes from 26 lines for six checks to 9; `providers` keeps
  every catalogued id whole instead of wrapping at 927 lines.
- `-o compact` no longer drops fields. A record carries every scalar the value holds beside it, at
  every nesting depth, so `healthy` and the `summary` counts survive. An empty listing answers
  with an empty stream rather than a line that parses as a record.
- `-o json` and `-o yaml` are byte-for-byte unchanged. Both adversary passes probed this
  specifically and found no difference.

### Fixed

- `scripts/gate.sh` runs the `crates/connectors-console` workspace. It was in the root workspace's
  `exclude` list and in no lane, so nothing had ever executed its tests; the package now carries
  three test binaries and 80 cases.

## 0.5.9 — 2026-09-04

### Added

- Four read-only Slack operations: `slack-conversations-replies` (a thread's parent and its
  replies, which nothing could reach before), `slack-conversations-list`, `slack-conversations-info`
  and `slack-users-list`. The first three are projected to a model; the workspace directory is
  catalogued and not projected.
- `confluence.service_api_token`, the deployment-owned bearer twin of the personal Basic token.
- `[catalog.usernames]` in the personal configuration: a value-free home for the non-secret user
  half of a `basic` credential, keyed by the credential it joins. `connectors connect --set` writes
  it, and `connectors auth status` reports whether it is present.

  **The section is optional and a configuration without it reads unchanged — but it is a new key,
  and the configuration is `deny_unknown_fields`, so a binary older than this release refuses a file
  that carries one.** Install before writing, not after; observed as a red `connectors doctor` on
  2026-09-04 when a 0.5.3 binary met a file a newer build had written.

### Changed

- **Both Atlassian connectors address the vendor's cloud gateway by cloud id** rather than the
  tenant's own site host, and Confluence's four reads moved from the `api/v2` surface to `rest/api`.
  This is a correctness fix, not a preference: measured on one tenant with a service-account API
  token, a project search returned HTTP 200 and `total: 0` against the site host and HTTP 200 with
  40 results against the gateway, while `api/v2` answered 401. A connector pointed at the old route
  reported an empty world rather than a refusal anyone could act on. `site` is replaced by
  `cloud_id` in both `[[catalog]]` entries.
- `confluence-page-get` now returns the page body, because it sends the expansion that asks for it.
- Both Atlassian connectors declare their service-account mechanism first, so a placement holding
  both a personal and a service-account token authenticates as the service account.
- The Slack user-token declaration no longer requests `im:*` or `mpim:*` scopes; no operation names
  them, and `slack-conversations-list` withholds the parameter that would reach a DM.

### Fixed

- A personal-local `basic` credential could not be assembled at all. The user half resolves through
  the configuration port, and both `CatalogBackend` constructors built a port that could only answer
  endpoint variables — so a stored Atlassian token refused with `not_granted: no stored credential
  satisfies this operation's declared mechanisms` while `auth status` reported it as stored.
- Personal-local Kubernetes served exactly one activated cluster. `cluster_connection` answered the
  first key of a map, so an operator with five authorized contexts saw one in
  `operation describe kubernetes.deployment.status` and got `not_found` from every other — a message
  about the operation for a fault in Connection selection.
- A non-2xx vendor answer carried one sentence for every cause. It now names the HTTP status, and
  401, 403, 404 and 429 each say what to check.
## 0.5.8 — 2026-09-04

### Changed

- Migrate the seventy-three `docs/stories/S-*.md` records into the AEP planning store, so the
  repository has one backlog instead of two that never named each other. Every source file keeps
  its text and gains a backlink to the artifact that now carries it; nothing was deleted.
- The thirty-five stories the sources call `done` are recorded as resting on an assertion rather
  than an observed run, and `aep artifact validate` reports that count on every run.

No crate source changed in this release. The version moves because it is the artifact identity
written into every catalog document, `connectors.lock` and the wire User-Agent, and those move
together or not at all.

## 0.5.7 — 2026-09-04

### Added

- `connectors completions <shell>` prints a completion script for bash, zsh, fish, elvish or
  PowerShell, generated from the same clap command tree that parses the arguments.

### Fixed

- The release gate passes again. `d3707aa` took `integration-gitlab`'s backend past its size
  waiver, which failed every release run from v0.5.3 to v0.5.6 before a binary was built; the
  repository-file path helpers it added now live in their own module.

## 0.5.6 — 2026-09-03

### Fixed

- Canonicalize the Identity repository source URL so downstream Cargo graphs cannot instantiate
  duplicate Identity client types from the same 0.5.6 commit.

## 0.5.5 — 2026-09-03

### Fixed

- Refresh every satellite workspace lockfile after the 0.5.4 dependency and artifact-identity
  changes so the sharded release gate remains reproducible under `--locked`.

## 0.5.4 — 2026-09-03

### Changed

- Upgrade the hosted Identity client from 0.4.0 to 0.5.6 so Connector grants use the current
  Identity contract throughout the deployed stack.

## 0.5.3 — 2026-09-03

### Fixed

- Encode validated repository-relative GitLab file paths as one API path segment, so nested files
  can be read at an exact commit without weakening generic Connector path safety.

## 0.5.2 — 2026-09-03

### Added

- Carry receiver-verified agent, attempt, delegation, Grant, and Grant-revision provenance in the
  admitted principal context so delegated calls cannot collapse into an owner-only identity.
- Add approval-gated GitLab operations for creating an `agentide/…` session branch, atomically
  committing reviewed file actions, and creating or updating the session merge request. These
  publication operations require `api` scope and stay out of the model-exposed tool inventory.

## 0.5.1 — 2026-09-03

### Fixed

- Refresh every satellite workspace lockfile after the 0.5.0 dependency changes, so the release
  gate and the local-identity refusal check remain reproducible under `--locked` on a clean runner.

## 0.5.0 — 2026-09-03

### Added

- Compose the generic catalog adapter into hosted deployments, with per-principal Connect Sessions,
  prepared credential transactions, crash recovery, exact public egress apertures, and catalog-
  derived setup profiles.
- Let a signed-in person connect an Anthropic API key through the generic flow and verify it with
  the catalog-declared Models request before the Connection becomes callable.

### Changed

- Preserve curated GitLab, Slack, and Grafana setup as the authoritative experience while generic
  catalog setup adds providers and credential profiles those integrations do not own.

### Security

- Bind every generic hosted Connection and credential address to its authenticated tenant and
  subject. Two principals never see or resolve one another's stored provider credential.

## 0.4.5 — 2026-09-02

### Added

- Keep SIP in ordinary Connector binaries by default while allowing an embedding product whose
  strict configuration disables SIP to omit the voice dependency graph. A SIP-enabled
  configuration still fails closed when that capability was omitted.

## 0.4.4 — 2026-09-02

### Added

- Add a typed, bounded hosted Catalog client with request correlation and closed-envelope
  validation for product integrations.
- Publish GitLab OAuth-user and personal-token setup profiles when the hosted GitLab backend is
  available, so products can derive self-service controls from runtime capability.

### Changed

- Retire the dormant, unshipped VitePress explorer and its duplicate site projection. Hosted
  Catalog protocol reads and product-owned interfaces are now the supported browsing path.

## 0.4.3 — 2026-09-02

### Added

- Add `connectors admin` for Identity-protected hosted Integration readiness and credential writes,
  with public authority discovery and typed status responses for GitLab, Slack, and Jira.

### Security

- Require the exact administrative audience, scope, and operator-group membership for credential
  changes; accept secret input only through hidden prompts, standard input, or owner-only files,
  and return and audit metadata without credential bytes.

## 0.4.2 — 2026-09-02

### Added

- Add `connectors login`, `logout`, and `mcp`: the native CLI discovers a hosted Connectors
  deployment's neutral Identity authority, completes browser Authorization Code + S256 PKCE login,
  and bridges local stdio MCP to the hosted `/mcp` transport.
- Automatically use the selected hosted deployment for Operation, Connection, and Event commands
  when no explicit personal-local configuration or state root is supplied.

### Changed

- Cache five-minute Identity access tokens in memory by their exact Connector scope, renew them
  inside a 30-second margin, and retry one hosted request with fresh authority after a 401.

### Fixed

- Admit the exact `connectors.approvals.issue` scope used by the hosted approval-issuance endpoint
  through the closed Identity verifier vocabulary.

### Security

- Keep the opaque Identity session only in the operating-system keyring and write only non-secret
  account/deployment selection beneath XDG state. The stdio MCP peer sees neither the session nor
  access tokens, and Connectors requests receive only the least-privilege token for that call.

## 0.4.1 — 2026-09-02

### Added

- Add governed outbound MCP as a generated Connector service. A reviewed profile freezes the full
  remote tool snapshot and assigns local operation identity, prose, and effect; the ordinary
  deployment overlay still owns exposure, risk, approval, grants, endpoint and credential
  bindings. HTTP exchanges stay inside Connection-bound egress and fetch bearer material from the
  Connector secret store per exchange.
- Add a generator-facing service factory contract and deterministic runtime bundle builder. A
  registered factory remains inert until an explicit deployment overlay assigns permanent provider
  identity and complete operation policy/resource bindings; provider and operation collisions,
  incomplete overlays, and catalog/dispatch drift refuse composition.
- Activate generated service bundles as ordinary hosted backends with exact durable Grants,
  explicit operation admission, and readiness reporting for every composed service.
- Add bounded, single-use human approval evidence bound to the authenticated subject, exact
  operation, Connection, description lease, canonical input, and a five-minute maximum lifetime.
- Route hosted GitLab OAuth, PAT verification, project discovery, repository reads, and operation
  dispatch through the exact-origin Connection-bound post-DNS transport.

### Changed

- Carry the optional realm only in receiver-verified principal context. It is absent from service
  operation coordinates, and an absent realm remains distinct from the literal realm `default`.

### Security

- Refuse hosted GitLab unless `connection_bound_post_dns_v1` is declared, and include GitLab in the
  exhaustive source fence that rejects raw HTTP clients, DNS resolution, and outbound sockets from
  credential-bearing Integration adapters.

## 0.4.0 — 2026-09-01

### Added

- Add a projected-Kubernetes-token remote adapter for the shared Secrets service, including
  metadata-only enumeration and atomic put-only prepared generations.
- Preserve the verified owner subject when subscription credentials are created or refreshed.
- Add an idempotent, scope-bounded Vault-to-Secrets migration utility which keeps all secret bytes
  in memory and never deletes or mutates the source.

### Changed

- Hosted credential-bearing integrations select exactly one complete `[secrets]` or `[vault]`
  backend. Provider exchange, refresh, and upstream revocation remain in Connectors.

## 0.3.3 — 2026-09-01

### Fixed

- Complete Claude subscription OAuth from the provider's exact manual-callback value: split its
  `authorization_code#state` form, verify the returned state against the pending browser flow, and
  exchange only the authorization-code component. Missing, mismatched, or multiply delimited state
  is refused before any token-endpoint request.

### Security

- Keep the correction wholly inside Connectors. Identity remains provider- and relying-party-
  agnostic, while Connectors continues to own provider state, PKCE material, and token custody.

## 0.3.2 — 2026-09-01

### Added

- Add a bounded, single-use OAuth2 PKCE connection flow for Claude subscriptions. Connectors keeps
  the verifier and provider tokens in custody; authenticated callers receive only the provider
  authorization URL, an opaque flow id, presence, and attempt-bounded lease results.
- Persist refresh-capable subscription records and refresh them before expiry during serialized
  lease redemption, including refresh-token rotation. Existing manually supplied credentials
  remain readable for compatibility.
- Expose typed start and completion operations through the hosted API, Rust client, and embedded
  OpenAPI document. Credential-bearing requests and responses are bounded and non-cacheable.

### Security

- Provider acquisition and token lifecycle remain wholly inside Connectors. Identity remains
  provider- and service-agnostic, and neither authorization codes, PKCE verifiers, refresh tokens,
  nor provider diagnostics are returned to callers or written to logs.

## 0.3.1 — 2026-09-01

### Fixed

- Teach the complete-history secret scan to distinguish the exact generated catalog-lock
  SHA-256 line shape from credentials, without allowlisting other content in the lock file.
- Admit plain HTTP only for loopback and fully qualified Kubernetes service DNS, so in-cluster
  callers do not route credential custody through an ingress; remote origins remain HTTPS-only.

## 0.3.0 — 2026-09-01

### Added

- **Claude Code subscription custody.** The `claude-code` catalog provider is explicitly
  `custody_only`: it owns one user-bound setup token but publishes no callable service or
  operation. Hosted deployments opt in with `[claude_code] enabled = true`, which requires the
  configured Vault store.
- **Attempt-bounded credential leases.** A new custody component stores the provider credential at
  a tenant/subject-derived address and issues cryptorandom capabilities bound to one exact Harness
  attempt, an expiry no longer than one hour, and a finite use count. Restart revokes every live
  lease; disconnect and credential replacement revoke every lease over the previous value.
- **Typed hosted client and OpenAPI surface.** Presence, connect, disconnect, lease, and redemption
  have bounded client methods and documented HTTP contracts. Credential-bearing responses are
  required to carry `no-store` and `no-cache`; capability and credential diagnostics are redacted
  and their allocations are cleared on drop.

### Changed

- **Breaking pre-deployment wire correction.** Identity authority fields now use the neutral
  `tenant_id`, `principal_kind`, and `deployment_id` vocabulary and accept only the
  `identity_access_v1_` opaque credential format introduced by Identity 0.3.0. The Connector
  session authority likewise uses product-neutral claim names and the
  `b10x-connectors-session+jwt` media type. Compatibility with the inherited former-product
  vocabulary is intentionally not retained.
- A custody-only catalog document now projects an empty summary base URL instead of panicking; it
  still cannot compose a request because it has no service or operation.

### Security

- Creating a provider lease requires the new least-privilege
  `connectors.credentials.lease` Identity scope. Connecting and disconnecting remain self-service
  under `connectors.connections.self`; redemption accepts only the attempt capability and exact
  attempt id, never an Identity session.

## 0.2.1 — 2026-08-25

No product change: `crates/`, `providers/`, `specs/` and `catalog/` are byte-identical to `0.2.0`
apart from the `generator` string every artifact carries. This release is CI, tooling and the
security baseline. The version moved anyway because the version *is* the artifact identity — there
is no way to ship a `generator` that says `0.2.1` without cutting one.

### Added

- **`.github/workflows/release.yml`** — the first CI in this repository. Pushing a `v*` tag runs the
  full repository gate, the history-wide secret scan and the `local-identity` release refusal, then
  builds `connectors` for four targets (x86_64 and aarch64 Linux, x86_64 and aarch64 macOS) and
  publishes a GitHub release with the archives, a `SHA256SUMS` file, and the notes read from this
  file's section for that version.
- **No Windows target, and the reason is recorded.** `connectors` does not compile for
  `x86_64-pc-windows-msvc`: `connectors-config` opens files with `O_NOFOLLOW` and compares the
  effective uid to the owner, `connector-secrets` binds custody to Unix file modes, and the personal
  posture serves on a Unix socket. Supporting Windows means deciding what owner-bound credential
  custody means in terms of ACLs, which is an architecture question rather than a build flag.
- The tag and `[workspace.package] version` must agree, and each built binary must report the tagged
  version from `--version`, or the run fails before an asset is uploaded. The version is the artifact
  identity written into every catalog document's `generator`, so a disagreement would ship binaries
  that misreport themselves.

### Changed

- `scripts/gate.sh` gains `--list-workspaces`, `--workspace <path>` and `--final`. No argument still
  runs everything, unchanged. The flags exist so CI can shard the gate one workspace per runner: the
  eleven workspaces do not share a `target/` directory and need about 39 GB between them, which no
  hosted runner has. CI reads the workspace list from this script rather than keeping a second copy
  that would drift.
- `scripts/check-local-identity-refused.sh` now holds the release workflow to the same rule it
  already held the Dockerfile to: no Cargo feature is selected for a build that ships. The image was
  the only such build when that guard was written; a tag now attaches archives for four targets.
- **History was rewritten** to remove a former brand from commit authorship, messages, paths and
  blobs across all 254 commits, and again to remove a named individual's work address from the one
  commit that carried it. Verified both times by the rewritten `HEAD` tree being byte-identical to
  the original — every substitution had to be a no-op there, and was. Every clone predating
  `2026-08-25` is incompatible and must be re-cloned.
- `.gitleaksignore` regenerated twice as a consequence: a fingerprint names a commit, so a rewrite
  invalidates every entry whose commit it touched. All 87 findings were reclassified from scratch
  rather than carried over by count. `scripts/check-secrets.sh` had been failing since `4520cf47`
  and now exits 0.
- `scripts/vendor-babelforce-specs.sh` no longer writes out the address it exists to scrub. The
  script already declined to name two AWS account ids for that reason; the rule now applies to a
  person's name too.
- AGENTS.md gains what this cycle taught: that the gate does not fit on one machine, that an offline
  check still needs a populated registry, what a version cut actually rewrites, why the published
  targets are Unix only, and how to rewrite history and rebaseline the secret scan without
  measuring it circularly.

## 0.2.0 — 2026-08-25

### Added

- **`crates/connector-oauth`** — one authorization-code OAuth implementation, replacing three
  hand-rolled copies. PKCE with S256, single-use state with a TTL bound, authorize-URL
  construction, token-response validation against a declared policy, and refresh timing with a
  configurable skew. Transport-free and clock-free by design: its three callers dial three
  different ways, and a crate that owned HTTP would have dragged a client into every consumer.
  ([S-069](docs/stories/S-069-one-oauth-implementation-not-three.md))
- **`custody_only`** — a provider may declare that it holds a credential and describes no request
  surface at all, so a credential whose *use* belongs to another component can still have an owner,
  an address and a lifecycle here. Every key that could describe an outbound request is refused by
  name, and the refusal reads the declared TOML key rather than the assembled value.
  ([S-070](docs/stories/S-070-a-provider-can-hold-a-credential-it-cannot-spend.md),
  [design 16](docs/design/16-subscription-credential-custody.md))
- The catalog document **publishes** `custody_only`. A consumer must be able to tell a provider
  that happens to have no operations from one whose declaration forbids ever having any; only the
  second is safe to hand a credential whose use belongs elsewhere. Additive, so the document
  `schema_version` stays `2` — an older reader sees no service and no operation, and has nothing to
  call.

### Changed

- `integration-gitlab`, `integration-jira` and `integration-slack` moved onto `connector-oauth`,
  one commit each. Slack shares the state table and nothing else, on purpose: forcing its
  `xoxp-`-prefix token judgement and per-scope charset parser through a shared policy would have
  changed a request that works today.
- **The pending-state table is bounded at 1024 in all three.** It was unbounded, and every connect
  session inserts one. Expired entries are swept before the bound is consulted. A genuine flood
  makes further connect sessions for that integration return `connection_unavailable` until
  entries expire.
- GitLab's refresh response is now length-bounded at 4096, as its exchange response already was.
- Jira's authorize-URL parameter order changed. Key set, values and percent-encoding are
  byte-identical; only the order moved, and order is not significant in a query string.
- `authorize_url` clears an origin's query and fragment rather than appending to them.

### Fixed

- `provider-toml.schema.json` referenced `#/$defs/authRequirements`; the definition is
  `authRequirement`. The document did not compile as a schema, so **every rule downstream of that
  `$ref` validated nothing**, silently. `every_ref_resolves_to_a_declared_def` now catches it.
- `PendingStates::contains`'s documentation claimed to be the answer to `owns_hosted_oauth_state`.
  Using it there turns an expired callback from a refusal into a not-found, because the dispatcher
  finds no claimant — a regression caught during the migration and reverted, which the comment
  would have re-sold to the next reader.
- In GitLab and Jira the fallible inserts now run before the infallible ones, so a refused connect
  session leaves no hosted session that `session_owners` has no row for.

### Documentation

- [Design 16 — subscription credential custody](docs/design/16-subscription-credential-custody.md),
  and stories S-069 through S-074.
- Architecture ruling: platform ADR 0056, which partially supersedes ADR 0014 for custody. ADR
  0014's rule that harness credentials remain in the harness was a rule about custody as well as
  about use; the boundary moved, and the new decision says so rather than reinterpreting the old
  one. Use is unchanged — a harness credential is spent by its harness adapter and by nothing else.

## 0.1.0

The first `beyond10x/connectors` artifact identity. The predecessor differential passed and its
`0.26` line is retired; generator, lock rows and wire User-Agent start again here.
