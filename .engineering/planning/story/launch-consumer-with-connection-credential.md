---
format: aep.planning-md/3
id: story:launch-consumer-with-connection-credential
kind: story
status: draft
title: A pinned consumer is launched with one connection's credential on fd 3
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: apps/connectors/spec/cli.yaml
- confidence: inferred
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: contracts/auth/custody/v1alpha1/semantics.md
- confidence: cited
  path: contracts/cli/v1alpha1/consumer-launch.md
- confidence: inferred
  path: contracts/cli/v1alpha1/fixtures/values.json
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-host/src/local/config.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/use_and_retirement.rs
- confidence: inferred
  path: crates/connectors-host/src/local/runtime.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/artifact.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime/launch.rs
- confidence: cited
  path: docs/local-consumer-launch.md
- confidence: cited
  path: docs/local-postgres-cli.md
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 21
---
## Outcome

`connectors connections launch` execs an operator-pinned consumer executable with one
connection's protected document on fd 3. No response, argv, env or file of the caller ever holds it.

## Why

Operator decision 2026-10-05: database credentials for a PostgreSQL store come from a Connectors connection held in Connectors' secret backends, and cortex never sees them. Design B of the cortex design note (chosen 2026-10-06, operator: "do it"): Connectors launches an operator-pinned `ekr` with the connection's password document on fd 3; EKR reads a `password_file`; cortex starts every `ekr` through the launch. Order: EKR `postgres-password-file` and the Connectors launch story can run in parallel. The
cortex story depends on both: its test needs a real `ekr` with `password_file`, and its stand-in
mimics only the Connectors verb.

## Work
1. Contract first.
   - Add a "consumer launch" section to `contracts/cli/v1alpha1/semantics.md` and a sibling `contracts/cli/v1alpha1/consumer-launch.md`.
   - The consumer is a new custody reader party. Amend `contracts/auth/custody/v1alpha1/semantics.md` § Scoped access.
   - Delivery is verbatim and uninterpreted, on fd 3 as a sealed (`F_SEAL_WRITE|GROW|SHRINK|SEAL`) memfd.
   - Admission is the existing bounded read use: live evidence is required, otherwise `not_granted`.
   - Rules: no shell, no PATH, no inherited credential env, and the caller's stdio is passed through. The exit status is the consumer's.
2. Config: add `[consumers.<name>]` with `executable {path, sha256}` and `permissions.connections` (adapter aliases). Omitted means deny.
3. Host: add a launch path that reuses `runtime/artifact.rs::capture`, resolves the current generation's material through the registry use guard, writes the memfd and calls `execveat(AT_EMPTY_PATH)` on the captured consumer.
4. CLI: add verb `connections launch --adapter --connection --consumer -- <args…>` to `apps/connectors/spec/cli.yaml` and regenerate the CLI contract artefacts.
5. Add docs: `docs/local-consumer-launch.md`, linked from `docs/local-postgres-cli.md`.

## Acceptance
- An opt-in `connectors-host` test with the disposable Secret Service fixture (`local-secret-service.md:112-126`) passes. It saves a fictional `{"password": …}` through `connections connect --credential-stdin`, then launches a pinned test consumer, which writes the SHA-256 of fd 3's bytes to its stdout. Assert the digest matches and the sentinel is absent from the CLI's stdout and stderr, the consumer's env and argv, and the state dir.
- With a wrong `sha256` the launch is refused before exec.
- With lapsed evidence the launch returns `not_granted` at admission.
- With the collection locked it returns `custody_unavailable`.
- An unlisted consumer/connection pair is refused.

## Files (from the design, unverified) `contracts/cli/v1alpha1/semantics.md`, new `contracts/cli/v1alpha1/consumer-launch.md`, `contracts/auth/custody/v1alpha1/semantics.md`, `apps/connectors/spec/cli.yaml`, `apps/connectors-cli-contract/*` (regenerated), `apps/connectors/src/local/connections.rs`, `crates/connectors-host/src/local/config.rs`, `crates/connectors-host/src/local/runtime/artifact.rs` (reuse; possibly expose `capture`), new `crates/connectors-host/src/local/runtime/launch.rs`, new `docs/local-consumer-launch.md`, `CHANGELOG.md`.

## Open design questions (scoped 2026-10-06)

Found 2026-10-06 by `story-scoper` against the tree (`9806140d7` + 0.29.0). The story came from a cortex design note and its Work does not hold as written on these points:

| # | question | evidence |
|---|---|---|
| 1 | The `-- <args…>` passthrough cannot be expressed in the CLI spec: the pinned `ess-cli/1` (ESS 0.52.0) has only option, positional (one value each), document and protected sources. Either ESS gains a trailing-arguments source, or the parser is hand-written (against "regenerate, never hand-edit"). | ess 0.52.0 `crates/specify/ess-cli-contract/src/wire.rs:130-159`, `crates/generate/ess-cli-project/src/runtime.rs:318-322` |
| 2 | Whether the generated handler can exec, pass stdio through and return the consumer's exit status; `call` returns a structured reply. | `apps/connectors/src/local.rs:59` |
| 3 | Lapsed evidence answers `unavailable` (readiness), not `not_granted`; the Acceptance needs a contract decision. | `use_and_retirement.rs:26-27`, `connections.rs:164-165`, `contracts/cli/v1alpha1/semantics.md:502` |
| 4 | "Registry use guard" is the `capture_read` → `dispatch_read` → `release_read` sequence, not a symbol. | `docs/local-runtime-foundation.md:84` |
| 5 | How a launch obtains the `Binding` (provider authority + profile) without starting the adapter. | `registry.rs:81-87`, `runtime.rs:338`, `supervisor.rs:830` |
| 6 | Which process may read the credential: every connection-credential read today happens in the owner process. | `supervisor.rs:840`, `owner/mutation/execution.rs:206` |
| 7 | `execveat(AT_EMPTY_PATH)` is used nowhere; the adapter path runs `/proc/self/fd/N` through `Command`, and fd 3 is its channel. | `runtime/process.rs:36-63` |
| 8 | Adding `[consumers]` may need a new config format (unknown fields refused; only `connectors-local/1` and `/2`). | `config.rs:30-31`, `:238`, `:252-264` |
| 9 | A release must also update the website guides and CLI examples. | `AGENTS.md:114-117` |

Questions 1, 5 and 6 are design decisions for this repository's owners (the custody model and the CLI contract); `decision-blocker:consumer-launch-design` holds the story until they are settled. Question 1 is filed upstream as an ESS gap.

## Design decisions (2026-10-06)

Decided 2026-10-06 by the coordinating session under the operator's delegation ("for decisions, until 10am, take all decisions yourself").

| # | question | decision |
|---|---|---|
| 1 | trailing argument list | No `-- <args…>` passthrough. A consumer's whole argv is pinned in its `[consumers]` entry; the launch takes only `--consumer <name>` and `--connection <id>`. The CLI stays expressible in `ess-cli/1` as it is, so the story no longer waits for beyond10x/ess#466. |
| 2 | exec and stdio | The handler starts the consumer as a child with stdin, stdout and stderr inherited, waits for it, and exits with the consumer's exit status; the structured reply goes to stderr only when the launch itself is refused. |
| 3 | lapsed evidence | Keeps the existing contract: `unavailable` (readiness). The Acceptance names `unavailable`, not `not_granted`. |
| 5 | binding without starting the adapter | From the saved connection record: provider authority and profile are read from the registry entry the connection was saved with. No adapter process is started for a launch. |
| 6 | which process reads the credential | The owner process, as every connection-credential read is today. It writes the secret into a sealed memfd (`F_SEAL_WRITE`, `F_SEAL_GROW`, `F_SEAL_SHRINK`, `F_SEAL_SEAL`) and passes it to the consumer as fd 3; the CLI process never holds the secret. |
| 7 | how fd 3 is passed | The existing pattern: `Command` with the descriptor placed at fd 3 before exec; no `execveat`. |
| 8 | config format | A new `connectors-local/3` that adds `[consumers]`; `/1` and `/2` stay readable. |
| 9 | release | The release updates the website guides and CLI examples, per `AGENTS.md`. |

The custody and CLI contract text (`contracts/cli/v1alpha1/semantics.md` and the custody notes) is written by the story's implementation, from this table.
