---
format: aep.planning-md/1
id: review-result:adversary-mcp-inbound-local-binding-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — MCP inbound local binding, wave 4a
relations:
- reviews: story:mcp-inbound-local-binding
revision: 1
---
```
unit: mcp-w4a — story:mcp-inbound-local-binding, worktree cv2-mcp-w4a-20260912 at 1d22f58 (base 4ad2ee5)
verdict: red
cases: executed 54→57, red 3
origin: introduced 6, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no
```

## 1. Diff

No tracked file modified. One untracked test file, `crates/connectors-build/tests/mcp_inbound_local_binding_adversary.rs`. No implementation, contract, scenario or planning file touched.

## 2. Cases added — three, all red

Each derives its bound from **two** files — `ess/domains/sessions.yaml` and `contracts/sessions/v1alpha1/semantics.md` §4.1 — and asserts they agree, so a fix that edits one source into agreement with a trace is caught by the other. No millisecond literal appears in the case file.

```
3 of the traces this story ships issue a data lease longer than the ceiling
contracts/sessions/v1alpha1/semantics.md §4.1 sets and ess/domains/sessions.yaml repeats.
ESS compiles them because it does not execute a trace, and nothing else compares a trace to
the timing obligations of the vocabulary it reuses:
  cancelled-request-answers-nothing.yaml:56 — lease of 4000 ms, ceiling 2000 ms
  progress-stops-at-the-result.yaml:56 — same
  transport-drop-loses-the-session.yaml:56 — same

progress-stops-at-the-result.yaml:96 — act 5 PermitData is admitted 1000 ms after the last
moment its lease can legally be live

partial-result-never-reported-complete.yaml:85 — BeginClose records a cutoff deadline
2000 ms after the deadline of the lease it ends, which the model says the earlier
deadline dominates
```

## 3. Suite

Before 54, after 57, red 3, exit 101. The unit's own eight cases stay green — they do not overlap what these measure.

## 4. Findings

**The class: the six traces were authored against the session vocabulary's *lifecycle* and never against its *timing obligations*.** The unit's eight checks read the model for which transitions exist and which outcomes perform them, which is real derivation — but §4.1 of the normative owner that same file names is read by nothing. The document's own §6 says why that gap is where a defect survives: ESS "compiles obligations but does NOT execute a sequential trace".

1. Three traces issue a `DataLease` of 4,000 ms where §4.1 sets at most 2,000 ms after authoritative issuance, including delivery delay, clock uncertainty, scheduling delay and buffered output. Reaches: the traces **are** the deliverable the acceptance names, and the document tells two sibling stories to add files to this directory in this shape. §4.1 also says a binding must **refuse session admission** if it cannot enforce the ceiling — so an implementation built to these traces refuses its own admission. CONFIRMED, introduced.

2. The progress trace admits data 3,000 ms after issuance with no renewal anywhere in it. This is the answer to whether the `permit_data` collapse costs anything: it does. The document rules out the only other `Ready`→`Ready` transition — "it is not `renew`" — so a request running longer than one lease has no modelled way to stay `Ready`, and the document never says it must renew or die. CONFIRMED, introduced.

3. `BeginClose` records a cutoff deadline 2,000 ms after the live lease's own deadline, which the model says the earlier deadline dominates and §4.1 names as the grace it forbids. Weaker than 1 and 2 and graded so: the trace's behaviour is right, the recorded deadline is not. CONFIRMED, introduced.

4. §3.1 assigns `-32700` to three conditions, one being an envelope whose `id` cannot be read — which is parsed JSON and therefore `-32600` by the pinned schema's own definitions. The same document's §4 row restricts `-32700` to an unparseable line, so the two halves disagree. Not made into a case: mechanising "condition C matches code K's meaning" needs judgement, not a program. NEEDS-CHANGE, introduced.

5. The enumeration gives version and capability mismatch exactly one transition, `deny_ready`, and one trace — a path that exists **only** on the legacy route of the interoperability revision. §3.6's own prose says that on the revision the document calls primary, "None of these closes the connection: each is one request's refusal, and the session stays `Ready`". A reader implementing from the table row denies readiness where §3.6 forbids it. NEEDS-CHANGE, introduced.

6. Scenario ownership is decided by a bare substring search for the story id anywhere in the file, then asserted at exactly six. The document tells two sibling stories to add files here, and the natural header comment cites this story — so a sibling is silently adopted and breaks this check in the sibling's tree. The structured `# Owner:` marker the files already carry is the discriminator. NEEDS-CHANGE, introduced.

## 5. Attacked and could not break

- **The `permit_data` collapse as such.** The vocabulary offers exactly two `Ready`→`Ready` transitions and the other is a lease transition. The document is right that nothing else exists, and it flagged the collapse rather than contriving. The cost is finding 2, not the collapse.
- **Citation accuracy.** All 34 quoted passages machine-checked against the decompressed archives at their cited ranges: 34 of 34 support the quote.
- **The single-principal boundary.** The check is evadable — a line-prefix scan misses a flow-style mapping, and a fourth field is uncovered — but nothing in the tree evades it, so there is no case and no promoted state that had to be built. *Nothing found.*
- **Behaviours the transport does not offer.** Every section of the stdio archive enumerated against the document's six rows; all six present with an archived citation. *Nothing found.*
- **Cloud, identity or network dependency.** Three lines carry a listed word and all three refuse or defer on their own text. *Nothing found.*

```findings
- file: adapters/mcp/contracts/server/v1alpha1/scenarios/progress-stops-at-the-result.yaml
  line: 56
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Three of the six traces this story ships issue a DataLease of 4,000 ms, double the
    2,000 ms ceiling the sessions contract sets and the model repeats, and that section
    says a binding which cannot enforce the ceiling must refuse its own session
    admission.
- file: adapters/mcp/contracts/server/v1alpha1/scenarios/progress-stops-at-the-result.yaml
  line: 96
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The progress trace admits data 3,000 ms after its lease was issued with no renewal
    anywhere in it, and the document rules out the only transition that could extend it,
    so it specifies an inbound request that cannot legally run longer than one lease and
    never says what happens when it does.
- file: adapters/mcp/contracts/server/v1alpha1/scenarios/partial-result-never-reported-complete.yaml
  line: 85
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The close act records a cutoff deadline 2,000 ms after the live lease's own deadline,
    which the model says the earlier deadline dominates and the sessions contract names as
    the grace it forbids.
- file: adapters/mcp/contracts/server/v1alpha1/semantics.md
  line: 143
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The parse-error code is assigned to an envelope whose id cannot be read, which is
    parsed JSON and therefore the invalid-request code by the pinned schema's own
    definitions, and the same document's later row restricts the parse code to an
    unparseable line.
- file: adapters/mcp/contracts/server/v1alpha1/semantics.md
  line: 130
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The one transition and one scenario given to version and capability mismatch cover
    only the legacy route of the interoperability revision, while the document's own prose
    says a mismatch on the primary revision closes nothing and keeps the session ready.
- file: crates/connectors-build/tests/mcp_inbound_local_binding.rs
  line: 299
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    Scenario ownership is decided by a bare substring search for the story id anywhere in
    the file, so a sibling story's scenario that merely cites this story in a header
    comment is silently adopted and trips the exact-count assertion; the structured owner
    marker the files already carry is the discriminator to use.
```
