---
format: aep.planning-md/1
id: review-result:adversary-mcp-profile-selection-matrix-pass-2-20260912
kind: review-result
status: active
title: Adversary pass 2 — MCP profile selection matrix, wave 3
relations:
- reviews: story:mcp-profile-selection-matrix
revision: 1
---
```
unit: mcp-w3 story:mcp-profile-selection-matrix, worktree cv2-mcp-w1-20260912 at cc7b121
verdict: red
cases: executed 52→54, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 11 paths under ~/.cache/cv2-mcp-waves-20260912/adversary-w3-pass2/
needs-coordinator: yes
```

## 1. Diff

No tracked file changed. One untracked path, `crates/connectors-build/tests/mcp_profile_selection_matrix_adversary_pass2.rs`, 420 lines. Clippy exit 0; `ess-boundary` with the pinned binary reports 7 adapter models, exit 0.

## 2. Cases added — two, both red, each run alone first

**Case 1** derives extension identifiers structurally, in the shape the matrix uses for its own three rules — the sentence form `[<Name> extension](<link>) identified as \`<identifier>\``.

```
the pinned revisions identify 2 optional extensions and the matrix names 1 of them:
  identified by the archives:
    MCP Apps `io.modelcontextprotocol/ui` at mcp-2026-07-28-basic-versioning.mdx:90
    Tasks `io.modelcontextprotocol/tasks` at mcp-2026-07-28-basic-versioning.mdx:105
  named by no line of the matrix:
    MCP Apps `io.modelcontextprotocol/ui`
```

The client row cites `:89-103` as its source. That range **is** the MCP Apps example; the word "tasks" does not occur in it. `io.modelcontextprotocol` appears 0 times in `selection.md`, "MCP Apps" 0 times, and 0 times in the adapter design or any `story:mcp-*`.

**Case 2**

```
1 inbound refusal of a request that omits the MCP-Protocol-Version header reaches the
legacy initialize the same matrix supports, and excepts it nowhere:
  selection.md:156 revision:2025-03-26 (inbound) — rejected with 400 Bad Request and
    HeaderMismatch -32020, never silently assumed into an unpinned revision
  selection.md:152 revision:2025-11-25 (inbound) — supported: the inbound binding answers
    a legacy initialize with a version it supports
  mcp-2026-07-28-basic-versioning.mdx:36 — 2025-11-25 and earlier establish a session with
    an initialize handshake
  mcp-2025-11-25-basic-transports.mdx:267 — that revision requires the header on "all
    subsequent" requests, so its initialize carries none
```

Case 2's liveness guard was corrected once after its first red run — the original conjunction would have failed on a valid fix rather than passing — and the quoted output is the re-run with the assertion unchanged.

Both oracles were checked against their own fixes on mutated copies in scratch; the tree was not mutated. Case 1 stays red on "delete the word only" and goes green on naming MCP Apps as unselected. Case 2 stays red on a reworded but unqualified refusal, and goes green either by qualifying it or by withdrawing inbound legacy support.

## 3. Suite

Before 52, after 54, red 2, exit 101. The unit's nine cases and pass 1's three stay green. `--no-fail-fast` is needed for the after-run; without it cargo stops at the adversary target and under-counts by 8.

## 4. Findings

**1. `selection.md:265` and `:237` — the two `extensions` deferrals rest on an enumeration of one, and the archives name two.** The archives identify `io.modelcontextprotocol/tasks` and `io.modelcontextprotocol/ui` by the same sentence form, 15 lines apart, and the client row's own cited range is the `ui` example. Reaches: the epic's acceptance 4 requires a matrix over optional capabilities and the archives call extensions exactly that. A later unit resolving the `UNMAPPED:` marker answers the tasks question and reads both rows as cleared, while `ui` — server-supplied inline UI — has never been dispositioned in either direction. CONFIRMED, introduced, blocker.

**2. `selection.md:156` — the inbound header refusal refuses the legacy `initialize` the row four lines above supports.** `2025-11-25` inbound is supported on "answers a legacy initialize"; the archives define legacy as the revisions that establish a session with that handshake, and that revision requires the header only on *subsequent* requests, with the version travelling in the `initialize` body. So the handshake's first POST carries no header and the `2025-03-26` inbound row refuses it unqualified. The cited source does not authorise the breadth: it scopes reject-or-assume to clients earlier than `2025-06-18`, which `2025-11-25` is not. Reaches: Streamable HTTP inbound is supported in the same matrix, so the handshake arrives over a selected transport; a binding written to this matrix rejects the first request and never performs the handshake the other row obliges — on the exact seam milestone 5 exists for. NEEDS-CHANGE, introduced, blocker.

**3. `selection.md:238` — `capability:server/tasks` defers against a marker that poses only the client-side question, and passes both halves of the new enforcement.** The quoted marker is attached to the client capability kind; the check strips the side and matches the stem, so the server rows pass on a client-side marker. The interop schema declares the server capability with *different children*, which the matrix's own paragraph says out loud, and the model's server capability kind omits it with no marker of its own. Reaches: whoever answers the marker settles the client row and leaves the server one where it was, while both read as cleared. Same shape as pass 1's side-blind union, one level up. No case written — asserting side-matching is a rule the unit's check deliberately declines, and the marker text belongs to another story. CONFIRMED, introduced, warning.

## 5. Attacked and could not break

- **The exhaustive revision scan.** Ten distinct tokens enumerated independently across all 54 archives with their counts; all four exclusions apply to every occurrence and to nothing else. Six revisions, six row keys, exact match.
- **The transport derivation, the unit's own declared soft spot.** Three headings in the interop transports document, one deprecation row, the same three families and no fourth in the primary revision. Four is right for this pin. The class hole is real — the scan reads only one revision's headings — but it could be constructed only by mutating an archive the digests fix, and no caller reaches that state, so it is not raised.
- **The capability derivation's premises.** Two interfaces per schema, all fields optional, 8 server and 6 client keys and 21 nested settings — exactly the document's counts.
- **Every error code a refusal cell names** resolves in the primary schema, and the code in finding 2's cell is right; only its scope is wrong.
- **The 18 refusal rows**, counted independently: 6 revision, 4 transport, 3 server, 5 client. Each states an outcome a caller observes. The rule the unit says is held by review is, on this pin, held.
- Pass 1's three cases and the unit's nine: all twelve green, none altered.

```findings
- file: adapters/mcp/contracts/protocol/v1alpha1/selection.md
  line: 265
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    Both extensions rows are deferred on the stated ground that the tasks extension is the
    only instance either pinned revision names, while the archives identify two by the same
    sentence form fifteen lines apart and the client row cites the other one's lines as its
    own source, so the epic's optional-capability coverage has a member nothing
    dispositions.
- file: adapters/mcp/contracts/protocol/v1alpha1/selection.md
  line: 156
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    The inbound 2025-03-26 row refuses every request that omits the protocol version header,
    which is exactly the legacy initialize that the supported inbound 2025-11-25 row four
    lines above promises to answer over a transport this matrix also selects, and the cited
    source scopes reject-or-assume to clients earlier than 2025-06-18.
- file: adapters/mcp/contracts/protocol/v1alpha1/selection.md
  line: 238
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    The server tasks row defers against a marker whose own text poses only the client-side
    modelling question, and passes the names-the-feature rule because that rule strips the
    side and matches the stem, so answering the marker would read as clearing a server
    capability whose interop-schema children differ and which the model omits without a
    marker.
```
