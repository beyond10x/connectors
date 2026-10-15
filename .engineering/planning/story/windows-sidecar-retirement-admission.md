---
format: aep.planning-md/3
id: story:windows-sidecar-retirement-admission
kind: story
status: implemented
title: A Windows sidecar being deleted does not refuse a store open
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T05:27:42Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T05:27:42Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T22:06:09Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

On Windows, opening the metadata store never answers `metadata_unavailable` because a sidecar
(`-wal`, `-shm`, `-journal`) is being deleted by an idle pooled Eventlog handle at that moment.

## Today

`main`'s `host and CLI on windows-latest` job on 3f3d8c780 failed one test, 400 passed:
`local::metadata::tests::a_store_a_newer_binary_with_another_model_wrote_is_refused_by_every_open`
answered `MetadataUnavailable` where `MetadataSchemaNewer` was expected; the same tree passed on the
pull request's Windows run. Inferred cause: a sidecar in delete-pending state answers access denied
or delete pending, or is open with no links left, and admission refused it.

## Change

`admit_sidecars` on Windows re-probes a retiring sidecar every 2 ms within `lifecycle_lock_wait()`
and treats it as absent once its name is gone; one still retiring at the bound is refused. A present
sidecar that is not being deleted keeps the full owner-only and single-link check. The CLI contract's
Windows owner-only rule states the wait.

## Acceptance

- Three Windows-only tests pass on windows-latest: a delete-pending sidecar is waited for, a retired
  one is absent, one still pending at the bound is refused.
- The Linux host suite is unchanged (387 passed in the lib lane).
