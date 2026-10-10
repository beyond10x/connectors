---
format: aep.planning-md/3
id: story:parity-gitlab-mr-writes
kind: story
status: active
title: GitLab merge-request notes, discussions, auto-merge and reopen
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-09T22:50:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-09T22:50:37Z", actor: "human:timo", revision: 3}
---

## Wave 20261010a (2026-10-10)

- Delivered: `merge_request.note.create`, `merge_request.discussion.reply`, `merge_request.discussion.get` and the guarded `merge_request.discussion.resolve`; note, reply and resolve covered. Two adversary passes (`tests/adversary_gitlab_mr_writes.rs`, `tests/adversary_catalog_body_types.rs`); their fixes added `body_types` and `body_required` to the selection format.
- Not delivered: merge when the pipeline succeeds and reopen need a body member fixed to one value. Waits for `story:catalog-selection-fixed-body-value`; `gitlab.mr.merge` and `gitlab.mr.update` stay partial.
- The story stays active until both are selected.
