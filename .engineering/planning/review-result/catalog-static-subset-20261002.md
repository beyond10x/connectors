---
format: aep.planning-md/3
id: review-result:catalog-static-subset-20261002
kind: review-result
status: active
title: 'Catalog frozen subset: bounded source review'
relations:
- reviews: story:catalog-cli-journeys
revision: 1
---
unit: frozen catalog lifecycle/background/owner-replay/recorded-state subset over f3fb222b7edc7fc29520dd30effdb58bfdec5274
verdict: nothing found in bounded read-only source pass
cases: inherited 332 ordinary, 21 ignored, 8/10 logical obligations and 11/15 variants; 1 failed fault case; not rerun
origin: introduced 0 / pre-existing 0 / undecided 0 findings
wrote-outside-worktree: own worktree lease registry only
needs-coordinator: final whole-candidate review after settlement correction freezes

```text
$ git --no-pager diff --stat -- <the four reviewed paths>
(empty: these candidate modules are untracked)
$ git status --short -- <the four reviewed paths>
?? adapters/catalog/tests/local_runtime/background_recovery.rs
?? adapters/catalog/tests/local_runtime/lifecycle.rs
?? adapters/catalog/tests/local_runtime/owner_replay.rs
?? adapters/catalog/tests/local_runtime/recorded_state.rs
reviewer tracked/source delta: 0
```

No concrete finding was established. No tests were added, selected or executed; no builds, live calls, mutation probes or AEP writes occurred. The only authored file is this assigned report. The inherited counts are the coordinator/author handoff, not measurements from this review. Author `report-public.md` hash read was `dd4b3831a042bc0090574ae10d8bf8bf785d8d0deb7f4b55e4179c24d053a453`; it distinguishes 332 ordinary package executions from 21 ignored and the separately selected failing settlement case. This pass supplies no approval or independence claim.

Fixed input hashes, checked at entry and after source inspection:

```text
3fceddf2a40fb76784cfd77a7d0cbd4c674cee86a3cba2c35fb84567b8f94deb  adapters/catalog/tests/local_runtime/lifecycle.rs
06e6318eb2b8700e8690e466be9f5e6bbae261eb3c68f7c6fa9d2422ae4dff62  adapters/catalog/tests/local_runtime/background_recovery.rs
1ddcf26de557b001278361a81713b7f4da089ccc7e68a72e89d50d9df6f222c2  adapters/catalog/tests/local_runtime/owner_replay.rs
9cb8956f1804f8387e105bd3eddb0f664aac236319a80f40f3c25fb2e0219f8c  adapters/catalog/tests/local_runtime/recorded_state.rs
```

The active `guarded_merge.rs` and `cli_journey.rs` were read only where needed to identify callers and shared cleanup behavior. Their evolving implementation, settlement correction and new fault helper are not subjects of this fixed-subset verdict.

Boundaries examined:

- `recorded_state.rs:43` admits existing metadata, reads the authority marker, then opens the actual recorded authority with production definitions. `:117` requires complete snapshots and rejects duplicate subjects/unknown states or fields. `:193` selects one exact logical attempt; `:204` joins its reservation by attempt id and checks fingerprint, receiver, caller key and settlement time. `:383` joins one Spent redemption and incomplete audit to the same attempt/request/connection/operation. `:427` verifies the physical typed-string identity rather than stripping an arbitrary prefix. No row substitution or legacy business SQL was found.
- `recorded_state.rs:274` reconstructs a declared audit record, and `:335` cross-checks owning Store observations at quiescent stages. This is not an atomic joint snapshot. Normal Reader teardown requires `Joined { provider: Ok(()) }` (`:146`); panic teardown records the outcome instead of making a second assertion. This pass does not claim all abnormal observer retirement paths were exercised.
- `background_recovery.rs:23` verifies the exact seeded unkeyed reference before waiting for recovery. The two-attempt case separately checks unkeyed Prepared versus keyed Dispatching/Pending, later Indeterminate/Quarantined, no extra provider effects, and passive outcome-unknown replay (`:156`). The removed-target path parses the rewritten configuration and asserts the old target is absent before its no-custody/no-worker recovery checks (`:50`). Broad state polling was considered with these actual bounded fixture populations and callers; no concrete false-positive join was established.
- `owner_replay.rs:13` starts the same libtest image, acquires the verified lifetime lock, passes real duplicated FD3/FD4, and connects with `start=false`. The existing unmodified owner checks the inherited lock inode (`crates/connectors-host/src/local/owner/transport.rs:698`). Normal helper shutdown requires matching incarnation, exit success, socket removal and reacquired lifetime lock (`owner_replay.rs:107`). Caller context stops/waits for the production owner before replacement and checks replay/audit/no-child/no-provider behavior. No forged build identity, fabricated greeting or hidden production-command fallback was found. This is the explicitly scoped same-image host-library segment, not an entire production-CLI replay.
- `lifecycle.rs:6` pairs permission, repair and admission refusals with provider-count and current-revision controls. Protected-stdin capture uses the real CLI, with a new pending Acquisition observed before stop. The real-expiry case deletes the credential input, requires pending/no-dispatch, then performs explicit revalidation and checks literal provider data (`:205`). Its native 503 versus 401 distinction remains observable. Source alone does not prove wall-clock bounds or every panic cleanup path.
- Serialization in the reviewed modules carries fixture approval subjects, typed state references and audit fields. No serialization of a real credential/keyring value was found. The byte check at `lifecycle.rs:195` scans the named main metadata file for the fictional token; it is not an independent proof covering every WAL, memory or log surface. This review did not read credential or keyring files.

Limits: no executable adversarial counterexample was produced; no broad correctness, process-residue or sensitive-data guarantee is inferred. The failed chmod fault and unsettled modes8–11 remain outside this pass and cannot be turned green by it. Final integration must review the corrected whole candidate and its actual executions.

Outside-worktree writes: only worktree CLI updates for own `codex-catalog-static-review` lease in `$HOME/.local/state/worktree/registry.sqlite3` and SQLite-managed transaction files if used. The lease is released with handoff. No other source, report, provider or process state was changed by this reviewer.

```findings
[]
```
