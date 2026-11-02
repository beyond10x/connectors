---
format: aep.planning-md/3
id: story:catalog-provider-failure-is-named
kind: story
status: active
title: A catalog provider that cannot run is named, not blamed on the endpoint
summary: 'D370 item 2: provider failure refusal, setup check, init writes local/5'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-02T07:27:31Z", actor: "human:operator", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "proposed", to: "active", at: "2026-11-02T07:27:31Z", actor: "human:operator", revision: 3, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
---
# Story: catalog-provider-failure-is-named

## Outcome

An adopter whose catalog provider cannot run is told so, with the provider's path and the reason,
instead of being told the endpoint they gave is invalid. `setup check` does not report ready while
every catalog connect would fail. A configuration that `setup init` creates takes `connect --endpoint`.

## Context

D370, item 2; https://github.com/beyond10x/connectors/issues/230. `connections connect --endpoint`
derives the instance by running `connectors-catalog-provider` from the directory of the running
`connectors` (`crates/connectors-host/src/local/config.rs:1504-1514`). Every failure of that run is
`invalid_input` naming `endpoint` (`apps/connectors/src/local/endpoint.rs:343-352`, `:420`).
Reproduced on Linux with the v0.64.0 archive: `connectors` alone in a directory refuses
`https://<host>`, `https://<host>/` and `https://<host>/api/v4` alike. `setup init` writes
`format = "connectors-local/2"`, which `connect --endpoint` refuses (`endpoint.rs:278-290`), and
`setup migrate` keeps /2.

## Acceptance

- With no provider beside `connectors`, or with a provider that exits non-zero, is killed or prints
  nothing, `connections connect <c>/<i> --endpoint https://<host>` is refused with a code other than
  `invalid_input`. The refusal names the stage `catalog_provider`, the provider path, and one reason
  out of a bounded set (`missing`, `not_executable`, `killed`, `exited`, `deadline`, `unreadable_output`).
- An endpoint the provider itself refuses (`https://<host>/api/v4` for GitLab) is still
  `invalid_input` naming `endpoint`.
- `setup check` lists a `catalog_provider` prerequisite: `ready` when the provider beside
  `connectors` prints its version, `failed` with the reason otherwise.
- `setup init` creates a `connectors-local/5` configuration, and `connect --endpoint` on it writes
  the instance. `setup migrate --apply` moves a `/2` file with no legacy entries to `/5`.
- The GitLab onboarding page and the README state the accepted endpoint form, and on macOS the
  `xattr -dr com.apple.quarantine <dir>` step for the unsigned archive.
- Each line above is held by a named test in `apps/connectors/tests`.

## Out of Scope

- Signing and notarization of the macOS binaries (Apple developer account; operator's).
