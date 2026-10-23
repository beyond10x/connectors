---
format: aep.planning-md/3
id: review-result:w1022-u2-adversary-r2
kind: review-result
status: active
title: 'Adversary pass 2, wave 20261022 U2: first-run docs path'
summary: 'break: 2 warnings, 1 note; all addressed in 558bf99b8'
relations:
- reviews: story:docs-first-run-path
revision: 1
---
Adversary pass 2 on the U2 rework. **Verdict: break.** 2 findings and 1 note; 1 case added, 1 red.

| severity | path | finding | disposition |
|---|---|---|---|
| warning | crates/connectors-build/src/docs.rs `default_state_directory` | The exact-literal exemption checks only the next byte, so rendered markup (`</code>/owner.sock`, the JS-bundle form, `\/`) extends the admitted path. | The audit admits `~/.local/state/connectors` and any path under it. A `~/`-rooted path names no user or repository; every other `.local/` is still refused. |
| warning | website/docs/connectors/zendesk.md:125 | `$subject_sha256` is never set. | Set from `result.preparation.subject_sha256`, as drive.md and gmail.md do. |
| note | website/docs/connectors/mcp.md | The clearance date is local time. | Use the UTC date, 2026-10-06. |

Held: the Zendesk walkthrough against the provider and approval code; the README's custody, writes and blocker lines; the getting-started archive layout against v0.56.0; the default paths; the `auth_library` row; the `semantics.md` link.
