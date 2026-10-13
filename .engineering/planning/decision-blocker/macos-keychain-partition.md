---
format: aep.planning-md/3
id: decision-blocker:macos-keychain-partition
kind: decision-blocker
status: cleared
title: Nobody has decided what 0.46.0 ships for macOS when a second binary is refused
relations:
- blocks: story:platform-credential-custody
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T00:12:11Z", actor: "human:timo", revision: 3}
---
## Question

The macOS access decided as option A of `decision-blocker:platform-custody-limits` does not reach a
second binary on the CI runner. What does 0.46.0 ship for macOS?

## Observed

Run 38322932181 (PR https://github.com/beyond10x/connectors/pull/159, head d3518250e) on
`macos-latest`: the writer example stored an item with an access whose decrypt ACL admits any
application; the reader example, a second binary, read it and got `Unavailable` with OS status
-25293 (`errSecAuthFailed`); the writer then deleted it. Verified: the refusal. Inferred, not
observed: the cause is the item's partition list, which macOS has checked beside the ACL since
10.12. It names the creating binary, and changing it needs the keychain password.

## Options

| option | what | cost |
|---|---|---|
| A | Ship 0.46.0 with the any-application ACL as it is. The second-binary step records the outcome without failing the job. Upgrade access moves into `story:host-runs-on-macos`, where signing the release binary (a team-id partition that survives upgrades) is decided. | No user is affected: the CLI does not build for macOS before that story. A known gap stays open until then. |
| B | Hold the wave and try partition-list routes on the runner. | Each try re-runs the full pull-request CI (about 45 min). It may end at "needs the keychain password" anyway. |
| C | When a credential is saved, ask for the login keychain password and set the item's partition list to admit any binary of the user. | One more secret in terminal entry, and Connectors handles the user's keychain password. |

Recommended: A.

Default on silence: A.

## Decided

Option A, 2026-10-13: 0.46.0 ships the any-application decrypt ACL. The second-binary step in the macOS custody job records its outcome without failing the job. Upgrade access (an upgraded binary reading what an earlier one wrote) moves into story:host-runs-on-macos, together with the verified refusal (-25293, `errSecAuthFailed`) and its inferred partition-list cause. Signing the release binary needs an Apple developer account; it is an open item of that story. The release notes name the gap under known limits.
