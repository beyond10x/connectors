unit: story:contracts-credential-evidence; impl/contracts-credential-evidence working tree over 1e567571d9ac62070933c7099b93a4e030613e58
verdict: green
cases: executed 0→0, red 0
origin: introduced 0, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes

Own-write diff stat (the one new authored scenario; captured source digests separate this from the implementor's unstaged work):

```text
 .../retained-lineage-expires-before-dispatch.yaml  | 61 ++++++++++++++++++++++
 1 file changed, 61 insertions(+)
```

Whole-unit `git --no-pager diff --stat`, unchanged by this attack because the additional scenario is untracked:

```text
 contracts/auth/capability/v1alpha1/semantics.md |  18 ++-
 contracts/auth/connection/v1alpha1/semantics.md |  15 +-
 contracts/auth/evidence/v1alpha1/semantics.md   |  70 +++++++++-
 docs/adapters/kubernetes.md                     |   7 +
 ess/domains/credential_evidence.yaml            | 174 +++++++++++++++++++++++-
 5 files changed, 265 insertions(+), 19 deletions(-)
```

All 278 source files captured before the attack retain their original digests. The only new source path is `contracts/auth/evidence/v1alpha1/scenarios/retained-lineage-expires-before-dispatch.yaml` (61 lines). The existing prose/model modifications and five existing scenarios are the implementor's work, not adversary writes. No implementation, shared file, planning file or existing test was edited. The full before/after digest manifests and own-write audit are preserved in assigned scratch.

1. Added case, written before any suite run

`retained-lineage-expires-before-dispatch.yaml` extends the claimed evidence-transfer rule in `contracts/auth/evidence/v1alpha1/semantics.md:106` and dispatch freshness rule at `:96`: the new generation retains identity evidence collected at 08:59:30 with a 09:00:30 deadline, is admitted at 09:00:29, and is refused for stale evidence at 09:00:31. Subsequent dispatch and attempted re-admission both select the terminal-state refusal. This is a reachable contract flow when a same-account refresh is followed by a queued dispatch; a new generation must not extend the inherited identity deadline.

The case is green under the structural compiler. It does not run a clock or prove that runtime code selects `RejectAdmission`. The initial isolated invocation refused my duplicated timeline instant. That was an authoring error, not a substantive defect or a failing runtime case; its exact output is retained below. I changed only the new case's first instant from 09:00:29 to 09:00:28 and reran it alone before running the suite.

Initial isolated output:

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated.json
refusal[ESS-AUTHOR-023]: `connectors.credential_evidence/authored/retained-lineage-expires-before-dispatch` in /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated/retained-lineage-expires-before-dispatch.yaml
  `2026-09-08T09:00:29Z` does not come after `2026-09-08T09:00:29Z`
  help: give each act an instant later than the one before it; the file's order is the scenario's order and `at:` is what states it
0 authored scenario(s) from 1 file(s), 1 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated.json
exit: 1
```

Corrected isolated output:

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated-fixed.json
1 authored scenario(s) from 1 file(s), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-isolated-fixed.json
exit: 0
```

2. Suite after the case existed

The implementor supplied the before count: five authored F05 scenarios, 72 combined synthesized scenarios, 15 existing operations authored scenarios. After adding this case: six authored F05 scenarios, 73 combined (67 generated plus six authored), 15 operations authored scenarios, zero compiler refusals. Runtime execution remains 0→0, red 0. No runtime target, provider call, Rust build or full integration gate was invoked.

Exact suite output, commands and exit statuses:

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 7 file(s), valid
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-ir.json
connectors v1 — 7 file(s), 66 declaration(s), compiled to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-ir.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/evidence/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-authored.json
6 authored scenario(s) from 6 file(s), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-authored.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/auth/evidence/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-synthesized.json
73 scenario(s) (6 authored), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-synthesized.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-operations.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence/adversary-p1-operations.json
exit: 0

$ git diff --check
exit: 0
```

3. Findings

No substantive defect found in the assigned diff and the attacked boundaries. This result is not approval and claims no review independence.

| File:line | Severity | Verdict | Origin | What was measured | What reaches it |
|---|---|---|---|---|---|

No finding rows. The timestamp authoring correction above is not a finding against the unit.

4. Attack boundaries and limits

- Account A→B replacement and reuse of A's age-valid evidence: the normative contract refuses identity/generation mismatch and separately admits validation; the existing authored cases state the corresponding refusal and no dispatch.
- Configured path changes, exec output changes and certificate/key replacement: prose requires immutable capture and prohibits reading new bytes or invoking the helper again under the prior evidence. Actual transport and file races remain explicitly unexecuted obligations.
- Admission followed by refresh publication, revocation, expiry or uncertain refresh: the contract cuts off old pending admissions, while already-opened dispatch is not retroactively undone; no contradictory success path was found in F05's authored traces.
- Evidence retained through same-account refresh: provenance and original deadlines remain, grants are recomputed, provider permission/verification are invalidated; the additional case records refusal after retained identity evidence expires.
- Shared credential seam and sibling refresh source were read for consistency: expected identity at capture is not validated identity; aliases cannot obtain independent authority to use possibly consumed rotating material. No sibling source was edited; no other adversary report was consulted.
- Public projection excludes private generation, snapshot and secret-version identifiers. No serializer exists here to execute a redaction test.
- The model deliberately cannot reject a well-typed trace that chooses admission despite mismatched identity, stale checks or a different nested generation. This limitation is already explicit in `verification.md` and the model's UNMAPPED obligations, so it is not presented as a new regression. Compiler success checks declarations, types, events, views and lifecycle shape; it does not prove the host's admission predicates or atomic ordering.

5. Handoff

The coordinator must record this report, add the new exact scenario path to the story's typed scope, preserve the additional compiled counts in closing verification, and run the combined integration gate. The existing unit verification document was not edited by the adversary. No further attack is requested by this report.

Managed tree: `connectors-v2-auth-evidence-20260908`, path `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908`, branch `impl/contracts-credential-evidence`, base `1e567571d9ac62070933c7099b93a4e030613e58`. Changes remain unstaged. Next owner is the coordinator. The adversary releases only session `codex-auth-evidence-adversary-p1-20260908`; coordinator lease and managed-tree lifecycle remain untouched. No directories or build output were removed.

Retained report, isolated copies, logs, compiler JSON and source digests live under `.local/waves/auth-hardening-20260908/evidence/`, the assigned scratch directory. Exact report path: `.local/waves/auth-hardening-20260908/evidence/adversary-pass-1.md`. Outside-worktree artifact paths: none. Required worktree lease bookkeeping is not an external deliverable.

```findings
[]
```
