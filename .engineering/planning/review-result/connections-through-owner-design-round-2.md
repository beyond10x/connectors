---
format: aep.planning-md/3
id: review-result:connections-through-owner-design-round-2
kind: review-result
status: active
title: Design critic, round 2, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: approve

All six round-1 findings are resolved. I found no new finding. The Design can be built as written, and no two items own one surface. I ran `validate`, which printed `valid` (it also lists old unrecorded review-results of other stories, which are not mine). I did not build or run tests.

1. **Document builder: resolved.**
   - The Design now has the owner return the registry's observation as data. The CLI keeps `summary`, `description` and the acquisition code, and `connection_value` is untouched.
   - `ObservedConnection` and `Page` (`registry/observation.rs:30-55`) are plain owned fields, so they can be sent as data. `state` is a `State` enum, not a `&'static str`.
   - Acceptance 2 names a pending connection, one without identity and one with a binding, with a host test as the stated fallback.
2. **Refusals: resolved.**
   - A refusal by the owner now falls back to the direct read, which prints its own code and stage. That matches `owner.md:257-261` and removes the need for a mapping.
   - It is consistent with `Client::connections` returning one `Err` for both outcomes. The cost is one CLI open on a refusal path, which the Design states.
   - Acceptance 3 lists five refusals with an owner running, and acceptance 5 covers the wrapper returning `None` on a refusal or a dropped stream.
3. **Describe and `SELECTING`: resolved.**
   - `connections-describe` leaves `SELECTING`, so `select` (`local.rs:233`) no longer runs `Registry::held` in the CLI before dispatch.
   - The owner resolves the held connection. With no answer, the arm runs the `select` logic and then the direct describe, so both refusals of `select` keep their code and stage. `connections-status` is not in `SELECTING`, so nothing else changes.
   - Acceptance 3 covers none and two held connections. When the story is built, the `select` body needs a callable form that the describe arm can call (today it is guarded by `SELECTING`), and the arm must read the resolved connection from a local, not from the unchanged `call.input`.
4. **Deadline, lock and concurrency: resolved.**
   - The Design states the 30 s span with fallback on expiry. It states that the authority read of `Client::connect` stays, that the lifecycle-lock wait is 10 s and then falls back, and that the owner may do its own fresh open.
   - It scopes the guarantee to the Entity Runtime open and the complete read in the CLI process.
   - Acceptance 6 reports a run with a read invoke in flight and says whether any owner run fell back.
5. **ESS model: resolved.**
   - `story:owner-observe-request-is-modelled-in-ess` has a `depends_on` edge to this story, which the graph confirms. The graph has no cycle.
   - It reads what this story produces (the request and its `owner.md` paragraph), and the edge records that.
   - Both stories edit `contracts/cli/v1alpha1/owner.md`, and the same edge orders them. This story writes the prose and the follow-up adds the type names. Acceptance 10 and the "Shares files" section state that this story edits no `ess/` file.
   - I traced two acceptance reads to a producer in this set, and an edge records both.
6. **`owner.md` and CHANGELOG: resolved.**
   - Acceptance 7 names `observe` in the sentence at `:257-261`. The CHANGELOG entry (acceptance 9) says "any running owner".
   - Acceptance 8 keeps `semantics.md` at 2,334 lines. I confirmed the file has 2,334 lines and that `ess/domains/cli.yaml:1952` and `:2112` cite `:1508` and `:1512`. An edit to `:1378-1380` that adds no net lines leaves those citations valid.

I walked 11 edges from the graph and none lead outside this story and its follow-up. The parallel-safety and scope concerns, such as the `CHANGELOG` and `owner.rs` regions shared with other stories, belong to those critics and did not affect this verdict.

```findings
[]
```
