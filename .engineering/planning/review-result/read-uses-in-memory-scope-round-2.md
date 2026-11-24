---
format: aep.planning-md/3
id: review-result:read-uses-in-memory-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, story:settled-read-uses-leave-the-store
relations:
- reviews: story:settled-read-uses-leave-the-store
revision: 1
---
verdict: needs-revision

**Promises of the ask, each against an acceptance line**
- "Read uses are no longer recorded": acceptance 1 (zero events and blobs) and 2 (no recorded `ReadUse`).
- "Open ones live in the owner process": acceptances 5, 6 and 7.
- "Connectors only": acceptances 3 and 13 (no ESS or generated file changes).
- Item 1, "grows with every read": acceptances 1 and 12.
- D3 "A then B": A is a `depends_on` story. B is this story.

All four promises are claimed. Nothing is drafted that the ask excluded. Docs and contract edits (14 to 16) follow from the stated "recorded dispatched" wording. The `#[cfg(test)]` switch is test machinery for the Design, not scope.

**Round-1 findings**
1. Existing stores: answered. Source names ways A, B and C, Out of scope states Timo's pick, and acceptance 12 puts the limit on #267.
2. CLI revoke: answered. Difference 4, acceptance 5, 15 and 17.
3. Cross-process visibility: answered. Differences 2 and 3, with "one owner serves a state directory" now established in the tree facts.
4. Expired per leftover use: answered. Outcome, difference 5, acceptance 8 and 17.

**Findings**

1. The Outcome overclaims against what acceptance 1 measures. Line 39 area says an instance "that reads all day keeps a store whose size follows its connections and its writes". `registry.rs:675-681` still records a clock-floor event for any registry transaction more than 60,000 ms past the recorded floor, and reads are registry transactions. A read-only instance therefore still gains about one event per minute of use. Acceptance 1 stays inside the 60,000 ms window, so it never touches this. The growth is deliberate under `contracts/service/clock.md`, so it is not a defect to fix here, but the story neither says it nor excludes it.
   - Change: in the Outcome and in Out of scope, say that the clock-floor event (at most one per 60 s of use, a separate contract) still records. Narrow "follows its connections and its writes" to "records nothing per read".

2. The Out of scope entry on existing stores is honest about the gap, but it names no carrier. "The question goes to host-operator after the release" is a future act that no acceptance line claims and no artifact holds. The AGENTS rule "A default proceeds" has the question put to host-operator with a default, so nothing requires waiting for the release.
   - Change: either send the question to host-operator now, with a recommended default, and cite that message in Out of scope; or add an acceptance line that the report names the question and that it was sent.
   - Change: add to acceptance 12 that the pull request carries no closing keyword for #267, so the issue stays open for old stores.

**What I read**
- The story in full.
- Round-1 result.
- Epic `epic:review-20261123-findings`.
- Graph, filtered for clock, read-use and existing-store items.
- `registry.rs` lines 50-70 and 665-690.
- The scope-critic procedure and the rubric.

**What I could not establish**
- I did not verify the story's other line citations.
- I could not tell whether the clock events are bounded by retention, so I could not say how large the residual growth is.
- Whether acceptance 8 (a fixture with one use) proves "one Expired event per use" for several uses is an acceptance-lane question. Acceptance 10's `expiry_batch_tests` may cover it.
- I did not read the issue text, only the story's Source quotes of it.

```findings
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 39
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Outcome says a store that reads all day has a size that follows its connections and its writes, but registry.rs:675-681 still records a clock-floor event per 60 s of any registry transaction, reads included, and acceptance 1 stays inside the 60,000 ms window; state in Outcome and Out of scope that the clock-floor event still records and narrow the claim to 'records nothing per read'"
- file: .engineering/planning/story/settled-read-uses-leave-the-store.md
  line: 322
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Out of scope on existing stores says nobody carries them and the question goes to host-operator after the release, which no acceptance line or artifact claims; send the question to host-operator now with a recommended default and cite it, or add an acceptance line that the report names and sends it, and add to acceptance 12 that the pull request has no closing keyword for #267"
```
