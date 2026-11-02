---
format: aep.planning-md/3
id: approval-record:connectors-d370-wave
kind: approval-record
status: approved
title: 'Wave D370: macOS adopter fixes'
summary: 'Four units: provider failure named, start failure reason, programs on macOS, exec plugin on macOS'
relations:
- decides: story:catalog-provider-failure-is-named
- decides: story:program-start-failure-reason
- decides: story:provider-programs-run-on-macos
- decides: story:kubernetes-exec-plugin-on-macos
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-02T07:27:31Z", actor: "human:timo", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
---
## Wave D370

Four units on the integration branch `integration/d370` (tree `connectors-d370`), from `origin/main`
`cf0135687` (v0.64.0 plus the glovebox plan merge). Targeted tests per unit, one local full gate
at the end, one pull request, then one release (0.65.0). A pre-release for the reporting adopter on
macOS is allowed if the release process supports one.

| unit | story | depends on |
|---|---|---|
| U1 | `story:catalog-provider-failure-is-named` | none |
| U2 | `story:program-start-failure-reason` | none |
| U3 | `story:provider-programs-run-on-macos` | U2 (the reason shows the macOS cause on CI) |
| U4 | `story:kubernetes-exec-plugin-on-macos` | none; its tests join the U3 macOS job |

Issues: https://github.com/beyond10x/connectors/issues/229 to
https://github.com/beyond10x/connectors/issues/234.

## Approval

The orchestrator session, 2026-11-02, item D370, under the grant of the workspace instructions
(decisions.md row D370): "Approved under the grant ... Timo asked for this (\"help him, use the b10x
chef session to fix if needed\", relayed by the brain session)." Scope: "fixes for 1 and 2; the
start-failure reason surfaced (instances status and/or a log file path); macOS (aarch64-apple-darwin)
prebuilt binaries for connectors-grafana, -prometheus, -loki, -kubernetes in the release workflow and
a macOS CI job running those programs' tests ...; README updates; eks exec-plugin credential path on
macOS. Scope inside D370 is decided; no Timo decision needed."

Later in the same item, 2026-11-02: "Folding the connectors-local/2 init/migrate fix into item 2:
approved. Also in scope: setup check tests the catalog provider runs ..., and the item-2 error
names the provider failure instead of invalid_input on endpoint. Signing/notarization of macOS
binaries is out of this wave (needs an Apple developer account = credentials/spend, Timo's);
document the xattr step in the README instead."

## Diagnosis at open

- Item 2, verified on Linux with the v0.64.0 x86_64 archive: a failed catalog provider derivation is
  reported as `invalid_input` on `endpoint` (`apps/connectors/src/local/endpoint.rs:343-352`, `:420`).
  `https://<host>` is the accepted GitLab form. On macOS the failing provider is inferred.
- Item 1, inferred: the start of a pinned program on macOS fails before or during the handshake. Its
  stderr goes to `/dev/null` (`crates/connectors-host/src/local/runtime/process.rs:84`). The cause
  is still to be verified on the macOS runner.
