# PostgreSQL through the local CLI

Development increment under `story:postgres-local-cli-journey`. The provider
batch and the parent goal remain open.

## Implemented behavior

`adapters/sql` gains an executable composition, so the adapter is reachable from
the local CLI for the first time. `--local-config` loads an owner-only
`connectors-sql-local/1` document and `--print-local-bootstrap` prints the
independently computed bootstrap and configuration revision. The federated
`--config` service mode is unchanged.

This is the third execution family in the repository, and it reuses none of the
HTTP composition. There is no `ScopedHttp`, no bearer header and no probe
endpoint: `Sql` already took `password: Arc<dyn Credential>`, so the composition
binds a credential rather than an HTTP port. The profile selects
`session_authority`/`session-authority`, because a password establishes a session
instead of signing each request.

**No validation query was invented.** PostgreSQL accepts or rejects the password
during the startup exchange, before any statement can run, so
`Sql::validate_session` opens one session and closes it. The handshake proves
more than a `SELECT` would, and a rejected password surfaces as the server's own
`28P01`. The saved identity is `role@database`; host, port and database changes
alter the configuration revision and are refused at startup instead.

`adapters/sql/spec/adapter.json` gains the local configuration shape beside the
federated one, both branches titled, and the descriptor was regenerated from it.

## Verification

Linux x86_64, `CARGO_BUILD_JOBS=2`, ESS 0.22.2 at `6b666e58f2e8`, AEP 0.55.0 at
`4eb999e0` with patch `73d78e50`, both rebuilt from pinned source for this tree.

| gate step | exit |
|---|---:|
| 2 `ess-boundary` | 0 |
| 3 `cli --check` | 0 |
| 4 `fmt --check`, 14 packages | 0 |
| 5 descriptor drift ×3 | 0 |
| 6 `build --workspace` | 0 |
| 7 `test --workspace` — **422 passed, 0 failed, 33 ignored** | 0 |
| 8 `clippy -D warnings` | 0 |
| 9 adapter boundary ×3 | 0 |
| 10 generic CLI boundary | 0 |
| 11 `+1.88.0 check --workspace --all-targets` | 0 |
| 12 `ess verify conform synthesize` — 315 scenarios, 22 refusals | 0 |
| 13 `aep plan artifact validate` — 303 artifacts valid, 0 warnings | 0 |

Four new cases in `adapters/sql/tests/local_runtime.rs` run against a PostgreSQL
wire fixture on loopback, in the same shape as the existing `protocol.rs`: no
database, no Docker, no network outside loopback, fictional credentials.

| case | establishes |
|---|---|
| session is the credential check | profile is `session_authority` with no scopes and one `password` field; identity `reader@fixture`; no scopes and no expiry recorded; spawning opens no session and validation opens exactly one |
| rejected password and malformed entry | a wrong password does not validate; an undeclared profile, a duplicate key and an empty password each refuse without opening a session |
| a dispatched read reaches the database | `query.read` opens its own session and returns the server's refusal rather than a fabricated result; an out-of-bounds query is refused before any session |
| bootstrap mismatches | changed instance, revision, digest or executable refuse at spawn with zero sessions |

## One pre-existing test was repointed

`crates/connectors-spec/tests/compiler.rs` read the SQL configuration schema at
`configuration_schema.properties.service`, which assumed SQL was the one adapter
without a `oneOf` — GitLab and Kubernetes already had one. The four paths now
address the federated branch. What the test asserts is unchanged, and it now also
covers resolving a shared `$ref` import inside a `oneOf` arm.

## Limitations

No dedicated PostgreSQL sandbox evidence exists. Every result above comes from a
loopback wire fixture, so the story stays `active`. MySQL, writes, DDL,
transaction control, cursors and pooling are not implemented.

The pinned ESS binary had to be rebuilt: it was not on disk, having been removed
with the worktrees that held it. `connectors-build toolchain --source` refused
the primary ESS checkout because that tree has 12 uncommitted files belonging to
another session, so the build used a clean managed worktree at the pinned commit
instead.
