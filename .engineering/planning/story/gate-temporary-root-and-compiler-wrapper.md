---
format: aep.planning-md/3
id: story:gate-temporary-root-and-compiler-wrapper
kind: story
status: active
title: The gate starts its compiler wrapper outside the checkout-local temporary root
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: .github/workflows/rust-gate.yml
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: docs/development.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-01T18:10:57Z", actor: "human:timo", revision: 5}
---
## Observed

2026-10-01, adversary pass on story:socket-path-limit-in-long-checkouts
(review-result:adversary-socket-path-limit-in-long-checkouts-pass-1): with the gate's `TMPDIR`
(`<checkout>/.local/tmp/gate-XXXXXX`, 94 bytes from a managed worktree), a fresh sccache server fails with
`sccache: error: failed to start server process` / `caused by: path must be shorter than SUN_LEN`. The machine's
Cargo configuration sets `rustc-wrapper = "/usr/bin/sccache"`, and `crates/connectors-build/src/gate.rs:16` passes the
temporary root to cargo, so the gate fails from a long checkout whenever no sccache server is already running. Seen
before at `docs/waves/auth-hardening-20260908/integration/full-gate-attempt-1.log:11`. The owner-socket tests no
longer have this limit (story:socket-path-limit-in-long-checkouts).

## Acceptance

- The full gate passes from a managed worktree with no sccache server running beforehand and the machine's wrapper
  configured, or the gate refuses at start naming the cause and the setting that avoids it.
- `docs/development.md` states the resulting rule; the `RUSTC_WRAPPER=` note added on 2026-10-01 is removed if no
  longer needed.
