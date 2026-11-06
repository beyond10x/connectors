---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-06
kind: approval-record
status: approved
title: Wave 2026-11-06 (0.69.0) go and acceptance from the orchestrator
relations:
- decides: story:provider-programs-run-on-macos
- decides: story:owner-bounded-read-realization-matches-projection
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-11-06T08:31:07Z", actor: "human:timo", revision: 2}
---
## Go (orchestrator, 2026-11-06)

The directing session `b10x-chef-orchestrator` gave the go for connectors wave 2026-11-06, release 0.69.0, and accepted one draft into it:

> Go for wave-2026-11-06-connectors, release 0.69.0 (your proposal).
> - U1 story:provider-programs-run-on-macos as proposed: macOS proof only through GitHub macos-latest CI on the wave PR, no device run; no new #[ignore]. Second gap: option A — on green CI record the non-reproduction on macos-latest, point to story:macos-release-provider-starts-through-the-private-copy, move to implemented.
> - U2: I accept draft owner-bounded-read-realization-matches-projection into this wave (move it draft->proposed->active and cite this message as the acceptance). bounded.rs only, ~5G/15 cargo min. U1 and U2 build one at a time.
> - Keep owner-invoke-frame-effects-boundary and mcp-projection-cases-carry-read-or-mutating as drafts.
> - Limits: tell at 8G or 20 cargo min, stop at 10G or 30 per unit; gate S1-S5c serial with your stage figures (S4a 13G stated), BUILD_SLOTS=2 for stages only, discard between stages.
> - Adversary per unit before merge; one gate, one bot PR, merge on green, tag, release.

## What it authorises

- U1: story:provider-programs-run-on-macos. The Grafana CLI journey runs on the macOS `programs` job of `.github/workflows/macos-host.yml` against the login keychain. Its only macOS proof is that job on the wave pull request.
- U2: story:owner-bounded-read-realization-matches-projection, accepted by this message (draft to proposed to active).
- The unit commits, their merges into `wave/2026-11-06-connectors`, the store commits, the 0.69.0 release-prepare commit, the push, one bot pull request, the merge on green, tag `v0.69.0` and its release page.

## Left out

- story:sustained-live-read-loop: credential-blocker:live-read-loop-no-configured-connections is open.
- story:macos-release-provider-starts-through-the-private-copy: the kill does not reproduce on macos-latest; its cause needs the adopter's answers from its own machine.

## Delivered (2026-11-06)

Released as v0.69.0: pull request https://github.com/beyond10x/connectors/pull/242, merge commit b64fc27ae on
`main` (tree equal to the wave head db0bca03f), tag v0.69.0 (peels to b64fc27ae, tagger b10x-bot[bot]).

| unit | merge | review | result |
|---|---|---|---|
| U2 bounded read follows the projection | 9413efbdd (f42d53975, 9a73f986e, 5f6f513b2, f0528c178) | adversary green, security green; one doc sentence fixed in the unit | story:owner-bounded-read-realization-matches-projection implemented |
| U1 Grafana CLI journey on macOS | 4e3f7da01 (e77320412, f0cec3305), 700c77675; reverted by db0bca03f | adversary green (4 notes, 3 fixed); security green on 700c77675 | two red macOS runs; taken out; story:provider-programs-run-on-macos stays active with both runs recorded |

Local staged gate on 9413efbdd: S1 318, S2 1540, S4a 1200, S4b 281 passed, S5a-c clippy green; S3 563 passed and 2
failed in `connection_lease_adversary` (load-sensitive; routed by the orchestrator to
story:lease-early-recollection-relaunch-margin). Peak 13.6G (S4a). Pull-request CI: all 10 checks green on
db0bca03f after re-running two macOS jobs the runner never acquired and the Windows clock-check flake
(story:windows-clock-check-bracket-flake).

Drafts filed by the wave: story:owner-bounded-generic-read-inline-schemas, story:lease-early-recollection-relaunch-margin.
