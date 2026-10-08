---
format: aep.planning-md/3
id: decision-blocker:approve-wave-20261008e
kind: decision-blocker
status: cleared
title: Nobody has approved wave 20261008e (Loki query, Slack reads)
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T19:38:16Z", actor: "human:timo", revision: 3}
---
## Question

Approve wave 20261008e: story:parity-loki-query and story:catalog-slack-reads, delivered on one integration branch (`wave/20261008e`) with one pull request, released when it merges. The integration branch also carries the AGENTS.md "Cutting a release" step 7 fix: `b10x-gates gh -- release create` needs `--policy` and `--repository`.

## Units

| unit | from | surface | calls since 2026-09-09 |
|---|---|---|---|
| story:parity-loki-query | `unit/loki-query-20261008d` (28d72d124: library, ESS reads, contract section 11, 21 tests) | Cargo.toml, Cargo.lock, adapters/loki, crates/connectors-build/src/gate.rs, crates/connectors-host/src/local/runtime.rs, docs/fluxplane-plugin-parity.md | 334 |
| story:catalog-slack-reads | origin/main | adapters/catalog (slack provider, bundle, index), adapters/slack/upstream, docs/catalog-slack.md | 710 (thread 236, message.list 93) |

Loki still needs: the executable binding, the bearer profile with the configured identity probe, `operations invoke` acceptance, and the parity rows moved to covered. The two scopes share no file.

## Options

| option | what | cost |
|---|---|---|
| A | both units | two unit builds inside the wave's disk slot |
| B | Loki alone | Slack (710 calls) waits one more wave |
| C | Slack alone | Loki branch keeps drifting from main; Grafana (394) stays blocked on Loki |

Recommended: A. Disjoint surfaces; Slack is the highest call count and Loki unblocks Grafana.

## Decided

Option A, 2026-10-08: story:parity-loki-query and story:catalog-slack-reads in wave 20261008e, plus the AGENTS.md release step 7 fix. The units run one after the other inside a 10G build budget; builds pause while / is under 20G free.
