---
format: aep.planning-md/3
id: review-result:standing-grant-local-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:local-invoke-admits-a-keyed-write-under-a-standing-grant and story:standing-grant-closes-the-input-to-listed-members
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
- reviews: story:standing-grant-closes-the-input-to-listed-members
revision: 1
---
verdict: needs-revision

I read both stories, the record, round 1, and the code named below. Nothing was built or run. "Inferred" marks what I did not read directly.

Round 1 answers that hold against the code:
- **Recheck sites:** :172, :375, :440 and :502 are all of them, plus :426 inside `spend`. A revocation, expiry or rotation during the preflight (:277) is caught at :375, which runs after the fresh clock at :290. The key lease is held from :331, so rotation cannot slip in after that.
- **Lost update:** the exclusive lock plus `publish_replace` closes it.
- **(b) Burning the cap:** `take` before the anchor and preflight cannot be reached by an uncovered caller. An in-grant caller can hold slots during a 120 s preflight and starve other in-grant callers. That is inherent to a shared cap.
- **(c) Removing instead of revoking:** no consequence for `Uses`, for `grant_of`, which reads by attempt id only, or for the served format, which still accepts `revoked`. `list-standing` cannot show a revoked grant.
- **(d) Record and story 3:** I found no contradiction with decisions 1 to 7. Story 3 stays consistent with the local story's D10 and O4.

```findings
- file: crates/connectors-host/src/local/owner/mutation/execution.rs
  line: 502
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Rechecks lists :502 as a recheck site and also says open_dispatch is the commit point and a later revocation does not stop the write, but :502 runs after the gate (:469) and on failure cancels the native commit and settles Unknown. The Design should drop the :502 recheck for grants (the file is not read between gate and commit, as the comment at :498 requires) or say it stays and answers Unknown. It should also say each check reads clock.now() fresh (the :169 value is a BoundedClock, not an interval) and verifies with the held `keys` lease."
- file: apps/connectors/src/local/operations.rs
  line: 237
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "The Design makes the CLI gate at invoke.rs:1155 refuse an empty --approval-file, but that gate only tests approval_file.is_none() and never reads the file. The file is read at operations.rs:237 (file_until is crates/connectors-host/src/local/protected.rs:274, not apps/), after observe_original :223 and before WriteClient::connect :251. The owner maps an empty frame to None (transport.rs:2640), so the owner cannot tell empty from absent; WriteClient is used only here, and MCP and approvals never reach it. Inferred: with pending set the CLI drops the proof (:233) and sends key Some and proof None, so a lost race at transport.rs:2597 takes the grant path. The Design should name operations.rs after file_until with replay still first, say the owner treats an empty proof as none, and note the pending case."
- file: crates/connectors-host/src/local/owner/approval_issuance.rs
  line: 223
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Both Designs say the walk and the targets read the input Value, but the local path holds only the string request.input. It must pass the Value that read_json builds at approval_issuance.rs:223, which refuses duplicate members, not a fresh serde_json parse where the last duplicate wins and could differ from the bytes sent at execution.rs:282. State it in both stories."
- file: crates/connectors-host/src/local/standing.rs
  line: 70
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Uncovered::rank (:70, impl below covering) is an exhaustive match, so OutsideInput needs a rank: 4, between OtherTarget and Revoked, since a grant reaching the check matched every target; Revoked and Expired shift up. Also say: pointers into arrays can never be satisfied, because arrays are leaves (fail closed, so document it); the prefix test compares tokens or `a + \"/\"` so `/` and `//x` work; the walk's recursion has the depth bound that channel::depth gives locally and the served path must state its own. Reading these as correct: escaping, empty-object leaf, root leaf and the /body/text vs /body/text2 boundary."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "O5 is incomplete. No caller of another UID can restore an older file: admitted() (standing.rs:265) and open_delivered refuse other writers. But the same UID by backup, sync or snapshot restore does it with no caller involved, and the file carries no sequence, so a restored file admits until not_after or key rotation. State both. Also say where the Lease lock file lives (Lease::acquire takes a directory and creates LOCK in it), what grant-standing does when the existing file does not verify, and what it does as expired grants fill GRANTS_LIMIT 256 (standing.rs:40)."
- file: crates/connectors-host/src/local/mutations.rs
  line: 300
  category: design
  severity: note
  verdict: approve
  origin: introduced
  message: "Read: approval_mode is dispatched on only at :300 (not_required may use open_dispatch with no live check) and :552 (required only); er.rs copies it. Concretely, a reader of the ledger now sees a write the operation's policy required approval for as not_required, which a local ledger never held for a write before (candidate default :382, overridden to Required at execution.rs:376). The ledger gate does not enforce the grant, so 'bind before open_dispatch' is executor order only. State both in the contract text. Bound rows are bounded by the ledger's 10,000 attempts per instance (mutations/types.rs:189)."
- file: .engineering/planning/story/standing-grant-closes-the-input-to-listed-members.md
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 8 reads the generated-file checks of acceptance 30 in the other story, and no edge records it (the edge runs the other way, so adding one makes a cycle). This story edits ess/domains, which inputs.json digests, so its own gate is red until the other story regenerates. Move acceptance 8 into the local story and state that this story leaves the generated checks red by design inside one pull request, or merge the two into one wave."
- file: crates/connectors-host/src/local/standing.rs
  line: 412
  category: design
  severity: note
  verdict: approve
  origin: introduced
  message: "Uses::open always creates the store (create_private), so the CLI replay's 'read-only, never creates' needs a new constructor; name it. A bound use stays counted when :440 refuses after bind, so an aborted not_attempted answer can carry standing.grant_id; say so. The same-UID section should add that approvals issue is limited to one input and one spend, while grant-standing yields up to 10,000 a hour with no horizon."
```
