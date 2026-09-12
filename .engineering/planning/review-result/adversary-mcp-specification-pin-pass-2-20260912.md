---
format: aep.planning-md/1
id: review-result:adversary-mcp-specification-pin-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — MCP specification pin, wave 1
relations:
- reviews: story:mcp-specification-pin
revision: 1
---
```
unit: mcp-w1 — story:mcp-specification-pin, worktree cv2-mcp-w1-20260912, commit 1e15b49 (4192883 + correction round 1), plus one untracked test file
verdict: red
cases: executed 24→28, red 4
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: /home/timo/.cache/cv2-mcp-waves-20260912/adversary-w1-pass2/ (14 files, 1.8M)
needs-coordinator: no
```

## 1. Diff

One added path, `crates/connectors-build/tests/mcp_specification_pin_adversary_pass2.rs`. No implementation file edited, no existing case deleted, skipped, weakened or rewritten. `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings` both exit 0, so the red is assertions and not lint.

## 2. Cases added

| Test | Asserts | Now |
|---|---|---|
| `design_document_accounts_for_every_transport_binding_the_pinned_revisions_specify` | design.md's transport statement names every binding the archives specify | RED |
| `documented_provenance_check_refuses_a_wholesale_revision_to_commit_swap` | the record's own jq program refuses a manifest whose two commits are exchanged | RED |
| `documented_provenance_check_refuses_a_path_repointed_within_its_own_revision` | the same program refuses an entry repointed at another file of its own revision | RED |
| `record_does_not_count_a_banner_document_twice_when_naming_running_text_mentions` | documents the record calls "more" than the banner set are not inside it | RED |

Cases 2 and 3 extract the jq program from the fenced block of the record and run **that text**, not a restatement, so strengthening the record strengthens the case. Every count is derived from the archives; none is written into the test. Mutated manifests are built in `CARGO_TARGET_TMPDIR`; the tree under attack is never mutated.

Each oracle survives its own fix: naming HTTP+SSE in the design document, binding revision to commit and file to path in the jq program, and rewording "two more" each turn the respective case green without touching the archives.

## 3. The suite

bin target 20 passed; pass-1 adversary 4 passed; pass-2 adversary 0 passed / 4 failed; exit 101. Before is 24, from a run with only the pass-2 file deselected. `gate.rs:76` runs `cargo test --workspace`, which selects this target, so the gate goes red with it.

## 4. Findings

All against `1e15b49`. Origin read at `4192883^`, where the adapter directory does not exist, so nothing here is pre-existing.

**1. `design.md:37` — the corrected transport count is still an undercount.** design.md contains neither `HTTP+SSE` nor `2024-11-05`. The primary revision gives that binding its own section at `…streamable-http.mdx:695`, with SHOULD NOT adopt and SHOULD migrate at :703-705 and a full fallback procedure at :710-737, and `…deprecated.mdx:31` lists it under Deprecated, above Removed — still part of the specification. The interop revision specifies the same fallback at `…transports.mdx:286-311`, ending "should use that transport for all subsequent communication". Reaches: the bullet exists to bound the option space and says so; the interop revision is pinned *for* the legacy seam, and that seam's documented fallback terminates in HTTP+SSE, so a selection matrix built from this sentence has no row for the binding the interop direction actually lands on. CONFIRMED, introduced by the correction.

**2. `specification-sources.md:60-73` — the new provenance program accepts a wholesale revision-to-commit swap.** With the two commits exchanged and every url rederived, the record's own program prints `provenance holds for 54 entries … one commit per revision` and exits 0. It counts distinct pairs and never binds a revision to *its* commit. The Rust guard passes it too, asserting only that the record contains the commit, and the record names both. Reaches: the record states the archives rather than the network are the retained authority from here on, so the recorded url is the sole pointer back upstream. **22 of the 54 swapped urls still return HTTP 200**, and one fetched at the swapped commit hashes to `45a6e8b7fb8c96e7…` against the recorded `3f7e73f428298cb3…` — the swap resolves to plausible later bytes rather than failing loudly. Against the tree as committed all 54 re-fetch to exactly the recorded digests, so this is a missing guard and not a wrong value today. CONFIRMED, introduced by the correction.

**3. `specification-sources.md:104-109` — nothing enforces the naming rule the record states.** Repointing one entry's path at another document of its own revision and rederiving the url leaves the program at exit 0. It checks that the archive name derives from the file, never that the file derives from the path — which is the rule the record writes down as the one a later appender must extend. Smaller than 2 because the record does not claim to check it: a gap, not drift. CONFIRMED, introduced by the correction.

**4. `specification-sources.md:139-142` — the corrected paragraph counts two documents twice.** It says 18 of the 22 archived interop documents carry the banner, then "two more" name it in running text, naming two documents that both carry the banner and are inside the 18. The next sentence closes the set with the four that carry none. 18 + 2 + 4 = 24 against a set of 22. Reaches: this is the replacement for the paragraph pass 1 filed as the unit's one blocker, and the correction's own message names it as the trap the eleven stories must not walk into. A reader summing its figures gets 20 of 22; the answer is 18. The substance is right — banner count, the four names, the line numbers and the primary revision's zero all verify — only the word "more" is wrong. CONFIRMED, introduced by the correction.

**5. `mcp_specification_pin_adversary.rs:133-140` — the amended case discriminates the record it was amended against, and almost nothing else.** The discrimination was re-derived rather than trusted: running its oracle over the pre-correction blob fails on the guard, so the amendment is not a rubber stamp. But the body asserts only that the literal `18 of the` and the words `Protocol Revision` appear *somewhere*. Three mutants of the committed record are all accepted: the denominator changed to `18 of the 30 archived`; the four named exceptions replaced with four documents that do carry the banner; and the primary-revision claim inverted to "all 30" when 0 do. The count is derived from the archives; nothing binds it to its subject. Reaches: the record cites the suite as what a reader runs, and this guard stands behind the unit's most-read paragraph. CONFIRMED, introduced by the correction. Judgement, no case.

**6. `mcp_specification_pin_adversary.rs:236-241` — the Rust content guard is weaker than the `diff` the record documents.** It compares the archive count with the entry count. A manifest that records one archive twice and omits another keeps both counts at 54 and every digest still re-derives, so the guard passes while the record's documented `diff` compares both directions as sets and refuses it. The record claims a missing archive, an unrecorded archive, an altered digest and an altered byte length each fail — true of the shell check, not of the suite meant to make it enforceable. Reaches: the record invites later appends to this directory, which is exactly when a hand-edited manifest duplicates a line. CONFIRMED, introduced by the correction. Judgement, no case.

**7. `specification-sources.md:119-131` — 9 of the 40 numeric citations land on a heading the row's Section column does not name.** Every cited line lands on a real heading and every named section has a citation, but the reverse does not hold: the transports row cites `## stdio` and `## Streamable HTTP`, the negotiation row cites Request, Response and When to Call, the auth row cites Authorization Server Discovery and Client Registration Approaches, and the tasks row cites Capabilities and Protocol Messages, none of them named. Reaches: the acceptance requires the record to name which sections later contract statements will cite, and an author reading the Section column cannot tell why a line is in the row. CONFIRMED, introduced by the original commit; the correction already did this for one row, which is the shape of the fix. Judgement, no case.

## 5. Attacked and could not break

- The rewritten banner claim, counted from the bytes: exactly 18 of 22 interop documents carry the banner, 17 at line 7 and one at line 5; the four named exceptions are exactly the four without; 0 of the primary revision's 30 carry any. Every figure correct.
- Every version-string trap verified verbatim, including the supported array and the era detection.
- The completed exclusion account: both schema renderings fetched at their pinned commits are 1,771 and 2,316 bytes as stated, and carry zero occurrences not only of MUST, SHOULD and MAY but of SHALL, REQUIRED, RECOMMENDED and OPTIONAL. Against the full upstream trees at both commits, every normative document and both schemas are archived and the five exclusion patterns cover the remainder exactly — the gap pass 1 found is closed.
- The naming-collision warning holds: both excluded paths do flatten to the same archive name.
- The boundary obligation, all three claims, verified against the code, the policy file and the adapter index.
- The custom-transport citations verified in both revisions, headings and MUST lines.
- All 55 line citations land on a real heading or named constant; the new version-negotiation citation does specify the legacy handshake.
- Provenance against the network: all 54 recorded urls re-fetched and hashed, 54 match, 0 mismatch.
- The documented content check and its cross-revision control behave as the record says, refusing a cross-revision repoint with exit 5.
- Manifest internals: 54 entries, 54 archives, digests and byte lengths all re-derive.

```findings
- file: adapters/mcp/design.md
  line: 37
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the corrected sentence bounds the transport option space at three families and never
    names the fourth binding both pinned revisions specify, HTTP+SSE from 2024-11-05,
    which has its own section in the primary revision, a SHOULD-level fallback procedure
    in both, and a Deprecated-not-Removed row.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 60
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the new provenance program counts revision-and-commit pairs but never binds a revision
    to its own commit, so exchanging the two commits between the two revisions and
    rederiving every url exits 0 and prints one commit per revision, while 22 of the 54
    swapped urls still return HTTP 200 with bytes that are not the pinned ones.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 104
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the record states that the archive name is the upstream path below its revision
    directory flattened, and nothing checks that rule, so an entry repointed at another
    file of its own revision passes both documented checks and the recorded url stops
    identifying the archived bytes.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 139
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the corrected version-string paragraph calls the tasks and client-elicitation
    documents two more than the 18 that carry the banner, but both carry it, so summing
    the paragraph's own figures gives 24 documents out of a set of 22.
- file: crates/connectors-build/tests/mcp_specification_pin_adversary.rs
  line: 133
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the amended case does discriminate the pre-correction record, verified by re-running
    its oracle against that blob, but its body asserts only that the literal 18 of the
    appears somewhere, so a record with the denominator changed to 30, the four exceptions
    replaced by banner-carrying documents, or the primary-revision claim inverted is still
    accepted.
- file: crates/connectors-build/tests/mcp_specification_pin_adversary.rs
  line: 236
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the Rust content guard compares the archive count with the entry count rather than the
    two sets, so a manifest that records one archive twice and omits another passes it
    while the diff the record documents refuses the same manifest.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 119
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    nine of the forty numeric citations land on a heading the row's Section column does
    not name, so an author cannot tell what those lines were cited for.
```
