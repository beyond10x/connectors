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
  the only such build when that guard was written; a tag now attaches archives for five targets.

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
