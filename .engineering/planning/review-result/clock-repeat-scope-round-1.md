---
format: aep.planning-md/3
id: review-result:clock-repeat-scope-round-1
kind: review-result
status: active
title: 'clock repeat story: scope critic, round 1'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: needs-revision

Promises extracted from the parent (the incident's "What happened" and "Cause"): 7. Traced to a change or a named exclusion: 7. The parent itself defines "Done outside the product" as not claimed.
- Three acquisitions per write: change 1.
- No repeat after a refusal: change 2.
- Dropouts of the two answering sources and of the uplink: change 2.
- The bare `unavailable` with no source named: change 3.
- The Cloudflare wire version: out of scope, named in the story.
- The moved `time.txryan.com` address: change 4, the part the author already knows.
- Contract and guide stating the behaviour: change 4.

No gap and no unnamed narrowing. Both findings below are reach beyond the story's own list.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 59
  category: scope
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 3 puts `clock_refusal` and `clock_sources` on `issue` (stage readiness) and `invoke` (stage admission). The CLI contract limits both members to `approvals clock-check` at `stage = observation`: ess/domains/cli.yaml:946-951 (\"only on an approval clock's `unavailable` (`stage = observation`)\") and contracts/cli/v1alpha1/semantics.md:126-130. The code builds them only in apps/connectors/src/local.rs:561-569. `Error::clock` (crates/connectors-host/src/local/owner.rs:261-274) returns a bare `Code::Unavailable`. So change 3 edits the CLI answer's members and presence rule. The story names none of the files it reaches (ess/domains/cli.yaml, semantics.md, apps/connectors-cli-contract/binding.json, contracts/cli/v1alpha1/fixtures/values.json, website/docs/reference/contracts/cli.md, apps/connectors/src/local.rs) and no consumer that reads the members (repo rule: a CLI answer-member change names its consumers). Fix: add to change 3 the list of files and consumers above, and the new presence rule. Say which stage values carry the members (readiness and admission, not only observation). Say whether the other `Error::clock` callers (grant-standing, owner/approval_issuance.rs:628) carry them."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 61
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 4 lists only `contracts/service/clock.md` and `docs/local-clock.md`. Three more places say \"no retry\" and become false once the repeat lands: docs/design/approval-clock-quorum.md:18 (\"One exchange, 2 s deadline, no retry\"), :133 (\"There are no retries\") and :346 (network-attacker row); and the doc comment of `acquire()` at crates/connectors-host/src/local/clock.rs:373-375 (\"One parallel exchange per source ... or retry occurs\"). Inside clock.md, line 99 (\"There are no automatic network retries\") is the sentence to change, not only to extend. Fix: add the design document and the `acquire()` doc comment to change 4, and name clock.md:99 as the line replaced."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 58
  category: scope
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Callers found (open question 2). `ApprovalClock::acquire`/`Configuration::acquire`: owner/approval_issuance.rs:387-388 (`issue`), owner/mutation/execution.rs:84-97 (`time()`, called at :297 and :433), owner/approval_issuance.rs:627-628 (`standing_reading`, grant paths), owner/maintenance.rs:109-111 (recovery, `.ok()` swallows the refusal). `acquire_explained`: clock.rs:508 (the wrapper, used by `approvals clock-check` in apps/connectors/src/local.rs). Change 2 covers the first two only. Leaving `standing_reading` and recovery unrepeated is a narrowing the story does not record. Fix: add to \"Out of scope\" that `standing_reading` and recovery keep one exchange, or move the repeat into `acquire()` (the design critic rules). Note also that `time()` at execution.rs:297 serves the standing-grant route too, so change 1 applies to it. Acceptances 1 and 2 test only the per-input proof route."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 53
  category: scope
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Citations checked and true: clock.rs:28 (`QUERY_TIMEOUT` 2 s), :375 (\"or retry occurs\"), :377 (`acquire` maps `acquire_explained` to `refused.failure`), approval_issuance.rs:381-388 (it contains the single `acquire`, at 387-388), execution.rs:84-97 (`time()`, called at :297 and :433), `BoundedClock::now` at clock.rs:834. Out of my lane (design): execution.rs:428-433 comments the second acquisition as a \"fresh clock acquisition\" after native preflight (`prepare_write_until`). Change 1 reuses the first clock, which changes how fresh the reading is. The parent's cause claims only the count of acquisitions."
```
