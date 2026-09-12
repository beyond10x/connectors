---
format: aep.planning-md/1
id: review-result:adversary-mcp-outbound-connection-lifecycle-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — MCP outbound connection lifecycle, wave 4b
relations:
- reviews: story:mcp-outbound-connection-lifecycle
revision: 1
---
```
unit: mcp-w4b
verdict: red
cases: executed 63→68, red 5
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: 2 logs under ~/.cache/cv2-mcp-waves-20260912/adversary-w4b-pass2/
needs-coordinator: yes
```

One untracked test file, 626 lines, 5 cases. No implementation file touched, nothing under `.engineering/planning/`. Each case asserts its premises — that the passage it drives still carries the claim it is read for — before the assertion resting on them; register rows, revision names, the scenario a row points at and the binding's fields are read out of the documents and the model, none hard-coded.

Two honest notes on order: case 2's first run stopped on its own premise, a case-sensitive `contains`, and clippy rejected one closure. Both were the adversary's typos, fixed before the finding assertion was reached; neither is a finding.

Suite: before 63 with the new target deselected, after 68, red 5, exit 101. The unit's 11 cases and pass 1's 6 all still pass. fmt and clippy exit 0.

**The root cause behind findings 1 and 2 is the correction's own shape.** `every_section_and_scenario_resolves_which_revision_it_holds_for` reads a scenario's `given` and nothing else, and asks only that *some* supported revision be named there. A scenario whose `given` was widened to admit both revisions, while its `when` and `then` kept the primary revision's wire, satisfies the new rule **vacuously**. Pass 1's class is not closed for scenarios; it moved from "names no revision" to "names both and specifies one".

**1.** The only scenario deciding the ordinary selection outcome admits a binding configured for the interoperability revision, then requires every request of it to declare the revision in its body `_meta` — which §6 of the same document says that revision has no mirror for, and whose one required header it scopes to after the handshake. So `initialize` carries neither of the two things the bullet says every request carries.

**2.** A scenario says its outcome holds "on both" revisions and states its only `when` as reading the `server/discover` result, which §1 says the interoperability revision does not have at all. For half the bindings the file claims, the `when` never occurs and the outcome is decided by nothing.

**3.** A scenario says a probe "records the determined era", writing a field of the binding entity that §4 deliberately declines to say is written — the exact sentence pass 1's finding 4 was about. The correction's own case misses it twice: it reads only `semantics.md`, and its verb regex is passive-voice only.

**4.** The row added to close pass 1's lost-answer gap files `answer-never-arrived` under `state:transport-unreachable`, beside `transport-unavailable`. One state, two outcomes that deny each other's precondition in the document's own words — one was framed and the endpoint was reachable enough to take it, the other says no request was ever framed — and the state's name denies the newer row.

**5.** The new fourth column reports `transport-unavailable` as the act of the peer, while its own section enumerates a name that never resolved to a server and a bound within which nothing answered — the preamble's own definition of `unobserved`. The sibling row inverts it: `answer-never-arrived`, where the endpoint demonstrably acted, is `unobserved`. One actor per row cannot express this enumeration.

**6, no case.** The `legacy-server-request-answered` row carries `n/a` in both new columns, which the preamble defines as an observation and not an error, while its own section says a non-`ping` server request is answered with a JSON-RPC error and calls it "the same refusal" as a row carrying `unsupported` and `peer`. Mechanising it would key on that phrase, which pass 1 showed rewording breaks.

**7, no case.** A `never` bullet on a scenario whose `given` admits both revisions justifies itself with a reason true only of the primary one. Same class as 1 and 2, but only the reason drifts, not the rule.

Attacked and could not break: the dual-era decision, gone and not reintroduced; the resumption cost statement, true against the archive, with "never a repeated effect" right because resumption replays rather than re-executes; 22 rows against 22 scenarios, exact in both directions on every column; `route_refused` as `operator`, which the preamble's own definition supports; the other three new rows; the retry quarantine; every citation spot-checked into the interop archives; and outbound stdio, where the collective "the other transport" is gone.

```findings
- file: adapters/mcp/contracts/client/v1alpha1/scenarios/explicit-selection-of-a-configured-binding.yaml
  line: 18
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The only scenario deciding the ordinary selection outcome admits a binding configured
    for the interoperability revision and then requires every request of it to declare the
    revision in its body meta, which section 6 of the same document says that revision has
    no mirror for and whose one required header it scopes to after the handshake.
- file: adapters/mcp/contracts/client/v1alpha1/scenarios/unknown-advertised-capability-is-not-support.yaml
  line: 17
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The scenario says its outcome holds on both revisions and states its only when as
    reading the discover result, which section 1 says the interoperability revision does
    not have at all, so for half the bindings it claims the outcome is decided by nothing.
- file: adapters/mcp/contracts/client/v1alpha1/scenarios/era-ambiguous-400-does-not-fall-back.yaml
  line: 18
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    A scenario says a probe records the determined era, writing a field of the binding
    entity that section 4 deliberately declines to say is written, and the correction's own
    case cannot see it because it reads only the semantics document and matches only the
    passive voice.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 71
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The row added to close pass one's lost-answer gap files the never-arrived outcome under
    the transport-unreachable state beside transport-unavailable, so one state's two
    outcomes deny each other's precondition and the state's name denies the newer row.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 70
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The new fourth column reports transport-unavailable as the act of the peer while its own
    section enumerates a name that never resolved to a server and a bound within which
    nothing answered, which is the preamble's own definition of unobserved.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 64
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The legacy-server-request-answered row carries not-applicable in both new columns, which
    the preamble defines as an observation and not an error, while its own section says a
    non-ping server request is answered with a JSON-RPC error and calls it the same refusal
    as a row carrying a code and an owner.
- file: adapters/mcp/contracts/client/v1alpha1/scenarios/stream-ends-without-a-final-response.yaml
  line: 25
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    A never bullet on a scenario whose given admits both revisions justifies itself with a
    reason true only of the primary one, while the interoperability revision's archive says
    a server may terminate the stream when the session expires.
```
