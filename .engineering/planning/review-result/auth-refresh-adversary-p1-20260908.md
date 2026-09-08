---
format: aep.planning-md/1
id: review-result:auth-refresh-adversary-p1-20260908
kind: review-result
status: active
title: F04 refresh coordination adversarial pass 1
relations:
- reviews: story:contracts-refresh-coordination
revision: 1
---
unit: story:contracts-refresh-coordination; unstaged worktree connectors-v2-auth-refresh-20260908 against 1e567571d9ac62070933c7099b93a4e030613e58
verdict: green
cases: executed 0→0, red 0
origin: introduced 0, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes

1. Own-write diff stat

```text
 .../scenarios/revocation-before-authorization.yaml | 67 ++++++++++++++++++++++
 1 file changed, 67 insertions(+)
```

Source digests captured before the attack cover 285 tracked/untracked files. The final comparison has zero modified/deleted pre-existing paths and exactly one added path: `contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-authorization.yaml` (67 lines). Every source write by this adversary is an authored scenario. Scratch reports, compiler outputs and the isolated scenario copy stay inside the assigned worktree. No implementation, model, prose or planning file was edited.

The whole-unit `git --no-pager diff --stat` below includes the implementor's inherited unstaged changes; it is not this adversary's own write diff:

```text
 contracts/auth/acquisition/v1alpha1/semantics.md |  63 ++++-
 contracts/auth/custody/v1alpha1/semantics.md     |  20 +-
 docs/adapters/atlassian.md                       |   7 +-
 ess/domains/refresh.yaml                         | 280 ++++++++++++++++++++++-
 4 files changed, 352 insertions(+), 18 deletions(-)
```

That tracked stat excludes 14 untracked unit files: thirteen scenario YAML files and verification.md. Twelve of those scenarios and verification.md predate this attack. The digest manifests preserve the distinction; inherited implementation changes are not claimed as adversary writes.

2. Added case and first isolated result

`contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-authorization.yaml` exercises a missing boundary in the original twelve traces: reservation succeeds, revocation is observed at authorization, authorization emits nothing, the reserved owner is fenced, and neither that stale attempt nor a successor reservation regains authority. The trusted `source_unavailable` decision represents revocation; ESS does not derive it from a concurrent revocation operation.

The case and its isolated copy existed before any compiler suite was run. The first isolated result was green; there is no red output or failing runtime case to claim:

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios .local/waves/auth-hardening-20260908/refresh/adversary-isolated --out .local/waves/auth-hardening-20260908/refresh/adversary-isolated.json
1 authored scenario(s) from 1 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/adversary-isolated.json
exit: 0
```

The addition checks declared command inputs, refusal outcomes, typed error reasons, event expectations and lifecycle shape. It does not measure a provider call count or prove that a live coordinator selects the correct decision.

3. Suite after the addition

The implementor's report supplies the before count: 12 authored and 120 generated scenarios compiled, runtime executed 0. After this addition the private author lane compiles 13, synthesis compiles 133 (120 generated + 13 authored), and the operations regression lane remains 15. Every compiler refusal count is zero. Runtime execution remains 0→0.

All commands ran in the assigned refresh worktree, with TMPDIR set to its assigned `.local/waves/auth-hardening-20260908/refresh` scratch and CARGO_BUILD_JOBS=2. No Rust source changed and no Rust build was run. The coordinator owns the combined-model and full Rust gate.

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/adversary-authored.json
13 authored scenario(s) from 13 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/adversary-authored.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/adversary-synthesized.json
133 scenario(s) (13 authored), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/adversary-synthesized.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 7 file(s), valid
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/waves/auth-hardening-20260908/refresh/adversary-ir.json
connectors v1 — 7 file(s), 76 declaration(s), compiled to .local/waves/auth-hardening-20260908/refresh/adversary-ir.json
exit: 0

$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/adversary-operations.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/adversary-operations.json
exit: 0

$ git diff --check
exit: 0
```

4. Findings

No finding was established by this attack.

| File:line | Severity | Verdict | Origin | Measurement and reachable contract flow |
|---|---|---|---|---|
| — | — | — | — | No finding; the added pre-authorization revocation trace compiled without refusal. |

The subject was the complete F04 working diff against `1e567571d9ac62070933c7099b93a4e030613e58`, including all twelve original authored scenarios and verification.md. The shared credentials model and the sibling F05 evidence semantics were read for seam consistency; no other adversary's report was read. This report is an agent's attack result, not approval or independently produced runtime evidence.

5. Attack boundaries and limits

- Owner loss before and after irreversible authorization: fencing closes only Reserved; Authorized cannot authorize again, including a crash before send.
- Response-storage/quarantine race and late response: the selected terminal state does not reopen the source, and durable-response recovery grants publication only.
- Stale owner, changed binding, expired evidence and publication/revocation ordering: prose requires atomic refusal/cutoff; authored traces express the selected decisions. Pinning cannot override invalidation.
- Same material under aliases or across restart: acquisition explicitly prohibits independent refresh authority and refuses unresolved aliases; UUID uniqueness is not treated as sufficient.
- Custody versus coordinator: the secret port remains immutable write/read/delete; recovery and authorization belong to durable host metadata, and loose orphan material is not a committed response.
- F04/F05 seam: capture records expected identity rather than validation; new-generation publication invalidates old admissions, unresolved refresh withholds old dispatch, and candidate validation follows F05.

I did not turn the documented UNMAPPED obligations into purported regressions. No ESS check here executes a network exchange, derives trusted transaction decisions, models provider revocation timing, verifies material alias detection, persists a crash-safe ledger or arbitrates a real publication/dispatch race. No runtime harness, provider call or bespoke interpreter was created. No implementation file was mutated, even temporarily; no existing scenario was changed or weakened.

6. Storage and handoff

Files written outside this worktree: none. Worktree lifecycle hooks maintain their normal external registry state; no source, build, log or scratch artifact was placed outside the assigned checkout.

Preserved inside `.local/waves/auth-hardening-20260908/refresh/`: `adversary-pre-source-digests.json`, `adversary-source-boundary.json`, `adversary-isolated/revocation-before-authorization.yaml`, `adversary-isolated.json`, `adversary-isolated.log`, `adversary-authored.json`, `adversary-synthesized.json`, `adversary-ir.json`, `adversary-operations.json`, `adversary-suite.log`, and this `adversary-pass-1.md`. The new authored source remains unstaged. No commit, staging, checkout, stash, reset, AEP command, cleanup or additional agent was used.

The `codex-auth-refresh-adversary-p1-20260908` lease is released immediately after preserving this report; the coordinator's lease is untouched. The coordinator owns recording this report verbatim, reconciling the final 13-case count with the historical 12-case implementor record, combined F04/F05 validation, the full integration gate, local commits/merge and worktree handoff.

```findings
[]
```
