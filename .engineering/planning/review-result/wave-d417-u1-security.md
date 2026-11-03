---
format: aep.planning-md/3
id: review-result:wave-d417-u1-security
kind: review-result
status: active
title: Security review of the cold owner start
relations:
- reviews: story:cold-owner-start-verifies-once
revision: 1
---
**Verdict: nothing found.**

Security reviewer on `e60844a1d`, wave D417 unit U1. Four green cases in commit `2b6e32c15` (branch
`review/d417-u1-sec`, merged into `unit/d417-cold-owner-start` at `6a72056d7`):
`apps/connectors/tests/cold_owner_start_security.rs` (a cold read without `--connection` on an altered store refuses
before any provider call; a warm read without `--connection` on a store altered after the owner started refuses) and
`checkpoint_tests.rs::a_wait_for_a_used_handle_ends_at_the_callers_deadline_and_never_spans_stores`. Executed 12 to 16,
red 0. `cargo test --locked -p connectors --test cold_owner_start --test cold_owner_start_security` and
`cargo test --locked -p connectors-host --lib checkpoint_tests` exit 0. Logs are in
`~/.cache/connectors-wave-20261103/u1-sec/`.

```findings
[]
```

Seen, not tested, not findings: the `connections` request carries its own fixed 30 s span (`transport.rs`
`Client::connections`), separate from the read's deadline; the accepted trade-off in `docs/local-er-metadata.md` (a
running owner accepts raw page damage until its next fresh open) now also covers the connection listing for reads
without `--connection` while an owner runs.
