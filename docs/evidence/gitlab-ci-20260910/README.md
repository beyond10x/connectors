# GitLab CI through the local CLI — 2026-09-10

The adapter adds `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get` and
`job.trace` beside its original three reads. Generated bindings call native
codecs through injected authenticated HTTP capabilities. The production CLI uses
the saved connection, current permission and exact owned adapter process.
Dedicated GitLab sandbox C09 acceptance and the broader initiative remain open.

## Inputs and boundaries

Source parent: `935ae0b372ceae19ea3070c90a1c7cc4c13c730c`. The commit containing
this receipt identifies the increment. [Source hashes](implementation-sha256.txt)
cover 459 non-planning, non-receipt inputs; [executable hashes](executable-sha256.txt)
identify the tested CLI, adapter, pinned ESS, keyring and D-Bus artifacts.
[Tool versions](tool-versions.txt) record the environment. Dependencies, the exact ESS pin, original GitLab vendor bytes
and SQLite migrations are unchanged. Shared ESS has 392 declarations; the native
GitLab model has 12, including transient Pipeline, Job and Trace values.

Full SHA/ID equality is checked against every projected result. Native statuses
remain visible, including unknown statuses; another SHA's successful pipeline
cannot satisfy selection. Current job attempts are paged with opaque continuations
bound to instance, descriptor, connection partition and exact selectors. Mutable
offset pages do not constitute a provider snapshot.

The additive SDK/host HTTP prefix capability retains a fixed bounded body and
reports complete only after observed EOF. Unsupported ports refuse without an
unbounded fallback. Native trace interpretation narrows the 512000-byte transport
ceiling to the caller limit and trims a split UTF-8 suffix only on incomplete
data. It never promises that a running job's trace will stop growing. No range,
automatic retry or raw trace diagnostic is introduced.

The CLI fixture exposed a prior error-projection discrepancy. Provider missing
resources, rate limits and internal errors now retain their shared codes through
closed private failure variants and the existing `Failure.service_code`. Local
selection/admission errors retain their meanings. The existing typed CLI failure
and service error-code owners suffice; this adds no persistent entity or authority.

## Verification

All Cargo commands use two jobs and task-owned
`TMPDIR=.local/tmp/gitlab-runtime-20260910`, with its `target` child as
`CARGO_TARGET_DIR`. Production CLI tests set `CONNECTORS_TEST_CLI` to the built
`debug/connectors`. They use fictional PATs and isolated HTTPS, D-Bus and qualified
GNOME Keyring processes. Builds and the long CLI fixture suite run serially when
they share executable paths: a rebuild is correctly refused by an existing
connection's exact artifact selection.

| Command | Result | Receipt |
|---|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed: Rust 1.88, ESS, generation, conformance, workspace tests, Clippy and boundaries | [Gate](repository-gate.log.gz) |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All four production CLI journeys passed, 176.03 seconds | [CLI tests](gitlab-cli-tests.log.gz) |
| `cargo run --locked --offline -p connectors-build -- package --output .local/tmp/gitlab-runtime-20260910/ci-package --image connectors-v2-gitlab:ci-20260910 --jobs 2` | Local Linux image built; ESS realization validated and compiled | [Build](package.log.gz), [identity](package-build.json) |
| `npm run build` in `website` | Passed: 113 indexed pages, 465 public files audited | [Website](website-build.log.gz) |
| `npm run typecheck` in `website` | Passed | [Typecheck](website-typecheck.log.gz) |
| `cargo run --locked --offline -p connectors-build -- docs --check` | Passed: 42 contract pages, 92 reference pages, no drift | [Reference check](docs-check.log.gz) |
| `aep plan artifact validate` | 163 artifacts valid; 89 review notices, including seven explicit empty-list approvals | [Verbatim validation](aep-validation.log) |

The exact CLI fixture executables are retained under the task's `ci-verified-bin`
directory at the recorded digests. Later Cargo feature selections can replace
the mutable `target/debug` paths without changing that retained evidence.

The CI journey deletes its credential source, restarts the owner and uses the
saved connection to observe pending, running and failed status for one full SHA.
It rejects a different SHA's successful pipeline, pages two jobs, reads the
selected failed trace, exercises caller truncation and the transport ceiling with
a split UTF-8 suffix, and distinguishes an erased trace, service unavailability
and rate limiting. It explicitly revalidates saved credentials between workflow
stages; ordinary reads never revalidate implicitly. Removing permission refuses
an invocation using previously captured operation/schema selectors before any
provider call.

Native tests cover malformed/foreign targets, unknown statuses, wrong schemas,
project scope, nonadvancing/duplicate paging, cursor partition changes, empty
pages and UTF-8/error handling. HTTP fixtures cover fixed/chunked EOF, exact and
oversized bodies, premature close, a stalled remainder, original deadline and
caller cancellation, header limits, redirect refusal and unsupported ports.

Initial failed runs are not acceptance evidence. Fixture corrections covered
empty-query URL counting, the CLI's service-code envelope, and capturing selectors
before removing permission. The missing-trace assertion exposed the real provider
failure-mapping defect described above. A packaging rebuild during a restart
fixture changed its selected executable and correctly caused configuration
refusal; final runtime verification uses fixed executables without concurrent
builds. These observations are kept separate from the final passing results.

## Packaging and planning

The existing Linux service realization packages all eight operations into a local
development image, `sha256:6b1a239d1cdf38c222ffad18c40f30d46f5eeef65900928958f8a9c47d6bf907`.
Its [source snapshot](package-source.json.gz), [rootfs manifest](package-rootfs.json)
and [ESS realization](package-realization.json) identify the precise payload.
The packaged binary has SHA-256
`a5a49929deb1c3f25f5b73bb4db826734418dc26e770e60d673f9c32b15e6d9a`;
the CLI fixture's Cargo test feature selection produces the separately recorded
adapter executable. Packaging captured the uncommitted source snapshot under the
recorded parent; subsequent edits to fixture assertions and planning/receipts do
not change its runtime source. This is local packaging evidence, not dedicated provider
acceptance, deployment or reproducible-distribution proof. Two isolated generation
trees are compared by the generation suite; full reproducible distributable/image
acceptance remains an initiative obligation.

Four independent AEP planning critics ran in two bounded rounds. One acceptance
finding was fixed and recorded; the second round approved all four perspectives.
The exact immutable review texts include empty findings lists. AEP 0.55.0 still
reports those empty-list approvals as missing blocks; the records were preserved.
The story remains active because dedicated sandbox evidence is missing.

## Remaining work and workspace

GitLab MR/changed-record reads and admitted MR create/update/merge, with their
required approval/audit/attempt/dispatch/idempotency controls, precede Kubernetes.
Kubernetes and PostgreSQL then receive the complete local lifecycle and selected
workflows, followed by MCP and the remaining providers. Shared management gaps
and all selected sandbox requirements remain in the initiative.

Connectors stays local on `main` under the repository's single-agent rule.
Worktree inspection found no linked Connectors trees. GC dry-run reviewed the
profile's 47 records and found none belonging to this task; no cleanup was applied.
The active goal's build cache and local package remain task-owned. MCP's primary
documentation-workflow edit is unchanged and its checkout remains ten commits
behind remote-tracking main. Harness is unchanged. No provider sandbox or desktop
keyring was touched, and no Connectors source was published externally.
