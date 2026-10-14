---
format: aep.planning-md/3
id: release-plan:connectors-v0490-windows-host
kind: release-plan
status: implemented
title: 'Release 0.49.0: the CLI on Windows'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-13T15:32:14Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-13T20:14:31Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Minor release 0.49.0: the CLI and its local host build and run on Windows, with credentials in the
Windows Credential Manager and a consumer credential delivered as one inherited read-only handle.

Minor rather than patch: a new platform, Windows rows for the seven platform guarantees, and a
consumer contract section for Windows.

## Scope

| surface | change |
|---|---|
| `crates/connectors-host` | named-pipe owner transport with a salted name and integrity-aware peer admission; `DuplicateHandle` passing; held executable copies with the PE header check; kill-on-close jobs; owner-only files by SID and access control list; `MoveFileExW`; interrupt-time clocks; console entry; owner-only scratch under the state directory |
| `ess/domains/cli.yaml`, `contracts/cli/v1alpha1/*` | Windows guarantee rows and weaker forms, `connectors.cli.ConsumerCredentialDelivery`, Windows sections of the owner, private adapter, consumer launch and semantics contracts |
| `apps/connectors/tests`, host tests | Unix-only fixtures gated; Windows cases |
| `crates/connectors-build` | Windows fixture handling; platform-gated tests named in the ignored inventory |
| `.github/workflows/windows-host.yml` | workspace build, host and CLI suites on `windows-latest` for every pull request and push to `main` |
| CHANGELOG, README | Windows support and its limitations |

Stories: `story:host-runs-on-windows`; reviews `review-result:windows-host-unit-a-adversary-20261013`,
`review-result:windows-host-unit-b-adversary-20261013`.

## Evidence

- Package gates on the integrated head before the push: Linux clippy and tests of connectors-host
  and connectors-build, Windows cross-check and clippy of connectors-host and connectors with 0
  errors, macOS clippy exit 0, specification valid with ess 0.59.0, generated docs current.
- The Windows cases first run on `windows-latest` in the wave pull request; the run is recorded on
  the story when it passes.

## Completion boundary

- No Windows release binary is built.
- A consumer's own children end with it on Windows; executables run from a copy and are
  self-contained; an inheritable handle exists during one creation; leftovers of a crashed
  elevated session are refused until removed; an elevated CLI and a normal owner of one user
  refuse each other until the owner retires.
- macOS release signing waits on the signing secrets.
