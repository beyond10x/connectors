---
format: aep.planning-md/3
id: review-result:clock-repeat-acceptance-round-1
kind: review-result
status: active
title: 'clock repeat story: acceptance critic, round 1'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
needs-revision

Findings (acceptance lane only). Each line is `artifact — reason — citation`.

1. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 1 never exercises the repeat in the owner's write path. "Every source drops the first exchange" is spent by `approvals issue`, so the later write's single acquisition answers on the first exchange. Change 2's write-path half has no check that is red before the change. Add a test whose approval proof already exists and whose sources drop the first exchange of the write's own acquisition; it ends `unavailable` before and admits and dispatches once after. — `.engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md:68`
2. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 1 joins two outcomes with "and": issue succeeds, and the write is admitted and dispatched once. Split it so either can fail alone. — `…/an-approved-write-survives-one-refused-clock-acquisition.md:69`
3. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 2 contradicts itself: "one per source (two when the first is dropped), not two". Today `time()` is called at `execution.rs:297` and `:433`, so one write costs two exchanges per source. Rewrite it as: "`Server.requests` of each answering local source is exactly 1 after one admitted write; it is 2 today". Keep the first-dropped case as a separate statement (see finding 1). — `…:71-72`; `crates/connectors-host/src/local/owner/mutation/execution.rs:297,433`
4. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 3 packs five assertions into one: `unavailable`, within the caller's deadline, exactly two exchanges per source, `clock_refusal` `below_quorum`, and one `clock_sources` line per source. It also does not name the `stage`. Today `issue` answers `stage` `readiness` and `invoke` answers `admission` (story line 20-21). The CLI contract says `clock_refusal` and `clock_sources` appear only at `stage = observation` (`ess/domains/cli.yaml:946`). The test needs one statement per outcome, and the story must say whether `stage` changes. — `…:73-75`; `ess/domains/cli.yaml:946`; `apps/connectors/src/local.rs:549-570`
5. story:an-approved-write-survives-one-refused-clock-acquisition — Change 3 promises `clock_refusal` and `clock_sources` on `invoke`, and no acceptance tests it. Acceptance 3 covers only `approvals issue`, and acceptance 6 only checks that a document lists the members. Add a test that an `invoke` clock refusal carries both members. Today `owner::Error` has no such member (`owner.rs:145-170`; `Error::clock` at `owner.rs:261-272` returns a bare `Code::Unavailable`). — `…:59-60`; `crates/connectors-host/src/local/owner.rs:261-272`
6. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 4 passes before the change too (one exchange per source already happens), and it names no outcome. State the outcome (`unavailable`, `clock_refusal` `below_quorum`, `next_action` `check_configuration`, the bad source's line `unverified`). Label it a guard that is green before. — `…:76-77`; `contracts/service/clock.md:220`
7. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 5 is not observable. "No narrower than the first", "no later than the expiry check allows" and "existing tests pass" are three claims, and only the last names a command. Today the second reading runs after native preflight (`execution.rs:426-433`) and is checked against `expires_at_ms`. Replace it with one statement: a named test shows the second reading stays inside `expires_at_ms` after a preflight delay. Name the test command for the existing expiry and ledger tests. — `…:78-79`; `crates/connectors-host/src/local/owner/mutation/execution.rs:426-433`
8. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 6 does not say how to check "the contract and the guide state the repeat" or that the CLI contract "lists" the members. The `rg` for 149.56.214.17 is green before (`docs/local-clock.md:51`). Name a grep per file with its expected string, or name the docs or contract check that fails today. The CLI contract files that hold the members are `ess/domains/cli.yaml:946-955`, `contracts/cli/v1alpha1/semantics.md:123-130` and the generated `website/docs/reference/contracts/cli.md`. — `…:80-81`
9. story:an-approved-write-survives-one-refused-clock-acquisition — Change 3 alters CLI answer members and names no consumer that reads them. The repository rule requires it, and the story already names the callers' post script. Add the consumers (post script, `apps/connectors/src/local.rs`, binding and fixtures) to "What changes" or the acceptance. — `…:59-60`; `apps/connectors-cli-contract/binding.json`
10. story:an-approved-write-survives-one-refused-clock-acquisition — Acceptance 7 names a time but no way to read it. The production deadline is 2 s per acquisition (`clock.rs:28`), so a never-answering source takes at least 4 s with the repeat. Tests 1 and 3 need `set_test_query_timeout` (`clock.rs:36-45`) to stay under 5 s. Say so, name the command that prints per-test time (`slot cargo nextest run -p connectors-host <filter>`), and drop the redundant "none over 60 s". — `…:82`
11. story:an-approved-write-survives-one-refused-clock-acquisition — The citation `approval_issuance.rs:381-388` does not show the acquisition. Line 381 is `.map_err(|_| output_refused())?` and 388 is `.approval_clock`. The call is `selected.acquire()` at 392-393, so change the citation to `:391-393`. — `…:30`; `crates/connectors-host/src/local/owner/approval_issuance.rs:391-393`

The other citations hold: `clock.rs:28` is `QUERY_TIMEOUT`, `:375` is "no … retry occurs", `:377` is `acquire_explained().map_err(|refused| refused.failure)` and drops the detail. `execution.rs:84-97` is `time()`, called at `:297` and `:433`. `BoundedClock::now` at `clock.rs:833-838` answers later readings through `interval_at`.

What I read: the 1 story given, via the file path in the brief (`aep plan artifact show` not run). I also read the clock, owner, CLI and contract files cited above, and the clock test fixture (`crates/connectors-host/tests/fixtures/clock/server.rs`, which counts `requests` and supports `respond` and silent). The fixture can support tests 1-4 once it can drop the first exchange per source.

What I could not establish: whether `stage` already changes to `observation` for `issue` and `invoke` once `Error::clock` carries the refusal. That is the design critic's.

Out of my lane: `acquire()` is also called at `recovery.rs:52`, `maintenance.rs:110`, `approval_issuance.rs:392` (issue) and `:632` (grant-standing), and `execution.rs:92`. The scope of open question 2 follows from that list. The attacker-gain question is a design ruling.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 68
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 1 drops only the first exchange of the whole test, which `approvals issue` consumes, so the owner write path's own repeat (change 2) has no check that is red before the change; add a test with an existing proof whose sources drop the first exchange of the write's acquisition and that ends unavailable before and admitted-and-dispatched-once after"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 69
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 1 joins two outcomes with 'and' (issue succeeds; the write is admitted and dispatched once); split into two statements"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 71
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 contradicts itself ('one per source (two when the first is dropped), not two'); state: Server.requests of each answering local source is exactly 1 after one admitted write (2 today, execution.rs:297 and :433), and move the first-dropped case to its own statement"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 73
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 packs five assertions into one and names no stage, while the contract limits clock_refusal and clock_sources to stage = observation (ess/domains/cli.yaml:946) and today issue answers readiness and invoke admission; split it and say whether stage changes"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 59
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "change 3 promises clock_refusal and clock_sources on invoke and no acceptance tests it (acceptance 3 covers approvals issue only; acceptance 6 checks a document); add a test that an invoke clock refusal carries both members, since owner::Error (owner.rs:145-170, :261-272) has none today"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 76
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4 passes before the change and names no outcome; state unavailable, clock_refusal below_quorum, next_action check_configuration and the bad source's line unverified, and label it a guard that is green before"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 78
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 5 is not observable (three claims, only 'existing tests pass' names a command); replace with one statement naming a test that shows the reused clock's second reading stays inside expires_at_ms after a preflight delay (execution.rs:426-433) and name the command for the existing expiry and ledger tests"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 80
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 names no check for 'the contract and the guide state the repeat' or for the CLI contract listing the members, and its rg is green before (docs/local-clock.md:51); give a grep string per file (ess/domains/cli.yaml:946-955, contracts/cli/v1alpha1/semantics.md:123-130, docs/local-clock.md) or the docs/contract check that fails today"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 59
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "change 3 alters CLI answer members and names no consumer that reads them, which the repository rule requires; add the consumers (the callers' post script, apps/connectors/src/local.rs, apps/connectors-cli-contract/binding.json)"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 82
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 names no way to read a test's time and ignores that production's 2 s deadline (clock.rs:28) makes a never-answering test with a repeat at least 4 s; require set_test_query_timeout (clock.rs:36-45) in tests 1 and 3, name the nextest command that prints per-test time, drop the redundant 60 s clause"
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the citation owner/approval_issuance.rs:381-388 does not show the acquisition (381 is output_refused, 388 is .approval_clock); the call is approval_issuance.rs:391-393; change the citation"
```
