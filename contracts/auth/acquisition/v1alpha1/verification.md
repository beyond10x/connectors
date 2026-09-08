# Refresh coordination verification

This verifies the **proposed semantic contract** for `story:contracts-refresh-coordination` (F04), against opening commit `1e567571d9ac62070933c7099b93a4e030613e58`. It adds no runtime refresh implementation. ESS 0.20.0 compiles declarations and authored expectations; it does not execute a Connectors target, crash a process, arbitrate replicas or count provider calls. Runtime cases executed: **0 → 0**. The unit gate is semantic; the coordinator runs the full Rust integration gate and records the separate adversarial review before closing the story.

The private [refresh domain](../../../../ess/domains/refresh.yaml) references the shared [credential generation](../../../../ess/domains/credentials.yaml). The optional candidate reference is zero-or-one before storage and fixed thereafter. A refresh attempt is a real durable coordinator record. Its `Reserved`, `Authorized`, `ResponseStored`, `Recovering`, `Published`, `Fenced`, `Uncertain` and `Discarded` states distinguish recovery authority. Trusted transaction decisions are values, not extra persisted evidence entities or public permissions.

The first acceptance-bearing scenario was authored before the refresh declarations; its missing-entity/command/view failures are preserved below. Authoring subsequently refused multiple unconditional command outcomes; explicit typed transaction decisions and predicates resolved that declaration ambiguity. A later authored error payload tried to compare a captured reference, which ESS 0.20.0 refuses (`ESS-AUTHOR-021`); `OwnershipConflict` now carries the concrete private reason `stale_owner`, preserving the refusal assertion without an unsupported reference comparison. These compiler diagnostics are not runtime defects or proof of real exclusion.

## Semantic decision matrix

The source scenarios below compile typed ledger expectations. “Observed” is the required future runtime observation, not an observation from this semantic gate.

| Scenario | Chosen rule and required observation | Authored source / remaining runtime check |
|---|---|---|
| Two replicas contend | One reserve/authorize wins; contender has no send authority | [two replicas](scenarios/two-replicas-one-authorization.yaml); exercise atomic per-source index and count provider calls |
| Loss while durably Reserved | Fence old attempt before a successor reserves; old authorization fails | [fence before successor](scenarios/reserved-owner-fenced-before-successor.yaml); concurrent CAS and stale process |
| Authorization beats owner-loss fence | Even loss before send cannot reclaim an Authorized attempt | [authorization wins](scenarios/authorization-wins-fence-race.yaml); inject crash immediately before send |
| Loss after possible send | Quarantine, require repair; another attempt on source is refused | [authorized owner loss](scenarios/authorized-owner-loss-no-second-exchange.yaml); rotate then drop response |
| Response not committed or arrives late | Quarantine remains terminal; a late result cannot reopen it | [late response](scenarios/late-response-after-quarantine.yaml); memory/custody/metadata fault boundaries |
| Response storage wins recovery race | Quarantine loses; recover only the committed candidate | [stored response wins](scenarios/durable-response-wins-owner-loss-race.yaml); durable write/metadata CAS race |
| Stale owner publishes after transfer | Refuse stale fence; successor can only publish the same validated candidate | [stale publication](scenarios/stored-response-recovery-rejects-stale-publisher.yaml); actual owner/fence and candidate comparison |
| Revocation wins publication race | Refuse publication and discard; never clear revocation | [revocation first](scenarios/revocation-before-publication.yaml); serialize publication and revocation on current binding |
| Replacement wins publication race | Refuse stale binding and discard | [replacement first](scenarios/binding-replacement-before-publication.yaml); CAS active source and revision |
| Candidate evidence expires/invalidates | Refuse despite durable bytes; repair rather than re-exchange | [candidate evidence](scenarios/expired-candidate-before-publication.yaml); derive F05 validity in publication transaction |
| Recovery owner also lost | Discard/repair in first profile, never re-exchange | [recovery owner loss](scenarios/recovery-owner-loss-never-reopens-source.yaml); detect loss without treating lease expiry as authorization |
| Publication reply lost | Read committed Published state; source stays consumed | [publication reply loss](scenarios/published-reply-loss-no-reexchange.yaml); response loss after metadata commit |
| Publication wins revocation race | Revoke the new active generation; no new dispatch after revocation | Prose §4.1 plus F05 admission scenarios; no revocation entity/command invented in this unit |
| Authorization or publication cutoff versus dispatch | Atomically invalidate old-generation admissions; an already-opened dispatch is not rolled back | Prose §4.1 and F05; actual dispatch gate coupling is UNMAPPED |
| Same token aliased/re-captured | No second independent refresh authority, including across restart or another connection | Prose §4.1; opaque UUID uniqueness is insufficient, material-association enforcement is UNMAPPED |
| Provider/custody failure after authorization | Consumed source remains unavailable; unusable candidate requires repair | Prose failure matrix; failure injection and safe public-outcome mapping remain runtime obligations |

## Scope and limits

All three cited files were read: acquisition, custody and the Atlassian adapter design. All inferred private paths were checked: `ess/domains/refresh.yaml` was an empty registered domain; the scenario directory and verification record were absent and are the required new authoring surfaces. No inferred path was wrong. The prescribed mechanism is conservative: moving the durable boundary before the send trades potentially unnecessary reauthorization after a pre-send crash for exclusion of a second uncertain exchange.

The source-generation relation and optional candidate-generation relation are explicit. Current ownership after recovery, candidate-field assignment, per-source uniqueness/consumed retention, alias detection, non-replayable send authority, lease expiry, byte capture/durability, candidate identity/scope validation, publication/revocation/dispatch atomicity, and real provider-client retry suppression remain **UNMAPPED executable obligations**. Typed transaction decisions are supplied by the trusted coordinator in the same atomic operation, not evidence that ESS derives these facts. A caller-supplied `allow` would violate the contract; no public method accepts these values. No declaration claims that an admitted command alone proves these runtime properties.

Custody still exposes only `write_new/read/delete`; coordination belongs to host metadata. Baseline CAS is distinguished from the stronger rotating-refresh transaction. The adapter design references this same policy without adding runtime scope or a provider-grace exception. The public wire/version decision, read-retry decision, broader acquisition flows, binding implementation and provider extraction remain owned by their other stories.

## Compiler evidence

Refresh authored compilation: **0 → 12**, refusals **7 → 0** (the first run used one scenario before declarations; final run contains twelve). Synthesis: **132 compiled**, comprising **120 generated + 12 authored**. Existing operation scenario compilation: **15 → 15**, zero refusals on both opening model and changed model. Unchanged regression counts are expected because this unit adds its cases to the refresh directory. Runtime execution remains **0 → 0**, with no runtime conformance claim.

Commands below ran from the assigned refresh worktree with `TMPDIR` set to its `.local/waves/auth-hardening-20260908/refresh` scratch directory and `CARGO_BUILD_JOBS=2`; no shared target directory was configured. No Rust file changed in the unit, so no unit Rust build/lint/test lane was run. `git diff --check` is the applicable whitespace check; the coordinator owns formatting/clippy/full integration verification.

### First acceptance scenario before declarations

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/red-authored.json
refusal[ESS-AUTHOR-005]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.RefreshAttempt` is not an entity this specification declares
  help: name an entity the specification declares; an authored scenario acts on the model's own instances and invents none
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.ReserveRefresh` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.AuthorizeExchange` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.QuarantineRefresh` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.AuthorizeExchange` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.ReserveRefresh` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-012]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.RefreshStates` is not a view this specification declares
  help: name a view the specification declares
0 authored scenario(s) from 1 file(s), 7 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/red-authored.json
exit: 1
```

### Final unit gate and baseline regression

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path .local/waves/auth-hardening-20260908/refresh/baseline-ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/baseline-operations.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/baseline-operations.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 7 file(s), valid
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/waves/auth-hardening-20260908/refresh/ir.json
connectors v1 — 7 file(s), 76 declaration(s), compiled to .local/waves/auth-hardening-20260908/refresh/ir.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/authored.json
12 authored scenario(s) from 12 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/authored.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/synthesized.json
132 scenario(s) (12 authored), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/synthesized.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/operations-authored.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/operations-authored.json
exit: 0
```

```text
$ git diff --check
exit: 0
```


## Adversarial extension

A separate adversary added [revocation-before-authorization](scenarios/revocation-before-authorization.yaml) and returned no findings in `review-result:auth-refresh-adversary-p1-20260908`. The original unit command outputs above are historical. After the addition, the private author lane compiled 13 scenarios and synthesis compiled 133 (13 authored), with zero refusals; each command exited0. The existing operations lane remained15. Runtime execution remained0. The immutable review contains the exact commands, outputs and source-write audit; the [wave record](../../../../.engineering/planning/specification/auth-hardening-wave-20260908.md) owns combined gate evidence.
