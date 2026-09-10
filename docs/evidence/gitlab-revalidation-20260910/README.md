# Saved GitLab credential revalidation — 2026-09-10

The production CLI now supports `connections revalidate --adapter ALIAS
--connection REF --expected-revision REVISION`. It renews native baseline evidence
from the exact saved credential without re-entry. GitLab's 60-second evidence
deadline remains enforced; ordinary reads perform no implicit identity probe.
Dedicated provider acceptance and the broader initiative remain open.

## Inputs and implementation

Source parent: `de91b848d5e7f692c1686a10505e3f853fe5b280`. The commit containing
this receipt identifies the increment. [Source hashes](implementation-sha256.txt)
cover 87 affected inputs and support files; [executable hashes](executable-sha256.txt)
identify the tested CLI, adapter, pinned ESS, keyring and D-Bus artifacts.
[Tool versions](tool-versions.txt) record the verification environment.
Dependency versions, vendor inputs and SQLite migrations are unchanged.

The existing Connection baseline and immutable generation/custody owners support
same-generation recollection. Three additive CLI values were modeled and validated
before dependent implementation; shared ESS compiles 390 declarations and native
GitLab eight. The evidence contract now distinguishes a fresh capture from creation
of a new generation, consistent with the existing connection contract. No new
entity, acquisition, credential write or evidence-history store is introduced.

The registry captures the existing version and fence in a bounded transient use,
consumes it once before native validation, and replaces the baseline only under
the current fence. Successful publication advances that fence, preserving the
connection identity, public revision, generation and version. Repair, revocation,
competing publication, invalidity and original deadlines refuse stale results.
Known expiry cannot disappear from recollected evidence. Positive current-material
identity mismatch or native invalidity cuts off use; transient failure preserves
any still-valid baseline. An unknown acknowledgement grants no replay authority.

## Verification

All Cargo commands use two jobs, `TMPDIR=.local/tmp/gitlab-runtime-20260910`
and its `target` child as `CARGO_TARGET_DIR`. Explicit native tests set
`CONNECTORS_TEST_CLI` to the built `debug/connectors` there. They use task-owned
private HTTPS, D-Bus and qualified GNOME Keyring processes and fictional tokens.

| Command | Result | Receipt |
|---|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed: Rust 1.88, ESS, generation, conformance, workspace tests, Clippy and boundaries | [Gate](repository-gate.log.gz) |
| `cargo build --locked --offline -p connectors` | Built the tested production CLI | [Build](cli-build.log.gz) |
| `cargo test --locked --offline -p connectors-host --lib local:: -- --include-ignored --test-threads=1` | 31 passed, including native custody fixtures | [Host tests](host-native-tests.log.gz) |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime cli_journey:: -- --ignored --test-threads=1` | All three production CLI journeys passed, 140.32 seconds | [CLI tests](gitlab-cli-tests.log.gz) |
| Pinned `ess generate cli` in two empty task-owned output roots, then `diff -qr --exclude=.ess-output` between both and the checked-in package | Ten generated artifacts identical | [Generation](cli-regeneration.log) |
| `npm run build` and `npm run typecheck` in `website` | Passed: 111 indexed pages, 459 public files audited | [Build](website-build.log.gz), [typecheck](website-typecheck.log.gz) |
| `cargo run --locked --offline -p connectors-build -- docs --check` | Passed: 41 contract pages, 90 reference pages, no drift | [Documentation](docs-check.log.gz) |
| `aep plan artifact validate` | 154 artifacts valid; the same 82 historical prose-only review warnings remain | [Verbatim validation](aep-validation.log) |

The new runtime journey deletes the credential source, shuts down the owner,
waits for real evidence expiry, observes `pending`, and proves that a refused
business read starts no owner and makes no provider call. Explicit revalidation
then starts the owner, performs the two declared native checks, and restores a
successful project read. A forced 503 leaves valid evidence usable; a forced 401
changes readiness to `reauthorization_required` and subsequent revalidation
refuses before startup. The existing two journeys also pass on these inputs.

Four new registry tests cover exact-version reuse after expiry, known-expiry
preservation, lost publication acknowledgement followed by restart observation,
one-use dispatch/completion, unchanged deadlines, competing recollections,
pending-read cutoff, both repair/publication orders, terminal revoke, transient
failure, identity mismatch and refusal to resurrect positively invalid material.
The generated-parser conformance suite includes the sixteenth command and proves
that revalidation never consumes a protected source.

The first full gate correctly refused the old fifteen-command inventory assertion.
That expectation and the typed result fixture were updated; the final gate passes.
The initial registry test also expected repair refusal at preparation, while the
existing implementation correctly refuses earlier at consumption. Its assertion
now checks that earlier boundary. Neither initial failure is counted as evidence.

## Remaining work and workspace

Dedicated GitLab sandbox access remains missing under
`credential-blocker:gitlab-runtime-sandbox`; no desktop keyring or real provider
resource was touched. The paid AEP driver protocol-loading blocker remains
unrequalified. This fixture evidence does not close either blocker or the story.

The remaining local management cases include failed-startup cohort propagation,
positive credential-invalidity reduction after business reads, complete paging
and bounded expiry/retirement scheduling. Kubernetes and PostgreSQL still require
the local lifecycle binding, before MCP and the remaining providers. Full provider
workflows and reproducible distributable/image digests remain unverified.

Connectors remains local on `main`, following the single-agent checkout rule.
[Worktree inspection](worktree-inspection.json) found no linked trees; GC dry-run
examined 47 profile records and found none belonging to this task, so nothing was
removed. The active goal's build cache remains task-owned. The MCP primary's
pre-existing `.github/workflows/b10x-docs-pages.yml` edit remains untouched; its
checkout is still ten commits behind remote-tracking main. Harness is unchanged.
