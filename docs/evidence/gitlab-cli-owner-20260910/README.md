# Protected CLI and persistent GitLab owner — 2026-09-10

The production CLI now connects and repairs a GitLab connection through protected
input, saves its exact credential version in qualified Secret Service custody,
and invokes the existing reads through a persistent local owner. Disposable
GitLab HTTPS/keyring fixtures prove reuse after CLI, owner and keyring restart,
including four concurrent reads without credential re-entry. This is a local
runtime increment; dedicated provider acceptance and the full goal remain open.

## Inputs

Source parent: `66cd3305e33a32d82a6c30d1b080d70f2fc81ea0`. The commit containing
this receipt identifies the increment. [Implementation hashes](implementation-sha256.txt)
cover the affected source/model/contract roots, generated CLI/native inputs,
dependency pins and support documents. [Executable hashes](executable-sha256.txt)
identify the tested production binaries, pinned ESS, GNOME Keyring and D-Bus.
[Tool versions](tool-versions.txt) record Linux x86_64, Rust 1.98.1, the checked
Rust 1.88.0 compatibility toolchain and the observed AEP/website tools.

No dependency version or vendor input changed. Five additional shared owner
configuration/capture/cache values were modeled and validated before dependent
implementation. Shared ESS now compiles 387 declarations; GitLab still compiles
eight. The generated CLI and native bundles are unchanged and pass drift checks.
SQLite migration three adds cached bootstrap and durable stop suppression without
moving credentials out of Secret Service or combining acknowledgement groups.

## Verification

All Cargo commands used two jobs, `TMPDIR` at
`.local/tmp/gitlab-runtime-20260910` and `CARGO_TARGET_DIR` at its `target` child.
The explicit native tests also set `CONNECTORS_TEST_CLI` to that target's built
`debug/connectors`. Tests ran with one test thread in the environment-dependent
suites; each fixture owned its processes, Unix sockets, private directories and
fictional credential material.

| Command | Result | Receipt |
|---|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed: ESS, generation/drift, conformance, workspace tests/Clippy, adapter boundaries and Rust 1.88 all-target check | [Full gate](repository-gate.log) |
| `cargo build --locked --offline -p connectors` | Built the production CLI used by the native tests | [Build](cli-build.log) |
| `cargo test --locked --offline -p connectors-host --lib local:: -- --include-ignored --test-threads=1` | 27 passed, including three native custody/registry fixtures and two subprocess fixture entries | [Local host/native log](host-native-tests.log.gz) |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime cli_journey:: -- --ignored --test-threads=1` | Both production CLI journeys passed | [CLI journeys](gitlab-cli-tests.log.gz) |
| Workspace tests within the full gate | The three existing GitLab private process/TLS cases passed; CLI compatibility and management tests passed | [Full gate](repository-gate.log) |
| `npm run build` in `website` | Static build, reference/examples/search generation and 459-file public audit passed | [Website build](website-build.log.gz) |
| `npm run typecheck` in `website` | Passed | [Typecheck](website-typecheck.log) |
| `cargo run --locked --offline -p connectors-build -- docs --check` | 41 contract pages, 90 reference pages; no drift | [Documentation check](docs-check.log) |
| `aep plan artifact validate` | 154 artifacts valid; the same 82 historical prose-only review warnings remain | [Verbatim validation](aep-validation.log) |

The first concurrent restart fixture exposed a real clock-ordering defect:
timestamps sampled before waiting for the metadata lock could be rejected as
regression. The production registry now samples inside its transaction; a separate
test retains the refusal for an actual clock below the durable floor. The first
project assertion was also corrected to use the native `item` envelope. Neither
failure was treated as successful evidence. The final gate and native suites ran
after the implementation corrections, including cancellable partial reply reads.

## Observations

The two CLI fixtures use the real `connectors` and `connectors-gitlab` binaries,
a private HTTPS server with a captured CA, and a separately owned qualified
GNOME Keyring daemon. They exercise:

- Rejection before protected-source access when profile permission is absent;
  unsafe file refusal; successful owner-only file and deliberate stdin entry.
- Native identity validation, acknowledged immutable storage, connection
  publication, cached schema discovery and a real allowed project read.
- New CLI processes throughout; owner and keyring restart; deletion of the
  original fixture credential source; four simultaneous successful reads using
  the retained credential version with no new native identity calls.
- A changed-identity repair refusal that preserves the existing ready connection
  and public revision; successful reuse of that still-valid credential.
- Duplicate business JSON, stale schema and narrowed policy refusal before a
  provider request; metadata containing no fixture credential bytes.
- Stale stop leaving the current child intact; exact stop suppression; explicit
  resumed startup; responsive status/stop while a provider read is blocked.
- A pending capture from before stop refusing completion without reviving the
  child; unavailable custody refusing before restart; terminal local revocation
  refusing another read without starting the owner or contacting the provider.

Host tests additionally cover PTY echo suppression and restoration after SIGINT,
partial-frame cancellation without a reset deadline, source path/mode/link/size
refusals, migration 2→3 authority preservation, publication/revoke/dispatch races,
unknown custody/publication acknowledgement and qualified daemon restart/deletion.
Exact private-process mismatch and result-schema tests remain covered by the gate.
These establish different facts from the structural CLI/ESS conformance checks.

## Remaining work and workspace

`credential-blocker:gitlab-runtime-sandbox` remains open. No dedicated GitLab
account, desktop keyring or real provider resource was accessed. The current
validation evidence lasts at most 60 seconds; explicit revalidation of retained
material is required for lasting reuse without repair/re-entry. Remaining
management work includes sharing a failed startup outcome across its waiting
cohort, reducing positive native credential-invalidity into connection readiness,
complete paging, and bounded expiry/retirement scheduling. An already dispatched
read can finish under its original deadline after its CLI disconnects.

This increment admits no business writes or automatic business retries. It does
not qualify a different keyring implementation/filesystem, power-loss behavior,
MCP interoperability, all-provider runtime support or reproducible distributable
payloads/images. No presentation code changed; unrelated browser interaction
tests were not repeated. The AEP paid-driver blocker was not requalified.

Work used Connectors' single-agent primary-checkout rule. The
[worktree inspection](worktree-inspection.json) reports no Connectors linked
trees; the workspace GC dry-run found no Connectors/GitLab task record, and no
foreign tree was removed. The active goal's ignored build cache is retained.
Atlas primary was clean and verified against remote main for bot commit authority;
the MCP primary's pre-existing documentation-workflow edit was preserved.

The story, initiative and three-provider goal remain active. Finish GitLab evidence
renewal and the remaining local management cases, then integrate Kubernetes and
PostgreSQL, followed by MCP and the remaining providers. Connectors remains local.
