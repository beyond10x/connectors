# Prometheus promql-range profile/v1alpha1

- **Status:** proposed, not implemented.
- **Shared dependency:** this adapter binding specializes the shared datasource contract of the same family and version. Native rules and future conformance fixtures are owned here.
- **Family:** datasources. Siblings: [records](../../../../../contracts/datasources/records/v1alpha1/semantics.md), [logs](../../../../../contracts/datasources/logs/v1alpha1/semantics.md), relational (in [service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md)).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `datasource.series/v1alpha1` |
| Profiles | selected: `promql-range`; reserved/refused: `promql-instant`, `promql-labels` |
| Shared with records | provenance and completeness semantics; not the page shape |

A series read returns labeled time series of numeric samples for a native query over a time range with a step. The design requires instant/range semantics, timestamps, step/resolution, label sets, the native query, and partial/error behavior to be preserved, and states that a series is not a log record (`docs/design.md:455`).

Support disposition is explicit: `promql-range` is the only proposed selectable
profile. `promql-instant` is reserved/refused because the old range projection's
optional instant value does not define a separately admitted request/result
contract. `promql-labels` is reserved/refused because no selected source operation,
schema, completeness rule or conformance input defines it. Neither reserved name
may appear in an advertised profile list or be accepted as an invocation profile.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `prometheus-query-range`: one operation from the pinned Prometheus HTTP API spec; direct or Grafana-mediated | `../connectors/providers/prometheus.toml` | preserve as `promql-range` |
| Old projection: requires `status = success`; `result_type`; ≤ 500 series each with `labels`, ≤ 2,000 `samples` `(timestamp, value)`, optional instant `value`; `truncated` when > 500 series | `../connectors/crates/integration-monitoring/src/projection.rs`, `project_prometheus` | preserve shape and bounds as first-profile defaults; add per-series truncation |
| Prometheus API parameters `query`, `start`, `end`, `step`, `timeout` | vendor reference cited in old toml `docs_url` (`https://prometheus.io/docs/prometheus/latest/querying/api/`) | preserve; `timeout` is set by the adapter from the provider deadline, never by the caller |
| Prometheus warnings/partial results (`warnings` array in the response) | vendor reference | preserve as `complete: false` with `warnings` (safe, bounded strings) |
| Old `alertmanager-alerts-list` | `../connectors/providers/alertmanager.toml` | not a series; stays in `datasource.records` |

## 3. Types

Input, `promql-range`:

```json
{ "query": "sum(rate(http_requests_total[5m])) by (status)", "start_unix_s": 1757300000, "end_unix_s": 1757303600, "step_s": 60, "max_series": 500, "max_samples_per_series": 2000 }
```

Output:

```json
{
  "result_type": "matrix",
  "series": [
    { "labels": { "status": "200" }, "samples": [ [1757300000, "12.5"] ], "samples_truncated": false }
  ],
  "series_truncated": false,
  "complete": true,
  "warnings": [],
  "window": { "start_unix_s": 1757300000, "end_unix_s": 1757303600, "step_s": 60 },
  "provenance": { "instance": "…", "resource": "prometheus:…", "observed_at_unix_ms": 0, "source_revision": null }
}
```

| Field | Rule |
|---|---|
| `samples[i]` | `[timestamp_unix_s (number, may be fractional), value (string)]`; values stay strings as Prometheus returns them (`NaN`, `+Inf`, precision) |
| `result_type` | `matrix` for range; `vector`/`scalar`/`string` reserved for instant |
| `labels` | the series' metric label set verbatim, including `__name__` when present |
| `complete` | false when the provider reported warnings or partial data, or when any truncation applied |
| `window` | echoes the effective window and step after adapter normalization |

Errors: base codes; provider 400 (bad PromQL) → `InvalidInput` with a safe classification; provider 422 (execution error) → `UpstreamProtocol`; 503 → `Unavailable`; too many samples on the provider side is reported by the provider as an error and mapped to `Capacity`.

## 4. Rules

- Native query preserved unchanged; the contract does not parse PromQL. A configured query scope (allowed metric prefixes or label matchers) is enforced before dispatch where configured (`docs/design.md:500`).
- Window and step bounded: `end - start` ≤ maximum (first-profile default 7 d); `(end - start) / step` ≤ `max_samples_per_series`; violations are `InvalidInput` before dispatch.
- Step is explicit: no adapter-chosen resolution; the descriptor states the minimum step (default 1 s) through the selected operation input_schema (`step_s` minimum). The maximum window is a selected profile cross-field constraint. `min_step_s`/`max_window_s` in the proposed authoring model refer to those constraints; they are not extra keys in the five-field service Operation.limits object.
- Ordering: series order as returned by the provider; samples ascending by timestamp within a series.
- Truncation: series beyond `max_series` are dropped and flagged; samples beyond `max_samples_per_series` are dropped from the end and flagged per series. Both set `complete: false`.
- No aggregation, downsampling, or unit conversion in the adapter.
- Cache: a range read is cacheable by (connection, query, window, step) with the provider's observation time; a window ending at "now" is not cached.

## 5. Limits (first-profile defaults, to be measured)

| Bound | Value | Source |
|---|---|---|
| Series | 500 | old projection |
| Samples per series | 2,000 | old projection |
| Window | 7 d | new |
| Minimum step | 1 s | new |
| Deadline | provider deadline (15 s today) | base |

## 6. Conformance scenarios (`docs/design.md:988`)

- Fixture returns 600 series → 500 returned, `series_truncated: true`, `complete: false`.
- Fixture returns `warnings: ["…"]` → `complete: false`, warnings copied bounded to 256 bytes each.
- `(end - start) / step` exceeding the sample bound → `InvalidInput`, zero requests.
- Value `"NaN"` preserved as string; timestamp fractional preserved.
- Prometheus 400 → `InvalidInput`; message has no provider body.
- Tenant header configured → present on the request; not settable by input.
- Mediated route (Grafana proxy) → same outputs as direct against the same fixture (route transparency, see [mediated_route](../../../../../contracts/discovery/mediated_route/v1alpha1/semantics.md)).

## 7. Compatibility

- New contract. Old projection fields `status` and `value` (instant) are not carried in `promql-range`; a future instant profile needs its own complete declaration.
- [Service compatibility](../../../../../contracts/service/compatibility.md) is authoritative for the binding. Series schemas require explicit profile support and compatible declared limits. Their numeric timestamps and string sample values cannot be coerced into existing record or SQL payloads; instant/labels remain reserved/refused and unadvertised.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `Series`, `SeriesRead` types | `crates/connectors-contracts/src/lib.rs` |
| Configured extra headers (tenant) | `crates/connectors-host/src/http.rs:11-25` |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `OperationDeclaration.profile = promql-range` with `min_step_s`, `max_window_s` | value |
| Series | payload, not entity |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Sample values as strings versus numbers | strings (lossless) |
| Window maximum | 7 d |
| Instant and label profiles | reserved/refused and unadvertised until a separately reviewed source operation, schema and conformance set selects each one |
