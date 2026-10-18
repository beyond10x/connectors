---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-scope-r1
kind: review-result
status: active
title: Plan critic scope, round 1, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
needs-revision
story:invoke-definition-reuse-credential — the candidate set is narrowed to non-revoked connections "of the same connector id", while D12 promises reuse of "a credential already stored in connectors"; the narrowing is not listed under "Not in scope" and nothing marks it as a drafter decision awaiting the operator — .engineering/planning/architecture-decision-record/provider-definitions-and-runtimes.md:229 (story body .engineering/planning/story/invoke-definition-reuse-credential.md:57)

**What I read:** 4 artifacts, using `aep plan artifact show` on the parent and the three stories, plus `aep plan artifact graph`. I also read `crates/connectors-catalog/src/definition.rs` (`Request`) and `docs/local-connection-registry.md` lines 100-158.

**Promises:** I extracted 14 from D12 (lines 193-241) and traced 13 to an item. The 14th is the same-connector narrowing above.

| Promises | Claimed by |
|---|---|
| Points 1-2: second digest, four-row change table, approvals unchanged | `story:connection-binding-excludes-operations` |
| Docs and old-rule tests (from the task statement) | `story:connection-binding-excludes-operations` |
| Point 3: lowering into an owner-only cache, operator configuration unmodified, host path | `story:invoke-definition-file` |
| Point 3: ask once, token or OAuth, connect fresh | `story:invoke-definition-file` |
| Point 3: ask once, reuse | `story:invoke-definition-reuse-credential` |
| Point 3: `invoke` stays the legacy explicit command | `story:invoke-definition-file` |
| "Not decided by D12" (D6, persisting into the D5 cache, signing) | Excluded in the stories' "Not in scope" sections |

The graph shows no older item claiming any part of D12. No two items claim the same outcome. The drafter named the authority/profile-change refusal and D6 as exclusions, so I did not count them as gaps.

**What I could not establish (none of these set the verdict):**
- **D12 point 4:** it forbids installing, updating or lowering from creating a connection, and requires the D5 digest checks for definitions taken from an installed index. `story:invoke-definition-file` states neither. I judged this as an invariant that the command taking only a file and the non-interactive refusal already hold, not a gap.
- **Existing connections:** `story:connection-binding-excludes-operations` makes connections saved before the change keep today's rule until their first revalidation. The story states this, so it is not a silent narrowing. It is the drafter's call, so the operator may want to see it.
- **Out of my lane (design):** `docs/local-connection-registry.md:118-121` refuses "a new connect under the same instance id" after an authority or profile change. `story:invoke-definition-reuse-credential` publishes a new connection under the definition's instance after such a change. Neither the story body nor its "Not in scope" says how that registry rule gives way.
- **Out of my lane (acceptance):** `story:connection-binding-excludes-operations` tests only the selection-change case. The executable-only row has no acceptance.

```findings
- file: .engineering/planning/architecture-decision-record/provider-definitions-and-runtimes.md
  line: 229
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:invoke-definition-reuse-credential narrows the candidate set to non-revoked connections of the same connector id, while D12 promises reuse of a credential already stored in connectors; the narrowing is not listed under Not in scope or marked as a drafter decision (story body .engineering/planning/story/invoke-definition-reuse-credential.md:57)"
```
