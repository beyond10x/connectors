---
format: aep.planning-md/3
id: review-result:connections-through-owner-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: needs-revision

Round 1's seven findings are resolved. I found two new overlaps the story does not state, both warnings, each fixed by one line in "Shares files with". Neither blocks working the story beside the others.

**Round 1 findings**
1. Resolved. The story's scope no longer holds any ESS or generated file, nor `metadata-emissions/inputs.json`. `story:owner-observe-request-is-modelled-in-ess` is drafted with `depends_on` to this story, and its scope holds `ess/domains/cli.yaml`, the generated directories and `owner.md`. That edge records the `owner.md` ordering, so the one reader beside its producer is covered. Acceptance 8 and 10 match the new scope.
2. Resolved, by reading the pin's own branch. In the `er-0330-pin` worktree the pin story has 14 scope entries, `inputs.json` is changed in its diff, and `CHANGELOG.md` is `cited`. It is not in the main-based store, which is why round 1 saw none.
3. Resolved (`connections-list-answers-through-the-owner.md:196`). `CHANGELOG.md` is named with the pin story and the other two, with the rule that the second to merge rebases its entry.
4. and 5. Resolved. `definition-files-in-user-config`, `timeout-refusal-says-how-long-it-waited` and `guarded-write-credential-refusal-says-repair` are named, with this story's regions. I checked the regions against their bodies and they are disjoint:
   - `guarded-write` edits `owner.rs:263-313` and `semantics.md:1229-1240`; this story edits `owner.rs` near `:683` and `:1411` and `semantics.md:1378-1380`.
   - `definition-files` edits `semantics.md:432` and `:485`.
6. Resolved. Acceptance 1 now says "or its module in `apps/connectors/tests/main.rs`".
7. Resolved. The `transport.rs` sharers and `story:test-children-die-with-their-parent` are stated as unassessed, with this story's three regions in `transport.rs`.

**New findings**
- `host-gate-load-sensitivity-w4` (draft, no scope) cites `owner/admission_tests.rs:84, :105, :133, :248, :293` in its body. This revision added `admission_tests.rs` to this story's scope, and the story names `w4` only for `transport.rs`. The overlap is cited, but only from `w4`'s body.
- The pin story's scope lists `apps/connectors/tests/cold_owner_start.rs` (inferred), which acceptance 1 to 4 and 6 of this story edit. The story names the pin only for `CHANGELOG.md`. The pin's branch diff does not edit that file; its acceptance 8 only runs `binary(cold_owner_start)`, but it says any other file those tests live in that changes is named in its report. This one rests on an inferred scope entry.

`story:invoke-admission-queues-under-load` is not a finding. Its store status is `active` and its scope holds `owner.md`, `owner.rs`, `transport.rs` and `semantics.md`. But base 5983a6b9502 is its merge ("0.75.0: invoke --deadline-seconds (#283)"), and `--deadline-seconds` is already in `semantics.md` at 2,334 lines.

**What I read:** 16 artifacts via `aep plan artifact show`, `list`, `graph` and `waves`, plus the story file, the `er-0330-pin` branch and the base commit. I also scanned every draft or active story's scope for this story's paths.
- Surfaces: this story 1 cited and 8 inferred. In the set: 11 placed (cited or inferred), 4 unplaced (`w4`, `owner-refuses-buildless-callers`, `owner-counters-are-panic-safe`, `served-provider-memory-growth-under-repeated-syncs`; `test-children-die-with-their-parent` and `test-binary-per-package` also have no scope but were not counted).
- Acceptance reads traced to a producer in the set: 1 (the ESS story's `owner.md` read), and 1 recorded by a `depends_on` edge.

**Not established**
- The regions of the `transport.rs` and `owner.rs` sharers that have no scope.
- Whether the pin story edits `cold_owner_start.rs`.
- `admission-checks-token-scopes` (draft, `apps/connectors/src/local` as a directory, inferred) and `mcp-outbound-stdio-runtime` (draft, `crates/connectors-host/src/local` as a directory, inferred) are directory-wide scopes I could not narrow. I did not raise them.
- Out of my lane, for the acceptance critic: acceptance 8's literal "2,334 lines" is a count against the base. It fails after any of the three `semantics.md` sharers merges first, unless it is read as "unchanged from the base".

```findings
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 205
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:host-gate-load-sensitivity-w4 (draft, no scope) cites owner/admission_tests.rs at :84, :105, :133, :248, :293 in its body, and this revision added admission_tests.rs to this story's scope (acceptance 5), but the Shares files with section names w4 only for transport.rs; add admission_tests.rs to the w4 entry as unassessed, with this story's region (a new test beside the fixture at :140-183 and answer_once at :191)"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 196
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:entity-runtime-0330-records-name-definition-by-digest (branch story/er-0330-pin) lists apps/connectors/tests/cold_owner_start.rs in its scope (inferred; its acceptance 8 only runs binary(cold_owner_start), its branch diff does not edit it), and this story edits that file in acceptance 1 to 4 and 6, but the pin is named only for CHANGELOG.md; add cold_owner_start.rs to the pin's entry and say that the pin runs it and does not edit it (inferred surface)"
```
