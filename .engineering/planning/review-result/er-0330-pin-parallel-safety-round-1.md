---
format: aep.planning-md/3
id: review-result:er-0330-pin-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

1. story:entity-runtime-0330-records-name-definition-by-digest — `story:spec-models-operation-admission` (draft) lists `crates/connectors-host/src/local/metadata/er.rs`, `crates/connectors-build/src/metadata_entities.rs`, `entity-runtime-definitions.json` and `ess/domains/declarations.yaml` in its scope (cited, spec-models-operation-admission.md:13-21). Three of those collide with this story:
   - `er.rs` is in both scopes (cited here, :19), and the "Shares files with" section does not name it.
   - `metadata_entities.rs` is one of three `SOURCES` of the emissions digest (`metadata_emissions.rs:41-45`).
   - `ess/**` is the other digest input (`metadata_emissions.rs:84`), so that story's `declarations.yaml` edit also invalidates `inputs.json`.
   Accepted change: name the story in "Shares files with" (`:130`) with the shared file, and add an ordering edge or split the surface. I do not choose which.
   Citation: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md:130

2. story:entity-runtime-0330-records-name-definition-by-digest — the section names `ess/**` and `Cargo.lock` as generic classes, but draft stories hold those files by name and are left out:
   - `ess/**`: `admission-checks-token-scopes` (scope `ess`), `definition-files-in-user-config` (`ess/domains/cli.yaml`), `standing-write-approval-for-unattended-agents` (`ess/domains/*.yaml`, `ess/system.yaml`).
   - `Cargo.lock`: `definitions-pin-covers-catalog` (also `docs/development.md` and `gate.rs`, and its :114 says it collides with any unit on those), `google-consent-on-the-oauth2-core`, `mcp-outbound-stdio-runtime` (also `Cargo.toml`).
   - `docs/development.md`: `claude-getting-started-tutorial` (cited, along with `crates/connectors-docs/src/development.rs`). The section names only `sustained-live-read-loop` for this file.
   - `CHANGELOG.md`: `agent-tool-projection`, `catalog-zendesk-reads`, `guarded-write-credential-refusal-says-repair`, `mounted-custody-kind-secret-volume`, `zendesk-oauth-ticket-writes`, `claude-getting-started-tutorial` and `standing-write-approval-for-unattended-agents`. The story names none of them. This is a text conflict at merge time, not a digest one.
   Accepted change: list those ids per file, or state that the edit to `CHANGELOG.md` is a single line added at the release.
   Citation: :130 against scopes in definitions-pin-covers-catalog.md:21,39; claude-getting-started-tutorial.md (scope); standing-write-approval-for-unattended-agents.md (scope)

3. story:entity-runtime-0330-records-name-definition-by-digest — `story:read-cost-test-reports-bytes-per-read` (draft) also adds a printed bytes-per-invoke figure to `read_invoke_cost_by_store_size` (read-cost-test-reports-bytes-per-read.md:23-28). It sets its bound "on the number measured on the commit that adds it", so its bound goes stale if the pin lands after it. Neither story has a `depends_on` edge, so `aep plan artifact waves` puts them in one wave. The body says "not worked beside" in prose only. `story:test-binary-per-package` (active) is the same: it moves `metadata_store_previous_pin.rs` per this story's own text, and its body cites no path, so it is unassessed.
   Accepted change: add `depends_on` or `blocks` edges that give the shared file as the reason, for `story:read-cost-test-reports-bytes-per-read` and `story:test-binary-per-package`.
   Citation: :102 and :130; read-cost-test-reports-bytes-per-read.md:23-28; test-binary-per-package.md:11-19 (no scope, no path in the body for connectors-host)

4. story:entity-runtime-0330-records-name-definition-by-digest — the scope omits files that acceptance 7 admits may change, and lists one that acceptance 7 omits. Six of the files in acceptance 7 walk `connectors_er_blobs` (git grep): `checkpoint_tests.rs`, `binding_scope_security_tests.rs`, `metadata_reopen_adversary.rs`, `cold_owner_start.rs`, `cold_owner_start_security.rs` and `store_cost_tests.rs`. Of these only `store_cost_tests.rs` is in scope. `registry/tests.rs` is in scope (inferred, :23) but not in acceptance 7. `apps/connectors/tests/setup_checkpoints_enable.rs:48` reads the 0.26.0 fixture and is in neither list. No other non-terminal story names these files today, so the risk is low. A later story that does name one of them would collide with this story unseen.
   Accepted change: add `registry/tests.rs` to acceptance 7. Put every acceptance-7 file in scope as "may change", or state that none will change.
   Citation: :111-118 against :10-29

5. Unassessed stories (no scope and no path in the body, so not safe by default): `invoke-admission-queues-under-load` (its PR #283 already regenerated `inputs.json`, so it touched an emissions input its body never names), `invoke-exits-promptly-on-sigterm`, `owner-cancel-timing-test-under-load`, `settled-read-uses-leave-the-store`, and `test-binary-per-package` (see 3). 155 of 183 non-terminal stories carry no scope. This story's "Shares files with" cannot say these are clear. Accepted change: a sentence in the section that these five are unassessed. This is not a defect of the story's own text.

Pin effects checked, no finding:
- The digest at `metadata_emissions.rs:63` takes every `entity-*` line, so `entity-query` 0.32.0 moves with the lock. The story says so.
- `inputs.json:5-9` is the only non-lock, non-manifest `0.32.0` entity string.
- `metadata_store_previous_pin.rs:33` and `setup_checkpoints_enable.rs:48` use the 0.26.0 fixture, and v0.74.2 pins 0.32.0 (`git show v0.74.2:Cargo.toml:24`).

Counts: 9 stories placed by cited scope or body path, 0 inferred, 5 unplaced. 3 acceptance reads traced to a producer in the set (acceptance 4, the read-cost output; acceptance 2, the `ess` digest; acceptance 3, the v0.74.2 fixture). 1 of them is recorded by a `depends_on` edge, `settled-read-uses-leave-the-store`, as the reverse direction. Out of lane: acceptance 2 lists `metadata-entities --check` and `cli --check`, which the pin does not change (acceptance critic). `LEVEL` stays 9, so `ess/domains/cli.yaml` is not touched.

```findings
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 130
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:spec-models-operation-admission shares er.rs and, through metadata_entities.rs and ess/domains/declarations.yaml, the emissions digest inputs; the Shares files with section does not name it"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 130
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "ess/**, Cargo.lock, docs/development.md and CHANGELOG.md are named as classes while draft stories hold them by name (definitions-pin-covers-catalog, claude-getting-started-tutorial, admission-checks-token-scopes, definition-files-in-user-config, standing-write-approval-for-unattended-agents, others); list the ids"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 102
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "read-cost-test-reports-bytes-per-read adds the same figure to the same test and test-binary-per-package moves metadata_store_previous_pin.rs, with no depends_on or blocks edge recording either shared file"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 111
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope omits five files acceptance 7 allows to change and lists registry/tests.rs, which acceptance 7 omits; align the two lists"
- file: .engineering/planning/story/entity-runtime-0330-records-name-definition-by-digest.md
  line: 130
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "invoke-admission-queues-under-load, invoke-exits-promptly-on-sigterm, owner-cancel-timing-test-under-load, settled-read-uses-leave-the-store and test-binary-per-package name no surface and are unassessed, not clear"
```
