---
format: aep.planning-md/1
id: review-result:adversary-mcp-specification-pin-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — MCP specification pin, wave 1
relations:
- reviews: story:mcp-specification-pin
revision: 1
---
```
unit: mcp-w1 — story:mcp-specification-pin, worktree cv2-mcp-w1-20260912, commit 4192883 plus one untracked test file
verdict: red
cases: executed 20→24, red 2
origin: introduced 6, pre-existing 2, undecided 0
wrote-outside-worktree: /home/timo/.cache/cv2-mcp-waves-20260912/adversary-w1/ (28 entries, 3.8M)
needs-coordinator: yes
```

## 1. Diff

One added path, `crates/connectors-build/tests/mcp_specification_pin_adversary.rs`. No implementation file edited. `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings` both exit 0 on it, so the red is the assertions and not a lint.

## 2. Cases added

| Test | Asserts | Now |
|---|---|---|
| `interop_revision_names_its_own_version_string_outside_a_documentation_path` | the record's claim at `specification-sources.md:99` holds against the bytes it pins | RED |
| `version_negotiation_row_cites_the_interop_revisions_negotiation_section` | the citation row at `specification-sources.md:81` cites the section it names | RED |
| `every_recorded_digest_and_byte_length_rederives_from_the_archived_bytes` | executes the record's own reproduction snippet | green |
| `manifest_provenance_fields_agree_with_the_revision_each_entry_claims` | url derives from commit and path; path lies under the revision it claims | green |

Red run of case 1, captured when written: `specification-sources.md:99 states that within the 2025-11-25 revision the string 2025-11-25 appears only in documentation paths, but 18 archived documents of that revision declare it in prose`, then 18 lines each reading `<Info>**Protocol Revision**: 2025-11-25</Info>`. `test result: FAILED. 0 passed; 1 failed; 3 filtered out`, EXIT=101.

Red run of case 2: `specification-sources.md:81 names version negotiation for the 2025-11-25 seam but cites mcp-2025-11-25-basic-lifecycle.mdx lines [38, 248, 265]; that revision specifies version negotiation at line 167, which the record cites nowhere`. EXIT=101.

## 3. The suite

`cargo test -p connectors-build --offline --locked`: bin target 20 passed; adversary target 2 passed, 2 failed; exit 101. With the adversary target deselected, 20 passed, exit 0 — that is the before count.

Boundary gate run with the pinned ESS selected: `gate: shared and 6 adapter ESS models validated and compiled independently; exit=0`. Not vacuous for this unit — see finding 7.

## 4. Findings

1. `specification-sources.md:99` — the record states the 2025-11-25 revision names its own version string only in documentation paths. 18 of its own archived documents open with a Protocol Revision line declaring it in prose; also the tasks document at line 11 and the client-elicitation document at line 329. Reaches: the paragraph is written for the eleven stories that read this pin, and the commit message names it as a trap they must not walk into. CONFIRMED, introduced.
2. `specification-sources.md:81` — the version-negotiation citation row cites lines 38, 248 and 265, which are Lifecycle Phases, Timeouts and Error Handling. That revision specifies version negotiation at line 167 and the record cites it nowhere. Reaches: the acceptance makes this table the thing later contracts cite, and the interop seam is the legacy initialize handshake, specified only in that section. CONFIRMED, introduced.
3. `specification-sources.md:60` — the exclusion list has four patterns; upstream trees at both pinned commits hold one blob per revision matching none of them, at 1,771 and 2,316 bytes. Both are typedoc heading templates with no normative text, fetched and read, so nothing normative is lost. Reaches: a reader asking whether a file is pinned or deliberately excluded gets no answer. A name collision is the likely cause, since both flatten to the same archive name. CONFIRMED, introduced.
4. `specification-sources.md:40` — the documented verification reads only the digest, the byte length and the archive. On a scratch copy, repointing one entry's path, commit and url at the other revision's file still printed the success line and exited 0. Reaches: the record states the archives rather than the network are the retained authority from here on, so nothing can ever detect a wrong URL. CONFIRMED, introduced.
5. `crates/connectors-build/src/gate.rs:4` — no step in the gate and no code under `crates/` reads any evidence manifest, for kubernetes, loki, docker or atlassian either. Reaches: a later unit editing an archive or a digit of a digest goes undetected. CONFIRMED, pre-existing.
6. `adapters/mcp/design.md:36` — the document says stdio and Streamable HTTP are the transports the pinned revisions define. Both revisions also define Custom Transports with MUST-level requirements. Reaches: the profile selection matrix reads this sentence to bound its option space. CONFIRMED, introduced.
7. `crates/connectors-build/src/ess_boundary.rs:299` — creating the adapter directory adds its name to the shared-ESS forbidden vocabulary implicitly; owner words go into the term set unless the name is listed as a shared protocol name, and only `sip` is. Shared ESS is clean today so the gate exits 0. Reaches: any later story in this epic needing shared vocabulary to name MCP is refused by the gate, and it will look like that story's own bug. CONFIRMED, introduced.
8. The story cites initiative line 35 for a milestone at line 33. The unit's own record and design document say 33 and are right. CONFIRMED, pre-existing.

## 5. Attacked and could not break

- All 54 digests and byte lengths re-derive from the archives; byte lengths are of the uncompressed content.
- Provenance holds, 54 of 54. Every recorded URL was re-fetched and compared against the archived bytes at the recorded commit. A first run reporting 54 one-byte mismatches was the harness stripping a trailing newline, not a defect.
- Tag to commit mapping checked independently through the refs API.
- Every normative document under both revision directories is archived, and nothing archived is absent upstream.
- All 55 line citations land on a real section heading of the file they name; none is out of range.
- Every version-string trap the unit documented is true, including the exact byte figures for the excluded renderings.
- The authority claim holds, at the sibling library's lines 15 and 17.
- One row added to the adapter index and nothing else; all three document link targets resolve.
- All 54 archives carry zeroed mtime and no filename flag.
- 54 unique files, archives and digests; the flattened-name derivation rule holds for all 54.
- The design document declares no entity and states no relation.

```findings
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 99
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the record states that the 2025-11-25 revision names its own version string
    only in documentation paths, while 18 of the archived documents it pins
    declare a Protocol Revision line carrying that string in prose.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 81
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the citation row for the 2025-11-25 version-negotiation seam cites Lifecycle
    Phases, Timeouts and Error Handling and never cites that revision's own
    Version Negotiation section at line 167, which is where the legacy initialize
    handshake is specified.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 60
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    a schema rendering exists for each revision at both pinned commits and
    matches none of the four documented exclusions, so the not-archived account
    is incomplete, though both files are typedoc templates carrying no normative
    text.
- file: adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md
  line: 40
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the documented verification reads only the digest, the byte length and the
    archive, so an entry repointed at another revision's path, commit and url
    still prints the success line and exits 0.
- file: crates/connectors-build/src/gate.rs
  line: 4
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    no step in the gate and no code under crates reads any evidence hash manifest
    for any adapter, so an altered archive or recorded digest cannot turn the
    suite red.
- file: adapters/mcp/design.md
  line: 36
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the design document says stdio and Streamable HTTP are the transports the
    pinned revisions define, while both revisions also define Custom Transports
    with MUST-level requirements.
- file: crates/connectors-build/src/ess_boundary.rs
  line: 299
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    creating the MCP adapter directory silently makes its name a forbidden
    shared-ESS term, as sip needed an explicit shared-protocol-name entry to
    avoid, and no document in this unit records that obligation for the later
    stories.
- file: .engineering/planning/story/mcp-specification-pin.md
  line: 55
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: >-
    the story cited initiative line 35 for the MCP pin milestone, which is line
    33 of that artifact; the unit's own documents cite 33 and are correct.
```
