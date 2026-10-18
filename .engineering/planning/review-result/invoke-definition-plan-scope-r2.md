---
format: aep.planning-md/3
id: review-result:invoke-definition-plan-scope-r2
kind: review-result
status: active
title: Plan critic scope, round 2, on the invoke-definition drafts
relations:
- reviews: story:connection-binding-excludes-operations
- reviews: story:invoke-definition-file
- reviews: story:invoke-definition-reuse-credential
- reviews: architecture-decision-record:provider-definitions-and-runtimes
revision: 1
---
approve

Round 1's one scope finding is resolved. `story:invoke-definition-reuse-credential` now marks the same-connector narrowing as "Candidates (drafter's decision, 2026-10-18)" (lines 53-57). It also lists "reusing a credential of another connector id" under "Not in scope" (line 93) and calls widening "a later decision for the operator". That makes the narrowing a recorded exclusion, not a silent one. The three items still claim exactly the D12 promises, and none claims something D12 did not ask for.

**What I read:** 4 artifacts: the parent D12 section (lines 186-246) and the three drafted stories, whole bodies, using `aep plan artifact show`. I also read `review-result:invoke-definition-plan-scope-r1` and ran `aep plan artifact list` and `aep plan artifact graph`. The graph shows no older artifact claiming any part of D12. I grepped the tree for tests that pin the old stale rule. `configuration_refusal_adversary.rs`, `revision_conflict.rs`, `revalidate_transient.rs` and `failed_connect.rs` do not pin the selection-change rule, so the old-rule test list in `story:connection-binding-excludes-operations` looks complete.

**Promises:** I extracted 13 from D12 and traced all 13 to an item, each claimed once.

| Promise | Claimed by |
|---|---|
| Binding narrowed to authority, profile declaration, identity, trust inputs and executable digest (point 1, second digest) | `story:connection-binding-excludes-operations` |
| Four-row change table (bundle/selection only, executable only, trust inputs, authority/profile) | `story:connection-binding-excludes-operations` |
| Approvals keep their binding | `story:connection-binding-excludes-operations` |
| Docs `local-connection-registry.md` and `semantics.md`, and the tests pinning the old rule | `story:connection-binding-excludes-operations` |
| `--definition` command, with `invoke` staying the legacy explicit command | `story:invoke-definition-file` |
| Lower into an owner-only cache keyed by digest, operator configuration unmodified | `story:invoke-definition-file` |
| Host path with custody, admission, approval and audit unchanged | `story:invoke-definition-file` |
| Ask once when no connection exists: token prompt or OAuth, then connect fresh | `story:invoke-definition-file` |
| Ask once when authority or profile changed: reuse or connect fresh | `story:invoke-definition-reuse-credential` |
| Reuse validated by the new profile, published only when the identity matches | `story:invoke-definition-reuse-credential` |
| Point 4: lowering never connects, and a definition is data | `story:invoke-definition-file` |
| Point 4: D5 digest checks for definitions from an installed index | `story:invoke-definition-file` (excluded by name, line 71) |
| "Not decided by D12" (D6, persisting into the D5 cache, signing) | the "Not in scope" sections of the file and reuse stories |

**What I could not establish (none of these set the verdict):**
- D12 says the CLI "computes the configuration revision". `story:invoke-definition-file` leaves that to the host path ("everything after lowering is the configured-instance path"), and its third-run acceptance needs it. I judged it covered.
- Some details are not in D12: `--definitions-root`, the instance id `definition`, the non-interactive refusal, and the reuse story revoking the definition's own old connection. I judged each a recorded implementation detail serving a D12 promise, not reach.
- Out of my lane (design): `story:invoke-definition-reuse-credential` says "either answer revokes" the old connection while also saying "the source connection is unchanged either way". Those read as contradictory when the source is the instance's own old connection.
- Out of my lane (acceptance): `story:connection-binding-excludes-operations` has an acceptance case for executable-only change and the old-rule tests are named in its scope. `story:invoke-definition-reuse-credential` has no acceptance for the revoke-then-publish path on the definition's own instance.
- Out of my lane (parallel-safety): both reuse and binding stories scope `docs/local-connection-registry.md` and `registry/lifecycle.rs`. The `depends_on` chain orders them.

```findings
[]
```
