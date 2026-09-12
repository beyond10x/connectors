# Wave proposal — helm release reads, 2026-09-12

Skill version 0.9.1. Coordinator: this session, in the primary checkout at `ae9950f`,
branch `main`, clean at dispatch. Operator granted standing approval — *work on waves
yourself* — which waives the stage-1 stop and nothing else.

## Verdict

**N is 1, and it is forced rather than chosen.** One story in the whole store is ready.

`aep plan artifact list --kind story --format json`, 59 stories:

| status | count |
|---|---:|
| implemented | 52 |
| active | 5 |
| draft | 2 |

Of the 7 that are not implemented, 6 are blocked, and the coordinator cannot clear any
of them. `aep plan artifact blocked`:

| blocker | blocks | who clears it |
|---|---|---|
| `credential-blocker:gitlab-runtime-sandbox` — open, withholding `test_result` | all 5 active GitLab stories | operator — sandbox URL, project, protected credential path |
| `tooling-blocker:kubernetes-driver-protocol-loading` | `story:kubernetes-spec-service` | a governed run that resolves `adp/1` |

That leaves `story:kubernetes-helm-release-reads`, `draft`, `blocked_by` empty.

## The operator's goal, against the store

The goal set during this session is **10 more stories handled**. It is not reachable from
this backlog: 7 stories exist that are not implemented, 6 of them blocked on something
only the operator or a governed run can supply. Reaching 10 requires **creating**
stories. `epic:mcp-contracts` is `draft` and undecomposed, and its eight contract
deliverables are authoring work that needs no provider, so a decomposer was dispatched
against it in parallel with this wave. That is recorded here as what the coordinator did,
not as a claim that it satisfies the goal.

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

Exit 0. Zero unassessed. The verb's wave 2 is excluded here for a reason the verb cannot
see: `story:kubernetes-spec-service` is blocked by `tooling-blocker:kubernetes-driver-protocol-loading`.
The collision it reports would have excluded it from this wave anyway — both stories land
on `adapters/kubernetes`.

The 5 active GitLab stories were not selected. `aep plan artifact waves --kind story
--status active` returns 5 waves and 57 collisions among them; every one is withheld by
the credential blocker and none can reach `implemented`.

## Scope, cited

`story:kubernetes-helm-release-reads` carries 7 typed scope entries, **all `cited`**, none
`inferred`:

```
adapters/kubernetes
adapters/kubernetes/contracts
adapters/kubernetes/generated/descriptor.json
adapters/kubernetes/spec
adapters/kubernetes/src
adapters/kubernetes/tests
docs/local-kubernetes-cli.md
```

No scoper was dispatched: the story already carries typed scope at full confidence, and
re-deriving it would have cost an agent and changed nothing. Blast radius is one adapter
plus one document.

## Pre-flight

| check | value | source |
|---|---|---|
| primary checkout | `ae9950f`, `main`, clean | `git status --short` empty |
| free disk | **51 G, 94% used** | `df -h .` |
| memory | 62 G total, 16 used, 46 available | `free -g` |
| network | reachable, HTTP 200 | `curl api.github.com` |
| measured gate | 476 s at `CARGO_BUILD_JOBS=1`, warm target, primary checkout | wave-2026-09-12-next-three.md |
| `aep` | protocol 0.55.0 | `aep --version` |

**Refusal taken and overridden, with the reason written down.** `git worktree list`
showed six trees from previous waves still standing, which the pre-flight treats as a
refusal. They were inspected rather than removed:

| tree | state | disposition |
|---|---|---|
| `cv2-ess3-adoption` | 8 uncommitted files, another session's | left alone |
| `wt-35ad348b4293` | clean, head `525ec4d` **not** an ancestor of `main` | left alone — it holds a commit `main` does not |
| `initiative-heading-fix-20260911` | clean, merged | `worktree gc` retains it |
| `kubernetes-local-cli-20260911` | clean, merged | `worktree gc` retains it |
| `kubernetes-rebase-20260911` | clean, merged | `worktree gc` retains it |
| `postgres-local-cli-20260911` | clean, merged | `worktree gc` retains it |

`worktree gc --dry-run` retained every tree it listed, machine-wide, on
`worktree-dirty` or `no-remote-recovery-proof`. This repository has no authorized
publication target — `AGENTS.md` § Workspace says a local recovery remote is not public
distribution, and `configuration-blocker:gitlab-v020-publication-target` is open on
exactly that question — so the four merged clean trees cannot produce the remote
recovery proof `gc` requires and cannot be retired by the manager. None of them collides
with this wave's branch or path. Proceeding with a uniquely named tree, and recording the
four as a standing cleanup that needs the publication question answered first.

## Unit 1

| | |
|---|---|
| story | `story:kubernetes-helm-release-reads` |
| objective served | `vision:independent-contract-adapters` |
| worktree | `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/cv2-helm-reads-20260912` |
| managed id | `cv2-helm-reads-20260912` |
| build output | that worktree's own `target/` |
| scratch root | `~/.cache/cv2-wave-b-20260912/unit-1/` |
| branch | `unit/kubernetes-helm-release-reads-20260912` |
| head at dispatch | `ae9950f` |
| integration branch | `wave/helm-reads-20260912`, created at `ae9950f` |
| brief | `~/.cache/cv2-wave-b-20260912/unit-1-brief.md` |
| `subagent_type` | `aep-drive:implementor`, then `aep-drive:adversary` |

**One worktree, not two.** N is 1, so the integration branch has nothing to integrate
that the unit branch does not already hold. The full gate will run in the unit's own
worktree after the adversary passes, on the integration branch fast-forwarded to the
unit's head. This saves roughly 6 G of build output on a disk at 94%. Recorded as a
deviation from the skill's separate-integration-checkout shape.

## Commits this wave makes

One opening commit (this page and the story's move to `active`), one unit commit, the
merge into `wave/helm-reads-20260912`, one closing store commit, and the merge into
`main`. No push, no tag, no release, no version bump. That is the whole of what the
standing approval authorises.

## Deviation recorded at dispatch

The skill says to write this page before dispatching. It was written immediately after
the two agents were launched, not before. Nothing was lost — every path, branch and id
above was assigned before dispatch and passed to the agents in the brief — but the page
did not exist for the minutes between.

## Stages

| stage | state |
|---|---|
| decomposer on `epic:mcp-contracts` | dispatched |
| implementor dispatched | done |
| implementor returned | pending |
| adversary pass 1 | pending |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | pending |

## Unit 1 — implementor returned

Verdict green. Package lane executed 11 → 20 exit 0; CLI journey lane 3 → 4 exit 0.
Committed by the coordinator as `5e4b0c4`, 31 files, +2699 −26.

The implementor did not commit, and was right not to: the operator's standing rule
forbids committing without the operator's own instruction and says a brief asking for
one does not grant it. The brief asked. The rule wins, the coordinator holds the wave's
approval, and the coordinator committed.

### Checks, each with its own exit status

ESS validate, ESS compile, shared ESS, adapter boundary gate, descriptor drift, clippy
with `-D warnings`, fmt across 15 members, library independence with
`--no-default-features`, `+1.88.0 check`, `build --workspace` — all 0.

### Discrimination was measured twice, because the first injection proved the wrong thing

The CLI journey case passed as written, so the implementor injected the defect the
acceptance forbids: emit the recorded literal in `value_digest`. That was caught by the
declared `^[a-f0-9]{64}$` pattern at the wire (`"service_code":"upstream_protocol"`) —
a real refusal, but not the assertion under test. The second injection put the literal
in `path`, which no schema constrains, and the journey's own assertion fired at
`cli_journey.rs:543`. Injection reverted and rebuilt before every recorded run;
`grep -c "DELIBERATE DEFECT"` returns 0.

### What the pin established, and what it un-established

Fourteen upstream Helm sources pinned with URL, uncompressed SHA-256 and byte length,
both lines — v4.3.0 at `bec5b06` and v3.22.0 at `144ca65` — because a cluster's Secrets
may come from either.

The story instructed that the release-to-revision cardinality be recorded `UNMAPPED:`,
on the reasoning that it could not be drafted honestly before a source was pinned. With
the source pinned it became readable, and the implementor cited it rather than obeying
the instruction: one release name to many version-keyed records (`storage.go:327-328`,
`:212-215`), and more than one may be `deployed` at once (`:171-175`), which is why the
status read returns them all. **This is the story's own condition being met, not an
instruction ignored** — the story said "it cannot be drafted honestly before the
upstream source above is pinned", and then it was.

Six relations stayed `UNMAPPED:`: cluster-wide release identity, revision to managed
resources, whether an observed history is complete, how a value path reads back, unset
against defaulted, and manifest document identity.

## Two environment facts this wave measured, that outlive it

| fact | effect | workaround |
|---|---|---|
| `~/.cargo/config.toml` sets `rustc-wrapper = /usr/bin/sccache`; under a `TMPDIR` inside a managed worktree sccache aborts `path must be shorter than SUN_LEN` before rustc runs | every cargo command in every managed worktree | export `RUSTC_WRAPPER=""`; builds are uncached, not wrong |
| a dbus fixture builds a Unix socket path from `TMPDIR`, capped at 108 bytes; the managed-worktree path is 93 | any CLI-journey lane is red **on untouched base code** under `<worktree>/.local/tmp` | give that lane a short `TMPDIR` under the wave's scratch root |

`SCCACHE_SERVER_UDS` does not fix the first — sccache derives a second path from
`TMPDIR` too. Both were relayed to every agent still in flight.

## Coordinator decision — the stale-surface patch, corrected before applying

The implementor found four documents claiming a three-operation Kubernetes surface that
this unit makes stale, none of them its file, and left a 57-line patch unapplied:
`README.md`, `docs/running-services.md`, `website/docs/adapters/kubernetes.mdx` and
`website/publication.json`.

Three of the four hunks are taken as written. The fourth is corrected: the patch gives
the new website page `"status": "implemented"`, and that string is not in this
repository's vocabulary. `website/publication.json` uses eleven status values across 46
entries; the closest existing and truthful one is
`"Read-only observation; dedicated sandbox acceptance open"`, already used once. This
unit's evidence is a fixture cluster, not a real one, and `AGENTS.md` § Verify requires
the website to keep specification, example and runtime support claims distinct. A
website page claiming `implemented` for a binding no real cluster has answered is the
runtime claim that rule forbids.

**Held, not applied.** The adversary is working in that worktree against commit
`5e4b0c4`; moving `HEAD` under it would change the baseline it was given. The patch
applies after the adversary returns.

## Stages

| stage | state |
|---|---|
| decomposer on `epic:mcp-contracts` | done — 12 stories, 2 blockers |
| critic panel, 2 rounds | done — 4 findings, all fixed |
| implementor pass 1 | green, `5e4b0c4` |
| adversary pass 1 | dispatched |
| stale-surface patch | held until the adversary returns |
| full gate with Rust 1.88 | pending |
| store evidence and body update | pending |
| commits | 1 unit commit made |

## Unit 1 — adversary pass 1

Verdict red. Executed 20 → 25, red 5. Eight findings, **all `introduced`, zero
pre-existing, zero undecided** — so every row routes to the implementor and none
becomes a separate story. Recorded as
`review-result:adversary-helm-reads-pass-1-20260912`.

| # | verdict | what reaches it | coordinator's row |
|---|---|---|---|
| 1 manifest digest over JSON encoding, not text | NEEDS-CHANGE | every `helm_releases.manifest` result | fix the code |
| 2 CLI doc forbids its own example | NEEDS-CHANGE | an operator reading the invocation table | fix the prose |
| 3 unbounded `path` against declared `maxLength 1024` | INFEASIBLE | *nothing found* | **fix anyway** |
| 4 empty recorded key against `minLength 1` | INFEASIBLE | *nothing found* | **fix anyway** |
| 5 non-object payload reports `complete:true` | INFEASIBLE | *nothing found* | **fix anyway** |
| 6 `value_digest` is an offline confirmation oracle | CONFIRMED | every `values` result | keep the property, document its limit |
| 7 evidence table claims ten read fields, two are read | CONFIRMED | the pinned-evidence artefact | correct the table |
| 8 `namespace` is the requested one, not the observed | INFEASIBLE | *nothing found* | fix anyway |

### Why three INFEASIBLE findings are being fixed rather than filed

The adversary traced 3 and 4 to `crates/connectors-host/src/local/runtime/process.rs:248-253`:
the host **terminates the local runtime child** when a result fails its own published
schema. The failure mode is a killed adapter, not a bounded refusal. An adapter that
provider-shaped input can kill is worth closing whether or not a chart in evidence
produces one, and the fix is a bound and a refusal.

5 is the unit's own argument applied one layer down. `semantics.md:68-72` refuses to
skip a foreign labelled object because `complete:true` would then claim a history
nobody observed; a payload decoding to `null` reports exactly that.

### Finding 6 is kept, not fixed

The `value_digest` equality property is what the digest is for, and the adversary said
so itself — asserting non-reproducibility would contradict a property the contract
states on purpose. What is missing is the warning, so the correction documents it:
the digest bounds disclosure of an unknown value and does not protect one an attacker
can enumerate. The two digests in this unit now differ on purpose, and the contract
says why — one exists to be reproduced, the other to be compared.

### What survived the attack

All 14 pinned digests and byte lengths reproduce from the archives; roughly 50 cited
lines say what the unit claimed, in both the v4.3.0 and v3.22.0 archives; v3 and v4
agree field by field; `resource_kinds` is untouched in effect as well as in the enum;
disclosure through `path`, `kind`, `bytes`, `index`, error messages and provenance is
closed; neither paging routine reports `complete:true` over a truncation; and the
three-outcome discrimination the acceptance turns on holds — scope refusal, RBAC denial
and empty history are distinct.

Routed to **the same implementor**, which still holds its context. No case has failed
twice, so the fresh-implementor row does not apply.

## MCP wave 1 — story:mcp-specification-pin

Implementor green, committed `4192883`, 58 files, +780. 54 archived specification files
across both revisions with URL, uncompressed SHA-256 and byte length. Adversary pass 1
dispatched.

The unit found a wrong citation in its own story: `initiative:complete-local-connectors:35`
for a sentence at line 33. Corrected at revision 5, with the old citation named in the
text rather than swapped silently.

It also resolved the authority question the story left open: the two dates are upstream
specification revisions, not releases of the sibling `../mcp` library, which declares
those same two strings as its own current and legacy protocol versions — a consumer's
selection, not their source.

**One unapplied patch is held for the integration branch**, not for this unit:
`~/.cache/cv2-mcp-waves-20260912/w1/source-hashes-gate-check.patch`, 291 lines. It
closes a class rather than a defect — this repository has six manifests recording
archived source digests and nothing re-derives any of them, so a manifest is evidence
only by assertion. The implementor verified all six by hand today and all six are
clean; the defect is the missing check. Measured with the patch applied:
`connectors-build` tests 20 → 22, clippy 0, and a new gate step printing
`84 archived upstream source(s) across 6 manifest(s) match their recorded uncompressed
digests and lengths; exit=0`. It adds no dependency and no lockfile change. It applies
after both units merge, so the full gate checks it against seven manifests including
the Helm one this wave created.
