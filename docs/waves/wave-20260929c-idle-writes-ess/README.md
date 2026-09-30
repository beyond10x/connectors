# Wave 2026-09-29 (third): owner idle exit, provider refusals on writes and timeouts, ESS 0.44.0

Skill version: aep implementing 0.18.0. Approved by the operator ("approved next /aep:wave").

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:owner-idle-exit | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/owner-idle-exit | `<managed-trees>`/wave0929c-idle | `<tree>`/target | `<wave-scratch>`/idle | merged (b8c0fea84), target deleted |
| story:provider-refusal-stage-for-writes-and-timeouts | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/provider-refusal-stage-for-writes-and-timeouts | `<managed-trees>`/wave0929c-writes | `<tree>`/target | `<wave-scratch>`/writes | merged (509e98f9f), target deleted |
| story:ess-pin-newest-release | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/ess-pin-newest-release | `<managed-trees>`/wave0929c-ess | `<tree>`/target | `<wave-scratch>`/ess | merged (6fe1bb813), target deleted |

Integration branch: `wave/20260929c-idle-writes-ess` off `main` 8b04b0b20 (release v0.16.0).

## Decisions

- owner idle exit: an owner with no client and no child work for 10 minutes exits; the bound has a
  test-only override.
- writes and timeouts: see the story's "Decided for the wave".
- ESS: target 0.44.0.

## Left out

- owner-refuses-buildless-callers: shares `owner/transport.rs` and `owner.md` with owner-idle-exit; next.
- configuration-change-error, revision-conflict-wire-code: share CLI failure files with the writes unit.
- catalog-confluence-reads: decision-blocker:confluence-openapi-redistribution.

## Commits approval authorises

One commit per unit (3), the merges into the integration branch, the closing planning-store commit,
the merge into `main` through a pull request once the single wave gate is green, then the next release.

## Close

- Gate: `cargo run -p connectors-build -- gate --msrv` at `b8c0fea84`, `CONNECTORS_ESS` = ESS 0.45.0:
  `gate: all checks passed`, exit 0; 95 suites, 699 passed, 0 failed, 33 ignored; local metadata
  authority conformance 289 scenarios, 20 synthesis refusals.
- Implemented: owner-idle-exit, provider-refusal-stage-for-writes-and-timeouts, ess-pin-newest-release.
  Scopes rewritten from each unit commit (all cited).
- Filed during the wave: beyond10x/ess#251 (ESS-SYNTH-003 since 0.43.0).
