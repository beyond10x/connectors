unit: story:contracts-read-refresh-retry — Version and bound read redispatch after refresh
verdict: green
cases: manual 14 audited; executed ESS checks 2 before / 2 after; runtime cases 0
origin: n/a
wrote-outside-worktree: none
needs-coordinator: yes — .local/spec-completion-20260909/read-retry-coordinator.patch

Acceptance: each selected read profile determines one bounded 401-response sequence without silently changing existing service semantics. This unit resolves F15/E05 in scoped specification files; review/outcome/lifecycle changes and shared-file integration remain with the coordinator.

Worktree: /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-read-retry-20260909
Branch: specs/read-retry-20260909
Base: 8e1836cad8ae1b2127ce9ae306c6d8131960db4c
Session: specs-read-retry-implementor-20260909
All changes remain uncommitted. No git staging, stash, commit or branch operation was performed.

The final binding selects an exact combined native Operation.profile, P.read-refresh-once.v1alpha1, on v1alpha2 with a changed projection revision. No new wire field or current adapter selection is implied. Eligible direct managed bearer unary reads receive at most one source refresh participation and one redispatch after a definitive first business 401. They retain the original immutable target/context, one 15s provider-work cutoff inside 20s execution, consumed F08 permissions ledger and fresh-generation admission. A second 401 is terminal. Refresh auth-result/lineage validation cannot introduce identity/verification probes. Native combined profiles must declare finite HTTP metadata/framing limits in their adapter-owned contracts; body bounds alone are not a complete transport bound.

Actual git diff --stat (tracked changes):

```text
 contracts/auth/acquisition/v1alpha1/semantics.md | 5 +++--
 contracts/auth/capability/v1alpha1/semantics.md  | 6 ++++--
 contracts/service/v1alpha1/semantics.md          | 5 +++++
 3 files changed, 12 insertions(+), 4 deletions(-)
```

New files, not included in that tracked diff statistic:
- contracts/auth/capability/v1alpha1/read-refresh-once.md (241 lines)
- docs/evidence/read-refresh-retry-20260909/baseline-cases.md (24 lines)
- docs/evidence/read-refresh-retry-20260909/verification.md (64 lines)
- docs/evidence/read-refresh-retry-20260909/ess-before-validate.log
- docs/evidence/read-refresh-retry-20260909/ess-before-compile.log
- docs/evidence/read-refresh-retry-20260909/ess-after-validate.log
- docs/evidence/read-refresh-retry-20260909/ess-after-compile.log

The pre-correction audit was written before contract edits. It records fourteen concrete cases and the missing new-profile sequence; it does not fabricate a red runtime test count. In particular RR02/R03 lacked an explicitly selected sequence, RR06/R07 lacked shared deadline/consumed-slot behavior, RR08 lacked retry-invocation error mapping and RR14 lacked exact profile selection. All fourteen now map to a stated observation/refusal in verification.md. F15's compatibility contradiction and E05's missing disposition are fixed as one class: changed execution behavior requires an explicitly supported profile/revision and cannot be inferred from unchanged wire bytes or capability names.

Executed checks (each command ran from this worktree):

```sh
TMPDIR="$PWD/.local/tmp" /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
```

Before and after, exit 0:

```text
connectors v1 — 14 file(s), valid
```

```sh
TMPDIR="$PWD/.local/tmp" /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/spec-completion-20260909/ess-before.json
TMPDIR="$PWD/.local/tmp" /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/spec-completion-20260909/ess-after.json
```

Each exited 0:

```text
connectors v1 — 14 file(s), 218 declaration(s), compiled to .local/spec-completion-20260909/ess-before.json
connectors v1 — 14 file(s), 218 declaration(s), compiled to .local/spec-completion-20260909/ess-after.json
```

`cmp .local/spec-completion-20260909/ess-before.json .local/spec-completion-20260909/ess-after.json` exited 0, no output. ESS files are unchanged; this is expected model validity/IR stability, not execution of the new textual scenarios. The existing RefreshAttempt, CredentialGeneration and DispatchAdmission identities/lifecycles suffice. No new typed entity or unresolved ownership relation was found. The new document explicitly retains runtime cross-model predicates as UNMAPPED.

`git diff --check` exited 0, no output. Individual `git diff --no-index --check /dev/null <new markdown>` checks returned the expected 1 for new-file differences with no whitespace diagnostics. The ten explicit local link targets were checked with test -f, exit 0; the two referenced existing heading anchors were inspected. Searches for the withdrawn 'read retry remains unsettled' wording in the changed capability/acquisition documents returned no matches (rg exit 1).

Exact coordinator patch (apply_patch format), left unapplied:
`.local/spec-completion-20260909/read-retry-coordinator.patch`
SHA256: eab73d7b20055a56c8b0f3fd193876fcf76622337cb5e04525badb09391733aa
It updates contracts/service/compatibility.md (selection, capability matrix and shared budget) and contracts/service/v1alpha2/semantics.md (execution rule and conformance case). Shared indexes/design/planning and any capability vocabulary reconciliation remain with the coordinator. Cross-document closure requires applying this patch; no runtime binding or current adapter combined profile is advertised.

No runtime, provider, codec, schema generator, ESS or planning file was edited; no Python/helper scripts or external publication were used. No cold Rust build/full gate was run, per the specification-only unit brief. Coordinator performs combined independent review and integration gate.

Only this session's worktree lease is released at handoff. The dirty tree, scratch IR/logs, exact patch and evidence are retained for the coordinator, who owns commit/integration and cleanup. No direct file writes were made outside this worktree.
