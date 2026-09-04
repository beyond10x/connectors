---
format: aep.planning-md/1
id: story:the-cli-runs-without-being-configured-by-hand
kind: story
status: active
title: The CLI runs without being configured by hand
refs:
- provider: legacy
  reference: S-035
relations:
- derived_from: epic:local-product
scope:
- confidence: cited
  path: crates/protocol
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-035-the-cli-runs-without-being-configured-by-hand.md:109`. **read**

- [x] `connectors init` writes a configuration the daemon reads back, at `0600`, refusing to
      overwrite silently, and refusing to leave an invalid file on disk.
- [x] Every personal-local subcommand runs without `--config` and `--state-root`.
- [x] `-o json` and `-o yaml` emit failures on stdout in that format; `text`/`compact` use stderr.
- [x] A Connector refusal exits non-zero and forwards the Connector's own error code.
- [x] Results render as their payload, not their transport envelope; `-o compact` is one record
      per line.
- [x] `connectors doctor` names a state root too deep to bind a Connect Session endpoint, and exits
      non-zero.
- [x] `operation invoke` accepts input from an argument, a file, or stdin.
- [x] `connectors providers` reports, per catalogued provider, its authority, credential
      requirement, required configuration fields, verify probe and operation count — so coverage is
      a measured fact rather than a claim.
- [ ] A one-shot command composes the runtime in-process when no daemon holds the state lock, so
      `connectors operation search` answers with nothing running. The in-process path must enter at
      the same `BackendRegistry` entry point, so there is exactly one authority path.
- [ ] `ConnectSessionStatus` carries a non-secret failure code. Today `state: Failed` is all a
      caller can see, so `integration-slack` logs its reason with `eprintln!` and a CLI can only
      print "failed". Needs `ConnectSessionTerminal::Failed` to carry a code, which touches every
      caller in `integration-slack` and `integration-jira`.

## Context

Make `connectors` a program someone can install and use on their own machine, with no deployment,
no Identity and no PostgreSQL — the first of the six stories that turn the local posture from a test
harness for the hosted service into a product. This story is the entry surface: writing a
configuration, finding out why something does not work, and getting results a script can read.

Source frontmatter: pillar Platform · areas [cli, config, protocol] · priority 1. **read**

Source `note:` field, quoted: “init/doctor/providers/output and correct exit codes have landed, with the operator surface split into connectors-console; the in-process one-shot and the Connect Session failure reason remain”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-035-the-cli-runs-without-being-configured-by-hand.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-035-the-cli-runs-without-being-configured-by-hand.md`, which is not deleted and now names this artifact.

- First written 2026-08-20 · last touched 2026-08-20 · 1 revision(s)
- Legacy id `S-035`, recorded as the reference `legacy:S-035`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
