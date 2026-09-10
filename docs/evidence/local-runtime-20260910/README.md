# Local CLI foundation verification — 2026-09-10

This is an implementation checkpoint, **not completion of milestone 1 or the
seven-milestone initiative**. The persistent GitLab restart journey has not run.
No new provider workflow, persistent credential custody, managed connection,
local supervisor, MCP client/server, approval/mutation handling, reproducible
package or cloud deployment is claimed.

## Inputs and ownership

- Connectors base: `14bd1adb03c7c48361fb4a082125324b811bee16`; initially clean `main`.
  The local commit containing this report owns the changes. Exact changed
  implementation bytes are recorded in [implementation-sha256.txt](implementation-sha256.txt).
- ESS source remains `6f7ef46163e758f3401945d1a946e0fc80ebc003`; the repository
  resolver verifies its existing executable receipt and source pin in the gate.
  No ESS source or generated CLI/parser bundle was changed.
- Rust/Cargo used for the main gate: Rust 1.98.1 / Cargo 1.98.1. The gate also
  runs the installed Rust 1.88.0 compiler over every workspace target.
- Final SQLite binding: rusqlite 0.40.2, libsqlite3-sys 0.38.2, bundled SQLite
  3.53.2. Final Cargo.lock SHA-256:
  `287cb5a93adb47373cf4ee1734999e76e352d98ce2bacbf3e3575ab32f093ec9`.
- Bot authority: a clean existing managed Atlas checkout at
  `0602933d597c47f900d86a9946940b2fad74ad96`, matching the current remote main
  advertisement. No Atlas records or primary checkout were modified.
- MCP primary retained its existing one-line-change documentation workflow edit,
  `.github/workflows/b10x-docs-pages.yml`; no MCP source changes, adoption,
  publication or dependency repin were performed in this checkpoint.

## Commands and results

The final command was:

```sh
TMPDIR="$PWD/.local/tmp/local-runtime-20260910" \
CARGO_BUILD_JOBS=2 \
CARGO_TARGET_DIR="$PWD/.local/tmp/local-runtime-20260910/target" \
cargo run --locked --offline -p connectors-build -- gate --msrv
```

Exit **0**. [The complete final gate log](gate.log) includes:

- Shared ESS plus five authored adapter roots validated and compiled independently.
- Ten generated CLI artifacts current; structural/cached/acquisition/page fixture
  consistency checks passed. Those are specification fixtures, not runtime custody.
- Formatting, descriptor drift, full offline workspace build/tests/Clippy passed.
- Adapter library and generic CLI dependency boundaries passed.
- `cargo +1.88.0 check --workspace --all-targets --locked --offline` passed.
- Authored contract scenario synthesis and AEP validation passed.

Nine new tests run through the production CLI or host infrastructure: three CLI
process tests and six host tests. The concurrency test includes sixteen rounds
of four independently opened metadata handles. The existing two legacy CLI
compatibility tests also pass. A separate focused run of the six host tests is
retained losslessly in [lifecycle-lock-test.log.gz](lifecycle-lock-test.log.gz).

The initial concurrency implementation released its file lock before SQLite
closed the last database handle; another opener could race sidecar deletion. The
final code keeps an exclusive lifecycle lock until after SQLite closes. An
initial root-help regression was corrected while retaining legacy request and
raw-result compatibility. A fixture's incorrect private-directory assumption was
corrected without relaxing production parent admission.

The [earlier gate](gate-before-sqlite-upgrade.log) passed with SQLite 3.50.2, but
that is **superseded verification**. Review of SQLite's documented WAL-reset
concurrency defect led to the final 3.53.2 pin and a runtime refusal of overrides
older than 3.51.3. The final gate above ran after this change and the lifecycle
lock correction.

`connectors-build docs --check` initially failed because the ignored website
reference cache did not exist. The owning generator then ran successfully, and
`connectors-build docs --check` passed: 41 contract pages, 90 total reference pages,
no drift. See [generation](docs-generate.log) and [check](docs-check-final.log).
No website presentation, example or selected public-input source changed; Node
browser/build checks were not run. New local documentation links and factual
support boundaries were checked, and `git diff --check` passed.

## Runtime observation

Separate executions of the final binary performed `setup init` and `setup check`
against fresh task-owned configuration/state. Both exited 0. The observation at
2026-09-10T10:05:18Z reported configuration and metadata ready, the existing local
keyring available, and **persistent_custody_qualification failed**. See the actual
[init result](smoke-init.json) and [check result](smoke-check.json).

The observed Secret Service is GNOME Keyring 50.0. No secret was read, written,
deleted, migrated or entered. Availability inspection checks the local runtime
bus socket's UID/kernel peer and the service's unique bus-owner UID; it does not
prove durable credential writes or recovery.

The reviewed executable is retained locally at
`.local/tmp/local-runtime-20260910/bin/connectors`, SHA-256
`867b3da4766800805e7ebe6bfd41d74e3b2a6c66720972ae4906cb79a11df1d0`.
This native development binary is not a reproducible distribution or a published
artifact. Build outputs were isolated under this task and later moved to a
private task-owned tmpfs directory when shared disk space became scarce; logs,
source hashes and this binary were retained separately before build cleanup.

## Planning and remaining work

`initiative:complete-local-connectors` and `story:persistent-gitlab-journey` remain
active. The initiative records all seven requested milestones and relates the
existing MCP epic and Kubernetes generation story without changing their history.
There is one decomposing story so far; the planning skill's fewer-than-two-child
rule skips the critic panel at this checkpoint. Full later decomposition and
reviews remain outstanding. This was an interactive run, with zero approval
bypass records and one planning-store writer.

AEP's [final unedited validation output](aep-validation.log) reports 82 historical
reviews with prose-only findings, then `valid`; they were preserved. An initial
attempt to move the story to proposed was refused for its missing objective
relation. It was related to the existing `vision:independent-contract-adapters`
through AEP, then advanced through proposed to active using legal moves.

`credential-blocker:gitlab-runtime-sandbox` records the missing dedicated sandbox
URL/project and protected credential source. It withholds live acceptance evidence,
not permission to continue implementation. The next runtime work is authored
GitLab native auth/profile and local bootstrap binding, protected source admission,
qualified immutable custody, connection publication and exact owner supervision,
followed by real restarted credential reuse. No first-milestone acceptance case
can be closed from this foundation alone.
The concrete `credential-blocker` kind is admitted through the store's open blocker
family and queried lifecycle; `kinds` does not list it as a separate concrete entry.

The installed 0.7.0 Connectors client was used for integration diagnostics. Its
operation search returned `connector-unreachable` against the existing dead local
socket; this was reported before read-only Git was used for remote authority
verification. That installation and all existing credentials remain unchanged.
No paid governed run was launched, and the existing Kubernetes driver protocol
loading blocker remains open. No task-owned linked worktree or recovery repository
was created. The lease on the existing Atlas authority tree is released after the
local commit; its lifecycle and other owners are preserved.
