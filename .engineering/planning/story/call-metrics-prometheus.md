---
format: aep.planning-md/3
id: story:call-metrics-prometheus
kind: story
status: draft
title: Per-call metrics a caller exports to Prometheus, with no credential or content in any label
revision: 1
---
## Outcome

A caller of the Connectors CLI exports per-call metrics to Prometheus: calls by provider, operation and outcome; a call-duration histogram; rate-limit waits; retries. No metric carries a credential, a request or answer body, a prompt or any document content; labels are drawn only from declared identifiers (provider id, operation id, outcome code, connection alias).

## Shape (to settle in the specification first)

The CLI runs per call and the owner daemon is optional, so a scrape endpoint alone would miss calls made without an owner. Proposed: every invocation already writes the local audit record; `connectors metrics --format prometheus` renders counters and histograms from it in the Prometheus text exposition format, for a node-exporter textfile collector or a caller's own scrape. A live endpoint on the owner is a later option, not this story.

## Metrics

| name | type | labels |
|---|---|---|
| `connectors_calls_total` | counter | `provider`, `operation`, `outcome` (applied, refused code, outcome_unknown, failed) |
| `connectors_call_duration_seconds` | histogram | `provider`, `operation` |
| `connectors_rate_limit_waits_total` and `connectors_rate_limit_wait_seconds_total` | counter | `provider`, `operation` |
| `connectors_retries_total` | counter | `provider`, `operation`, `reason` (transient, rate_limited) |

Label sets are bounded: an operation id comes from the loaded bundle, never from caller input.

## Acceptance

- The metric names, types, labels and buckets are declared in the ESS specification (spec first) and the CLI contract; the generated reference lists them.
- A test drives fixture calls (applied, refused, rate-limited with one wait, one transient retry) and asserts the exact exposition text, parsed by a Prometheus text-format parser.
- A test proves no label value comes from a request body, a credential field or an answer: a call whose inputs carry a marker string produces exposition text without it.
- `docs/` documents the command, each metric and the textfile-collector setup, with the release link.
