---
format: aep.planning-md/3
id: release-plan:connectors-v0620-served-forge-access
kind: release-plan
status: implemented
title: 'Release 0.62.0: served forge access for credential-free agents'
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T14:42:23Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "approved", at: "2026-10-30T14:42:24Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "active", at: "2026-10-30T14:42:25Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-30T14:42:26Z", actor: "human:timo", revision: 5}
---
## Outcome

Minor release 0.62.0, the closing release of waves D243-C1 and D265-C4 under
`approval-record:d268-one-release-for-c0-c4`:

- The definition lowerer reads the four features connectors-definitions 0.5.2 publishes.
- Catalog auth scheme `github_app`: an instance authenticates as a GitHub App installation.
- A served host admits a Kubernetes service account token by TokenReview and serves catalog
  instances from a mounted credential.
- Opt-in keyed writes under a standing grant on a served host (D269).
- A served catalog instance relays git smart-HTTP with quarantine, ref policy, commit identity
  and a configured pre-push check.

No breaking changes.

## Scope

| story | status |
|---|---|
| story:definition-lowerer-reads-published-features | implemented |
| story:catalog-github-app-installation-token | implemented |
| story:served-instance-admits-a-service-account-token | implemented |
| story:served-keyed-writes-under-a-standing-approval | implemented |
| story:git-smart-http-relay-with-quarantine | implemented |

## Limitations

See the CHANGELOG's 0.62.0 Limitations. Locally the touched crates' tests and the projection
checks ran, under the shared target budget; CI's gate on the pull request is the full gate.
