---
format: aep.planning-md/3
id: epic:incident-collection-workflow
kind: epic
status: draft
title: Operations an incident workflow calls to collect its data without a model
summary: Typed, concurrent reads for a call, its traces, metrics, alerts and incidents, deployments.
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

An incident troubleshooting workflow collects its data through Connectors deterministically and
concurrently, with no model in the loop: each step is one admitted operation, called with typed
input, returning typed output that a later step reads. The workflow (built on a workflow SDK, not
in this repository) decides the order; Connectors supplies the operations, their output shapes
and the guarantee that independent reads can run at the same time.

The measure the workflow is judged by is model calls, tokens and cost per incident; every piece of
data a person or a model would otherwise look up by hand should be reachable as one operation.

## The data an incident needs, and where it comes from

| step | data | source | in this store |
|---|---|---|---|
| 1 | a call by its number: one line per leg, with direction, numbers and timing | the SIP proxy's CDR lines in Loki, joined to the platform's call table (SQL on a read replica) on direction, numbers and a time window; Homer SIP capture later | Loki: `story:parity-loki-query` (implemented); SQL reads: to be drafted; Homer: `story:parity-homer-sip-reads` (draft, waits on a login-exchange credential profile) |
| 2 | the call's live or recent channel state | Asterisk Manager Interface | `story:parity-asterisk-ami` (draft; reads only for this epic) |
| 3 | its traces across sources: logs by call id or correlation id | Loki, direct and through Grafana | `story:parity-loki-query`, `story:parity-grafana-datasource-loki` (implemented) |
| 4 | metrics around the incident window | Prometheus, direct and through Grafana | `story:parity-prometheus-query` (implemented) |
| 5 | alerts and incidents, and which alert belongs to which incident, over a since window | Opsgenie; Grafana Cloud IRM | `story:parity-opsgenie-alerts` (draft) and the IRM reads, queued |
| 6 | the deployments and pipelines that ran before the onset | GitLab, Kubernetes | `story:gitlab-ci-runtime`, the GitLab deployment reads and the Kubernetes reads (implemented) |

## A call by its number

The first source is the SIP proxy's CDR lines in Loki joined to the platform's call table, read
with SQL on a read replica, matched on direction, the numbers and a time window, one line per leg.
Homer follows once its credential profile exists. An existing operator tool answers the question
this way and found:

- A transfer leg writes no call-table row, and the CDR line carries no leg type, so the join needs
  both sources: neither alone lists every leg with its type.
- The SQL operation returns at most 100 rows whatever the query's `LIMIT` says, so the operation
  this epic needs pages through results.
- End-user numbers are masked by default; the typed output keeps them masked.

## What this epic adds beyond the per-source stories

To be decomposed; each line is a candidate story, none is designed yet:

- **Typed output for every operation the table names.** `operations describe` states an output
  shape a later step can rely on; an operation whose output is untyped text is listed and given a
  shape.
- **Correlation keys as parameters.** A call number, a SIP call id, a trace or correlation id and
  a time window are accepted directly by the reads that can filter on them, so a step needs no
  model to translate one source's identifier into another's query.
- **Concurrent invocation.** Several reads on one host, across connections and adapters, run at
  the same time without serialising on the owner process or the metadata store; measured, with a
  bound stated in the contract. Today's behaviour under concurrent invokes is not measured.
- **A bounded, typed collection result.** Each read's refusal is typed (`not_found`,
  `rate_limited` with its retry, `unavailable`), so a workflow branches on codes, not on prose.

## Not in scope

- The workflow itself, its ordering and its report: they belong to the workflow SDK's repository.
- Any write to a provider: acknowledging, closing or annotating alerts and incidents.
- Jira Service Management.

## Open before decomposition

- Whether concurrency is a host guarantee with a contract bound, or a measured property only.
