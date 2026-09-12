# Guarded GitLab write failure coverage

This inventory makes the remaining acknowledgement cases in
`story:guarded-gitlab-merge` explicit. It follows the
[local coordinator contract](../contracts/service/local-mutations.md) and current
[execution order](../crates/connectors-host/src/local/owner/mutation/execution.rs).
It does not replace the selected provider workflows or dedicated sandbox
acceptance. A storage-port test, an owned-child protocol fixture and a production
CLI journey establish different facts.

| Boundary | Required behavior | Existing evidence | Remaining production CLI evidence |
|---|---|---|---|
| Audit anchor | No preflight or business dispatch without its definite acknowledgement; a committed record whose acknowledgement was lost cannot recreate a live admission receipt. | Audit port tests inject rollback/lost acknowledgement and exit a process on both sides of commit. | Inject these failures into an approved CLI invocation and verify no native preflight or PUT. |
| Native preparation | Preflight may read, but preparation does not send a business write. Lost/malformed preparation, cancellation and expiry destroy its capability and clean up the exact child. | Private child write-protocol tests cover preparation, cancellation, malformed replies, deadline, duplicate commit and old peers. | Join the remaining preparation-failure cases to approved CLI execution and verify attempt, approval and audit state. |
| Attempt/key preparation | Reserve the key and create Prepared atomically. A failed or uncertain acknowledgement returns no live preparation receipt. | Mutation port tests cover rollback/uncertainty and subprocess death. CLI recovery uses a durable-port producer that exits after preparation. | Crash an actual approved native invocation around this acknowledgement. The current producer uses Approval::NotRequired and is not that case. |
| Approval spend | A failed/uncertain acknowledgement grants no dispatch gate. A committed spend remains spent after abort or restart. | Approval port tests cover both failures, four process exits, expiry during acknowledgement and spent-then-aborted reuse. | Inject the spend acknowledgement failures into the approved CLI path and inspect its exact original attempt and proof tombstone. |
| Connection dispatch admission | Current connection publication/revocation and exact generation use govern final dispatch. Lost admission cannot be replaced with fallback credentials. | Registry tests serialize repair, revoke, publication and dispatch. Existing CLI repair/revoke and preflight-revocation journeys exercise normal admission failures. | Inject the connection-use acknowledgement failure while an approved native preparation and spent proof exist. |
| Attempt dispatch gate | Only the original live acknowledged winner can commit the matching native preparation. Uncertain gate acknowledgement remains Unknown and never supplies another send receipt. | Mutation/approval port tests cover rollback, uncertain gate, competing abort and receipt identity. | Inject failure before and after gate commit in the full CLI path; prove no PUT, conservative retained state and no redispatch after restart. |
| Native business response | Preserve Applied/Refused when known. A lost response or owner death after a possible effect remains Unknown and cannot retry the PUT. | Production CLI fixture covers applied/refused/lost response, exact owner death after one provider-applied PUT, spent-proof reuse refusal and new-owner observation. | Dedicated GitLab sandbox behavior remains required. |
| Attempt terminal settlement | A persistence failure cannot replace the live known effect. Recovery uses the durable record and stays conservative when the known result was not retained. | Mutation port tests preserve the known answer across ambiguous terminal writes and retain the first terminal fact. A production CLI journey makes host metadata unwritable while the provider holds a known Applied and a known Refused response: the live reply keeps that result with an `unavailable`/`attempt_store` cause, the unretained record stays Dispatching, and a restarted CLI/owner reports the quarantined attempt after one PUT. | That journey covers one persistence-failure shape: a read-only metadata file, which fails every read-write SQLite open. Disk exhaustion and `SQLITE_BUSY` fail at commit rather than at open and can leave the attempt completed, which is a different recovery path; they and dedicated GitLab sandbox behavior remain required. |
| Audit final append | Preserve the exact final-observation UUID and fields through bounded acknowledgement recovery. Conflict, unavailable evidence or exhausted recovery leaves audit incomplete while preserving the known business result. | Real-SQLite port tests cover rollback, lost acknowledgement, a second failure, conflicts, exact identity and expired budget. The production finalizer test contains 35 combinations across four business classifications, including an Applied effect with an unusable native response. | The finalizer tests use constructed safe results. Joined CLI fault injection during final append remains required. |
| Final result disclosure | Current policy/connection admission governs disclosure independently of the historical attempt. A lost or refused disclosure never authorizes another business effect. | CLI revocation during preflight completes the admitted audit and refuses dispatch. Revoked/removed-target background recovery later refuses result disclosure. A production CLI journey revokes the connection after a known Applied provider effect and before disclosure: the live reply is refused `revoked` at admission rather than disclosing the known result, the record stays Dispatching, and one PUT and one effect are asserted after the refusal and again after restart. | The policy and key races after the native response, and the race during final audit handling, remain required. The connection race after the native response is covered by the journey named opposite. |
| Abandoned attempt recovery | A pending observation is not proof of quiescence. Recovery waits for the existing worker or excludes worker creation, and never creates provider-send authority. | CLI exact-key, live-write/owner-crash, unkeyed and revoked/removed-target journeys; deterministic worker coalescing and exact-instance ledger tests. | Remaining approved pre-dispatch crash cases above; real sandbox acceptance. |

The public command and proof-publication cases remain in the story's separate
verification inventory: protected output failure, unchanged subject, caller,
policy/key/clock revisions, absent/expired/reused proof, schema/permission refusal,
idempotency conflict and selected executable/profile compatibility. They are not
discharged by the table's persistence cases.

## Evidence owners

- [Audit tests](../crates/connectors-host/src/local/audit/tests.rs) own anchor/final
  persistence faults and exact final-observation acknowledgement recovery.
- [Production finalizer tests](../crates/connectors-host/src/local/owner/mutation/tests.rs)
  own preservation of safe business results while that real helper handles faults.
- [Mutation tests](../crates/connectors-host/src/local/mutations/tests.rs),
  [approval tests](../crates/connectors-host/src/local/approvals/tests.rs) and
  [registry tests](../crates/connectors-host/src/local/registry/tests.rs) own their
  separate persistence/admission boundaries.
- [Private write-protocol tests](../crates/connectors-host/src/local/runtime/process/write_tests.rs)
  own the controlled native child exchange and cleanup cases.
- [Guarded merge CLI fixture](../adapters/gitlab/tests/local_runtime/guarded_merge.rs)
  and [background recovery journeys](../adapters/gitlab/tests/local_runtime/background_recovery.rs)
  join production CLI/owner/native binaries to disposable HTTPS, clock and
  qualified custody services. Retained receipts describe exact tested revisions.

## Original audit after process loss

The [audit contract](../contracts/service/audit.md) selects an immutable anchor
and one exact final observation. It explicitly defers general reconciliation.
Current CLI anchors precede native preflight and attempt creation; their optional
attempt reference is absent. The implementation has no durable final-observation
intent or explicit original-audit link on the later attempt.

Consequently, an owner restart cannot reconstruct a lost observation merely from
an incomplete audit and a similar request ID. A background original-audit
finalizer would need a reviewed persistence/correlation binding before runtime
decomposition. Existing incomplete records remain truthful. This question is
separate from retrying the exact observation still held by its live finalizer,
and from quarantining an abandoned mutation without another provider effect.
