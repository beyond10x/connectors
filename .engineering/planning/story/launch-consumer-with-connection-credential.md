---
format: aep.planning-md/3
id: story:launch-consumer-with-connection-credential
kind: story
status: draft
title: A pinned consumer is launched with one connection's credential on fd 3
relations:
- serves: vision:independent-contract-adapters
revision: 1
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