# Changelog

## 0.26.0 — 2026-10-05

### Added

- The catalog provider reads Zendesk Support: `tickets.incremental`, `users.incremental`
  (cursor-based incremental exports by `start_time`), `organizations.incremental` (the
  time-based export; the pinned document has no cursor export for organizations),
  `ticket.show`, `user.show`, `organization.show` and `ticket.comments` (paged by
  `page[after]`), from the pinned Zendesk Support API document, with an API token as HTTP
  basic `<email>/token:<token>` (profile `zendesk.basic`, identity `zendesk.user` from
  `GET /api/v2/users/me`). See `docs/catalog-zendesk.md`.
- The catalog inventory follows a parameter's local `$ref` to
  `#/components/parameters/<name>`, and reads an exploded `deepObject` query parameter over
  scalar properties as one `name[property]` parameter per property. No other committed
  bundle changes.
- `connectors-build redact` writes a vendor document with example credentials, real-looking
  email addresses and phone numbers replaced (rule `upstream-redaction/2`); the gate refuses
  a redacted pinned source whose digest drifted or that the rule would still change. The
  Zendesk document is pinned redacted.

### Specification

- MCP contracts with document cases, checked by the repository gate: outbound invocation
  results, inbound capability projection, outbound credential lifecycle, inbound mutation
  replay, local composition provenance, and the selected local CLI spelling with its
  static discovery inventory (`docs/local-mcp-cli.md`). None of them is a runtime: the
  CLI has no MCP server, adapter configuration or connection, and compatibility metadata
  stays `mcp: deferred`.

### Compatibility

- No configuration or bundle changes for existing providers. The other committed bundles
  are byte-identical under the new parameter `$ref` and `deepObject` handling.

### Limits

- Zendesk has not been run against a live account. Every read was verified against a local
  HTTPS fixture with synthetic records only; the API-token basic header follows Zendesk's
  guide and is unconfirmed live. Whether organization export pages overlap at their
  `end_time` boundary, and which ticket changes move a ticket into the export, are unchecked.
- The provider does not walk pages, bound `per_page` or `page[size]`, or retry on `429`.
  35 Zendesk operations with `deepObject` parameters stay unsupported; none is shipped.
- MCP delivery, Entity Runtime issue 51, sustained-read acceptance and the remaining
  provider plan stay open.

## 0.25.1 — 2026-10-02

### Fixed

- PostgreSQL SQLSTATE `0A000` now returns `unsupported` instead of `unavailable`.
  Neighboring custom SQLSTATE values retain their existing classification.
- The Google export test fixture flushes its TLS response before closing, fixing
  intermittent truncation at the 4 MiB acceptance boundary. A bounded TLS
  regression detects the missing flush; production response limits are unchanged.

### Verified

- Six real PostgreSQL 17.6 cases cover saved-credential restart, incident reads,
  result bounds, read-only refusals, native invocation-drop cancellation and
  local lifecycle controls. Five use the production CLI; cancellation invokes
  the production adapter directly.
- Four new k3s v1.31.5 cases cover reads across restart, RBAC denial versus empty
  results, continuation scope/revision binding and lifecycle controls. Four
  existing CLI journeys also passed separately.
- All eighteen current GitLab catalog reads returned HTTP 200 in the dedicated
  sandbox, followed by saved-credential reuse across an owner restart. Tags,
  releases and deployments returned empty lists. This replay made no provider writes.
- Catalog acceptance covers ten logical obligations and fifteen historical
  lifecycle/mutation variants, including uncertain effects, approval spend,
  owner recovery and post-effect revocation. Exact-file fault injection exercises
  settlement failure. Production CLI invocations establish original effects;
  some settled replay observations use a same-image host-library helper.
  Results span multiple runs with documented unchanged-source reuse.
- The ignored-test runner classifies these additional provider cases, excludes
  subprocess helpers and reports their Docker, Kubernetes and seccomp prerequisites.

### Limits

- The first GitLab revalidation refused before a later explicit retry succeeded;
  its cause remains unexplained. Empty-list observations do not establish
  nonempty payload behavior.
- PostgreSQL acceptance used plaintext loopback, not TLS. Native cancellation
  timing is a fixture observation, not a universal deadline. Kubernetes
  acceptance does not establish SSAR or remaining execution/Helm workflows.
- Catalog fault cases require Linux x86_64 seccomp user notifications. The
  fixture's revoked-path restoration control is a read-only fsync, not a
  production recovery write. These checks do not claim every variant ran on
  one final executable or establish all failure-cleanup paths.
- The registry-clock candidate was rejected and reverted: it broke the
  same-millisecond revision fence and failed the store-size cost bound.
  Production clock behavior is unchanged. Entity Runtime issue 51, sustained-read
  acceptance, MCP delivery and the remaining provider plan stay open.
- The browser connection-count and missing pre-handshake adversary fixture
  observations recorded in 0.25.0 remain open; this release does not claim a
  passing complete ignored suite.

## 0.25.0 — 2026-10-02

### Added

- `connectors-build ignored` inventories compiled ignored tests, classifies their
  prerequisites and reports executed, failed and skipped cases. Disposable
  fixtures are selected by default; live-provider and timing tests require
  explicit family selection. `--required` fails on missing prerequisites.

### Fixed

- Timed-out metadata work retains the lifecycle lock until its worker has
  finished. Cleanup runs outside the caller's destructor, preserving bounded
  return while preventing a second owner from racing an unfinished batch.
- SQL test fixtures recognize PostgreSQL cancellation requests separately from
  authenticated sessions, retaining exact session counts and rejecting malformed
  control traffic.

### Limits

- Metadata operation cost still grows with event-store size. This release does
  not resolve Entity Runtime issue 51 or complete sustained-read acceptance.
- Provider acceptance and MCP delivery remain work in progress. A skipped
  ignored test is reported as missing evidence, not as a passing provider check.
- The first disposable-suite run exposed an existing browser connection-count
  assertion failure and a missing pre-handshake adversary binary. The command
  reports both and exits unsuccessfully; this release does not claim that all
  ignored fixtures pass.

## 0.24.0 — 2026-10-02

### Added

- Catalog selections can withhold a declared parameter (`withhold` in `operations.json`): the
  operation no longer declares it, `operations describe` does not list it, and an input carrying it
  is refused as `invalid_input` before any request.
- `setup init` with an existing configuration and no metadata database creates the state and
  answers the new disposition `state_initialized`. An existing configuration is never rewritten and
  an existing database is never touched.

### Changed

- GitLab `commits.list` withholds `pagination` and its keyset cursor `page_token`: both are now
  refused as `invalid_input` (they were sent to GitLab before). The provider pages by
  `page`/`per_page` only.

### Compatibility

- A catalog provider binary older than 0.24.0 refuses a GitLab `operations.json` that uses
  `withhold` (selections reject unknown fields). Keep the binary and the selection file at the same
  release.
- Every GitLab catalog configuration's revision changes with the new selection; cached operation
  descriptions go stale and are refreshed by `operations describe`.

## 0.23.0 — 2026-10-01

### Added

- GitLab catalog read `deployments.list`: a project's deployments, each with its `environment.name`
  and the deploy job as `deployable`, filtered by `order_by`, `sort`, `updated_after`/`updated_before`,
  `finished_after`/`finished_before`, `environment` and `status`, paged by `page`/`per_page` (1–100).
  It needs only `read_api` and comes from the pinned GitLab document already in the bundle.

### Changed

- A metadata write that a concurrent commit changed under now fails with `revision_conflict`
  (`stage = publication`, `next_action = retry_explicitly`; nothing was written) instead of
  `metadata_unavailable`. That covers connections and the approval key and policy stores. Guarded
  writes keep their own rule (`next_action = retry_status`). An unreadable store still answers
  `metadata_unavailable`.

### Fixed

- The gate no longer hands its checkout-local temporary directory to cargo and the compiler wrapper,
  so sccache starts from a checkout with a long path. Test binaries get that directory through a Cargo
  runner keyed to the host triple, which a user's own runner for that triple does not override.

### Limits

- Values outside a pinned enum and strings that are not date-times are still sent to GitLab rather
  than refused locally.
- Build scripts get the caller's `TMPDIR`; on Cargo older than 1.89 doctests do too.
- A non-host build target (`CARGO_BUILD_TARGET`) bypasses the gate's runner; the gate and CI set none.

## 0.22.0 — 2026-10-01

### Added

- GitLab catalog reads `commits.list` (one ref's commits, filtered by `ref_name`, `since`, `until`,
  `first_parent`, `path`, `author` and more, paged by `page`/`per_page` with `per_page` 1–100) and
  `repository.compare` (`from`, `to`, `straight`; not paged). A compare GitLab cut short answers with
  `"compare_timeout": true`, which the caller reads to tell it from an empty compare. Both need only
  `read_api`, and both come from the pinned GitLab document already in the bundle.

### Fixed

- A `^C` that flushes typed terminal input before it is read is reported as interrupted, not as
  invalid input. An empty read after `poll` now waits again within the deadline on the terminal,
  standard input and document reads.
- The owner-socket tests reach the socket through a descriptor on the state directory, so the
  test suite and the gate pass from a checkout with a long path, such as a managed worktree.

### Changed

- The CLI specification declares the owner handshake frames: the greeting the CLI sends, the
  owner's reply and the fallback build request, each with its `kind` tag, UUID fields and a
  64-character lowercase-hex build digest. Contract citations point at the current lines. No
  generated CLI contract byte changed.

### Limits

- `commits.list` supports offset paging only; `pagination=keyset` is passed to GitLab unchanged and
  is not supported, because the provider does not follow GitLab's `Link` header.
- Values outside a pinned enum and strings that are not date-times are sent to GitLab rather than
  refused locally.
- A compare body over the provider's 4 MiB response limit answers `capacity` with no body; walk
  `commits.list` to place commits between two tags.
- With `rustc-wrapper = "sccache"` and no sccache server running, the gate from a long checkout
  needs `RUSTC_WRAPPER=` set empty.

## 0.21.1 — 2026-10-01

### Fixed

- `npm run build` in `website/` passes again with the pinned ESS 0.45.0. Its examples step built the
  example model from every domain, and the CLI domain's `connectors-local/1` enum variant is not a Rust
  identifier for the ESS Rust target (failing since 0.18.0). The example model now holds only the domains
  the examples read and their references; wire values are unchanged.
- The repository gate synthesizes the website example model for Rust and web, so a refusal like this one
  fails the gate and CI.

### Limits

- ESS's Rust generator ignores `code:` on enum variants; the WASM example build still runs only in
  `npm run build`.

## 0.21.0 — 2026-10-01

### Added

- Google providers for the catalog: `google-drive`, `google-slides`, `google-calendar` and
  `google-gmail`, compiled from Google Discovery documents pinned from
  `googleapis/google-api-go-client` (BSD-3-Clause). Reads: Drive about, files, export and
  changes; Slides presentations and pages; Calendar calendar list and events; Gmail profile,
  messages, threads, history, labels and attachments. Guarded writes: Drive metadata create,
  update and copy; Slides create and batchUpdate; Calendar insert, patch and delete; Gmail drafts
  create and send (no direct `messages.send`). Guides: `docs/catalog-google-*.md`.
- `connectors-build discovery` projects a Discovery document into OpenAPI 3.0.3, refusing every
  construct outside its rule table by JSON pointer; `connectors-build catalog --derived-from`
  records the derivation in the bundle.
- Auth scheme `oauth2_refresh`: the stored entry is `{client_id, client_secret, refresh_token}`,
  exchanged for an access token per call and cached in memory. `connections connect
  --credential-file <Google Desktop client JSON>` obtains it by browser consent (loopback redirect,
  PKCE S256); without a terminal the consent address is printed as `connectors: consent-url <url>`.
- Catalog selections: query parameters declared as form-exploded arrays take a JSON array (one
  pair per element); `required` marks parameters a provider needs; `rate_limit_reasons` reports a
  quota 403 as `rate_limited`; `body_keys` closes a write body.

### Changed

- An unauthorized `operations invoke` of a read reports `repair_connection`; connect, repair,
  revalidate and guarded writes keep their previous next action.
- An empty text response body is `""`, not `null`.

### Compatibility

- Every GitLab, Jira, Confluence and HubSpot bundle is rebuilt, so their configuration and descriptor
  revisions move: copy the new `configuration_revision` into the adapter entry, connect an
  instance that was already connected under a new instance id, and issue write approval policies
  again.
- A repeated query parameter still accepts the comma-joined string form.

### Limits

- A Google OAuth app in Testing issues refresh tokens that expire after 7 days.
- The engine does not check enum values (for example Calendar `sendUpdates`, Gmail `format`).
- Drive `files.update` on a shared-drive file is not supported yet.
- Not run against live Google yet; the live read of a deck waits on a Desktop OAuth client.

## 0.20.0 — 2026-09-30

### Added

- A catalog native configuration accepts an optional `request_prefix` naming the API-gateway
  part of `api_base` (for example `/ex/jira/<cloud id>`), so service-account API tokens connect
  Jira and Confluence through `https://api.atlassian.com/ex/<product>/<cloud id>/…` with the
  `atlassian.basic` profile (verified live for Jira). Configurations without it keep their
  revision.
- `adapters list` and `operations list` page with `--limit`/`--cursor` and answer `next_cursor`.
  A cursor is bound to the list's source, the adapter and the page limit, expires after 300 s,
  and answers `stale_cursor` once the list changes.

### Changed

- Catalog parameters are declared with their pinned document's type: integers take a number or
  its decimal string, booleans `true`/`false`, strings any string or an integer; other values,
  and non-integer numbers for typed parameters, are refused as `invalid_input` before any
  request. Bundle digests and every catalog provider's descriptor revision change; approval
  policies bound to an earlier revision must be set again.
- A connect, repair or revalidation the owner definitely failed answers the owner's code
  (`unavailable`/`timeout`) at `dispatch` with `retry_explicitly` instead of `outcome_unknown`;
  `outcome_unknown` remains for any reply that is missing, cut short or malformed, and for a
  revalidation the owner stopped waiting for.
- An operation id the adapter does not expose answers `not_found` on describe, invoke and
  approvals prepare; refusals are decided in the order existence, grant, revision. `operations
  describe` checks the profile grant and the id format like invoke, and `operations list` hides
  operations whose profile is not granted.
- The legacy `describe`, `invoke` and `serve` commands accept `--output` before the command word;
  their parse errors are the fixed `cli_parse` refusal (exit 2) and never echo arguments.
- `setup check` reports `next_action: none` for ready metadata; an unknown adapter alias answers
  stage `admission` on every adapter command.
- The CLI contract states the `ess-cli/1` parser and input codes, where the JSON decoder's
  recursion limit takes over from the owner's depth bound, which commands read the business
  document before the alias, `stale_cursor` and pre-publication connect rows, and a 128-byte
  selector bound.

### Compatibility

- Every catalog provider's descriptor revision changes with the typed parameters: cached
  operation descriptions go stale and are refreshed by `operations describe`, and approval
  policies bound to the previous revision must be set again.

## 0.19.0 — 2026-09-30

### Added

- The catalog provider reads HubSpot CRM records: `objects.list` (one object type, paged by
  `after`) and `object.get`, from the pinned CRM Objects `2026-09` document, with a
  private-app access token as a bearer header (profile `hubspot.private-app`, identity
  `hubspot.portal` from the account details). No time filter: search is a POST, which the
  engine does not admit as a read. See `docs/catalog-hubspot.md`.

## 0.18.0 — 2026-09-30

### Added

- The catalog provider reads Confluence Cloud: `pages.changed` (pages newest first by
  modification date, walked to a cutoff), `space.pages`, `page.get` (body in storage
  format) and `page.comments`, from the pinned REST v2 document; `limit` is bounded to
  1–250. See `docs/catalog-confluence.md`.
- Catalog bundles record each operation path below its document's server path; a path or
  operation that declares its own `servers` is named unsupported.

### Changed

- **Breaking:** Jira and Confluence share one auth profile. The Jira profile `jira.basic`
  (identity kind `jira.user`) is now `atlassian.basic` (identity kind `atlassian.account`).
  A configuration that names `jira.basic` must name `atlassian.basic` and connect again.

## 0.17.0 — 2026-09-30

### Changed

- The local owner exits after 10 minutes with no connected client and no work in flight.
  An idle adapter child, an empty recovery sweep or recovery that settles nothing do not
  keep it alive; an attempt that cannot settle stays pending for the next owner. The next
  command starts a new owner, and a command whose connection the exiting owner closes
  starts over instead of failing.
- A provider's refusal of a guarded write reports stage `dispatch` with
  `request_permission` (403) or `none` (404/410); the catalog provider keeps the refused
  status (400/409/412/422 `invalid_input`, 401 `unauthorized`). A host refusal before
  anything was sent reports `admission`. A write whose effect may have happened is never
  offered `retry_explicitly`.
- A provider timeout after the request was sent (including while its answer was being
  read, and on a connection probe), a database statement timeout and the database
  connection limit report `dispatch` / `retry_explicitly`; the host's own deadlines and
  limits stay `admission`.
- The ESS pin moves to 0.45.0 with the `ess-*` libraries. The local metadata conformance
  suite runs 289 scenarios (285 on 0.40.0): it gains 5 wrong-state scenarios and loses
  `ReviseLocalApprovalPolicy/outcome/exhausted` to an ESS-SYNTH-003 refusal present since
  ESS 0.43.0 (beyond10x/ess#251); a host test now checks that outcome.

### Compatibility

- A binary older than this release cannot read a write-attempt record that stores a
  provider refusal, nor an adapter child's `provider_timeout` / `provider_capacity`
  reply. Records it wrote itself still replay.

## 0.16.0 — 2026-09-29

### Added

- The catalog provider accepts an HTTP basic authentication profile: `auth.scheme: basic`
  with `account_label`, and a credential document `{"account", "token"}` entered through
  `--credential-file` or `--credential-stdin`. The provider sends
  `Authorization: Basic base64(account:token)`. Token configurations serialize as before,
  so existing configuration revisions do not move.
- The CLI refuses an owner process that runs a different executable build with
  `owner_build_mismatch` and next action `stop_owner`, including an owner from before
  this handshake. It never stops or replaces the owner itself. A same-build owner at
  capacity still answers `capacity`.
- The shipped GitLab catalog selection adds `projects.list`, `tags.list`,
  `releases.list` and `project.events`, paged by `page`/`per_page`.
- The catalog provider reads Jira Cloud: `issues.search` (JQL, `nextPageToken` paging),
  `issue.comments` and `issue.changelog`, from the pinned platform REST v3 document with
  HTTP basic auth; see `docs/catalog-jira.md`.
- Catalog selections can bound a query parameter
  (`bounds: {"<param>": {"minimum": n, "maximum": n}}`); the shipped GitLab list reads
  bound `per_page` to 1–100 and refuse other or non-integer values as `invalid_input`
  before any request.

### Changed

- A local process replays its metadata store once and then reads back only what was
  appended, instead of replaying every recorded event on every open and write. A read
  `operations invoke` does one full replay instead of eleven; on a 600-event store the
  median per-invoke time fell from 26.1 s to 5.5 s (release build). Per-invoke time still
  grows with the store inside Entity Runtime batch execution.
- `setup init` writes `connectors-local/2`, so an adapter entry that selects its private
  protocol passes `setup check`; existing `connectors-local/1` files load unchanged. The
  postgres and kubernetes guides add `private_protocol = "connectors-private/1"`.
- `invalid_configuration` for a `connectors-local/1` entry carrying `private_protocol`, or a
  `connectors-local/2` entry lacking it, names `configuration_format` and the first such
  entry's `instance_id` in its error data, on every command except `setup init`.
- A provider's own refusal of a read (upstream 403, 404 or 410) is reported at stage
  `dispatch` instead of `admission`. Refusals the host or an adapter raises from its
  configured scope still read `admission`. Adapter children send the new private-protocol
  failure `provider_forbidden`, which a host older than this release cannot read.

### Tests

- The repository gate runs the local metadata authority's synthesized conformance suite
  against the definitions the host embeds: 285 scenarios, all passed, none unsupported
  (previously 212 passed and 73 unsupported, 71 of them kernel invariant violations
  caused by placeholder inputs).
- `PrepareAttempt`, `ReserveKey`, `RecordApprovalRedemption` and `AcknowledgeAnchor`
  declare `fixture_inputs`; the conformance target supplies each named fixture and
  refuses an unknown name as an error rather than a default.
- A kernel invariant violation, or an invariant the kernel cannot observe, during a
  conformance run now fails the scenario instead of counting as unsupported.
- The mutation audit of the component kills 176 of 176 live mutants (88 stillborn).

### Specification

- The `credential_evidence` admission and read-use views expose `request_id`,
  `operation_ref`, `connection_ref`, `custody_version_ref` and `publication_fence`.

## 0.15.1 — 2026-09-29

### Fixed

- After a read use's expiry was recorded, every clock-only observation failed with
  `metadata_unavailable`: `connections status`, `connections list`, and the observation
  before `revalidate` and `operations invoke`. The observation check compared the SQL
  projection, which drops expired read uses and cursors, with the full Entity Runtime
  state. It now makes the same exception the write path already made. The defect came
  in with the Entity Runtime metadata authority in 0.12.0.

### Tests

- `observation_after_a_recorded_read_use_expiry_is_admitted` reproduces the failure.
- `the_same_identity_connects_again_after_revoke` pins that revoke ends one connection,
  not the identity.

## 0.15.0 — 2026-09-29

Hardened with six ESS hardening techniques and a design review of the 13 domains the
host runs on Entity Runtime. Constructs ESS or Entity Runtime cannot yet express are
filed as beyond10x/ess #231-#237 and stay host-enforced.

### Fixed

- Credential custody addresses the `login` Secret Service collection by its object
  path. It read the desktop's `default` alias and required it to resolve to `login`,
  which forced changing a setting every other application depends on. The alias is
  now never read or changed.
- A connection-material retirement batch sorts tied deadlines by version, so the same
  16 rows are chosen every time.
- `clock_exchange_precedes_leases_and_policy_is_rechecked_after_network` sets its own
  clock query timeout and observation width; under load its contended metadata write
  outlasted production's 2 s and 4,000 ms and the test failed with `Unavailable`.

### Specification

- The shared system and the Kubernetes adapter move to source format `ess/15`.
- Guards the host already enforced are now declared and enforced by Entity Runtime:
  a clock floor or registry epoch never goes lower, an approval policy revision is a
  compare-and-set that advances, an issuer revision advance is a compare-and-set to a
  fresh revision, a binding publication refuses a stale publication fence, an
  acquisition completion refuses a result for another target, a required-approval
  dispatch needs its captured subject, a dispatch refuses a generation other than the
  admitted one.
- Creating and updating commands store the inputs they carry (`sets:`) instead of
  leaving the host to choose those fields.
- Entity invariants for the audit anchor, attempt and reservation settlement,
  approval subjects and fingerprints, key validity bounds, cursor page limit, policy
  revision and operation bounds, and credential byte size.
- Views publish the state and fields a conformance suite needs to see; before
  integration, mutation testing killed 122 of 122 mutants against them.
- A `LocalHost` actor in each of the 12 domains the host runs names who may send each
  command.
- A Kubernetes cluster partition declares `namespace: null` with
  `presence: null_when_absent`.
- The three design documents that said twenty entities, no CLI persistence entity and
  no persistent clock record now match the specification.

### Tests

- New Entity Runtime tests for every declared guard and invariant, each seen failing
  with its rule removed.
- 13 metamorphic relations over the host (reopen, migration, replay, revoke isolation,
  refusal leaves views unchanged, namespace separation).
- A determinism test runs one command sequence twice and compares every decision.
- `connectors-build metadata-conformance` runs the component's synthesized suite and
  mutation emissions against the definitions the host embeds.
- A disposable-keyring test proves custody neither reads nor changes the `default`
  alias.

### Website

- `website/.npmrc` allows the git dependency for this project (npm 12 disables git
  dependencies by default).
- The realization example follows the current attempt model (`owner_nonce`,
  `publication_fence`, `request_fingerprint`, settlement fields, the unknown-attempt
  and missing-subject outcomes).

### Known limitations

- 70 synthesized scenarios whose inputs carry nested structs cannot run yet: synthesis
  fills those members with placeholders that the new invariants refuse (beyond10x/ess
  #234). The next release resolves them through fixtures.
- Five wrong-state scenarios on `CompleteAcquisition` and `PublishBinding` are not
  synthesized (ESS-SYNTH-003, #234).

## 0.14.0 — 2026-09-29

### Fixed

- `connectors --version` and `connectors -V` print `connectors <version>` and
  exit 0. They exited 2 with `cli_parse`, unlike every other Beyond10x CLI, which
  broke tools that detect an installed `connectors` by its version.
- Concurrent local writers — CLI commands, connection publication, repair,
  revoke and mutation preparation — wait up to 30 seconds for the metadata
  lifecycle lock, the bound SQLite's own busy wait already had. The approval
  policy and approval key leases and the custody writer locks read the same
  wait. It was two seconds; each writer holds the lock for its whole Entity
  Runtime replay and batch, so on a loaded host a writer queued behind a few
  others answered `metadata_unavailable` while the store was healthy. A holder
  that never releases is still refused, at the new bound.

### Changed

- The pinned tools are releases: ESS 0.40.0 and AEP 0.65.0, taken from `PATH`
  (the Beyond10x plugins install them). The source builds, build receipts and
  the local AEP findings patch are gone. Entity Runtime moves to 0.25.1 and
  Eventlog to 0.6.0; every other Rust dependency moves to its newest release,
  including reqwest 0.13 on rustls with `ring`.

### Compatibility

- Without a configured `ca_file`, HTTPS trust now comes from the platform verifier
  (the operating system trust store) instead of bundled webpki roots. A configured
  CA still replaces the built-in roots.
- `connectors-build toolchain` and `connectors-build aep-toolchain` are removed.
  Install ESS 0.40.0 and AEP 0.65.0 (`b10x upgrade`) or select them with
  `--ess`/`CONNECTORS_ESS` and `--aep`/`CONNECTORS_AEP`.

## 0.13.3 — 2026-09-27

### Specification

- 60 of the 99 open `UNMAPPED` markers are resolved: 11 declared from source, 9 decided, 4 deferred
  to named stories and 36 noted as constructs ESS 0.36 lacks. The 39 MCP markers stay, as
  `story:mcp-domain-model` requires.
- PublishBinding with decision allow on a Revoked connection answers `ConnectionStateConflict`;
  expiring an already Expired cursor answers the new `CursorStateConflict`.

## 0.13.2 — 2026-09-27

### Tests

- Tests hand subprocess and child-process bounds from the test: a preparation that fails only
  after its deadline is retried on a fresh child with a doubled budget and its discarded attempt
  is checked; a child honours a lock wait only from its own parent; keyring custody locks use the
  test-only wait; the terminal test gives its child 120 s and keeps its stderr. Production keeps
  its 2 s bounds.

## 0.13.1 — 2026-09-27

### Specification

- Every system under `ess/` and `adapters/*/spec/ess` declares source format `ess/14`.
- The ESS crates and the pinned ESS toolchain move from 0.35.0 to 0.36.0. No Rust API change was
  needed; the generated Entity Runtime definitions change only in their source and synthesis
  digests.
- `connectors.discovery_state`: publishing a view increments `last_generation` and replaces
  `publication_revision`; retiring a view or a collection replaces `publication_revision`
  (`contracts/discovery/resources/v1alpha1/semantics.md`). Each `ObservationChanged` names the state
  its transition enters instead of a generated value.

### Planning

- AEP moves from 0.60.0 to 0.61.1 for the toolchain, CI and the store's protocols pin. The local
  findings patch applies unchanged onto 0.61.1, which does not contain the correction.

### Tests

- `connectors-host` tests that contend on a lock or a recovery window bound it themselves: a frozen
  clock for the 250 ms audit recovery window and a test-only 120 s lock wait for the 2 s lifecycle,
  approval-policy and approval-key locks. Production keeps both bounds.
  `final_audit_recovery_preserves_every_live_business_result` is no longer ignored, and the gate no
  longer runs it alone. The library suite passed three of three runs at load average 25 to 37.
- New tests kill three model mutants: acquisition expiry, cursor expiry and key publication.

## 0.13.0 — 2026-09-27

The specification and the planning tools move to their newest releases.

### Specification

- Every system under `ess/` and `adapters/*/spec/ess` declares source format `ess/13`.
- `ess/13` requires an explicit source for every emitted payload field. 123 fields in `ess/domains`
  now map to their command input (`input.<field>`, 103) or are declared produced by the host
  (`{generated: true}`, 20). The generated Entity Runtime definitions drop the host-filled slots
  for those fields; all 55 commands remain.
- The ESS crates and the pinned ESS toolchain move from 0.31.0 to 0.35.0. No Rust API change was
  needed.

### Planning

- AEP moves from 0.59.2 to 0.60.0 for the toolchain, CI and the store's protocols pin. The local
  findings patch is rebased onto 0.60.0, which does not contain the correction. `aep --version` now
  prints `aep <version>`, and the toolchain check accepts it.

### Known limitations

- `connectors-host` `final_audit_recovery_preserves_every_live_business_result` fails on a heavily
  loaded machine at this release and at 0.12.0 alike; its recovery runs under a 250 ms bound.

## 0.12.0 — 2026-09-25

Local metadata has one authority. The host's registry, runtime stop state,
mutation attempts, execution audit and approval state are decided by the Entity
Runtime executor and recorded in Eventlog over SQLite; the SQLite file the host
reads is a projection of that record. This is milestone M7 of the ESS evolution,
`story:ess-evolution-er-metadata-adoption`.

### Metadata authority

- Every metadata write runs as an Entity Runtime command batch against an
  Eventlog SQLite provider. Existing stores migrate from metadata schema levels
  1–8 on first admitted write; the legacy source is retained for recovery and its
  digest is checked on every open.
- The ESS domains under `ess/` gain the typed homes the migrated records need, and
  `ess/components.yaml` names the component that owns them. The generated
  `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json` is
  lowered from that model.
- A read-only observation rebuilds its view from the record outside the metadata
  lock and takes the lock back before it acts. When a writer advanced the
  registry clock in that window, the retry now rebuilds while holding the lock,
  so it cannot lose the same race again; before, four concurrent CLI reads could
  lose it eight times in a row and answer `metadata_unavailable`. The locked
  rebuild held the lock 172 ms on average and 254 ms at most in the Kubernetes
  CLI journey, against the unchanged 2-second lock bound.
- A SQLite sidecar (`-wal`, `-shm`, `-journal`) retired by another process between
  listing and checking reads as absent instead of refusing the open. Non-private
  files and symlinks are still refused.
- `connectors_core::read_json`, `canonical()` and `digest()` read and digest JSON
  numbers alike whether or not `serde_json`'s `arbitrary_precision` is enabled,
  which Entity Runtime turns on in every host build.

### Contracts

- The 15 authored scenarios under `contracts/operations/v1alpha1/scenarios`
  supply the inputs the M7 mutation model requires: `request_fingerprint`,
  `owner_nonce` and `publication_fence` on `PrepareAttempt`; `settled_at` and
  `terminal_result_json` on `AbortPrepared`, `RecordCompletion` and
  `RecordRefusal`; `terminal_result_json` on `RecordUncertainty`. Values take the
  shapes the host writes. No input was made optional in the model.

### Gate

- `final_audit_recovery_preserves_every_live_business_result` runs in its own
  step, alone and single-threaded, right after the workspace test lane. Its
  250 ms recovery window is a product bound, and parallel neighbours on a
  4-thread runner consumed it. The step is green only when exactly that one case
  ran and passed.
- The shared ESS boundary check accepts a root `ess/components.yaml` that owns
  only domains the model's manifest lists.

### Pins

- Entity Runtime 0.23.0 (`77aac6ea`), Eventlog 0.4.0 (`70096af8`) and ESS 0.31.0
  (`f7de9f82`); `Cargo.lock` holds one source of each.
- The planning check uses AEP 0.59.2 (`d3d80e9b`) with the retained
  findings patch, which 0.59.2 does not contain.

### Planning

- The planning store moved to Eventlog authority and then to `aep.project/3`, a
  tree store Git merges, rendered as `aep.planning-md/2`. Six review-body digests
  were rebound after those migrations had rewritten home paths and literals in
  the immutable review bodies they name.

## 0.11.0 — 2026-09-14

GitLab has one runtime. The catalog provider serves every operation the native
GitLab adapter carried, from the pinned OpenAPI document and a selection set
reviewed and shipped in the repository, and the native adapter is deleted. This
is the convergence `architecture-decision-record:declarative-http-provider-runtime`
asked for: no handwritten Rust per endpoint for an ordinary HTTP provider.

### Catalog provider

- The repository ships a reviewed selection set per provider under
  `adapters/catalog/providers/<provider>/operations.json`, referenced from a native
  configuration by `operations_file` (format `connectors-catalog-local/2`). The
  GitLab set exposes every operation the native adapter exposes: eleven reads,
  `merge_request.create`, `merge_request.update` and `merge_request.merge`;
  `adapters/catalog/tests/shipped.rs` refuses a set that resolves against the
  committed bundle without one of them.
- A guard carries a list of checks in its preflight and its postflight; each
  compares a JSON pointer in the observed body with an input reference or a
  literal. The merge precondition (open, mergeable, pinned head, pinned pipeline
  successful) is five checks in the selection rather than a Rust handler.
- A selection may declare `"response": "text"` for a read whose source declares
  JSON where the provider answers plain text, as GitLab does for a job trace;
  without it the bundle's declared 2xx media types decide.
- Against the live sandbox, all eleven GitLab reads, a guarded merge with one PUT
  and three guard refusals with none ran through the shipped set; see
  `docs/evidence/gitlab-sandbox-20260913/README.md`.

### Removed

- The native GitLab adapter: the `connectors-gitlab` crate under
  `adapters/gitlab/` with its v3 specification, generated tree, tests, native
  contracts and packaging realization. GitLab has one runtime, the catalog
  provider with the shipped selection set. `adapters/gitlab/upstream/` stays as
  the pinned source the bundle is compiled from.
- The v3 write generator in `crates/connectors-spec` (`src/v3.rs`,
  `tests/write_generation.rs`) and the `connectors.adapter/v3` spec kind; GitLab
  was its only consumer. The v2 generator keeps its tests against the frozen
  fixture `crates/connectors-spec/tests/fixtures/gitlab-v2.json`.
- The standalone GitLab service and its conformance slice: the catalog provider
  runs under the local CLI only, so `connectors-conformance`, `examples/` and the
  live acceptance recipe cover Kubernetes and PostgreSQL. GitLab acceptance is
  the sandbox record under `docs/evidence/gitlab-sandbox-20260913/`.
- `docs/local-gitlab-cli.md`, `docs/gitlab-generation.md` and
  `docs/gitlab-write-failure-matrix.md`; the four `/adapters/gitlab/contracts/*`
  website pages. The CLI binding steps now live in
  `docs/local-catalog-provider.md`.

### Changed

- The repository gate checks the `kubernetes` and `sql` descriptors and the
  `kubernetes`, `sql` and `catalog-provider` library boundaries;
  `connectors-build package` defaults to `adapters/catalog/realizations/local.json`.
- The website walkthrough validates its fixture against the catalog provider's
  declared `issues.list` and shows the provider's status, body and provenance.
- `adapters/catalog/tests/local_runtime.rs` carries the host-child checks the
  native adapter's suite held: spawn against the printed bootstrap, identity and
  scope probes, reads through the shipped set, exact-incarnation stop, and
  refusal of a changed artifact, bootstrap or trust root. The production CLI
  journeys (approval spend, settlement, owner crash, background recovery,
  revocation) that ran with the native adapter as the child are not yet re-run
  with the catalog provider; `story:catalog-cli-journeys` owns that.

### Planning

- `epic:retire-native-gitlab-adapter` records the convergence: GitLab served by
  the catalog provider only, the native adapter removed once every operation it
  carries runs through the catalog with equal or stricter guarantees.
  `story:remove-native-gitlab-adapter` is the deletion.

## 0.10.0 — 2026-09-13

GitLab merge-request create and update now run from the pinned OpenAPI document
through a generic provider, with no adapter code per endpoint. This is the first
release of the catalog track's runtime half; the native GitLab adapter gains a
raced update and nothing else.

### Catalog provider

- `connectors-build catalog` compiles a pinned OpenAPI 3.0 or 3.1 document, JSON
  or YAML, into a digest-verified provider bundle and indexes it. The committed
  GitLab bundle carries all 1,847 operations of `openapi_v3.yaml` with none
  unsupported, and a test refuses a committed bundle a fresh run would not
  reproduce.
- `connectors-catalog-provider` is a new adapter executable behind the same
  private protocol as `connectors-gitlab`. Its configuration selects operations by
  `operationId`, declares each one's effect, names one token-header auth profile
  with declarative identity and scope probes, and may attach a guard: a preflight
  read that refuses before any request when a pinned value already differs, and a
  postflight comparison that leaves the outcome uncertain — never refused — when
  it differs afterwards. Reads are one bound GET; writes are one bound POST, PUT,
  PATCH or DELETE under the host's approval, audit and attempt controls.
- Against the live GitLab sandbox, through the provider: merge-request reads, a
  create that opened MR 10 at its pinned head, a stale pin refused with no request,
  a duplicate create refused by GitLab's 409, an update that retitled MR 10, and a
  create whose branch moved mid-flight, which opened MR 11 at the moved head and
  was classified `unknown`. One request per attempt that reached dispatch.
- `AuthenticatedWrite` gains `send_json` with a `WriteMethod`; `put_json` remains
  as a provided method. The v3 write generator accepts `post` as well as `put`.

### Native GitLab

- `merge_request.update` is a native write under the accepted C14 race boundary:
  preflight read, unguarded PUT, best-effort postflight comparison. The sandbox
  showed the comparison can miss a move: GitLab answered the PUT with the pinned
  head while the branch had already moved. `decision-blocker:gitlab-mr-create-update-head-guard`
  is cleared with that finding recorded.
- `story:gitlab-mr-reads` closes: a fork's merge request whose source project was
  destroyed reads with `source_project_id: null`, and a conflicting change reads
  `detailed_merge_status: conflict`.
- `merge_request.create` was deliberately **not** added to the native adapter.

Full record with identities, commands and outcomes:
[docs/evidence/gitlab-sandbox-20260913](docs/evidence/gitlab-sandbox-20260913/README.md).

### Limitations

- **The catalog bundle carries no request or response schemas.** A write body is
  passed through as supplied and validated only by the provider.
- **One provider has run through the engine.** The handoff specification requires
  a second ordinary HTTP provider and TOML-authored actions through the same
  engine before it is satisfied.
- **The postflight comparison is not detection.** A merge request's recorded head
  is eventually consistent with its branch; a match means no move was observed.
- Helm release reads stay fixture-verified; MCP stays contracts only; one
  instance, one project, one runner. Source release only: no binary, container
  image or website deployment.

## 0.9.0 — 2026-09-13

Every GitLab claim in 0.8.0 was verified against a fixture. This release verifies
them against a dedicated live GitLab, and four of the five GitLab stories close on
that evidence. No source file changed: the code that passed the fixtures is the code
that passed the provider.

### Verified against a live GitLab

- A dedicated GitLab 19.3.2 sandbox, operated by a non-administrator with a
  `read_api` token and project role Developer. The administrator token set the
  sandbox up and ran none of the operations under test.
- All eleven read operations answered with provenance. A pipeline was watched from
  `pending` through `running` to `failed` on one exact commit SHA, its jobs paged,
  and the failed job's trace read with explicit bounded completeness at two byte
  limits.
- A pinned merge-request validation reported `checks_passed: true`, and the same
  pinned request reported `head_changed` after the merge request's head actually
  moved.
- A merge request was merged through the approval chain: authenticated clock,
  signing issuer, published policy, prepared subject and issued proof.
- Two crash shapes were exercised against the provider. With the owner killed on
  the merge PUT, the merge applied and the response was lost; replaying the same
  business key returned the original attempt and the merge endpoint still shows one
  PUT. With the owner killed before the PUT, nothing merged and the replay demanded
  a fresh proof.

Full record with identities, commands and outcomes:
[docs/evidence/gitlab-sandbox-20260913](docs/evidence/gitlab-sandbox-20260913/README.md).

### Documentation

- `docs/local-gitlab-cli.md` now says how to stand up the sandbox, including the
  certificate chain the adapter requires.
- `docs/local-gitlab-merge.md` records that GitLab answers a merge **401**, not
  403, when it has identified the user and that user may not merge into a protected
  branch.
- `AGENTS.md` makes a hosted release page a step of cutting a release, authored by
  the organization bot like every commit and tag.

### Limitations

- **A single self-signed certificate does not work.** The adapter's rustls client
  rejects an end-entity certificate carrying `basicConstraints CA:TRUE`, which curl
  accepts. A CA and a leaf signed by it are required. This is unchanged behaviour,
  newly documented.
- **Merge-request read acceptance is incomplete.** Deleting an open merge request's
  source branch closes it in GitLab 19.3.2, and the mergeability check settles
  faster than a CLI process starts, so neither the nullable deleted-source shape nor
  an unknown merge status could be produced. `story:gitlab-mr-reads` stays open.
- **No crash between GitLab's acknowledgement and the ledger write.** The lost
  response was lost between the owner and the CLI; the ledger already held the
  outcome.
- **Helm release reads are still fixture-verified.** No real cluster has answered
  those four operations.
- **MCP is still contracts only.** No connection, no server, no persisted credential.
- One instance, one project, one runner, one executor. No concurrency and no
  credential expiry under load.
- Source release only: no binary, container image or website deployment.

## 0.8.0 — 2026-09-12

**This release replaces the implementation.** Every source file is new: the
codebase was rebuilt from scratch beginning 2026-09-08 around contract-driven
adapters, an explicit local lifecycle and typed semantic models, and it supersedes
everything up to 0.7.2. Entries below 0.8.0 describe the previous implementation
and are kept for history.

The version line continues rather than restarting. Three pre-release milestones
inside the rewrite were numbered 0.1.0, 0.2.0 and 0.3.0 during development; none
was published, and those numbers already belong to releases of the previous
implementation in this repository. Their content is folded into this entry.

### GitLab

- Persistent local lifecycle: setup, adapter lifecycle, protected connection
  entry, repair, revalidation, terminal revocation and cached operation discovery.
  SQLite holds metadata; a qualified Linux Secret Service collection holds
  credentials. Saved credentials survive CLI, owner and keyring restarts.
- Eleven read operations: projects, issues, repository files, exact-commit
  pipelines and jobs, bounded traces, merge-request inspection and update-window
  collection, and validation of a pinned MR head against a selected successful
  pipeline.
- An approved merge through the local CLI, with local approval policy, protected
  proof issuance, single-use write requests, an explicit private prepare/commit
  transport and authenticated bounded clock checks. A pinned head is checked at
  dispatch and an uncertain outcome is never retried into a second effect.
- Recovery for abandoned and keyed writes in the owner background, exact audit
  acknowledgement recovery, and revoked-merge audit finalization verified across
  owner crash and replay.
- A failed terminal settlement keeps the known provider result rather than
  replacing it, and a connection revoked after a known effect refuses disclosure
  at admission instead of answering `not_attempted`.
- Protected approval-signing key initialization, status, rotation, recovery,
  revocation and retirement.

### Kubernetes and PostgreSQL

- Both reach the local CLI with saved credentials through the same persistent
  lifecycle, verified against real servers — k3s `v1.31.5+k3s1` via k3d and
  `postgres:17` — rather than fixtures.
- A backend-less EndpointSlice reads as empty rather than malformed. Kubernetes
  serialises such a slice with null members, and one Service with no ready
  backends previously failed discovery for every Service beside it.
- Helm release reads: revision history, deployed status, recorded values and
  rendered manifest, with bounded pages, explicit completeness and provenance
  naming the exact release Secret each observation came from. Values and manifests
  are disclosed only as redacted projections, and the default is refusal.

### Catalog

- OpenAPI source ingest with preserved provenance, a deterministic operation
  inventory with named gaps, bundle write and load through a local index, a
  coverage report, and locally authored TOML actions expanding to the same
  template.
- Two writers into one bundle directory keep both index entries. The
  read-modify-write is held under an exclusive lock; before it, concurrent writers
  erased each other's rows while both were told they succeeded.

### Contracts, models and tooling

- Shared service, operations, authentication, datasource, discovery, session and
  media semantics, with explicit compatibility and support boundaries. Native
  contracts, designs and authored ESS models sit with their owning adapters so
  each can be extracted independently.
- Connection, profile, acquisition and custody ownership, execution audit,
  bounded discovery state, historical mediated bindings and reusable immutable
  artifact provenance are modelled alongside the execution and evidence models.
- ESS 0.22.2 and AEP 0.55.0 are pinned by source identity with receipt-checked
  local binaries. All 48 original contract-review findings are resolved, with the
  independent reviews and source evidence retained.
- The gate re-derives every archived upstream source digest — 84 files across six
  manifests. Nothing had previously read one, so every pinned-evidence record in
  the repository was evidence by assertion.

### MCP contracts

- Both MCP specification revisions this repository authors against are pinned —
  `2026-07-28` primary and `2025-11-25` for interoperability — as 54 archived
  files with source URL, uncompressed SHA-256 and byte length.
- A `connectors_mcp` ESS root declares the MCP nouns and the three credential
  kinds the design keeps distinct, with every relation no source answers carried
  as an explicit marker rather than a chosen cardinality.
- A coverage matrix dispositions every revision, transport and capability the pin
  names as supported, explicitly refused or deferred, each with a reason and a
  source line.

### Platform and compatibility

Linux x86_64, Rust 1.88. Existing installed configuration and credentials are not
imported. The initial custody profile requires the qualified GNOME Keyring 50.0
daemon on ext4 with an already-unlocked encrypted login collection; see
[custody qualification](docs/local-secret-service.md).

### Limitations

- **No dedicated GitLab sandbox evidence.** All five GitLab stories remain open on
  that credential. The write controls are verified by disposable CLI journeys
  against a private HTTPS fixture, not against a GitLab server.
- **Helm release reads are fixture-verified.** No real cluster has answered those
  four operations.
- **MCP is contracts only.** No connection is made, no server starts, no credential
  persists and no transport is selected.
- Helm chart rendering, linting, registry access and rollback stay outside
  Connectors pending a decision on running external provider binaries.
- Source release only: no hosted release page, binary, container image or website
  deployment.

## 0.7.2 — 2026-09-07

- Add `connectors inspect upgrade` to report the installed CLI version, embedded catalog schema
  and digest, supported credential-file formats, and hosted session-metadata version. The command
  supports text, compact, JSON and YAML output, needs no configured state or running service, and
  includes source-installation guidance without checking remote releases or changing files.
- Keep the report independent of async runtime startup, including its internal socket creation,
  while preserving normal command dispatch, embedded use and output-error handling.

Existing catalog-schema, credential-file and operation-protocol versions are preserved. The
inspection command provides information; updating the binary remains a separate step.

## 0.7.1 — 2026-09-07

- Add bounded Jira issue search, project inventory and paginated comment reads; Confluence
  page searches with explicit continuation; and GitLab activity, issue, merge-request,
  pipeline, deployment and commit reads. Preserve stable source identities and revisions,
  validate bounds before credential access, retain comment visibility metadata and omit provider
  identity objects.
- Keep the incremental contracts aligned across personal catalog and native hosted adapters,
  including required deployment ordering when update-date filters are used.
- Preserve structured rate-limit advice when incremental reads request pagination headers.
- Keep generated catalog operations callable when their provider has no optional Connection
  remediation metadata, and make expired authentication recovery fail consistently.
- Document complete consumer-contract checks before upgrading a source-built installation.

## 0.7.0 — 2026-09-07

- Catalog write discovery now considers every admitting Connection, so an earlier read-only
  placement cannot hide a writable one. Search and describe report required approval; invoke
  still checks the explicitly selected Connection.
- Default operation, Connection and event commands to the local target regardless of saved hosted
  login. Use `--target hosted` explicitly; incompatible local configuration flags are refused.
- Run bounded operation search, describe, invoke and Connection metadata reads without a daemon
  when its socket is absent. Stateful sessions, activation and event flows still require
  `connectors serve local`; uncertain transport outcomes never trigger a fallback invocation.
- Add `kubernetes.namespace.list` and `kubernetes.workload.list` for activated Connections,
  returning admitted namespaces and Deployments with container images and desired/ready replicas.
- Expose personal GitLab Connections and four source-grounded pipeline-schedule operations:
  list, create, update and delete. Preserve the official request schemas, including inputs and
  omitted versus null fields; mutations require the selected Connection's write admission.
- Treat a consumer closing a successful output pipe as normal completion across output formats.
  Protocol refusals and other output failures remain failures.
- Add Operation v0alpha2 structured `rate_limited` refusals, optional trusted retry delay and
  source-grounded advisory rate metadata. Retain v1 with explicit loss of the new fields;
  neither advice nor version selection resends an invocation.
- Advance the canonical catalog and matching readers to schema 4 for personal OAuth declarations,
  retaining schema 3 request-semantics/rate behavior and frozen schema 2 and 3 artifacts.
- Add explicitly configured GitLab public PKCE and device authorization, token-info verification,
  refresh and durable recovery through one dedicated OAuth store. This initial personal custody
  is unsealed and development-only; private instructions use a terminal or owner-only file.
- Default operations to v0alpha3 with typed `authentication_required`
  refusals. Trusted local setup binds remediation to the intended operation and Connection, then
  stops ready for a separate explicit invocation. `operation --protocol-version v2` retains
  explicit interoperability; no negotiation, automatic session creation or invocation replay.

### Also included since the last published release (v0.6.0)

- Add bounded, read-only GitLab fetch sessions over internal TLS for the admitted default branch
  and exact commit, including Git protocol v2 support and the hosted client operation. Recheck
  identity, grants and project membership; bound session lifetime, depth, requests and bytes.
- Reuse a bounded set of provider HTTP connections while rechecking destination policy and DNS
  for every request.
- Keep concurrent hosted Connect Session expiry terminal after credential verification and custody
  awaits, so an expired session cannot become completed.
- Keep legacy GitLab connections inactive until a verified reconnect binds current authority;
  preserve their metadata and refuse recovery when a credential transaction's grant is absent
  or superseded.
- Refresh all twelve Cargo workspace lockfiles together with the release identity so cold runners
  can fetch and verify the complete locked graphs.

## 0.6.5 — 2026-09-05

- Keep legacy GitLab connections inactive until a verified reconnect binds current authority,
  while allowing the host and unrelated integrations to start. Preserve legacy metadata and
  refuse recovery of credential transactions whose grant is absent or superseded.

## 0.6.4 — 2026-09-05

### Added

- Support Git protocol v2 on the internal fetch proxy, with targeted reference discovery and an
  exact-commit shallow-fetch grammar. Preserve legacy negotiation and the existing control payload.
- Verify the admitted branch and HEAD after filtering provider references; bound capabilities,
  packet framing and streamed pack sections. Reference commands leave the final fetch available,
  and interrupted transfers require a fresh source capability.

### Performance

- Reuse a bounded set of provider HTTP connections while rechecking destination policy and DNS on
  every request. Clients are separated by authority, origin and current admitted addresses;
  credentials and request deadlines remain request-specific.

## 0.6.3 — 2026-09-05

### Fixed

- Re-check a hosted Connect Session's `Pending` state after credential verification and custody
  awaits. Concurrent expiry is now terminal and cannot be overwritten with `Completed`; the ESS
  lifecycle marker for the former race has been retired with a deterministic regression test.

## 0.6.2 — 2026-09-05

### Fixed

- Refresh every satellite workspace lockfile after cutting the release identity, so cold release
  runners can populate each locked dependency graph before executing the sharded gate.

## 0.6.1 — 2026-09-05

### Added

- Add an internal TLS Smart Git byte plane that creates short-lived, read-only fetch sessions for
  one currently admitted GitLab project, provider default branch, and exact commit. Identity,
  Connection, Grant, project membership, branch and commit are revalidated at creation while the
  provider credential remains inside Connectors.
- Add the official hosted-client operation for creating a bounded Git fetch session. Stable
  idempotent locators survive retries while authorization and expiry rotate independently.

### Security

- Bound Git sessions by lifetime, depth, request count, transferred bytes, concurrent actor/global
  sessions, TLS handshakes and idle deadlines. Upload-pack is the only admitted Git service;
  protocol v2 and legacy grants without durable grant provenance fail closed.

## 0.6.0 — 2026-09-05

Every first-level word of the shipped binary moves, and bare `connectors serve` stops starting the
local server: the breaking change the preamble puts in a minor bump.

### Added

- Expose the hosted Slack organization bot's app and bot tokens through the tenant-bound
  administrative credential surface. A deployment can activate that bot without claiming that
  personal Slack OAuth is configured; the personal flow appears only when its paired client
  registration is present.

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

### Fixed

- Page the GitLab membership scan used by both project-binding discovery and redemption. A
  repository listed after the first 100 memberships no longer becomes inaccessible when a caller
  opens it, while malformed or unbounded provider pagination still fails closed.
- Accept a scope-omitting Claude Code refresh response only by carrying forward the scopes from the
  previously verified OAuth record. Initial responses without the required inference scope remain
  refused.

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
- The vendor specification script no longer writes out the address it exists to scrub. The
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
