---
format: aep.planning-md/1
id: review-result:adversary-mcp-outbound-connection-lifecycle-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — MCP outbound connection lifecycle, wave 4b
relations:
- reviews: story:mcp-outbound-connection-lifecycle
revision: 1
---
```
unit: mcp-w4b — story:mcp-outbound-connection-lifecycle, worktree cv2-mcp-w4b-20260912 at c1c5e16 (base 4ad2ee5)
verdict: red
cases: executed 54→60, red 6
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: 4 paths under ~/.cache/cv2-mcp-waves-20260912/adversary-w4b/
needs-coordinator: yes
```

## 1. Diff

One new untracked file, `crates/connectors-build/tests/mcp_outbound_connection_lifecycle_adversary.rs`, 532 lines, 6 cases. No implementation file touched, nothing mutated in place, nothing under `.engineering/planning/`.

Every case asserts its premise against an archive or the selection matrix **before** the claim resting on it, so it reports drift instead of going quietly green. All six were red the first time anything executed them. Each survives its own fix: the oracle is "the passage resolves the question", not "the passage says the words I chose" — stating the legacy rule, scoping the claim to the primary revision, or holding it and naming the blocker each turn it green.

## 2. Suite

Before 54, after 60, red 6, exit 101. The unit's own eight cases still pass; none of these contradicts one of theirs. Clippy and fmt exit 0.

## 3. Findings

**The root cause behind rows 1, 2 and 3: sections 3, 10 and 11 say which revision they hold for; sections 6 to 9 do not — and where the two selected revisions disagree, the document states the primary one's rule as the transport's.**

1. `semantics.md:357-368` — §9 states one cancellation signal for Streamable HTTP naming no revision: closing the stream is cancellation and no `notifications/cancelled` is expected. The interoperability revision's transport says the opposite — disconnection SHOULD NOT be read as cancellation and the client SHOULD send one. Reaches: a binding whose configured revision is the interop one, which the matrix dispositions supported outbound and whose handshake, session and shutdown this same document specifies. §8 routes every timeout here, so it is the ordinary path. CONFIRMED, introduced, blocker.

2. `semantics.md:313-315` — §7 refuses **every** JSON-RPC request arriving on a stream as `upstream_protocol` "because this revision defines no channel for it", naming no revision. Under the interop revision the server MAY send requests on that stream and answering `ping` is a MUST. No register row exists for a request this client must answer, and the document names `ping` nowhere. CONFIRMED, introduced, blocker.

3. `semantics.md:389-396` — a dropped connection routes to two outcomes, one requiring an open stream and the other requiring that nothing was framed. A request answered with the **single JSON object** §6 says the client supports, whose connection dies before that object arrives, reaches no row. Reaches: half of the answer shapes this document selects. CONFIRMED, introduced, blocker.

4. `semantics.md:204-217` against `:84-91` — `determined_era` is declared byte-identical before and after an observation in one paragraph and recorded from a wire probe in another. It is a field of the binding entity, and the model notes clients may persist it across restarts, so the second sentence writes a wire-derived value onto the binding without the hedge its sibling carries. This is the durable binding record the story asks a reviewer to look for. CONFIRMED, introduced, warning.

5. `semantics.md:367` — "the other outbound transport family is held by the stdio blocker" states a disposition for the two outbound families the matrix explicitly refuses under no blocker. The unit's own stdio case cannot catch it: that case exempts any line naming the blocker, so such a line may say anything about any transport. CONFIRMED, introduced, warning.

6. `semantics.md:95-97` against `:110-111` — a selection may name "the ordered pair of revisions" a binding may speak, while the outcome an operator observes says "one revision the binding will declare on every request", and the model's only carrier holds one string. Whether a dual-era binding is configurable is stated both ways and observable neither — and §3 and §4 both rest on it. CONFIRMED, introduced, warning.

7. `semantics.md:282-291` — filing `-32020 HeaderMismatch` under `upstream_protocol` blames the peer for correctly reporting that *this client's* header and body disagreed, a condition the framing outcome guarantees this client never produces. The stated reason for rejecting `invalid_input` is right and does not reach `upstream_protocol`; the shared vocabulary carries `internal`. No case — which code is right could not be mechanised. NEEDS-CHANGE, introduced, warning.

8. `crates/connectors-build/src/gate.rs:177` — the gate collects authored scenarios from three directories and not from this unit's, and the scenario format is validated by no schema, so the eighteen new files are held only by the unit's package-scoped test while two sibling stories are told to write into the same directory. CONFIRMED, introduced, note.

The same root cause produces `semantics.md:304-308` — "There is no resumption", unqualified, against a revision whose transport says the client SHOULD resume and MUST respect a retry field. No seventh case was written because fixing 1 and 2 fixes it.

## 4. Attacked and could not break

- **`unsupported` on three rows.** The shared error vocabulary is closed, that member is the right one for all three, nothing requires one code per outcome, and the outcome name carries the distinction the code does not. The judgement call stands.
- **`selected_revision` narrowing** — nothing in the document or any of the 18 scenarios narrows it. The only defect near it is row 6, which widens rather than narrows.
- **The five `UNMAPPED:` markers** — every one named, none settled; the snapshot durability sentence is correctly hedged.
- **Stdio specified without the word** — no passage states framing, cancellation, shutdown or process lifecycle for outbound stdio. Row 5 is the reverse failure.
- **The retry quarantine** — no scenario and no outcome answers a state by sending the request again.

```findings
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 357
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Section 9 states that closing the stream is cancellation and that no cancelled
    notification is expected, naming no revision, while the interoperability revision's
    transport says the opposite for a revision the matrix dispositions supported outbound
    and which sections 3, 10 and 11 of this same document specify.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 313
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Section 7 refuses every request arriving on a stream rather than answering it, but
    under the interoperability revision the server may send requests there and answering
    ping is a MUST, so an outbound state exists that has no register row and the document
    names ping nowhere.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 389
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    A dropped connection routes to two outcomes, one requiring an open stream and the
    other requiring that no request was framed, so a request answered with the single JSON
    object the document says the client supports, lost before that object arrives, reaches
    no row of the register.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 216
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The determined era is declared byte-identical before and after an observation in one
    paragraph and recorded from a wire probe in another; it is a field of the binding
    entity, so the second sentence writes a wire-derived value onto the binding without the
    hedge its sibling carries.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 367
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Attributing "the other outbound transport family" to the stdio blocker states a
    disposition for the two families the matrix explicitly refuses under no blocker, and
    the unit's own stdio case cannot catch it because it exempts every line naming the
    blocker.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 96
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Section 2 says a selection may name the ordered pair of revisions a binding may speak
    while its own outcome says an observer sees one revision declared on every request, and
    the model's only carrier holds one string, so whether a dual-era binding is configurable
    is stated both ways and observable neither.
- file: adapters/mcp/contracts/client/v1alpha1/semantics.md
  line: 287
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    Filing the header-mismatch code under upstream_protocol blames the peer for correctly
    reporting that this client's own header and body disagreed, a condition the framing
    outcome guarantees this client never produces.
- file: crates/connectors-build/src/gate.rs
  line: 177
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The gate collects authored scenarios from three contracts directories and not from this
    unit's, and the scenario format is validated by no schema, so the eighteen new files are
    held only by this unit's package-scoped test.
```
