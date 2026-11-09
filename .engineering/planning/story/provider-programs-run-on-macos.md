---
format: aep.planning-md/3
id: story:provider-programs-run-on-macos
kind: story
status: active
title: Grafana, Prometheus, Loki and Kubernetes programs run and ship on Apple-silicon macOS
summary: 'D370 items 1 and 3: macOS CI job, Grafana start fix, release binaries, READMEs'
refs:
- provider: github
  reference: beyond10x/connectors#229
- provider: github
  reference: beyond10x/connectors#231
- provider: github
  reference: beyond10x/connectors#232
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: .github/workflows/macos-host.yml
- confidence: cited
  path: adapters/grafana/Cargo.toml
- confidence: cited
  path: adapters/grafana/README.md
- confidence: cited
  path: adapters/grafana/tests/local_runtime/cli_journey.rs
- confidence: inferred
  path: crates/connectors-custody/src/keychain.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "proposed", to: "active", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 3, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
---
# Story: provider-programs-run-on-macos

## Outcome

An adopter on Apple-silicon macOS runs the Grafana, Prometheus, Loki and Kubernetes programs from
the release archive, and CI shows they work there.

## Context

D370, items 1 and 3; https://github.com/beyond10x/connectors/issues/229,
https://github.com/beyond10x/connectors/issues/231 and https://github.com/beyond10x/connectors/issues/232.
On macOS a pinned Grafana instance built from v0.64.0 answers `unavailable` at readiness. The
cause on macOS is not yet verified. `macos-host.yml` tests the host and the CLI only, and
`release-binaries.yml` packages `connectors` and `connectors-catalog-provider` only.

## Acceptance

- `macos-host.yml` has a job on an Apple-silicon runner that builds `connectors-grafana`,
  `connectors-prometheus`, `connectors-loki` and `connectors-kubernetes` and runs their test targets,
  each local-runtime journey included. A test that cannot run on macOS states why in the test itself.
- The Grafana journey (`adapters/grafana/tests/local_runtime.rs`) passes on that job, so a pinned
  Grafana instance connects and answers `datasources.list` there. The cause found is recorded in
  this story and fixed where it is.
- `release-binaries.yml` builds, version-checks and packages the four programs for
  `aarch64-apple-darwin` in that target's archive, covered by `SHA256SUMS`, and the gate that
  checks the binary list (`connectors-build release-binaries`) accepts it.
- The READMEs of the four programs state macOS support as the job proves it, and how to pin the
  program from the archive.

## Out of Scope

- Signing and notarization; x86_64 macOS archives of these programs; exec credential plugins
  (story:kubernetes-exec-plugin-on-macos).

## Checked 2026-11-03 (wave D417)

Read against `origin/main` `8150245c2` (v0.65.0) by wave D417; the code shipped in pull request #235.

- Held: the macOS `programs` job in `.github/workflows/macos-host.yml` ran clippy, the build and the
  tests of the four programs (run 41463903366); `release-binaries.yml` lists all four, held by
  `crates/connectors-build/src/release_binaries.rs`
  `the_release_workflow_packages_the_cli_the_catalog_provider_and_the_provider_programs`; the program
  READMEs state macOS support and the quarantine step.
- Gap: the Grafana CLI journey
  `cli_journey::grafana_cli_connects_with_a_service_account_token_and_lists_datasources_on_the_saved_connection`
  is `#[ignore = "requires built production CLI and qualified disposable Secret Service"]` and was
  reported ignored in that macOS run, so no Grafana start through the CLI ran on macOS.
- Gap: the cause of the reported macOS start failure is not recorded here; the only Grafana change is
  `#[command(version)]` in `adapters/grafana/src/main.rs`, and whether it fixes the reported failure is
  unproven.

The story stays active until both gaps are closed.

## Wave 2026-11-06: tried on macOS CI, taken out of 0.69.0

The wave ran the Grafana CLI journey on `macos-latest` against the login keychain (a macOS-only test sharing the
Linux journey body, run by name in `macos-host.yml`'s `programs` job). Two pull-request runs failed, at different
stages, and the unit was reverted from the release branch with its code kept on the unit commits (e77320412,
f0cec3305, 700c77675; merge 4e3f7da01).

- Run 1 (PR 242, run 42224840135): `setup check` and the wrong-token refusal passed; the first saving `connections
  connect` answered `custody_unavailable`, stage `custody`, `next_action` `unlock_keyring`. The custody job of the
  same run passed `macos_login_keychain_answers_the_custody_contract` from the test binary.
- Inferred cause of run 1, consistent with Apple's Security sources (review-result:wave-20261106-u1-ci1-security):
  `SecAccessCreate` with a NULL trusted list derives "the calling application" from the caller's executable on
  disk, and the owner runs from a private copy the CLI removes once the owner has answered. 700c77675 passed an
  empty trusted list instead (the item's ACLs end the same).
- Run 2 (run 42233699721, with 700c77675): the same connect got past custody and answered `outcome_unknown` at stage
  `publication`; the test's own search then found 0 items of its store. So the access was created, and either
  `SecItemAdd` returned a status other than success, duplicate or interaction-not-allowed (mapped to
  `OutcomeUnknown` in `keychain.rs` `write_new`), or the read-back after the add failed. Which one is not known.
- The failure-only diagnostics step showed the login keychain in the search list and `no-timeout` (unlocked,
  `show-keychain-info` exit 0). Its `log show` was cut by `tail -n 200` to lines after 14:35:51, after the failure
  at 14:35:46, and filled with AMFI lines for the host's `connectors-image.*/image` copies ("has no CMS blob?",
  "Unrecoverable CT signature issue, bailing out", then "bundle ... missing from disk"). The owner logs under the
  process name `image`, not `connectors`.

Next step for this story: log the `OSStatus` of the add and of the read-back in the owner (or in a macOS-only
custody test that runs the write from a binary deleted after start), and filter `log show` on the owner's process
and the failure minute, before another macOS run. Whether the owner's private copy being removed also breaks the
add or the read-back (code-signature or partition checks against a missing executable) is open.
