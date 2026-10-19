---
format: aep.planning-md/3
id: story:registry-passive-approval-test-order
kind: story
status: archived
title: The passive approval-target registry test holds under a full parallel run
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-10-19T18:40:22Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"diff":1}}}
---
## Observed
`local::registry::tests::approval_target_is_passive_but_requires_exact_retained_admission`
(`crates/connectors-host/src/local/registry/tests.rs:62`) failed once in a full
`cargo test -p connectors-host --no-fail-fast` run on 2026-10-16 (host lib, 418 passed, 1 failed,
at host load about 40-60 on 20 cores): the observer connection's `PRAGMA data_version` read 3
where the test expects 2, so something wrote the metadata store during a step the test
asserts is passive. Run alone it passed (`1 passed; 451 filtered out`, 12.57 s). It is not
introduced by the OpenRouter change; nothing in that change touches the registry.

Reproduction: `cargo test --locked -p connectors-host --lib --no-fail-fast` under the default
test threads at host load over 30; not reproduced alone.

## Acceptance
- The write that moved `data_version` is named from the code (a clock or expiry sweep, a
  concurrent test sharing state, or the approval-target path itself) and recorded here.
- If the approval-target path wrote: it is made passive and the test stays as it is. If another
  writer did: the test isolates its store from it, or asserts only the approval-target path's
  writes.
- The test passes 50 consecutive full-library runs at bounded load (rules for synthetic load:
  at most `--cpu 16 --io 2 --hdd 2`, `--timeout 600`).
