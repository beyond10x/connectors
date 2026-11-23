---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

1. Issue #267 item 1 (stores that already hold read uses) is excluded, but the exclusion has no stated reason and no owner. The Out of scope line reads "Shrinking a store that already holds read uses (ways B and C of question D5)". It does not say what B and C were, that Timo picked A over them, or who carries the existing-store case. The Source names the unchosen ways only as "a removal in Entity Runtime, a rewrite of the store". No other story or open question in the graph carries the shrink. The measured 11.0 MB and 2,710 `ReadUse` entries stay in every store that already has them. D3's wording for B is "settled read records leave the store", and D5 A narrows that to new writes. The narrowing is not recorded as a decision.
   - Change: state in Out of scope that Timo's D5 pick A chose "stop recording" over B and C, and name what B and C were.
   - Change: name the carrier of the existing-store case, either a follow-up question put to host-operator or a story. Say whether #267 item 1 stays open for those stores.
   - Change: make the acceptance 11 report on #267 say that item 1 is fixed only for reads from this release on.

2. "What is given up" omits a loss that the story's own Design states. A `connections revoke` from the CLI no longer releases the owner's undispatched uses.
   - Today `lifecycle.rs:469` does `UPDATE registry_uses SET released=1 WHERE connection_ref=?1 AND dispatched=0`. The story's facts put the revoke in the CLI process (`connections.rs:168-182`). Design says it "does not reach the owner's table".
   - Consequences not stated in the CHANGELOG list:
     - Such a use stays in the owner's table, and in the bound count, until its deadline.
     - The owner's dispatch of it now answers `revoked` or `lifecycle_conflict`. On a released use it answers `Conflict` (`use_and_retirement.rs:297`).
   - Change: add this as a fourth loss in the Design paragraph and in the acceptance 14 CHANGELOG entry.

3. The same paragraph also omits the cross-process consequence of the table. Design makes the table per owner process, and the "Not established by the scoper" paragraph leaves two things open: whether two owners can share a state directory, and whether the mutation executor runs only in the owner.
   - A use captured in one process cannot be dispatched, counted, or seen by `deletion_allowed` in another.
   - The story treats this as unestablished, not as given up.
   - Change: either state "a use is visible only to the process that captured it" as part of the given-up list, or settle both unestablished facts before the story is approved.

4. The Outcome says "A read stops writing to the local metadata store", but Design keeps the sweep in the capture transaction. The sweep is the `DELETE FROM registry_uses` at `use_and_retirement.rs:263`, committed as `ExpireReadUse` (`er.rs:4749-4762`). On a store written by an earlier release, the first read invoke after its leftover uses' deadlines therefore still writes one Expired event per leftover use (acceptance 7). The ask was "Read uses are no longer recorded". This write is not in the Outcome or the CHANGELOG list, and the story does not say that skipping the sweep was considered. It also means "do not shrink" is incomplete: those stores grow by one event per leftover use.
   - Change: state in Outcome and in the CHANGELOG entry that old stores get one Expired event per leftover use. Alternatively, record why the sweep stays and what the alternative would cost.

Checked and not findings:
- The ask has four promises: reads not recorded, open uses live in the owner process, connectors only, and store growth per read goes to 0. Acceptances 1, 2, 11 and 12 cover all four. Promises traced: 4 of 4, plus D3's B text, which is finding 1.
- The documents (acceptance 13) are needed because the contract text says a use is "recorded dispatched" (`semantics.md:2094`, `owner.md:210`). They are not beyond the ask.
- Acceptances 5, 6, 9 and 10 follow from the Design.
- The remaining Design choices trace to D5 or to "connectors only": the table, all three capture kinds, readers moved to the table, and ESS left untouched.

What I read: 1 story, the parent epic, `aep plan artifact graph`, `story:read-use-bound-counts-live-reads`, `story:ess-evolution-er-metadata-adoption` (§ read-use retention), and the cited lines in `semantics.md`, `owner.md`, `lifecycle.rs`, `use_and_retirement.rs` and `er.rs`.

What I could not establish: the epic's body does not name issue #267 (the story's Source does), so the ask is taken from your message and the story's Source. I did not check the story's line citations beyond those listed. Acceptance checkability, the soundness of the Design, and parallel safety are other critics' lanes (out of my lane).

```findings
- file:  (story:settled-read-uses-leave-the-store, section Out of scope)
  line: 1
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Out of scope excludes shrinking stores that already hold read uses (D3 B 'settled read records leave the store') with no stated reason, no account of what ways B and C were, and no owner or follow-up; the Out of scope entry and the acceptance 11 report on #267 must state that item 1 is fixed only for new writes, that Timo's D5 pick A chose this, and who carries the existing-store case"
- file:  (story:settled-read-uses-leave-the-store, section Design, last paragraph)
  line: 1
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'What is given up' omits the CLI revoke no longer releasing the owner's undispatched uses (lifecycle.rs:469 today): those uses stay in the table and the bound until their deadline and dispatch answers revoked or lifecycle_conflict instead of Conflict; add it to the paragraph and to the acceptance 14 CHANGELOG entry"
- file:  (story:settled-read-uses-leave-the-store, section What the tree holds today)
  line: 1
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The per-process table makes a use invisible to any other process (dispatch, bound, deletion_allowed) and the story leaves 'two owner processes on one state directory' and 'mutation executor only in the owner' unestablished; state the cross-process loss in the given-up list or establish both facts before approval"
- file:  (story:settled-read-uses-leave-the-store, sections Outcome and Design)
  line: 1
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Outcome says 'A read stops writing to the local metadata store' but Design keeps the capture sweep (use_and_retirement.rs:263), so a first read invoke on an old store still records one Expired event per leftover use; say so in Outcome and the CHANGELOG entry, or record why the sweep stays and the cost of dropping it"
```
