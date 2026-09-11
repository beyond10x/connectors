# Kubernetes through the local CLI

Development increment under active `story:persistent-kubernetes-journey`, based on
the source commit that carries this receipt. The Kubernetes provider batch and the
parent goal remain open.

## Implemented behavior

`adapters/kubernetes` gains an executable composition, so the adapter is reachable
from the local CLI for the first time. `--local-config` loads an owner-only
`connectors-kubernetes-local/1` document and `--print-local-bootstrap` prints the
independently computed bootstrap and configuration revision, matching the shape
`adapters/gitlab/src/local.rs` established. The federated `--config` service mode
is unchanged.

The native `kubernetes.token` profile is an `http_bearer` credential with no
required scopes, because Kubernetes issues no scope grant with a token. Identity
validation issues one SelfSubjectReview, the probe
`adapters/kubernetes/contracts/auth/v1alpha1/semantics.md` selects. The saved
identity is the authenticated username, since RBAC binds to that name and the
optional `uid` is authenticator-dependent. An accepted anonymous principal is
refused rather than saved. No credential expiry is observed, and the baseline says
so rather than reporting no limit.

The probe travels on a new check-specific capability. `connectors_sdk::AuthProbe`
posts one bounded document to an endpoint the trusted composition fixes when it
builds the port; `ScopedHttp::probe_capability` is the only constructor and a
business adapter holding `AuthenticatedHttp` cannot reach it. This keeps the
contract's requirement that ordinary business reads cannot invent the probe, and
it is separate from the existing consuming PUT capability.

`Kubernetes::with_authenticated_http` rebinds the business adapter to one
host-admitted credential and partitions cursor state by connection, so a
continuation issued under one connection is unreadable under another. The three
existing read operations are otherwise unchanged.

`adapters/kubernetes/spec/adapter.json` gains the local configuration shape beside
the federated one, and `generated/descriptor.json` was regenerated from it with the
pinned ESS; the generated file was not edited by hand.

Per-operation SelfSubjectAccessReview permission checks are **not** implemented.
The auth contract makes exact request/response interpretation and a supporting
capability implementation advertisement prerequisites; no result claims
`authorization` coverage.

## Verification inputs and results

Linux x86_64, Rust 1.98.1 with the Rust 1.88.0 compatibility check the gate runs.
`CARGO_BUILD_JOBS=2`. The repository gate used the task-owned `.local/tmp`.

| Command | Result | Log |
|---|---|---|
| the 13 gate steps, run individually | all exit 0 — see the rebase section below | [gate.log.gz](gate.log.gz) |
| `CONNECTORS_TEST_CLI=target/release/connectors cargo test --release --locked --offline -p connectors-kubernetes --test local_runtime -- --include-ignored --test-threads=1` | 9 passed, 0 failed, 0 ignored, 7.64 s (8 before the adversary pass) | [kubernetes-runtime.log.gz](kubernetes-runtime.log.gz) |
| `CONNECTORS_TEST_CLI=target/release/connectors cargo test --release --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1` | 12 passed, 0 failed, 177.98 s | [gitlab-journeys.log.gz](gitlab-journeys.log.gz) |

The GitLab run is the regression check for the shared `connectors-sdk` and
`connectors-host` additions; all twelve existing GitLab CLI journeys still pass.

Recorded executable digests are in
[runtime-artifacts.sha256](runtime-artifacts.sha256).

### What the eight Kubernetes cases establish

Six run against a local TLS fixture cluster through the private runtime:
SelfSubjectReview validation and two distinct identities; a POST to exactly the
review endpoint and no other; refusal of an unknown profile and of a malformed
protected entry without a provider call; namespace and resource-kind refusal
before provider work; paged `resources.list` continuation; cursor refusal across
connections; `endpoints.discover` expansion; `hosts.discover` absent from the
descriptor while `discover_hosts` is false and present while it is true; changed
CA, instance, revision and executable digest refused at spawn; and a lost
validation response closing the channel without replay.

Two run the production CLI against a disposable dbus/GNOME Keyring Secret Service:

- `persistent_kubernetes_cli_owner_and_keyring_restart` — fresh `setup init` and
  `setup check`, an undeclared profile refused before the owner starts, protected
  connect from an owner-only file, `operations describe` and `operations invoke`,
  then owner shutdown, keyring restart and deletion of the credential file,
  followed by four concurrent CLI processes reading through one newly started
  owner on the saved exact credential version. Six provider calls total: one
  identity probe and five reads, with no re-entry. An advertised operation left
  out of the configured permissions is refused `forbidden` at `operations
  describe`. Terminal local revocation survives the following restart.
- `kubernetes_cli_refuses_a_changed_cluster_identity_and_preserves_the_saved_credential`
  — `connections repair` carrying a different username is refused
  `identity_mismatch`, the connection stays `ready` at its original revision, and
  a subsequent `endpoints.discover` still succeeds on the preserved credential.

## Limitations

No dedicated Kubernetes sandbox evidence exists. Every result above comes from a
local TLS fixture cluster; no real cluster was contacted and no real credential
was used. Fixture tokens and usernames are fictional.

The managed worktree path is long enough to break two different Unix sockets,
both bounded at 108 bytes. The disposable dbus session the CLI journeys need
cannot be created under the documented `.local/tmp` there, and neither can
sccache's server socket, which fails the build with
`sccache: error: path must be shorter than SUN_LEN`. Both runs therefore used a
shorter task-owned `TMPDIR`.

**`connectors-build gate --msrv` cannot run as one command in a managed worktree,
and an external `TMPDIR` does not fix it.** `gate.rs:6` fixes its tempdir base to
`root/.local/tmp`, and `gate.rs:14`, `:213` and `:229` then set `TMPDIR` for every
child to that directory, overriding whatever the caller passed. Running the 13
steps individually is currently the only way, which is why this receipt carries
per-step exit codes rather than a single gate log. Reported; not changed here,
because nothing authorised that commit.

Kubernetes events, conditions, pod logs, exec, copy, port forwarding, every
mutation and the Helm workflows are not implemented. The v2 specification-derived
generation and packaging pipeline for Kubernetes remains open under
`story:kubernetes-spec-service`, which is separately blocked by
`tooling-blocker:kubernetes-driver-protocol-loading`.

## Adversary pass — 2026-09-11

A read-only adversarial review of this increment returned six findings. Package-
scoped runs only; no repository gate was run after these changes, so the gate row
above describes the earlier tree.

| Finding | Taken | What changed |
|---|---|---|
| 1. Anonymous refusal keyed on one username string | yes | `auth.rs` now refuses on the `system:unauthenticated` group as well as the `system:anonymous` username. Either half refuses alone, so a cluster that configures a different anonymous username is still refused. |
| 2. Published schema does not describe the documented config file | in part | Both `oneOf` branches in `spec/adapter.json` gained a `title` and `description` saying the local branch is the **effective** document the composition builds, not the file an operator writes; the descriptor was regenerated. `docs/local-kubernetes-cli.md` now says the same and says not to write the file to match that branch. The branch keeps validating the effective document, which is what `instance_descriptor` checks. |
| 3. Error paths with no test | yes | Three new cases, below. |
| 4. `journal.jsonl` conflicts at integration | noted, not fixable here | Recorded as an integration constraint; it cannot be resolved without landing a tree. |
| 5. `ca_digest` is not reproducible from the PEM | doc only | The guide now states that `configuration_revision` and its `ca_digest` are not `sha256sum` of the PEM and that no command reproduces them, so the value is copied from `--print-local-bootstrap`. The computation is unchanged and matches `adapters/gitlab/src/local.rs`. |
| 6. `main.rs` panic message, unlogged local path, `{error:?}` | no | Every line is byte-identical to `adapters/gitlab/src/main.rs`; see below. |

### Cases added

| Case | Establishes |
|---|---|
| `review_status_codes_map_to_distinct_failures_without_a_business_read` | 403 → `Forbidden`, 429/500/503/599 → `Unavailable`, 418/302 → `Protocol`, then a successful validation on the same child; every observed route was the review endpoint, so no business read was attempted |
| `http::tests::a_probe_endpoint_is_one_bounded_fixed_path` | `probe_capability` refuses an empty path, a 17-segment path and `""`, `.`, `..` segments with `InvalidInput`, and accepts a 16-segment path |
| `http::tests::an_oversized_probe_document_is_refused_without_any_request` | a document one byte over `PROBE_BODY_LIMIT` is refused `Capacity`; a document exactly at the limit is sent instead, reaching the transport |
| `an_accepted_anonymous_request_is_refused_rather_than_saved_as_an_identity` (extended) | both the well-known anonymous username and a renamed principal carrying only the `system:unauthenticated` group are refused |

The anonymous case was mutation-checked: removing the group half of the condition
makes it fail, and restoring it makes it pass.

### Finding 6, not taken

`src/main.rs:64` `expect("validated configuration mode")`, `logging()` on the
service path only, and `{error:?}` on the local path are each identical to
`adapters/gitlab/src/main.rs`. The local path deliberately stays unlogged: it
speaks the private protocol over inherited descriptor three, and the host reads
its stdout for `--print-local-bootstrap`. Changing one adapter alone would make
the pair inconsistent without fixing the class. It belongs in a separate change
covering both binaries.

### Pre-existing, found while establishing the baseline

`local::audit::tests::capacity_is_atomic_per_instance_without_eviction_or_append_restriction`
fails with `MetadataUnavailable` at `crates/connectors-host/src/local/audit/tests.rs:373`
when the whole `connectors-host` suite runs at default test parallelism. It passes
in isolation and passes at the gate's `RUST_TEST_THREADS=2`, which is how the gate
runs it. It is unrelated to this increment and is a parallelism sensitivity in that
test, not a defect this increment introduced.

## Rebased onto the new toolchain pins — 2026-09-11

The increment was re-based from `fc7bdf9` onto `6746696`, which carries ESS
`0.22.2` at `6b666e58f2e8` and the AEP pin `0.55.0` at `4eb999e0` plus patch
`73d78e50` — one of three builds cached for `4eb999e0`, and the one
`aep-toolchain.json` pins. No destructive git operation was used: a second managed tree
`kubernetes-rebase-20260911` was created on the new base and the work moved into
it, leaving `kubernetes-local-cli-20260911` intact as the backup.

**15 code paths applied with zero conflict.** `git diff fc7bdf9..6746696` touches
none of them — the two trees' only shared path was `.engineering/planning/journal.jsonl`.

**The 5 planning artifacts were re-minted, not copied.** Per decision 6's default,
the journal was not rewritten in place: 18 `aep plan artifact` commands re-created
the ADR, the specification, two stories and the blocker on top of the landed
store, plus the initiative's `Delivery sequence` section. `journal.jsonl` was never
hand-edited.

| gate step | exit | result under ESS 0.22.2 |
|---|---:|---|
| 1 ESS pin | 0 | `ess 0.22.2` |
| 2 `ess-boundary` | 0 | shared + 6 adapter models compiled independently |
| 3 `cli --check` | 0 | fixture matches |
| 4 `fmt --check` | 0 | 14 packages |
| 5 descriptor drift ×3 | 0 | gitlab, kubernetes, sql all byte-identical |
| 6 `build --workspace` | 0 | |
| 7 `test --workspace` | 0 | **292 passed, 0 failed, 33 ignored, 54 binaries** |
| 8 `clippy -D warnings` | 0 | 0 errors |
| 9 adapter boundary ×3 | 0 | |
| 10 generic CLI boundary | 0 | |
| 11 `+1.88.0 check` | 0 | |
| 12 `ess verify conform synthesize` | 0 | 315 scenarios (34 authored), 22 refusals |
| 13 `aep plan artifact validate` | 0 | 288 artifacts `valid`, **0 warnings** |
| Kubernetes journeys incl. ignored | 0 | 9 passed, 0 failed, 15.55 s |

Two results worth naming. The ESS bump moved **no** generated bytes: step 5 is
byte-identical for all three adapters, which the source predicted —
`crates/connectors-spec/src/main.rs:21-23` resolves ESS only inside
`if args.generate`, and Kubernetes uses the non-`--generate` path. And step 13's
139 `review-result` prose-findings warnings are now **0**, because the pinned
patched AEP reads a valid empty findings block correctly.
