---
format: aep.planning-md/3
id: decision-blocker:v1alpha2-write-before-audit
kind: decision-blocker
status: cleared
title: Nobody has decided whether v1alpha2 writes may dispatch before an audit record exists
relations:
- blocks: task:http-host-mutation-ledger
- blocks: task:v1alpha2-invoke-wire-spec
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T11:06:52Z", actor: "human:timo", revision: 3}
---
## The question

May the first v1alpha2 invoke binding dispatch an `external_write` operation before the HTTP host holds an acknowledged execution-audit record? `contracts/service/compatibility.md` § 5 requires one for an audited dispatch; `contracts/service/audit.md` is not implemented on the HTTP host (`crates/connectors-host/src/server.rs`).

## Options

- A: dispatch writes and answer `audit_status: unavailable`, `audit_ref: null` (the draft in unit commit 3138b500d9).
- B: refuse every write on v1alpha2 until the audit record exists.
- C: add a unit in which the HTTP host writes the audit anchor before dispatch, so writes answer `complete`.

## What it stops

`task:http-host-mutation-ledger` and the merge of `task:v1alpha2-invoke-wire-spec`.

## Decision (2026-10-08)

C. The HTTP host writes the audit anchor of `contracts/service/audit.md` § 1-2 before it dispatches a v1alpha2 write, and such a write answers `audit_status: complete` with its `audit_ref`. `contracts/service/compatibility.md` § 5 stays as written. A is not taken: dispatching a write without an audit record is a security trade-off outside the approved design. `task:http-host-audit-anchor` delivers it in this wave.
