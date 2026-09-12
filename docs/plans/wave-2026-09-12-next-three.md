# Wave proposal — the next three, 2026-09-12

Skill version 0.9.1. Coordinator: this session, in the primary checkout at
`bfd01ae`, branch `main`, clean.

## Verdict

Three waves are proposed. All three are N=1. No wave with N greater than one exists
anywhere in the store: `aep plan artifact waves` puts every remaining candidate in a
wave of its own, because they all collide on shared surfaces.

Wave 1 is running. Waves 2 and 3 are held. Two facts constrain all three, both
verified:

1. `cv2-ess3-adoption` holds eight uncommitted files that every candidate's scope
   claims, including `crates/connectors-spec/toolchain.json`, `ess/system.yaml`,
   `adapters/gitlab/spec/ess/system.yaml` and `adapters/kubernetes/spec/ess/system.yaml`.
   Wave 1 accepts this: it writes tests and host/adapter source, not adapter ESS
   documents or the toolchain pin. A later conflict in that session's merge remains
   possible and is recorded here rather than assumed away.
2. No remaining story can reach `implemented`. Every one needs dedicated provider
   sandbox evidence, and `credential-blocker:gitlab-runtime-sandbox` is open. Wave 1
   therefore produces an increment on an active story and closes nothing.

## The backlog, counted

`aep plan artifact list --kind story --format json`, 59 stories:

| status | count |
|---|---:|
| implemented | 52 |
| active | 5 |
| draft | 2 |

Five blockers are open. `aep plan artifact blocked`:

| blocker | blocks | who clears it |
|---|---|---|
| `credential-blocker:gitlab-runtime-sandbox` | all 5 active GitLab stories | operator — sandbox URL, project, protected credential path |
| `decision-blocker:gitlab-mr-create-update-head-guard` | `initiative:complete-local-connectors` | operator — accept the preflight/postflight race, or require a guard |
| `decision-blocker:helm-execution-family` | `initiative:complete-local-connectors` | operator — does Connectors run external provider binaries |
| `tooling-blocker:kubernetes-driver-protocol-loading` | `story:kubernetes-spec-service` | a governed run that resolves `adp/1` |
| `configuration-blocker:gitlab-v020-publication-target` | `release-plan:gitlab-v020` | operator — publication destination |

## What the verb returned, verbatim

`aep plan artifact waves --kind story --status draft`:

```
wave 1
  story:kubernetes-helm-release-reads
wave 2
  story:kubernetes-spec-service
collision: story:kubernetes-helm-release-reads story:kubernetes-spec-service adapters/kubernetes
2 wave(s), 1 collision(s), 0 unassessed
```

`aep plan artifact waves --kind story --status active`:

```
wave 1
  story:gitlab-ci-runtime
wave 2
  story:gitlab-mr-reads (inferred)
wave 3
  story:gitlab-mr-validation (inferred)
wave 4
  story:guarded-gitlab-merge (inferred)
wave 5
  story:persistent-gitlab-journey
collision: story:gitlab-ci-runtime story:gitlab-mr-reads README.md (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-reads adapters/gitlab (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-reads crates/connectors-spec (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-reads docs (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-reads spec-kinds/adapter/v2 (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-reads website (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-validation adapters/gitlab (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-validation crates/connectors-conformance (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-validation crates/connectors-spec (inferred)
collision: story:gitlab-ci-runtime story:gitlab-mr-validation docs (inferred)
```

Selection path: the verb, not a pairwise reading. `aep --version` is `protocol 0.55.0`.
Exit status 0 on both. Zero unassessed stories — every candidate carries typed scope.

## The three proposed waves

| # | unit | story status | what it delivers | can it close the story |
|---|---|---|---|---|
| 1 | `story:guarded-gitlab-merge` | active | metadata unavailability after a known native write effect: final disclosure and restart without a second effect | no — sandbox |
| 2 | `story:guarded-gitlab-merge` | active | general original-audit reconciliation, after a reviewed persistence/correlation binding | no — sandbox |
| 3 | `story:kubernetes-helm-release-reads` | draft | Helm release family A reads: history, status, values, manifest | no — sandbox |

Waves 1 and 2 are two increments of one story. That is what the backlog holds: the
GitLab phase has one active story with named remaining work, and the other four
active GitLab stories are locally complete and waiting on sandbox evidence alone.

Sources for waves 1 and 2: `initiative:complete-local-connectors`, section
"Verified audit acknowledgement recovery checkpoint — 2026-09-11", which names both
as the next work. Source for wave 3: `story:kubernetes-helm-release-reads`, section
"Sequence and boundaries", steps 1 to 5.

### Wave 3 breaks the declared provider order

`initiative:complete-local-connectors` states the order as GitLab, Kubernetes,
PostgreSQL, MCP, remaining providers, and repeats "Full GitLab still precedes
Kubernetes" in five separate checkpoints. GitLab cannot finish without the sandbox.
Scheduling wave 3 is a decision to start Kubernetes before GitLab closes.

Wave 3 also carries an unstated prerequisite its own body records: the Helm release
storage format is not pinned in this repository, and step 1 requires fetching the
Helm upstream storage-driver and release-type sources to pin URL, SHA-256 and byte
length. That is a network fetch against an upstream this repository does not vendor.

### What was left out, and why

| candidate | why not |
|---|---|
| `story:gitlab-ci-runtime` | locally complete; only sandbox evidence remains |
| `story:gitlab-mr-reads` | locally complete; only sandbox evidence remains |
| `story:gitlab-mr-validation` | locally complete at revision 6; only sandbox evidence remains |
| `story:persistent-gitlab-journey` | locally complete; only sandbox evidence remains |
| `story:kubernetes-spec-service` | `tooling-blocker:kubernetes-driver-protocol-loading` open |
| `epic:mcp-contracts` | draft, zero decomposing children. Its own body requires ESS contract authoring before implementation decomposition, and AGENTS.md requires modeling and review before decomposing unresolved semantics |

## Pre-flight

| check | value | verdict |
|---|---|---|
| working tree | clean, `main` at `bfd01ae` | pass |
| `git worktree list` | 6 linked trees present | **refuse** — see below |
| free disk | 30 G free, 97% used, `df -h /` | thin |
| build directories from earlier waves | 4 finished trees still on disk: 52M + 50M + 52M + 52M = 206M | present |
| other live sessions | 2 active managed trees | **refuse** |
| model budget | not asked | open |
| one measured build | not measured this session | open |
| `AGENTS.md` | read, 159 lines | pass |

### The two active trees

`worktree status`, filtered to this repository:

| tree | state | uncommitted | size |
|---|---|---:|---:|
| `wt-35ad348b4293` | active | 0 | 310M |
| `cv2-ess3-adoption` | active | 8 files | 1.2G |
| `kubernetes-local-cli-20260911` | finished | 0 | 50M |
| `kubernetes-rebase-20260911` | finished | 0 | 52M |
| `postgres-local-cli-20260911` | finished | 0 | 52M |
| `initiative-heading-fix-20260911` | finished | 0 | 52M |

`cv2-ess3-adoption` on `feat/adopt-ess3`, `git status --porcelain`:

```
 M adapters/atlassian/spec/ess/system.yaml
 M adapters/docker/spec/ess/system.yaml
 M adapters/gitlab/spec/ess/system.yaml
 M adapters/grafana/spec/ess/system.yaml
 M adapters/kubernetes/spec/ess/system.yaml
 M adapters/loki/spec/ess/system.yaml
 M crates/connectors-spec/toolchain.json
 M ess/system.yaml
```

`story:guarded-gitlab-merge` claims `adapters/gitlab` and `crates/connectors-spec`
as **cited** scope. `story:kubernetes-helm-release-reads` claims `adapters/kubernetes`
as **cited** scope, and its step 2 writes `adapters/kubernetes/spec/ess`. Both
collide with the uncommitted set above. `crates/connectors-spec/toolchain.json` is
the single ESS pin named by AGENTS.md; it is changing under any wave started now.

Those changes are not this session's to stash, commit or branch away.

## Parallelism

This repository does not admit a wave of more than one.

`initiative:complete-local-connectors`: "Follow the explicit single-agent
direct-checkout rule here; use managed worktrees and leases for MCP." Six separate
checkpoints repeat that root is the sole implementation and planning writer on
primary main, and that shared source, planning writes and build outputs serialize
there. AGENTS.md: "For single-agent work, the operator's repository-specific rule
is to work directly in the checkout."

The verb agrees with the rule independently: every pair of remaining candidates
collides on `adapters/gitlab`, `crates/connectors-spec`, `docs` or `website`.

The consequence for this proposal: the three waves run one after another in the
primary checkout, with no worktree created and no integration branch. Each is a
single implementor dispatch followed by a single adversary pass.

## Dispatch plan, if approved

`subagent_type` values, in full: `aep-drive:implementor` for the unit,
`aep-drive:adversary` for the pass that follows it. No scopers are needed — the
verb reports zero unassessed stories.

Per wave: one implementor into the primary checkout, gate package-scoped, then one
adversary against the same tree, then the full gate per `docs/development.md` with
Rust 1.88, per-step exit statuses captured individually.

## Commits this proposal authorises, if approved

Per wave: one unit commit and one closing store commit. Three waves, six commits
total, on `main` in the primary checkout. No merge commits — there is no integration
branch, because N is one and no worktree is created.

Not authorised: a push, a tag, a release, a version bump, any work outside the three
units, `release-plan:gitlab-v020`, or clearing any of the five open blockers.

---

# Stage 2 — running

Approval: the operator re-invoked `/aep-drive:wave` with no arguments after reading
the stage-1 proposal above. Read as approval to run, with the defaults this page
stated on silence: wave 1 runs, wave 3 is dropped because it breaks the declared
provider order, wave 2 is held because its prerequisite is a reviewed
persistence/correlation binding that does not exist yet.

## Unit 1

| field | value |
|---|---|
| story | `story:guarded-gitlab-merge` (active, revision 25) |
| objective served | `vision:independent-contract-adapters`, through `initiative:complete-local-connectors` |
| scope confidence | cited for `adapters/gitlab`, `crates/connectors-host/src/local`, `crates/connectors-sdk`; inferred for `docs` |
| checkout | the primary checkout, branch `main` — no worktree, per the repository's single-agent direct-checkout rule |
| build directory | that checkout's own `target/`, 1.3G warm |
| scratch root | `~/.cache/cv2-wave-20260912/` |
| brief | `~/.cache/cv2-wave-20260912/unit-1-brief.md` |
| `subagent_type` | `aep-drive:implementor`, then `aep-drive:adversary` |

### What unit 1 delivers

The **Attempt terminal settlement** row of `docs/gitlab-write-failure-matrix.md`,
whose remaining-evidence column reads: "Fail settlement after a known native
Applied/Refused response in the CLI; inspect the live response, restart observation,
key state and exact provider effect count."

The joined production CLI journey in
`adapters/gitlab/tests/local_runtime/guarded_merge.rs` gains a case that fails
terminal settlement persistence after a known applied native PUT, and asserts: the
live response still carries the known business result, restart observation stays
conservative, approval-proof and key state are as asserted, and exactly one PUT
reached the fixture provider.

Held out of unit 1, with reasons: general original-audit reconciliation (needs a
reviewed persistence/correlation binding first, per
`docs/gitlab-write-failure-matrix.md` § "Original audit after process loss"), C14
create/update head-guard semantics (`decision-blocker:gitlab-mr-create-update-head-guard`),
and dedicated sandbox acceptance (`credential-blocker:gitlab-runtime-sandbox`).

## Pre-flight taken at dispatch

| check | value |
|---|---|
| working tree | clean but for this page |
| branch | `main` at `bfd01ae` |
| free disk | 24 G, 98% used — down from 30 G measured minutes earlier in the same session, so another session is building |
| `git worktree list` | 6 linked trees; 2 active and owned by other sessions, 4 finished and left on disk (206 M) |
| model budget | not asked. N is 1, which is the floor |
| one measured build | not measured; the target is warm from prior work in this checkout |
| `AGENTS.md` | read this session, 159 lines |

The four finished-but-present trees are not this session's to remove:
`kubernetes-local-cli-20260911`, `kubernetes-rebase-20260911`,
`postgres-local-cli-20260911`, `initiative-heading-fix-20260911`.

## Stages

| stage | state |
|---|---|
| implementor dispatched | done |
| implementor returned | pending |
| adversary pass 1 | pending |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | pending |

## Commits this wave makes

One unit commit and one closing store commit, on `main` in the primary checkout.
No integration branch, because N is 1 and no worktree was created. No push, no tag,
no release, no version bump.

## Unit 1 — implementor returned

Verdict green. Executed cases 12 → 13 in the lane that runs this work, exit 0.

**No production source changed.** The implementor found the behaviour already
satisfied the row and delivered evidence instead of a fix. Files changed:

```
 adapters/gitlab/tests/local_runtime.rs               |  18 ++-
 adapters/gitlab/tests/local_runtime/guarded_merge.rs | 166 ++++++++++++++++++++-
 2 files changed, 180 insertions(+), 4 deletions(-)
```

New case:
`cli_journey::guarded_merge::gitlab_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively`,
journey modes 8 (known Applied) and 9 (known Refused). The fault is injected outside
the host: the disposable HTTPS provider holds the known merge response while the
fixture sets `metadata.sqlite3*` to mode `0400`, so `ledger.settle` cannot land.

### Two lanes, and which one counts

| lane | executed | exit |
|---|---|---|
| `-p connectors-gitlab --test local_runtime -- --ignored --test-threads=1` | 12 → 13 | 0, 522.48 s |
| `-p connectors-gitlab --test local_runtime` (default) | 3 → 3 | 0, 6.71 s |
| `-p connectors-host` | 169 → 169 | 0 |

All 13 CLI journeys are `#[ignore]`d — they need the built CLI, GNOME keyring and
dbus. The default lane reports the new case as ignored, so **the default lane's
unchanged count of 3 is not evidence about this unit**. The `--ignored` lane is.

### Discrimination was measured, not assumed

The case passed on unmodified code, so the implementor injected the defect the row
forbids into `crates/connectors-host/src/local/owner/mutation/execution.rs` — on
settle failure, clear `result` and set `Unknown`/`outcome_unknown` — rebuilt the CLI
and watched the case fail at `guarded_merge.rs:562`. The injection was reverted and
the CLI rebuilt from reverted source before the runs recorded above. `git diff`
shows no host change.

### Coordinator items raised

1. `~/.cache/cv2-wave-20260912/unit-1/matrix-row.patch`, unapplied — the rewrite of
   the matrix row this journey now covers. Held until the adversary pass finishes,
   because a finding could change what the row should say.
2. The implementor edited `adapters/gitlab/tests/local_runtime.rs`, not only
   `guarded_merge.rs`: the shared `Provider` fixture gained two hold-until-released
   branches on the PUT handler. The brief named only `guarded_merge.rs`. With N=1
   nothing else in this wave touches that file.

Evidence logs, all under the assigned scratch root
`~/.cache/cv2-wave-20260912/unit-1/`: `base-journeys.log`, `base-host.log`,
`after-journeys.log`, `after-host.log`, `final-journeys.log`, `final-host.log`.
Nothing was written to `/tmp`.

## Unit 1 — adversary pass 1

Recorded as `review-result:adversary-settlement-pass-1-20260912`, body byte-for-byte
as the agent returned it, findings block included.

Verdict `red`, 3 findings, all `origin: introduced`. The red is not the unit's case:
the adversary re-ran it and it passed — `13 passed; 0 failed` in the ignored lane,
533.85 s, exit 0, the unit's case alone at 64.06 s. The two red cases are the
adversary's own additions, in `adapters/gitlab/tests/local_runtime.rs`.

| # | file:line | verdict | routed to | outcome |
|---|---|---|---|---|
| 1 | `docs/gitlab-write-failure-matrix.md:20` | NEEDS-CHANGE | coordinator | `fixed` |
| 2 | `adapters/gitlab/tests/local_runtime/guarded_merge.rs:546` | INFEASIBLE | implementor, correction 1 | pending |
| 3 | `adapters/gitlab/tests/local_runtime/guarded_merge.rs:517` | NEEDS-CHANGE | implementor, correction 1 | pending |

### Finding 1 — taken by the coordinator

The **Attempt terminal settlement** row still listed the CLI evidence as remaining
while the journey discharging it shipped in the same change. Rewritten from the
implementor's `matrix-row.patch`, plus the adversary's addition: the journey covers
one persistence-failure shape, a read-only metadata file that fails every read-write
SQLite open. Disk exhaustion and `SQLITE_BUSY` fail at commit rather than at open and
can leave the attempt completed, a different recovery path; both remain required
beside dedicated sandbox behaviour.

### Finding 2 — a fixture race the adversary could not reach in the real journey

The mode-9 half releases its held refusal on `sent == 1 && merge_effects == 0`.
`merge_effects` is zero before the request exists, so the condition reduces to "a PUT
appears in `methods`", recorded at `local_runtime.rs:102` before the body is read and
before the merge branch latches `merge_mode` at `:127`. The release can overtake the
hold and the fixture answers the released mode: 200 applied with a real provider
effect, in a journey asserting `forbidden` and zero effects.

What was measured: `local_runtime.rs:619`, exit 101, with the request body withheld.
What reaches it: *nothing found*. In the real journey reqwest sends headers and body
together, the window is a microsecond against a 10 ms poll, and the adversary did not
see it fire in two real mode-9 executions. Hence `INFEASIBLE` rather than `CONFIRMED`
— a flaky-fixture risk, not a product defect. Fixed anyway, because a fixture that
can answer the wrong mode makes every future mode-9 result unreadable.

### Finding 3 — a deletion mutant that survives

`story:guarded-gitlab-merge` names five inspections; the case does four. The missing
one is final disclosure admission. The post-settlement `issuance::resolve`/`unchanged`
block at `crates/connectors-host/src/local/owner/mutation/execution.rs:518` is reached
by no CLI journey in a failing state — `guarded_merge.rs:683` revokes while the
preflight is held, `background_recovery.rs:69` revokes in the observe path — so
deleting the block leaves the new case green.

Coordinator call: extend the case, test-only. If covering it needs a change under
`crates/`, the implementor stops and reports, and the story's sentence is corrected
instead. That correction would be the coordinator's write.

### What the adversary attacked and could not break

The PUT-count assertion is present at `guarded_merge.rs:640-647` — the brief's
suspicion that it was missing was wrong. The fault reaches the intended write:
`mutations.rs:89` opens a fresh read-write connection per transaction, so `0400` fails
at open, and `cause.stage == "attempt_store"` is set only at `execution.rs:512`. All
four row items have assertions. No cross-journey leakage: `merge_mode` 5 and 6 are new
values, the `0400` window is inside a per-journey tempdir. 13/13 ignored journeys pass
in one serialized run.

### Correction round 1 dispatched

To the **same** implementor, which still holds its context — no case has failed twice,
so the fresh-implementor rule does not fire. Brief:
`~/.cache/cv2-wave-20260912/unit-1-correction-1.md`.

Attack budget used: 1 of 2 passes.

## Stages

| stage | state |
|---|---|
| implementor pass 1 | green |
| adversary pass 1 | red, 3 findings, recorded |
| finding 1 | fixed by coordinator |
| correction round 1 | dispatched |
| adversary pass 2 | pending |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | pending |

## Cost so far

| agent | tokens | tool uses | duration |
|---|---:|---:|---:|
| implementor pass 1 | 222,098 | 102 | 2,323 s |
| adversary pass 1 | 162,504 | 50 | 977 s |

## Unit 1 — correction round 1

Verdict green. Default lane 5 passed / 13 ignored, exit 0, 16.82 s. Ignored lane 13
passed, exit 0, 498.75 s. Clippy exit 0, `cargo fmt --package connectors-gitlab
--check` exit 0. `git diff -- crates/` is empty — still no production source change.

Both findings fixed, and both fixes were shown to discriminate:

**Finding 2** was fixed one level below where it was reported. The implementor did not
change the break condition; it latched the merge behaviour **before** the request
becomes observable, ahead of `observed_methods.push`, so a recorded method now implies
the latch. It added `Provider::merge_held`, incremented on entering a hold (modes 5 and
6), and the journey now waits on that counter rather than on `methods`/`merge_effects`.
The PUT count and effect count became assertions after the wait instead of being the
wait. Reverting the latch to its original site puts the adversary's case back to red at
`local_runtime.rs:633`; restoring it returns green.

**Finding 3** became journey mode 10, inside the same case (`for mode in [8, 9, 10]`):
a known Applied effect held at the provider, `connections revoke` while metadata is
still writable, then metadata read-only, then release. It asserts the CLI refuses with
`revoked` rather than disclosing the known result, the record is still
`dispatching`/`pending`, one effect and one PUT, and a restarted CLI is refused the
same way with no further provider call.

The deletion mutant now dies. With the whole `execution.rs:518` block removed the case
fails at `guarded_merge.rs:591` on `assert!(!output.status.success())` — without that
block the CLI **succeeds and hands back the applied merge result on a revoked
connection**. Block restored, case green at 74.99 s.

`cargo fmt --check` failed first on the revoke call's layout; the runs above are the
post-format ones.

### Residual the implementor declared, not asserted

In mode 10 the abandoned attempt may later be quarantined by background maintenance.
The case asserts record state only at the deterministic point, immediately after the
live reply while metadata is still read-only, and asserts counts rather than record
state after restart. Passed to adversary pass 2 to judge whether that leaves a gap.

### Outcomes recorded

All three findings of `review-result:adversary-settlement-pass-1-20260912` recorded as
`review_outcome ... --outcome fixed` against `story:guarded-gitlab-merge`. Store now
holds `test_result=10, review_outcome=3` for that story.

### Adversary pass 2 dispatched

To the same adversary, which holds its pass-1 context and can check whether its own two
cases were weakened rather than satisfied — the first attack line in its brief.
Brief: `~/.cache/cv2-wave-20260912/unit-1-adversary-2-brief.md`.

Attack budget: 2 of 2. There is no third.

### Disk recovered

61 G free, 93% used, up from 15 G at 99%. Another session released the space; nothing
in this repository was deleted by this wave.

## Stages

| stage | state |
|---|---|
| implementor pass 1 | green |
| adversary pass 1 | red, 3 findings, recorded |
| correction round 1 | green, all 3 findings fixed |
| adversary pass 2 | dispatched |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | pending |

## Cost so far

| agent | tokens | tool uses | duration |
|---|---:|---:|---:|
| implementor pass 1 | 222,098 | 102 | 2,323 s |
| adversary pass 1 | 162,504 | 50 | 977 s |
| implementor correction 1 | 276,014 | 136 | 1,463 s |

## Unit 1 — adversary pass 2

Recorded as `review-result:adversary-settlement-pass-2-20260912`, body byte-for-byte as
returned, findings block included.

Verdict `red`, 3 findings, all `origin: introduced`. The ledger, from
`aep plan artifact findings story:guarded-gitlab-merge`:

```
carried 0:
new 3:
  - docs/gitlab-write-failure-matrix.md:22  warning  contract-drift
  - adapters/gitlab/tests/local_runtime/guarded_merge.rs:682  warning  mutant
  - adapters/gitlab/tests/local_runtime/guarded_merge.rs:694  warning  acceptance
resolved 3:
  - docs/gitlab-write-failure-matrix.md:20  warning  contract-drift
  - adapters/gitlab/tests/local_runtime/guarded_merge.rs:546  warning  concurrency
  - adapters/gitlab/tests/local_runtime/guarded_merge.rs:517  warning  acceptance
```

Trend: 3 then 3, **carried 0**. Every pass-1 finding closed; every pass-2 finding is new
ground. A correction that did not land would show as carried, and none is.

### What pass 2 confirmed rather than found

- Both pass-1 adversary cases are byte-identical to what the adversary wrote. No
  assertion relaxed, no string loosened, no wait made unfailable.
- The latch is ahead of the observable: `local_runtime.rs:112` latches, `:113` is the
  single `observed_methods` push, nothing branches or awaits between them.
- `merge_held` introduces no race. For mode 5 the effect increment precedes the hold
  increment; for mode 6 it implies zero. One PUT per journey, per-`Provider` counter.
- Mode 10's `revoked` comes from the post-settlement block, not an earlier admission
  check. Failure-arm mutants kill it, not only deletion.
- No flakiness: two full ignored lanes (577.57 s, 554.60 s) and four extra runs of the
  mode-8/9/10 case.
- Both implementor figures confirmed independently.

| # | file:line | verdict | routed to | outcome |
|---|---|---|---|---|
| A | `docs/gitlab-write-failure-matrix.md:22` | CONFIRMED | coordinator | `fixed` |
| B | `guarded_merge.rs:682` | NEEDS-CHANGE | implementor, correction 2 | pending |
| C | `guarded_merge.rs:694` | NEEDS-CHANGE | coordinator decision | pending |

### Finding A — taken by the coordinator

Mode 10 revokes the connection after a known Applied effect and checks disclosure is
refused with one PUT and one effect. That is the **Final result disclosure** row's
remaining evidence — "connection races after native response ... with effect counts and
safe response checks" — not the Attempt terminal settlement row. Row 20 was rewritten
for this delivery and row 22 was left byte-unchanged, so the table understated coverage
and overstated what remains.

Row 22 rewritten: its existing-evidence cell now names the mode-10 journey; its
remaining cell is narrowed to the policy and key races and the final-audit-handling
race, which mode 10 does not cover.

### Finding B — one assertion, and it must be shown to discriminate

`refusal(output, "revoked")` at `:682` discards its return value, so mode 10 asserts the
refusal code and nothing about its shape. Measured shape today:
`{"code":"revoked","kind":"operational","next_action":"create_connection","stage":"admission"}`
— no mutation projection. A regression attaching a projection, a classification or the
original attempt id to a refused disclosure of a merge that **did** apply would pass mode
10 unchanged. `background_recovery.rs:157` already asserts the projection is absent on
the sibling journey.

### Finding C — the residual does not hold, and the coordinator keeps mode 11

The correction-1 residual was that background maintenance might sweep the abandoned
attempt, so record state was not asserted after restart. Pass 2 measured the path: the
revoked restart run is refused at admission and starts no owner — `owner.sock` absent —
so no sweep can run. The record sat at `("dispatching", "pending", None, None)` for a
full 30 s poll. The state is fixed, not timing-dependent.

The adversary's mode-11 case
`adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted`
already asserts the deterministic invariant and is green.

**Coordinator decision: keep mode 11 as it stands.** Folding it into mode 10 would delete
the probe that produced findings B and C. The implementor may not remove or weaken it;
it reports whether the finding-B change makes any of mode 11's assertions redundant, and
that removal decision stays here.

### Correction round 2 dispatched

To the same implementor. **This closes the attack budget — two passes, no third.** The
coordinator verifies this correction directly, per the rule that the code answering the
second pass is code no adversary will see.
Brief: `~/.cache/cv2-wave-20260912/unit-1-correction-2.md`.

## Stages

| stage | state |
|---|---|
| implementor pass 1 | green |
| adversary pass 1 | red, 3 findings, recorded, all resolved |
| correction round 1 | green |
| adversary pass 2 | red, 3 new findings, carried 0, recorded |
| finding A | fixed by coordinator |
| finding C | decided by coordinator — keep mode 11 |
| correction round 2 | dispatched, last one |
| coordinator verification of correction 2 | pending |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | pending |

## Cost so far

| agent | tokens | tool uses | duration |
|---|---:|---:|---:|
| implementor pass 1 | 222,098 | 102 | 2,323 s |
| adversary pass 1 | 162,504 | 50 | 977 s |
| implementor correction 1 | 276,014 | 136 | 1,463 s |
| adversary pass 2 | 226,874 | 24 | 1,775 s |
| **total** | **887,490** | **312** | **6,538 s** |

## Unit 1 — correction round 2, and coordinator verification

Verdict green. Default lane 5 → 6 executed, exit 0, 16.72 s — the adversary's row-22
case turned green on the coordinator's rewrite. Ignored lane 14, exit 0, 512.58 s.
Clippy exit 0. `cargo fmt --package connectors-gitlab --check` exit 0 on the first try.

**Finding B** fixed at `guarded_merge.rs:682`: the refusal's return value is now bound
and `assert!(refused.get("mutation").is_none())` matches the idiom at
`background_recovery.rs:157`. Shown to discriminate — a probe attaching a `mutation`
projection to that refusal made the case fail with
`"classification":"not_attempted"` on a merge that had applied. Probe reverted;
`git diff -- apps/` is 0 lines.

**Finding C**: mode 11 kept byte-unchanged, per the coordinator decision above.

### Coordinator verification of the correction that answered pass 2

Required because this is code no adversary will see. Two checks, both done by reading
the diff:

1. **No assertion dropped.** The whole change across both test files removes 4 lines
   and zero assertions. The removals are the latch relocation and a `merge_mode` store
   widened into a `match` that preserves every prior mapping — `6 => 3` and
   `other => other` both survive, so modes 0 to 7 are unchanged.
2. **Nothing re-pinned was relaxed.** No adversary assertion was edited; both pass-1
   cases and mode 11 are byte-identical to what the adversary wrote.

`git diff -- crates/` is 0 lines and `git diff -- apps/` is 0 lines. This wave changed
no production source at all.

### Two items the implementor handed back, both decided here

**The same discarded-refusal pattern at `guarded_merge.rs:694`** — the restart refusal.
Not extended. `background_recovery.rs:157` already asserts the absent projection on that
same revoked-observation path, the implementor measured that the probe did not affect
it, and the attack budget is closed. Recorded here rather than left as a silent choice.

**Mode 11's live-reply assertion is now implied by finding B's fix.** Kept. The two
encode different policies: the new assertion forbids any projection on that refusal;
mode 11 permits one that reports `applied`. If disclosure of a known effect is ever
added on purpose, mode 11 stays green and the new assertion goes red, which is the
disagreement worth keeping.

The implementor also corrected its own round-1 report: its declared residual — that a
background sweep could move the record — was wrong for the revoked path, because no
owner starts. Its mode-10 assertions are unaffected; the stated reason was wrong.

## The gate did not run

Three attempts, all stopped by the harness with "the system is running low on memory".
None reached the compile and test step. **No `.exit` file was written by any of the
three**, so no attempt produced a gate exit status.

| attempt | build jobs | `--msrv` | steps that printed `exit=0` | outcome |
|---|---|---|---|---|
| 1 | 2 | yes | 5 | killed |
| 2 | 1 | yes | 0 | killed, under a minute |
| 3 | 1 | no | 6 | killed |

Steps green on attempt 3, quoted from `~/.cache/cv2-wave-20260912/gate-3.log`:

```
gate: shared ESS provider vocabulary and source ownership hold; exit=0
gate: shared and 6 adapter ESS models validated and compiled independently; exit=0
gate: exit=exit status: 0 ["fmt", … 16 packages …, "--", "--check"]
gate: gitlab descriptor matches; exit=0
gate: kubernetes descriptor matches; exit=0
gate: sql descriptor matches; exit=0
```

Attempt 1 additionally recorded `10 CLI artifact(s), current in
apps/connectors-cli-contract` and `CLI values: 81 structural, 2 cached expectations, 5
acquisition and 3 page consistency cases passed`.

**The memory claim does not match what was measured.** `free -g` immediately after each
kill: 62 total, 18 used, 43–44 available. Largest RSS on the machine was `k3s` at
878 MB. Two builds at one job do not consume 43 G. That the harness watchdog is acting
on system-wide state rather than this process's usage is **inferred**, not verified; no
measurement here establishes the cause.

Retrying was stopped after the third kill rather than continued. The gate was **not**
reconstructed from its separate commands — a set of commands that resemble the gate is
not the gate, and recording one as the gate is the failure this repository has already
had once.

## Consequences

- **Unit 1 does not merge.** Only green merges, and the gate's exit status decides.
  Nothing was committed: no unit commit, no closing store commit, no merge.
- The work is in the working tree, uncommitted. `git status --short`:

```
 M .engineering/planning/journal.jsonl
 M adapters/gitlab/tests/local_runtime.rs
 M adapters/gitlab/tests/local_runtime/guarded_merge.rs
 M docs/gitlab-write-failure-matrix.md
?? .engineering/planning/review-result/adversary-settlement-pass-1-20260912.md
?? .engineering/planning/review-result/adversary-settlement-pass-2-20260912.md
?? docs/plans/wave-2026-09-12-next-three.md
```

- `story:guarded-gitlab-merge` stays `active`. No lifecycle move was made, and none was
  available: the story needs dedicated sandbox evidence regardless of this gate.
- `aep plan artifact validate` exits 0 and reports `valid`, with the store's existing
  prose-only findings notices on legacy review-results unchanged.

## What the next session needs

1. Run `bash ~/.cache/cv2-wave-20260912/run-gate.sh` when the machine is quieter. It
   runs `cargo run --locked -p connectors-build -- gate --msrv` at one build job and
   writes `~/.cache/cv2-wave-20260912/gate-2.log` and `gate-2.exit`.
2. If it exits 0, commit the two test files and the two doc rows as the unit commit, and
   the two review-results and the journal as the closing store commit. That is the whole
   of what this wave's approval covers. No push, no tag.
3. If it does not exit 0, the unit stays out and the tree stays as it is.

## Stages

| stage | state |
|---|---|
| implementor pass 1 | green |
| adversary pass 1 | red, 3 findings, all resolved |
| correction round 1 | green |
| adversary pass 2 | red, 3 new findings, carried 0 |
| correction round 2 | green |
| coordinator verification of correction 2 | done — no assertion dropped, nothing relaxed |
| full gate with Rust 1.88 | **not run** — killed three times |
| commits | **none** |

## Cost

| agent | tokens | tool uses | duration |
|---|---:|---:|---:|
| implementor pass 1 | 222,098 | 102 | 2,323 s |
| adversary pass 1 | 162,504 | 50 | 977 s |
| implementor correction 1 | 276,014 | 136 | 1,463 s |
| adversary pass 2 | 226,874 | 24 | 1,775 s |
| implementor correction 2 | 298,541 | 156 | 4,074 s |
| **total** | **1,186,031** | **468** | **10,612 s** |

Executed-case counts, final: default lane 6, exit 0; ignored lane 14, exit 0.

## The gate, fourth attempt — green

Run in a later session, on the same working tree, with `run-gate.sh` unchanged:
`cargo run --locked -p connectors-build -- gate --msrv` at `CARGO_BUILD_JOBS=1`.
Launched under `setsid` so a harness kill could not take the build down with the
shell that started it. It was not killed.

| fact | value |
|---|---|
| exit | `GATE-EXIT:0`, `~/.cache/cv2-wave-20260912/gate-2.exit` |
| final line | `gate: all checks passed`, `gate-2.log:1690` |
| wall time | 09:17:33 → 09:25:29, 476 s |
| suites | 69 `test result: ok`, 0 failed |
| `free -g` at launch | 62 total, 16 used, 46 available |

Every step exit 0: ESS vocabulary, 7 ESS models, fmt across 16 packages, three
descriptor-drift checks, `build --workspace`, `test --workspace`, clippy with
`-D warnings`, three no-default-features library boundaries, the generic CLI
boundary, `+1.88.0 check --workspace --all-targets`, scenario synthesis over 34
authored scenarios, and `plan artifact validate`.

`local_runtime` default lane: 6 passed, 0 failed, 14 ignored, 7.30 s. The 14
ignored are the CLI journeys, including this unit's two; the `--ignored` lane
that executes them was run by the implementor and both adversary passes, 14
passed, and is not part of the gate.

The three earlier kills are unexplained. Nothing about the tree changed between
attempt 3 and this one, so the difference is machine state at the time, which
was not measured then and is not a finding now.

## Commits made

| commit | contents |
|---|---|
| `f79a61b` Cover failed settlement and post-effect revocation in the CLI journey | the two test files, the two matrix rows |
| this one, Close the settlement journey wave | the journal, the two review-results, this page |

On `main` in the primary checkout. No merge commit, no push, no tag, no release,
no version bump. That is the whole of what this wave's approval covered.

## Stages, final

| stage | state |
|---|---|
| implementor pass 1 | green |
| adversary pass 1 | red, 3 findings, all resolved |
| correction round 1 | green |
| adversary pass 2 | red, 3 new findings, carried 0 |
| correction round 2 | green |
| coordinator verification of correction 2 | done — no assertion dropped, nothing relaxed |
| full gate with Rust 1.88 | **green, exit 0** |
| commits | **2, on `main`** |

`story:guarded-gitlab-merge` stays `active`. It needs dedicated GitLab sandbox
evidence to close and `credential-blocker:gitlab-runtime-sandbox` is open, so
this wave produced an increment on an active story and closed nothing. Waves 2
and 3 remain held and unstarted.
