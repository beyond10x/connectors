---
format: aep.planning-md/3
id: review-result:standing-grant-local-design-round-3
kind: review-result
status: active
title: Design critic, round 3, story:local-invoke-admits-a-keyed-write-under-a-standing-grant and story:standing-grant-closes-the-input-to-listed-members
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
- reviews: story:standing-grant-closes-the-input-to-listed-members
revision: 1
---
verdict: needs-revision

Round-2 findings against revision 3:
- **Blocker, post-gate recheck at execution.rs:502: closed.** Local story lines 300-306 drop it and keep the deadline check and `consume`, which matches execution.rs:500-511. `BoundedClock::now` (clock.rs:833-838) reads a fresh `Stamp` on every call and returns `lower`/`upper` (`bounds_ns` :803-832), so your description holds. The `keys` lease is held through commit (execution.rs:571-572 comment).
- **Blocker, empty `--approval-file`: closed in placement.** operations.rs:223, :233-234 and :237 and transport.rs:2640 match lines 271-281. `file_until` accepts an empty file (protected.rs:274-310). One residual, finding 4.
- **Warning, `Value` from `read_json`: closed.** `resolve` builds it at approval_issuance.rs:222-223; local lines 282-286 and closed-input 109-112 say so.
- **Warning, rank, arrays, prefix, depth: closed** (closed-input 90-92, 100-108).
- **Warning, O5, lock, expired grants at 256: closed.** Residual: finding 3 (file size) and finding 6 (wording).
- **Warning, generated-file edge: closed.** The check moved to local line 31, and there is no edge back and no cycle. I walked 14 edges around the three stories, ADR and vision, and found none. The three `depends_on` edges are local to closed-input, unattended to local, and unattended to two implemented stories. `aep plan artifact validate` printed `valid`.
- **Notes (ledger statements, `Uses::existing`, bound use stays counted): closed** (local 310, 317-324, 330).

Your open questions:
- **Instances sharing one grant path.** The second instance fails verification. The issuer and audience are per instance (approval_keys.rs:543, :593-594) and `kid` is per key.
- **`covering` continuing after `OutsideInput`.** This is the right reading: a grant is a sufficient permission (ADR decision 1). See finding 8.
- **Moving the `OutsideInput` check ahead of revoked and expired.** It changes no served answer.
  - The reason only reaches the log (v1alpha2.rs:281-287).
  - A file without `input_keys` skips the check.
  - The adversary tests compare `Expired` only, and its relative rank is unchanged (http_standing_grant_adversary.rs:708-714).

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 291
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Admission order 2 has `standing::admit` do the clock, `resolve` and `unchanged`, but the closure already does that at execution.rs:169-171, and `resolve` reads config, policy and key stores (approval_issuance.rs:222-264). That contradicts the seam at :345-348 (plain arguments, no owner). Keep the clock, `resolve` and `unchanged` in the closure and give `admit` the reading, `&ConfiguredApprovalKey`, instance, operation and `&Value`. `time()` (execution.rs:84-91) acquires a BoundedClock; `now()` only reads it. Also state which refusal a recheck gives when `now()` fails: it errors once more than 30 s have passed since the sample (clock.rs:816-818), and `approval_required` would be wrong."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 307
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'`take` returns a guard' changes a served-host surface: `take` returns `Taken::Slot(Use)` (standing.rs:~475-500), `check_grant`/`release_grant`/`prepare_keyed` pass `&Use` (service.rs:456-520), and http_standing_grant_adversary.rs:15 imports `Taken` and `Uses`. The closed-input story forbids any other served change and edits to those test files (its acceptance 6, line 153-160). Say the guard is a new local wrapper over `Taken::Slot(Use)` and that `take`, `Taken` and `release_grant` stay as they are."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 228
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`grant-standing` is bounded by 256 grants but not by size. The reader refuses a file over FILE_LIMIT (64 KiB, standing.rs:38, :334-345) as `Oversized`, and the JWS adds base64 overhead. A grant can carry 16 targets and, with the closed-input story, 16 `input_keys` of 256 bytes, so a success write can leave a file every call then refuses as `unavailable`, which ends all grants at once. Add a refusal (`invalid_input`) when the signed file would exceed the limit, and a case in acceptance 11 or 13."
- file: apps/connectors/src/local/operations.rs
  line: 237
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Local lines 271-277 refuse an empty approval file in the CLI for every instance. That changes bytes for an instance with no `standing_grants`: today the owner answers (execute :167 gives `not_attempted` with a `mutation` member), and the gate's shape differs (invoke.rs:1155-1160). O2 (line 609) and line 267 promise today's bytes. Refuse only when the instance has the member, or add an acceptance for the no-member case."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 505
  category: design
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "Acceptance 23 makes this story own the `Deferred` classification of `slack_approval::a_slack_bot_post_is_issued_the_approval...` (un-deferring it, or editing its arguments), but story:main-failing-ci-journeys lists the same test among its eleven (its lines 21-26), and no edge or note says which story wins. Either record `depends_on` story:main-failing-ci-journeys, or make the third case the only route: classify the new test `Deferred` and leave the sibling to that story."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 245
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "`paths.state` is one directory per host, not per instance (config.rs:1209-1212; the stores key on `instance_id`; the key lock `approval-issuer.lock` is state-wide, approval_keys.rs:23, :668). Say 'the state directory'. `Lease` is private to approval_keys.rs (:663-668), so name the helper made visible. Lock order is safe: the executor holds the shared key lease through commit and never takes `standing-grants.lock`; `grant-standing` takes it first, then `acquire_key` (:486-488), which is shared."
- file: crates/connectors-host/src/local/owner/mutation/execution.rs
  line: 440
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "Without the post-gate recheck, the grant path is unchecked from :440 through `dispatch_read` (:443-447) and the `open_dispatch` transaction (mutations.rs:273-296). Per-input checks `exp` at :502. This is ms-scale against an in-flight call that no revocation stops, but local lines 304-306 say only that a revocation after the recheck does not stop the write. Say `not_after` can also pass in that gap, or place the recheck after `dispatch_read`. The contract text should also say a refusal at this recheck leaves an aborted attempt carrying the grant id, which ADR decision 1 ('no attempt') does not cover."
- file: .engineering/planning/story/standing-grant-closes-the-input-to-listed-members.md
  line: 102
  category: design
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "`covering` going on to the next grant is right (a grant is a sufficient permission, and the `take` loop picks the first with room), but the contract text list (lines 119-122) does not say a closed list limits only its own grant, so an open grant for the same operation and target makes a closed one moot. Say so. Also state that a `target` pointer into an array fails closed the same way as a listed one (lines 104-106), whenever `input_keys` is present."
```

**What I read:** the three stories, the ADR, round 2, `aep plan artifact relations`, `graph` and `validate`, and the code at the cited lines. I built and ran nothing.

**Not established:**
- Whether the ledger refuses or prunes at 10,000 attempts per instance, which bears on the "bounded" claim for bound rows (local lines 322-323).
- The exact depth boundary of the walk against `channel::depth`.
- The size of the dispatch window, which I could not measure.
