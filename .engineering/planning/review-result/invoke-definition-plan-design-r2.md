---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-design-r2
kind: review-result
status: active
title: Plan critic design, round 2, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
approve

I read the three drafted stories, `story:configuration-change-error`, and D12. The revised set has the right shape.

- **Cycle:** none. The `depends_on` edges are `story:invoke-definition-reuse-credential` to `story:invoke-definition-file` to `story:connection-binding-excludes-operations`, plus the new `story:configuration-change-error` to `story:connection-binding-excludes-operations`. The new edge ends at the binding story, which depends on nothing in the set. Its only other edge is `informed_by`, which blocks nothing.
- **Chain:** the new edge makes the shape a fan-out, not a longer queue. The new story and the file story can both start once the binding story lands. The binding story's own table gives the reason for the new edge: it removes the bundle- and selection-only case that the new story's refusal is about.
- **Split abstraction and horizontal slice:** none. The cut between the file story and the reuse story is the same as in round 1. The reuse story adds a custody copy and the file story does not.
- **Hidden dependency:** I traced 3 acceptance reads to a producer in the set, and all 3 have an edge.
  - The file story's third run, after an added operation, reads the binding rule. The edge runs from the file story to the binding story.
  - The reuse story's acceptance runs `--definition`, which the file story creates. The edge runs from the reuse story to the file story.
  - The reuse story's "no terminal" refusal is "as in the previous story". It is covered by the same edge.
  - The binding story's acceptance reads only existing catalog tests.

What I read: 5 artifacts, with `aep plan artifact show` on the three stories, `story:configuration-change-error` and D12. I also ran `relations`, `graph` and `validate`, and read `review-result:invoke-definition-plan-design-r1`. I walked the 11 non-review edges out of the four stories, and the graph output covers the whole store. `validate` reported `valid` for these ids. Its only warnings were about old `review-result` records unrelated to this set.

What I could not establish:
- Out of my lane (acceptance and scope): `story:configuration-change-error` is still worded as "any native-config change", but the binding story's table says its advice must name only credential-side changes after this lands. The new story's body does not say that yet.
- Out of my lane (parallel safety): `story:invoke-definition-file` promises "today's `lifecycle_conflict`" for an authority or profile change. `story:configuration-change-error` would replace that code on connect, and no edge orders the two. Both tables already record the shared files and sections.
- I did not verify the cited line numbers in the source files.

```findings
[]
```
