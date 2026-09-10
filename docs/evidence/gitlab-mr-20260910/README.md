# GitLab MR reads through the local CLI — 2026-09-10

The adapter adds `merge_request.get` and `merge_requests.list` beside its eight
existing reads. A saved connection can inspect a project-local IID and traverse
an inclusive update window. The native contract bounds and validates observations,
windows and continuations; unknown statuses and missing heads never imply merge
readiness. Dedicated GitLab sandbox evidence and full C14/C21 acceptance remain open.

## Inputs and ownership

Source parent: `c22ddfe60abf43ab63c43526d49d87843ed9ec60`. The commit containing this
receipt identifies the increment. [Implementation hashes](implementation-sha256.txt)
cover 463 non-planning, non-receipt source inputs; every one matches the package's
captured source manifest. [Executable hashes](executable-sha256.txt) identify the
CLI, adapter, pinned ESS, qualified keyring and D-Bus used by the production CLI
journeys. [Tool versions](tool-versions.txt) record the local environment.

The original GitLab vendor source, Cargo dependencies and SQLite migrations are
unchanged. Native ESS validates and compiles 14 declarations, including the new
transient Observation value; shared ESS remains at 392. The
[MR contract](../../../adapters/gitlab/contracts/merge-requests/v1alpha1/semantics.md)
owns the native semantics.

`aep plan reverse openapi --domain connectors_gitlab.merge_requests adapters/gitlab/upstream/openapi_v3.yaml --out .local/tmp/gitlab-runtime-20260910/merge-requests-import.yaml`
succeeded. Its [full draft](aep-openapi-draft.yaml.gz) preserves unresolved full-provider
ownership/lifecycle questions; the authored native model selects a scalar read
projection, with no invented local provider entity graph. This draft is a derivative
of GitLab's pinned documentation source and retains its [CC BY-SA attribution](../../../adapters/gitlab/upstream/README.md).
The source's MR list response describes one object; array projection remains a
native finish obligation. Source state enum constraints are retained in source
order, including `locked`.

The additive generator seam preserves only a source string `format:date-time`
with an identical caller annotation. Unknown/non-string formats, mismatched
annotations and formatted binding values refuse; constants enable format assertions.
Native preparation validates calendar dates and interval order before transport.
Generated inputs remain strings. No write transport or approval authority is added.

## Verification

Cargo uses two jobs, task-owned `TMPDIR=.local/tmp/gitlab-runtime-20260910` and its
`target` child as `CARGO_TARGET_DIR`. Build/package commands are serialized with
production CLI journeys sharing selected executable paths. Those journeys use
fictional PATs and disposable HTTPS, D-Bus and qualified GNOME Keyring processes.
`CONNECTORS_TEST_CLI` selects the built `target/debug/connectors`.

| Command | Result | Receipt |
|---|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed: ESS, deterministic generation, conformance, workspace tests, Clippy, boundaries and Rust 1.88 | [Final gate](repository-gate.log.gz) |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | Five production CLI journeys passed in 225.97 seconds | [CLI tests](gitlab-cli-tests.log.gz) |
| `cargo test --locked --offline -p connectors-host --test local_foundation` | Seven tests passed, including held-lock recovery and 16 concurrent setup rounds | [Foundation tests](foundation-tests.log.gz) |
| `npm run build` in `website` | Passed: 114 indexed pages, 468 public files audited | [Website build](website-build.log.gz) |
| `npm run typecheck` in `website` | Passed | [Typecheck](website-typecheck.log.gz) |
| `cargo run --locked --offline -p connectors-build -- docs --check` | 43 contracts, 93 reference pages, no drift | [Reference check](docs-check.log.gz) |
| `cargo run --locked --offline -p connectors-build -- package --output .local/tmp/gitlab-runtime-20260910/mr-package --image connectors-v2-gitlab:mr-20260910 --jobs 2` | Local Linux image built; realization validated and compiled | [Package](package.log.gz), [identity](package-build.json) |
| `aep plan artifact validate` | 172 artifacts valid; 96 review-format notices | [Verbatim validation](aep-validation.log) |

The gate includes six native MR tests and eight generation tests. They prove
bounded projection, exact identities, null/deleted-source fields, unknown statuses,
calendar/window/filter/order/duplicate refusals, safe provider errors, exhaustive
selection and connection cursor binding, format constraint refusal, drift detection
and identical generated trees from isolated output directories.

The new CLI journey deletes its protected credential source, restarts the owner,
pages a fixed window to explicit exhaustion, retrieves an MR and preserves null
head/source information. Invalid calendars, changed cursor selectors, outside
projects and removed policy authority refuse before provider work. Missing native
MRs retain the service error code. Two explicit saved-credential revalidations
refresh the bounded baseline; ordinary reads do not implicitly authenticate.
The earlier persistence, repair/stop, expiry and CI journeys also pass.

The exact tested executables remain under the task's `mr-verified-bin` directory.
Later Cargo feature selections can replace mutable build paths; the retained
executables and their recorded digests preserve the production CLI evidence.

## Initial refusal and contention evidence

Generation first refused a narrowed/reordered state enum, then the unsupported
source date-time annotation. The final declaration preserves the source enum, and
the explicit format seam is separately tested. An early compile attempt while
generation was refused correctly lacked the new generated bindings; it was not a
successful runtime verification.

The [initial gate](repository-gate-initial.log.gz) failed concurrent setup with one
success, two MetadataUnavailable responses and one ConfigurationExists. The existing
metadata lifecycle lock already has a two-second deadline. The test had required
all losers to return ConfigurationExists without measuring elapsed time. The
follow-up leaves runtime code unchanged: a MetadataUnavailable result is acceptable
only after that existing bound, every loser must converge to ConfigurationExists
on a subsequent explicit call after the winner completes, and final metadata must
validate. A deterministic held-lock test proves bounded refusal and recovery.
Early unavailability still fails. The initial failure's exact internal stage was
not instrumented; attributing it to lock expiry remains an inference. Both the
focused follow-up and final full gate pass. This test-only change does not invalidate
the earlier production CLI executable evidence.

## Packaging, planning and remaining work

Local image ID: `sha256:f10a2929e039bcf72ae215a9d96c9bf95c2780beb66dfd1c8eddf41446cf7ea8`.
Packaged binary SHA256: `3982ac02be6b767f9fcb8f594c8102a90ae5e0155bdb86f93099c3a0d2b516f0`.
[Rootfs](package-rootfs.json), [source](package-source.json.gz) and
[realization](package-realization.json) retain exact inputs. Packaging captures
an in-progress tree under the source parent; later planning/receipt edits do not
change its 463 matched implementation inputs. Its normal-feature ELF differs from
the CLI test build and is identified separately. The image build is not a container
business-runtime or dedicated-provider test. Two isolated distributable/image builds
remain required by the final product milestone; only generation reproducibility is
proved here. No image or Connectors source was externally published.

The initiative has three decomposing runtime stories. Four independent critics ran
in two bounded rounds. Both acceptance findings were fixed; all final verdicts
approve. Review text is retained verbatim, including empty findings lists. The
installed AEP parser reports explicit empty lists as missing blocks; the 96 notices
include seven new MR approvals. Historical review records remain unchanged. The
requested Sonnet was unavailable, so reviewers used the disclosed inherited model;
three available slots staged the fourth perspective without sharing findings.

[Worktree inspection](worktree-inspection.json) finds no Connectors linked trees.
The profile-wide GC dry run reviewed 47 records, none belonging to this checkout;
no removal was applied. Active-goal build caches and verified artifacts remain local.
Atlas was clean and matched remote main at `0b5278d1075dc645bcd99b053d086fcbe804b35e`
when authority was checked. MCP was separately observed clean at
`bf7f9415fbd8a28abff789d7d39d2584a8f6d2b2`; this increment changed neither repository.
Its earlier primary-workflow edit is no longer present as a local diff; this receipt
does not claim to have integrated it.

Next: model native MR validation and guarded create/update/merge, then implement
shared approval/audit/attempt/dispatch/idempotency controls and those writes. Full
GitLab precedes Kubernetes, PostgreSQL, MCP and remaining providers. The dedicated
GitLab credential blocker, other selected workflow acceptance, full management across
three adapters, MCP interoperability and reproducible distribution remain open.
The distinct paid AEP driver protocol-loading blocker is unchanged; paid governed
runs are not required for this interactive implementation.
