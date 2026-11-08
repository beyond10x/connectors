---
format: aep.planning-md/3
id: review-result:wave-20261105b-u2-security
kind: review-result
status: active
title: Security review of the owner's bounded-read effects recheck
relations:
- reviews: story:owner-bounded-read-rechecks-semantic-effects
revision: 1
---
**Verdict: green. All four invariants hold, 3 conformance cases were added (all green), and there are 3 notes: 1 introduced and already resolved, 2 pre-existing.**

The security reviewer ran on `7ba0e97cc`, wave 2026-11-05b unit U2. It added `crates/connectors-host/src/local/owner/governed/tests/bounded_reads_security.rs`, registered in `governed/tests.rs`. Running that module alone gave 3 passed, 0 failed:
- `refused_effects_record_only_an_early_refusal_and_never_dispatch` covers 8 declarations: each semantic effect alone, all three together, `process`, `network` with `local_system`, `process` with `network`, and semantic with `process`. Each gets `unsupported`. The dispatch closure panics if reached. The audit is an early refusal with outcome `Refused` and no operation, access or descriptor revision.
- `revision_and_lookup_still_precede_the_effects_refusal`: under a stale projection revision the same 8 return `stale_description`, and an operation missing from the projection returns `not_found`.
- `worker_admission_refuses_effects_before_provider_startup`: `Pool::run` with a governed `Invoke` task that carries a budget refuses the 8 as `unsupported`. The admitted declaration fails later, at provider startup.

Invariants:
1. No refused operation reaches provider work through the bounded-read port. Holds. A `BoundedRead` frame (`transport.rs:2844`, `:2909`) passes through `bounded::resolve` before the anchor (`governed.rs:201`) and before dispatch (`:275`). Worker admission (`supervisor.rs:1064`) is verified. The final fence (`supervisor.rs:1699`) is inferred: it calls the same `resolve`.
2. The refusal comes before any provider request and before any audit that claims an attempt. Holds. The only write is the refusal audit record, which every refusal writes.
3. The other guards are unchanged in order and effect. Holds. Approval and idempotency still run first (`bounded.rs:30-37`), and realization and limits run after (`:55-70`).
4. Owner socket admission is unchanged. Holds. The socket is created with mode 0600. On Linux, `SO_PEERCRED` requires the peer's uid to equal the owner's (`runtime/channel.rs:147-170`); on macOS, `LOCAL_PEERCRED`. The owner admits at most 32 clients.

Findings:
- F1 (note, pre-existing): `owner/transport.rs:2826`. A same-uid client sending a `GovernedRead` frame with a projection revision (budget `None`) leaves `bounded.rs:12-14` before the effects rule runs. A plain `Invoke` (`:2666`) also runs a granted read without looking at the metadata. The rule therefore protects the bounded-read contract and is not a privilege boundary against a same-uid client. Inferred from code.
- F2 (note, introduced): `bounded.rs:47`. The story's Outcome promised a `not_found`-class answer, while the code answers `unsupported`. Already resolved by the story's "Outcome, amended" section.
- F3 (note, pre-existing): `bounded.rs:58`. The owner accepts `realization: implemented` for any operation profile. The MCP projection accepts it only for `resource` and requires inline schemas (`apps/connectors/src/mcp/projection.rs:194`, `:207-208`). Inferred from code.
