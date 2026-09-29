# Wave 2026-09-29 (third): owner idle exit, provider refusals on writes and timeouts, ESS 0.44.0

Skill version: aep implementing 0.18.0. Approved by the operator ("approved next /aep:wave").

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:owner-idle-exit | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/owner-idle-exit | `<managed-trees>`/wave0929c-idle | `<tree>`/target | `<wave-scratch>`/idle | dispatched |
| story:provider-refusal-stage-for-writes-and-timeouts | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/provider-refusal-stage-for-writes-and-timeouts | `<managed-trees>`/wave0929c-writes | `<tree>`/target | `<wave-scratch>`/writes | dispatched |
| story:ess-pin-newest-release | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/ess-pin-newest-release | `<managed-trees>`/wave0929c-ess | `<tree>`/target | `<wave-scratch>`/ess | dispatched |

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
