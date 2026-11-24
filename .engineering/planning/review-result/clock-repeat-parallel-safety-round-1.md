---
format: aep.planning-md/3
id: review-result:clock-repeat-parallel-safety-round-1
kind: review-result
status: active
title: 'clock repeat story: parallel-safety critic, round 1'
relations:
- reviews: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
verdict: needs-revision

The story has no scope list and no "shares files" section, so it names none of the collisions below. Surfaces I could place: 5 cited (`clock.rs`, `owner/approval_issuance.rs`, `owner/mutation/execution.rs`, `contracts/service/clock.md`, `docs/local-clock.md`) and 6 inferred (CLI contract `semantics.md`, `ess/domains/cli.yaml`, `apps/connectors-cli-contract/binding.json`, generated contract pages, `CHANGELOG.md`, the test files). I could not place none of them. I traced one acceptance read to a producer in the set, and no `depends_on` edge records it (see the last finding).

Collisions, as I read the three trees:

- **story:standing-grant-binds-a-pattern-or-a-value-set** (tree `pattern-grant`) edits `contracts/cli/v1alpha1/semantics.md`, `CHANGELOG.md`, `apps/connectors-cli-contract/binding.json`, the generated `website/docs/reference/contracts/**` pages and `owner/mutation/tests.rs`. The tree has no uncommitted change in `execution.rs`, `approval_issuance.rs`, `clock.rs`, `docs/local-clock.md` or `contracts/service/clock.md`. Its `execution.rs` change is commit 7810071f7, which is already in the clock story's base (0.77.1).
- **story:connections-list-answers-through-the-owner** (branch `story/connections-through-owner`) edits `CHANGELOG.md` and `contracts/cli/v1alpha1/semantics.md` (one hunk at `:1375`). It touches none of the clock story's other files.
- **story:the-local-gate-lints-the-windows-target** changes `gate.rs`, `rust-gate.yml` and `docs/development.md`. It shares no file with the clock story.

```findings
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 59
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Change 3 widens `clock_refusal` and `clock_sources` from `approvals clock-check` to `approvals issue` and `invoke`. `ess/domains/cli.yaml:946-953` says the pair appears 'only on an approval clock's unavailable (stage = observation)', so that comment must change. ESS source, `binding.json`, the generated model pages and the contract digest then regenerate. story:standing-grant-binds-a-pattern-or-a-value-set already regenerated `apps/connectors-cli-contract/binding.json` (hunk at `@@ -10057`), the digest footer of `website/docs/reference/contracts/model/domains/connectors-clock.md` and every other domain page, and `crates/connectors-core/generated/operation-types/*` in `pattern-grant`. The digest line and `binding.json` are one line or hunk that both stories rewrite and neither body mentions (cited for the pattern-grant side by its tree status; inferred for the clock side, since the story names no ESS file). Add a 'Shares files with' section naming `ess/domains/cli.yaml`, `apps/connectors-cli-contract/binding.json`, `website/docs/reference/contracts/model/**` and the digest. Name both remedies without choosing: an ordering edge after the pattern-grant story with that reason, or a regeneration-only rebase step after the first merge. Also list `ess/` and the generated files in the story's scope."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 80
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance 6 and change 4 edit `contracts/cli/v1alpha1/semantics.md` (the clock-check paragraph at `:120-135`, cited). The pattern-grant tree rewrites other regions of the same file (hunks at `:961-1075`, net +1 line). story:connections-list-answers-through-the-owner edits `:1375-1380` and its acceptance 8 requires no net line in `semantics.md`, because `ess/domains/cli.yaml` cites its lines (for example `cli.yaml:2045` and `:2205`). The three regions do not overlap, but lines added at `:128` shift every later citation. Name `semantics.md` in a 'Shares files with' section for both stories, state that the clock story edits `:120-135` only, and say that an added line there needs the `cli.yaml` citations re-pointed. Remedies: an ordering edge, or keep the clock text net-zero in lines."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 51
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "`CHANGELOG.md` is not in the story, but a behaviour change (a repeat, one acquisition per write, a new refusal member) needs an entry under the next heading (top section is `## 0.77.1`, `CHANGELOG.md:3`). The pattern-grant tree adds 25 lines at its head (hunk `@@ -4,6 +4,31 @@`) and connections-through-owner adds 29. All three insert at the same spot (inferred for the clock side). Add `CHANGELOG.md` to the scope and say that the second and third to merge rebase their entries."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 53
  category: parallel-safety
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Changes 1 and 2 land in `owner/mutation/execution.rs:84-97` (cited) and `owner/approval_issuance.rs:381-388`. The brief says pattern-grant changes `execution.rs`. Today it does not: `git status` of `pattern-grant` lists no `execution.rs`, and its last change there is commit 7810071f7, already an ancestor of 0.77.1. Round 2 can still reopen it, and the same write path now holds the standing-grant admission. The only overlap I can show is `owner/mutation/tests.rs` (a one-line edit by pattern-grant, `Bound::Value`), where the clock story's acceptance 1 and 2 tests most likely go (inferred). Name the test files in the scope so the overlap is visible, and say 'no overlap in `execution.rs` as of the pattern-grant tree on 2026-11-24'."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 68
  category: parallel-safety
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The story-before-Windows-lint pair does not collide on a file today. After story:the-local-gate-lints-the-windows-target lands, any new test-only helper or seam in `execution.rs` or the new UDP-source tests that has callers on Unix only fails the Windows lint, the defect that story records (`execution.rs:819` `seam::set`). The new UDP-source tests of acceptance 1 to 4 must compile and lint without dead code on `x86_64-pc-windows-gnu`. Add that to acceptance 7 or the notes. This is an ordering fact for the operator, not a collision."
- file: .engineering/planning/story/an-approved-write-survives-one-refused-clock-acquisition.md
  line: 78
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Reader beside a writer, by acceptance. Acceptance 5 says the existing expiry and ledger tests 'pass unchanged' and acceptance 1 says the write is 'admitted and dispatched once'. Both read the owner write path that story:standing-grant-binds-a-pattern-or-a-value-set is still reshaping (standing admission in `owner/mutation/execution.rs` and its tests). The story has no relation to it. Add an edge or a note that the acceptance 5 baseline is the tree after that story's merge. Without one, the 'unchanged' claim is checked against a base that moves."
```

Out of my lane: the two open questions (security and design ruling), the clock-source address in acceptance 6 (already known to the author), and whether acceptance 3's "exactly two exchanges" is checkable.
