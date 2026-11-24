---
format: aep.planning-md/3
id: review-result:connections-through-owner-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: needs-revision

1. story:connections-list-answers-through-the-owner and story:entity-runtime-0330-records-name-definition-by-digest share `crates/connectors-build/generated/metadata-emissions/inputs.json`.
   - The sentence "the second of the two to merge regenerates on top of the first" is not enough. Two reasons:
     - The pin branch (`story/er-0330-pin`) already changes the `digest` line and the `entity-*` lines of `ess_crates` in that file. This story's `ess/domains/cli.yaml` edit changes the same `digest` line (cited: `metadata_emissions.rs:14` and `:52`, and the diff in worktree `er-0330-pin`). Whichever merges second gets a conflict on that line, and the line can only be resolved by regenerating.
     - Acceptance line 8 is "generated files current". That holds only on a base that already carries the pin, because the pin's `Cargo.lock` change is an input of the digest. This is a reader beside its producer.
   - The regeneration costs three pool jobs of 8 to 27 minutes each. If the pin merges while they run, the "branch must be up to date" ruleset makes this story redo them.
   - Missing edge: `depends_on story:entity-runtime-0330-records-name-definition-by-digest`, reason "regenerate inputs.json on a base that holds the pin". The edge can gate only acceptance 8 (the regeneration step); the Rust and ESS work can start before it.
   - Alternative: no edge, but the sentence is changed to name which one merges first. The pin is nearly done, so I would name the pin as first.
   - Replace the sentence in `## Shares files with` and add the relation.
   - Cited: `.engineering/planning/story/connections-list-answers-through-the-owner.md:139,146-150`.

2. story:entity-runtime-0330-records-name-definition-by-digest names no scope entry, so the planner treats it as unassessed, not safe.
   - Its real surface (the branch diff): `Cargo.toml`, `Cargo.lock`, `crates/connectors-build/generated/metadata-emissions/inputs.json`, `crates/connectors-host/src/local/registry/store_cost_tests.rs`, `crates/connectors-host/tests/fixtures/metadata-store-er-0.32.0/`, `CHANGELOG.md` and `docs/development.md`.
   - `aep plan artifact waves` cannot report the overlap with this story's directory entry `metadata-emissions` until the pin story lists `inputs.json` as a file entry.
   - Add that scope list to the pin story.
   - Cited: `aep plan artifact show story:entity-runtime-0330-records-name-definition-by-digest` (no `scope` line).

3. story:connections-list-answers-through-the-owner and the pin story both edit `CHANGELOG.md`, and the "Shares files with" section does not say so.
   - The pin branch adds 34 lines to `CHANGELOG.md`, and acceptance 7 here adds a Changed entry. Both go under the same unreleased heading.
   - Add `CHANGELOG.md` to the shared list. State that the second to merge rebases the entry.
   - Cited: `.engineering/planning/story/connections-list-answers-through-the-owner.md:146-150`.

4. story:connections-list-answers-through-the-owner and story:definition-files-in-user-config share four paths and are not named to each other.
   - Shared paths (inferred in both): `ess/domains/cli.yaml`, `contracts/cli/v1alpha1/semantics.md`, `website/docs/reference/contracts/cli.md`, and `apps/connectors/tests` against this story's `cold_owner_start.rs`.
   - Their `website/docs/reference/contracts/model/domains/connectors-cli.md` is a file entry under this story's directory entry `website/docs/reference/contracts/model`. `ess/domains/cli.yaml` and `semantics.md` are line-addressed in the other story, so they shift.
   - The same overlap holds with story:timeout-refusal-says-how-long-it-waited, which shares `cli.yaml`, `semantics.md` and `contracts/cli.md`. That story depends on `invoke-admission-queues-under-load`, not on this one.
   - The section says "ten other drafts" and names only one of them.
   - Name these two stories. Say that every story editing `ess/**` regenerates `inputs.json`, `operation-types` and `contracts/model`, so those stories do not run in one wave with this one. Or add an ordering edge per pair that records `ess/domains/cli.yaml` as the reason.
   - Cited: `.engineering/planning/story/connections-list-answers-through-the-owner.md:152-153`.

5. story:connections-list-answers-through-the-owner and story:guarded-write-credential-refusal-says-repair share `crates/connectors-host/src/local/owner.rs`, `contracts/cli/v1alpha1/semantics.md`, `website/docs/reference/contracts/cli.md` and `CHANGELOG.md`.
   - That story's scope marks `owner.rs`, `semantics.md` and `contracts/cli.md` as cited. They are inferred here.
   - This story's `owner.rs` regions are the wrapper at `:683-692` and the request enum at `:1411-1413`. The other story's regions are unknown.
   - The section names the story for `transport.rs` and none for `owner.rs`.
   - Add this story to the `owner.rs` / `semantics.md` sharers, or state that the regions differ.

6. story:test-binary-per-package is active, names no scope, and claims `apps/connectors/tests` once it reaches that package.
   - Acceptances 1 to 4 and 9 here cite the path `apps/connectors/tests/cold_owner_start.rs`. After the move that path is a module of `tests/main.rs`.
   - The body admits the collision ("has not reached this package") but fixes nothing.
   - Add scope entries to test-binary-per-package. In acceptance 1, name the file "or its module in `apps/connectors/tests/main.rs`". Or add an ordering edge between the two.
   - Cited: `.engineering/planning/story/connections-list-answers-through-the-owner.md:152-153` and the test-binary-per-package body ("The other packages … still build one binary per test file").

7. Stories whose surface the store does not state, so they are unassessed.
   - The named `transport.rs` sharers are `owner-refuses-buildless-callers`, `owner-counters-are-panic-safe`, `host-gate-load-sensitivity-w4`, `write-reply-waits-out-the-owners-delivery-grace` and `served-provider-memory-growth-under-repeated-syncs`. None has a scope.
   - Also without scope: `test-children-die-with-their-parent` (active, and likely on the owner start path near `transport.rs:1681-1694`), `windows-host-job-releases-the-cli-binary`, `mcp-stdio-on-macos-and-windows`, `guide-google-and-hubspot-for-business-teams` and `sustained-live-read-loop`.
   - The section's "all draft or unassigned" presents the five named stories as a list to avoid. It does not say that their regions are unchecked.
   - Reword the section to say these are unassessed. Add scope entries to the five `transport.rs` stories, or a sentence in this story that the region in `transport.rs` (the new `action` arm and client) must be disjoint from theirs.

Not findings:
- story:invoke-admission-queues-under-load is still active in this store and shares nearly every path of this story: `ess/domains/cli.yaml`, `metadata-emissions`, `operation-types`, `owner.rs`, `transport.rs`, `owner.md`, `semantics.md` and the model pages. Its code is already in the base (`5983a6b9502`, "0.75.0: invoke --deadline-seconds (#283)"). The branch `planning/v0750-verified` marks it implemented.
- Nothing is live between the two. The store here is stale, so close that story or merge that planning branch before the next `waves` run.
- story:windows-host-job-releases-the-cli-binary (PR 284) touches only `.github/workflows/windows-host.yml` and `macos-host.yml`. I found no shared path with this story.

What I read: 13 artifacts via `aep plan artifact show`, `list`, `waves` and `graph`. I also read the pin branch diff in `er-0330-pin`, `inputs.json` and `metadata_emissions.rs`, and the git history of `invoke-admission`.
- Surfaces: 1 cited (`connections.rs`), 11 inferred, 1 with no scope (the pin story, placed only from the branch diff).
- Acceptance reads traced to a producer: 1 (line 8 reads the pin's `inputs.json` change). `depends_on` edges recording it: 0.

Not established:
- The regions of `owner.rs` and `transport.rs` in the other stories, because they have no scope.
- Whether `ess_crates` alone changes the contents of the three emission parts or only the digest. The pin diff shows no part changed, so the pin's cost is probably a digest refresh, but I could not run `--check`.
- The semantic fit of the stories (design lane) and the acceptance wording (acceptance lane).

```findings
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 150
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "inputs.json digest line conflicts with story:entity-runtime-0330-records-name-definition-by-digest (the pin changes Cargo.lock/ess_crates, this story changes ess/domains/cli.yaml) and acceptance 8 reads the pin's output; the 'second to merge regenerates' sentence names no order, so add depends_on story:entity-runtime-0330-records-name-definition-by-digest (or name the pin as first to merge) to avoid redoing three 8 to 27 minute pool jobs"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 1
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "no scope entries, so it is unassessed and waves cannot see its overlap with this story on metadata-emissions/inputs.json and CHANGELOG.md; add scope Cargo.toml, Cargo.lock, crates/connectors-build/generated/metadata-emissions/inputs.json, store_cost_tests.rs, the new fixture directory, CHANGELOG.md, docs/development.md"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 148
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "CHANGELOG.md is edited by both this story (acceptance 7) and story:entity-runtime-0330-records-name-definition-by-digest (34 lines on its branch) and is absent from Shares files with; add it with the rule that the second to merge rebases its entry"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 152
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "ess/domains/cli.yaml, semantics.md, contracts/cli.md and the model directory are shared with story:definition-files-in-user-config (file entry model/domains/connectors-cli.md under this story's model directory entry) and story:timeout-refusal-says-how-long-it-waited, but only 'ten other drafts' is written; name them and record an ordering edge or the rule that ESS-editing stories do not share a wave"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 146
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "owner.rs, semantics.md, contracts/cli.md and CHANGELOG.md are shared with story:guarded-write-credential-refusal-says-repair (cited there, inferred here) and not named; add it, or state that the edited regions of owner.rs differ"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 153
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:test-binary-per-package (active, no scope) will move apps/connectors/tests/cold_owner_start.rs into tests/main.rs, which acceptance 1 to 4 and 9 cite by path; name the file 'or its module in tests/main.rs' or add an ordering edge, and add scope to test-binary-per-package"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 151
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the five transport.rs sharers and story:test-children-die-with-their-parent have no scope entries, so they are unassessed rather than safe; say so, add scope entries to them, or state that this story's transport.rs regions are disjoint from theirs"
```
