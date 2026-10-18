---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-design-r1
kind: review-result
status: active
title: Plan critic design, round 1, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
approve

I read the three stories, the relations vocabulary, the full graph and the D12 text. The set is shaped correctly.

- **Cycle:** none. The declared chain is `invoke-definition-reuse-credential` depends_on `invoke-definition-file`, which depends_on `connection-binding-excludes-operations`. The binding story depends on nothing in the set. Its only extra edge is `informed_by` the proposed story `connection-follows-configuration-upgrade`, an "informed by" edge that blocks nothing.
- **Chain:** it has three items and each edge has a reason written in a body.
  - The file story's acceptance ("a third after only an added operation return results with no prompt") reads the binding story's output.
  - The reuse story's acceptance and outcome run `operations invoke --definition`, which the file story creates.
- **Split abstraction:** none.
  - D12 points 1 and 2 are the binding story. Point 3 is cut at a real seam: the file story is one selector, a cache and the existing acquisition paths, and the reuse story is a new custody copy.
  - The binding story's "Why one story" and the file story's "Why a separate story" give the reasons.
  - Each story can be shown working on its own.
- **Hidden dependency:** none.
  - I traced three acceptance reads to a producer in the set: the file story's third run, the reuse story's use of the file story's command, and the reuse story's refusal "as in the previous story". All three have a `depends_on` edge.
  - The binding story's acceptance reads only existing catalog tests.
- **Horizontal slice:** none. Each story is a behaviour cut through the CLI, registry and docs.

What I read: 3 artifacts, using `aep plan artifact show` on each plus `relations`, `graph` and `validate`.
- I walked 9 declared edges out of the 3 stories, and the graph output covers the whole store, so I went outside the set.
- `validate` reported nothing for these ids. It printed only its summary line, "887 artifact(s)".

What I could not establish:
- Out of my lane (scope critic): the file story covers only "no connection" or "binding matches". It doesn't say what happens on an authority or profile change before the reuse story lands. It probably falls to today's `lifecycle_conflict`, but the body doesn't say so.
- Out of my lane (parallel-safety critic): `registry/lifecycle.rs` and `registry/revalidation.rs` appear in the scope of both the binding story and the reuse story. The chain already orders them.
- I did not verify the cited line numbers in the source files, such as `local.rs` lines 1157-1186. That is the scoper's job, not a design question.

```findings
[]
```
