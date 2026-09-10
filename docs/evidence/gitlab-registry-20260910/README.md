# Connection registry checkpoint — 2026-09-10

This increment implements SQLite connection/acquisition authority, separate
immutable custody acknowledgement and publication, repair/revoke/read-dispatch
fences, guarded retirement, and production CLI connection list/describe/status/revoke.
The [binding decision](../../local-connection-registry.md) states the selected
transaction groups and limits. Protected connect/repair, explicit revalidation,
adapter bootstrap and supervised invocation still need implementation. The
persistent GitLab journey and three-provider goal remain open.

## Exact inputs

Source parent: `fded91e9188ef62b92c0dd0e594da465e9e32991`. The commit containing
this receipt identifies this increment; the receipt does not embed its own future
commit ID. [Input hashes](implementation-sha256.txt) bind the implementation,
shared/native model inputs, generated CLI, dependency pins and documentation.
[Tool versions](tool-versions.txt), [OS packages](os-packages.txt) and
[native executable hashes](os-artifacts-sha256.txt) record the observed environment.
This was Linux x86_64 on ext4, with the same qualified GNOME 50.0 source/artifact
as the [custody receipt](../gitlab-custody-20260910/README.md).

No dependency pin, vendor input or generated CLI byte changed. Generating directly
over the previously unowned CLI output was refused. Generation into a fresh
task-owned directory succeeded; every deliverable matched the checked-in files.
Only the generator's private `.ess-output` ownership record differed. The normal
`cli --check` passed. No ownership record or generated output was repaired by hand.

## Commands and results

Cargo commands used `CARGO_BUILD_JOBS=2`, with `TMPDIR` set to
`.local/tmp/gitlab-runtime-20260910` and `CARGO_TARGET_DIR` to its `target` child.

| Command | Result | Evidence |
|---|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed, including ESS ownership/compilation, CLI drift, native generation, conformance, workspace tests, Clippy, adapter boundaries and Rust 1.88 | [Full gate](repository-gate.log) |
| `CONNECTORS_TEST_CLI="$CARGO_TARGET_DIR/debug/connectors" cargo test --locked --offline -p connectors-host --lib local:: -- --include-ignored` | 15 passed, zero failed/ignored | [Native and coordinator log](native-registry-tests.log.gz) |
| `connectors-build docs --check` | 41 contract pages, 90 references, no drift | [Documentation check](docs-check.log) |
| `npm run build` in `website` | Static build, examples, search index and 459-file public-path audit passed | [Website build](website-build.log.gz) |
| `npm run typecheck` in `website` | Passed | [Typecheck](website-typecheck.log) |
| `aep plan artifact validate` | Valid; 82 unchanged historical review warnings | [Verbatim validation](aep-validation.log) |

The first gate run exposed a long argument list and test-module placement;
the final implementation fixes both. The native run is additional to the gate:
the ordinary gate deliberately skips the three environment-dependent fixtures.
Two comment-only clarifications followed the final gate; formatting was checked
again and no executable, contract, model, dependency or test behavior changed.
The first website build found no installed Docusaurus binary. `npm ci
--allow-git=root` restored the exact lockfile dependencies, and the subsequent
build passed. No dependency selection changed. Browser interaction tests were
not repeated because presentation and example behavior were unchanged.

## Runtime observations and limits

Nine SQLite coordinator tests cover private acquisition/publication boundaries,
restart, a committed publication whose reply is lost, preserved old credentials
after failed repair, identity/scope/expiry refusal, competing repairs, terminal
revoke, eight three-way revoke/repair/dispatch races, one-use dispatch, bounded
cursors, positive missing material and full retirement retention. Those tests use
explicit test-only custody receipts; they are metadata evidence, not native writes.
A separate migration test starts from the exact previous schema, proves passive
refusal without migration, then proves admitted migration preserves authority.

The native composition fixture uses real private D-Bus/GNOME processes and fictional
credentials. It writes through the coordinator, publishes, closes/reopens SQLite,
restarts the keyring locked and unlocked, reads the same exact retained version,
and exercises acquisition status and terminal revoke in separate production CLI
processes with an absent configured adapter executable. It then tests unknown
write acknowledgement, unpublished candidate status, delayed-writer refusal,
unknown delete acknowledgement, exact guarded reconciliation and deletion after
restart. No fixture accesses the desktop keyring or a real provider account.

CLI process tests also prove fresh authoritative empty lists, unknown references,
safe errors, stale cursors and missing metadata refusal without recreation or
adapter launch. The GitLab native auth test now includes scope implication overflow:
64 observed scopes cannot become 65 retained scopes after adding implied `read_api`.

These tests do not exercise a GitLab sandbox, simulate power loss, prove reproducible
OS images, supervise a native adapter, execute business writes or complete any
selected provider journey. `credential-blocker:gitlab-runtime-sandbox` remains open.
The AEP story and initiative remain active, with provider order unchanged:
GitLab, Kubernetes, PostgreSQL, MCP, then remaining providers.
