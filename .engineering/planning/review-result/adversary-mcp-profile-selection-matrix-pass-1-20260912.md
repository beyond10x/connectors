---
format: aep.planning-md/1
id: review-result:adversary-mcp-profile-selection-matrix-pass-1-20260912
kind: review-result
status: active
title: Adversary pass 1 — MCP profile selection matrix, wave 3
relations:
- reviews: story:mcp-profile-selection-matrix
revision: 1
---
```
unit: mcp-w3 story:mcp-profile-selection-matrix, worktree cv2-mcp-w1-20260912 at 4ad2ee5 (1010ca8 + 4ad2ee5)
verdict: red
cases: executed 46→49, red 3
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 4 paths under ~/.cache/cv2-mcp-waves-20260912/adversary-w3/
needs-coordinator: yes
```

## 1. Diff

One untracked path, `crates/connectors-build/tests/mcp_profile_selection_matrix_adversary.rs`, 482 lines. No implementation file touched, no existing case altered, nothing deleted. Clippy clean.

## 2. Cases added — three, all red, each run alone before the suite

**Case 1 `every_protocol_version_the_archives_tell_a_peer_to_assume_is_dispositioned`.** The unit wrote both halves of the revision derivation — the prose rule naming four version-declaring positions, and the code implementing those same four — and nothing compares either against the acceptance, which says *every* revision the pinned specification names. This case reads a fifth position from the archived bytes: the version a peer is told to **assume** when a request declares none.

```
the pinned archives tell a peer to assume 1 protocol version(s) the matrix dispositions nowhere,
while it does disposition `2025-06-18`, which the same archives state only as a header example:
  2025-03-26 — stated at mcp-2025-11-25-basic-transports.mdx:278 "assume protocol version `2025-03-26`";
  mcp-2026-07-28-basic-transports-streamable-http.mdx:279 "as protocol version `2025-03-26`"
the matrix's revision keys are {"2024-11-05", "2025-06-18", "2025-11-25", "2026-07-28", "DRAFT-2025-v3"}
```

**Case 2 `every_deferred_row_rests_on_a_blocker_whose_own_record_names_that_feature`.** The document claims this standard for itself twice, and the unit's own check only verifies that a row naming a blocker defers — it never opens the blocker.

```
1 of 2 blocker-backed deferrals rest on a blocker that does not reach the deferred feature:
  selection.md:118 `transport:streamable-http` (inbound) is deferred against
  `decision-blocker:mcp-caller-connection-assignment`, but that record never names
  "streamable http" — the string appears 0 times in that file
```

**Case 3 `every_nested_setting_is_named_in_the_section_of_the_side_whose_schema_declares_it`.** The unit's check unions server and client settings into one flat set and asks only that each appears somewhere, so a path declared on both sides is satisfied for both by one mention on either.

```
2 of the 21 side-qualified nested settings are named outside the section that dispositions them:
  `tasks.cancel` is declared by a client capability of a pinned schema and is named nowhere
  in the matrix's `## Client capabilities` section
  `tasks.list` — same
```

Each oracle was verified to survive its own fix on mutated copies in scratch.

## 3. Suite

Before 46, after 49, red 3, exit 101. The unit's six cases stay green.

## 4. Findings

1. `selection.md:92` — the matrix carries no `revision:2025-03-26` row. Both pinned revisions state it normatively: "the server **SHOULD** assume protocol version `2025-03-26`" and "**MAY** treat a request that omits the header as protocol version `2025-03-26`". It is also the "Deprecated in" value of the deprecation registry and the revision that introduced Streamable HTTP. Meanwhile `revision:2025-06-18` does get a row on strictly weaker footing — its own cell says it "reaches this matrix only as a header example". Reaches: `story:mcp-outbound-connection-lifecycle` must specify what a version mismatch refuses, and a server naming `2025-03-26` has no disposition in its stated authority — the acceptance's own "silent approximation". CONFIRMED, introduced.

2. `selection.md:118` — inbound Streamable HTTP is deferred against `decision-blocker:mcp-caller-connection-assignment`. That record contains zero occurrences of "http" or "streamable", and its own "What this stops" names exactly one thing, the caller-isolation fixture, while saying the other half "is specifiable now and is drafted". The sibling blocker names its transport nine times and explicitly exempts one by name, which is why the other deferral passes. Reaches: `story:mcp-inbound-cloud-profile` is chartered to specify exactly this binding, and epic acceptance 3 requires it; a `deferred` in the single owner of transport selection takes both off the table on a question the blocker does not claim. NEEDS-CHANGE, introduced.

3. `selection.md:196` — `tasks.list` and `tasks.cancel` are declared by `ClientCapabilities` in a pinned schema and named only in the server paragraph. The unit's check passes because it unions the sides into eighteen bare paths. Reaches: a reader goes to `## Client capabilities` for what a client tasks capability contains and finds two settings missing, while the document asserts all eighteen are "named in the rows that carry them". CONFIRMED, introduced.

4. Judgement, no case. The document defines `deferred` as resting on an open `decision-blocker:` **or** an `UNMAPPED:` marker, and the check keys only on the first. Three `supported` rows name `UNMAPPED:` markers in their reasons. No case was written because the implementor's distinction — the marker governs which operations appear, not whether the capability is selected — is defensible, and asserting the opposite would be the adversary's rule rather than the document's. The gap is in the check. CONFIRMED, introduced.

## 5. Attacked and could not break

- Every citation in all 33 rows: 54 archives extracted, 30 cited lines read by hand. All resolve and all say what the reason cell says.
- The transport derivation: exactly three headings in one revision's transport document, one registry row, and no fifth family in the other.
- The capability derivation: only two capability interfaces exist in either schema and every field of both is optional, so the optional-field rule misses nothing.
- Every other version-shaped token: ten distinct date or DRAFT tokens enumerated across the 54 files; the rest are example timestamps, an earliest-removal date and a placeholder. `2025-03-26` is the only genuine revision omitted.
- Server completions refused rather than deferred: no blocker reaches it and no drafted story names it. Held.
- Inbound `2025-11-25` supported: the initiative does cover both directions in one sentence. Thin but supported. What it silently drags in is not — the primary revision's changelog removes the `initialize` handshake entirely, so inbound support for the interop revision obliges the inbound binding story to carry a stateful legacy handshake beside the stateless model, and neither the row nor that story says so. A missing sentence, not a wrong cell.

```findings
- file: adapters/mcp/contracts/protocol/v1alpha1/selection.md
  line: 92
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Both pinned revisions state normatively that a request without a protocol
    version header is 2025-03-26, and the matrix dispositions it nowhere while
    giving a row to 2025-06-18, which its own cell says reaches the matrix only as
    a header example.
- file: adapters/mcp/contracts/protocol/v1alpha1/selection.md
  line: 118
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    Inbound Streamable HTTP is deferred against a blocker whose record never names
    a transport at all and whose stated scope stops only the caller-isolation
    fixture while saying the cloud profile is specifiable now and is drafted, so
    the deferral withdraws that story and an epic acceptance criterion on a
    question the blocker does not claim.
- file: adapters/mcp/contracts/protocol/v1alpha1/selection.md
  line: 196
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Two task settings declared by the client capability interface of a pinned
    schema are named only in the server paragraph, and the nested-setting check
    passes because it unions the two sides into eighteen bare dotted paths instead
    of holding each to the section that dispositions it.
- file: crates/connectors-build/tests/mcp_profile_selection_matrix.rs
  line: 549
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The document defines deferred as resting on an open decision-blocker or on an
    UNMAPPED marker, and the check keys only on the first, so half of the
    document's own definition is enforced nowhere while three supported rows name
    UNMAPPED markers in their reasons.
```
