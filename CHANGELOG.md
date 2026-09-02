# Changelog

Notable changes to `beyond10x/connectors`, newest first. Versions follow
[semantic versioning](https://semver.org/spec/v2.0.0.html); before `1.0.0` a minor bump is where a
breaking change goes.

Nothing here publishes to a registry — every crate is an internal workspace member (design 02 §2).
The version is an **artifact identity**: `[workspace.package] version` is what
`catalog-build`'s `generator` string carries, so cutting a version rewrites every catalog document,
every `connectors.lock` row, and the wire User-Agent. Those three move together, always.

## Unreleased

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
