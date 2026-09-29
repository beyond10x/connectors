---
format: aep.planning-md/3
id: release-plan:connectors-0-14-0
kind: release-plan
status: active
title: 'Connectors 0.14.0: released ESS and AEP, newest dependencies, --version'
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-09-29T00:53:05Z", actor: "human:timo", revision: 2}
---
## Scope

Release 0.14.0 of `beyond10x/connectors`, cut from `main` after PR #42 and the release PR.

- Pinned tools are releases, not source builds: ESS 0.40.0 and AEP 0.65.0 from `PATH`
  (the Beyond10x plugins install them). The `toolchain` and `aep-toolchain` build commands,
  build receipts and the local AEP findings patch are removed; unpatched AEP 0.65.0 validates
  this store with 0 problems. Planning CI installs the AEP release archive checked against
  `SHA256SUMS`.
- Dependencies: ESS crates 0.40.0, Entity Runtime 0.25.1, Eventlog 0.6.0; base64 0.23,
  hmac 0.13, jsonschema 0.58, reqwest 0.13.5 (rustls with `ring`), sha2 0.11, toml 1.1.
- `connectors --version` / `-V` answer `connectors <version>` with exit 0 (was exit 2,
  `cli_parse`).
- The production metadata lifecycle-lock wait is 30 s (`WAIT_BOUND`), was 2 s.

## Compatibility

- Without a configured `ca_file`, HTTPS trust comes from the platform verifier instead of
  bundled webpki roots.
- `connectors-build toolchain` and `aep-toolchain` no longer exist.

## Evidence

- PR #42: `connectors-build gate --msrv` exit 0, 80 suites, 538 passed, 0 failed, 25 ignored;
  CI shared source gates, planning validate and docs check green; merged as `84ac9cf04`.
- Version-flag commit `7733b2350`: `gate --msrv` exit 0, 80 suites, 540 passed, 0 failed,
  25 ignored.

## Not in scope

No MCP work, no sandbox acceptance, no binary or container publication. The three open
decision blockers and the MCP stories are unchanged.
