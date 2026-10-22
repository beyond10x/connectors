---
format: aep.planning-md/3
id: review-result:claude-epic-parallel-safety-r3-20261022
kind: review-result
status: active
title: 'Plan-critic parallel-safety, round 3: connectors with Claude'
summary: needs-revision, 1 blocker; addressed
relations:
- reviews: story:readme-names-mounted-custody
- reviews: story:docs-first-run-path
- reviews: story:mcp-stdio-runtime-branch
- reviews: story:standing-write-approval-for-unattended-agents
- reviews: story:mounted-custody-kind-secret-volume
- reviews: story:claude-getting-started-tutorial
revision: 1
---
# Plan-critic, parallel-safety lens, round 3: connectors with Claude

**Verdict: needs-revision. 5 findings, 1 blocker. All are addressed in the revisions of 2026-10-22 after round 3.**

| Story | Finding | Disposition |
|---|---|---|
| readme-names-mounted-custody (blocker) | edits `README.md:199-203`, inside the status block docs-first-run-path moves out | folded into story:docs-first-run-path, which names the replacement text and carries `the_readme_names_every_custody_binding`; the story is archived |
| docs-first-run-path ∥ definition-reader-feed-extensions | both on `status.rs`, `status.md`, `status.json` | docs-first-run-path's status block goes to `CHANGELOG.md` links; the three files are removed from its scope; the remaining shared file is `CHANGELOG.md` |
| mcp-stdio-runtime-branch, docs-first-run-path | both edit README | `depends_on story:docs-first-run-path` |
| standing-write | no scope, no edge: unassessed | scope recorded (9 paths); `depends_on` the runtime story and claude-connects |
| mounted-custody-kind-secret-volume, tutorial | no scope; ignored tests need `crates/connectors-build/src/ignored.rs` | scope recorded on both, `ignored.rs` included |
