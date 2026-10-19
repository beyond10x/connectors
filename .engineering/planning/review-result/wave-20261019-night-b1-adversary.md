---
format: aep.planning-md/3
id: review-result:wave-20261019-night-b1-adversary
kind: review-result
status: active
title: Adversary pass on the definition format write members
relations:
- reviews: story:definition-format-write-members
revision: 1
---
**Verdict: INFEASIBLE findings only; all four fixed in `065c4f5da`.**

Adversary (opus) on `04da73c29`, wave 20261019-night. 26 cases in
`crates/connectors-catalog/tests/definition_writes_adversary.rs`, 4 red, all states no committed
definition writes: body members on a read (`definition.rs:1284`), a dotted body key with
`body_enums` (`:1285`), member keys matched by spelling not path (`:1214`), `conflict_status`
beyond `u16` refused without naming the member (`:664`). After `065c4f5da`:
`cargo test -p connectors-catalog` 26 of 26 adversary cases pass, EXIT=0 (`~/.cache/connectors-night-20261019/b1b/`).
