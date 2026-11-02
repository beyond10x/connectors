---
format: aep.planning-md/3
id: story:provider-programs-run-on-macos
kind: story
status: implemented
title: Grafana, Prometheus, Loki and Kubernetes programs run and ship on Apple-silicon macOS
summary: 'D370 items 1 and 3: macOS CI job, Grafana start fix, release binaries, READMEs'
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 2, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "proposed", to: "active", at: "2026-11-02T07:27:32Z", actor: "human:timo", revision: 3, executor: "agent:connectors-d370", correlation: "connectors-d370-wave"}
- {from: "active", to: "implemented", at: "2026-11-02T15:48:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}}
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
