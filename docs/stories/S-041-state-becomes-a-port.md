---
id: S-041
title: "State becomes a port, with a SQLite backend"
pillar: Platform
status: in-progress
priority: 2
design: ../design/12-one-owner-for-every-outside-connection.md
epic: local-product
areas: [service, state, testing]
note: "the port, the in-memory and SQLite backends and the shared conformance suite have landed, all three verified equivalent including against a live PostgreSQL; converting the five Integrations off their private state shapes remains"
---

# State becomes a port, with a SQLite backend

## Goal

Make where an Integration's durable state lives a **port**, the way secrets and egress already are,
so a deployment chooses a backend instead of the code choosing a branch — and give it a SQLite
implementation that is in-memory for tests and a file for a workstation.

## Why

Two of the three deployment concerns were already ports. The third was not:

| concern | port? |
|---|---|
| secrets | `SecretStore` — memory / file / Vault |
| egress | `EgressTransport` plus destination rules |
| **state** | **five bespoke answers** |

```text
slack        Option<PostgresState> + state_root: PathBuf
gitlab       enum GitlabState { Postgres, Local { root } }
jira         state_store: PostgresState        ← no local branch at all
b10x   enum PersistedState + Option<PostgresState>
sip          Option<PostgresState>
monitoring   state_root: PathBuf only
kubernetes   stateless
```

Because each Integration answers the question in its own shape, `connectors-runtime` cannot compose
uniformly: it carries two hand-written ladders, one per posture, each of which has to know which
shape each Integration wants. That is what stops
[Design 12](../design/12-one-owner-for-every-outside-connection.md)'s
*"posture is which ports you bind, not which branch you take"* from being true today.

And the sharper consequence, which nobody asked for but everybody pays: **`integration-jira` cannot
be composed on a workstation at all** — not by policy, but because its only constructor takes a
concrete `PostgresState` and nobody wrote the other branch. A whole Integration that only CI can
run.

Timo, 2026-08-20: *"how about a sqlite state store, that's nice for testing with memory backend and
perfect for local persistence?"*

## What landed

- **`crates/connector-state`** — the port and nothing else. `trait StateStore { read, replace,
  append, delete }` over bounded keyed byte cells, the shared key grammar, and `MemoryState`. One
  dependency (`thiserror`), no transport, no driver, so every consumer can depend on it without
  inheriting a backend. Classified as a host library in the dependency fence.
- **`crates/state-sqlite`** — `rusqlite` with `bundled`, the same crate and version `substrate` and
  `identity` already use. `SqliteState::in_memory()` and `SqliteState::open(path)`. Bundled because
  the connectors image is distroless and has no system `libsqlite3`.
- **`hosted-state` implements the port**, additively: `PostgresState` keeps its inherent methods so
  existing Integrations keep compiling and can move one at a time.
- **`connector_state::conformance`** — one suite, run by every backend against itself. This is the
  part that makes the port real rather than aspirational.

## Two things that would have been wrong, and are not

**SQLite's `||` corrupts binary cells.** The obvious append is
`ON CONFLICT DO UPDATE SET body = body || excluded.body`. SQLite's `||` operates on text, so
applying it to a BLOB coerces both operands to TEXT — truncating at the first zero byte and mangling
anything that is not valid UTF-8. State cells carry whatever an Integration encoded. The append is
therefore a read-check-write inside one `Immediate` transaction, which is both correct for binary
and atomic against a concurrent appender. A conformance case appends across a zero byte specifically
to catch a reintroduction.

**`append` must refuse without mutating.** Over the caller's bound it returns `Capacity` and leaves
the cell exactly as it was. Appending and then discovering the bound was exceeded leaves a
half-written cell, and an append-only log that has lost its invariant surfaces as corruption much
later, somewhere else. It is easy to implement three subtly different ways, which is why it is the
first case in the suite.

## Acceptance

- [x] A `StateStore` port with a shared key grammar and error vocabulary.
- [x] An in-memory backend, a SQLite backend (in-memory and file), and PostgreSQL behind it.
- [x] One conformance suite, run by every backend, pinning byte-exactness, atomic bounded append,
      bounded reads, delete idempotence and key validation on **every** operation including reads.
- [x] The suite passes against a live PostgreSQL, so backend equivalence is measured rather than
      claimed.
- [ ] The five Integrations drop their private state shapes and take `Arc<dyn StateStore>`.
- [ ] `integration-jira` composes locally, which today it cannot.

## Evidence

Measured 2026-08-20:

- `connector-state` 3 passed, `state-sqlite` 4 passed (in-memory and file both run the full suite),
  `hosted-state` 2 passed plus 1 `#[ignore]`d.
- The PostgreSQL leg run against the live development stack database:
  `CONNECTORS_DATABASE_URL=… cargo test -p hosted-state -- --ignored` → **1 passed**, and
  `SELECT count(*) … WHERE state_key LIKE 'conformance.%'` → **0**, so the suite cleans up after
  itself in a shared database.

## Next

The remaining acceptance items touch `integration-{slack,gitlab,jira,b10x,monitoring,sip}`,
which a concurrent session has open. Sequence with it. Once they take the port,
[S-042](S-042-one-composed-local-placement.md) collapses the two composition ladders into one
`compose(config, ports)` that both `connectors-cli` and Zwirn call.
