---
format: aep.planning-md/3
id: release-plan:connectors-v0500-store-version-skew
kind: release-plan
status: implemented
title: 'Release 0.50.0: a store a newer binary wrote is refused by name'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-14T02:43:51Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-14T22:05:54Z", actor: "human:timo", revision: 3}
---
## Outcome

Minor release 0.50.0: a binary refuses by name a metadata store a newer binary with another record
model wrote (`metadata_schema_newer`, `next_action = upgrade`, both versions named) instead of
answering `metadata_unavailable` with `retry_status`; and the OAuth consent loopback answers a
request queued before the flow returns.

Minor rather than patch: a new failure code, two new failure fields and a new persisted record
(`connectors.cli.StoreWriter`).

## Scope

| surface | change |
|---|---|
| `ess/domains/cli.yaml`, `contracts/cli/v1alpha1/semantics.md` | the refusal code, its fields, the writer marker, SemVer ordering of store versions |
| `crates/connectors-host` | the marker written in each write's batch and never lowered; the check on every open, the owner path and the HTTP host's attempt reads; a newer database level and an undeclared record field named as a newer store; the OAuth drain's last poll |
| `apps/connectors` | the failure's presentation and JSON fields |
| `website/docs/guides/set-up-the-local-cli.md` | sharing a state directory across versions; a pinned consumer's own `--state-dir` |

## Evidence

- Package gates on the unit head 21897f37e: fmt, clippy and tests of connectors-host and
  connectors (605 passed, 0 failed, 39 ignored), specification valid with ess 0.59.0, generated
  docs and the CLI contract current, the local-metadata-authority conformance baseline 306 of 306.
- Adversary review `review-result:store-version-skew-adversary-20261014`: two findings fixed, one
  narrowed, one kept.
- The integrated head's package gates and the pull request's CI are recorded when they pass.

## Completion boundary

- Releases up to 0.49.0 cannot read the marker: on a store a newer binary wrote they keep
  answering `metadata_unavailable` with `retry_status`.
- An unlocked observation write is not re-checked against a writer that raised the marker after
  the open; it records only its own clock.
- macOS release signing waits on the signing secrets.
