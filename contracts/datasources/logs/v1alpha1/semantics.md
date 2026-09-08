# datasource.logs/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** datasources. Siblings: [records](../../records/v1alpha1/semantics.md), [series](../../series/v1alpha1/semantics.md), relational (in [service v1alpha1](../../../service/v1alpha1/semantics.md)).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `datasource.logs/v1alpha1` |
| Profiles | `logql-range` (Loki), `kubernetes-pod-logs`, `docker-container-logs` |
| Shared with records | provenance, completeness, cursor binding rules (`docs/design.md:434-446`); not the record page shape |

A log read returns time-ordered lines from one or more streams, bounded by time window, line count and bytes, with explicit truncation. The design requires Loki's native query language and stream/label/timestamp identity to be preserved rather than flattened into records or called SQL (`docs/design.md:454`). Pod and container logs share the shape (lines, timestamps, bounds) but have no query language and select by resource rather than by label selector; they are profiles of the same contract so that a client can render all three the same way while the descriptor stays honest about which selection each supports.

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| `loki-query-range`: one operation from the pinned Loki HTTP API spec; direct origin or Grafana-mediated | `../connectors/providers/loki.toml` | preserve as `logql-range` |
| Old Loki projection: ≤ 500 streams, ≤ 1,000 lines, line text cut at 8 KiB with `truncated` per line, deterministic pattern redaction with `redacted` flag, `result_type`, per-line `timestamp`, `labels`, `line`; overall `truncated` | `../connectors/crates/integration-monitoring/src/projection.rs`, `project_loki` | preserve the shape and bounds as first-profile defaults; redaction becomes a configurable host-side filter, not a contract promise (arbitrary text cannot be proven secret-free, old design 08 amendment) |
| `kubernetes.pod.logs`: `namespace`, `pod`, optional `container`, `tail_lines` 1–1000 default 200, `since_seconds` 1–86400; output `text`, `truncated`; 128 KiB bound; namespace grant is the containment | `../connectors/crates/integration-kubernetes/src/workloads.rs:1120-1145` | preserve as `kubernetes-pod-logs`; change output from one `text` blob to lines with timestamps (Kubernetes `timestamps=true`) |
| Loki tenant/header binding where applicable | `docs/design.md:500` | preserve as configuration (`tenant_header`), never caller input |
| Docker container logs | no old source; Docker Engine API `GET /containers/{id}/logs` with `stdout`, `stderr`, `since`, `until`, `tail`, `timestamps` (to verify against the vendor reference at authoring) | new profile `docker-container-logs` |

## 3. Types

Input, `logql-range`:

```json
{ "query": "{app=\"api\"} |= \"error\"", "start_unix_ns": "…", "end_unix_ns": "…", "direction": "backward", "limit": 1000, "max_line_bytes": 8192 }
```

Input, `kubernetes-pod-logs`:

```json
{ "namespace": "x", "pod": "api-0", "container": "api", "since_seconds": 3600, "tail_lines": 200, "max_bytes": 131072 }
```

Input, `docker-container-logs`:

```json
{ "container": "id or name", "streams": ["stdout", "stderr"], "since_unix_s": 0, "tail_lines": 200, "max_bytes": 131072 }
```

Output (all profiles):

```json
{
  "lines": [ { "timestamp_unix_ns": "…", "stream": { "app": "api" }, "line": "…", "line_truncated": false, "source": "stdout" } ],
  "window": { "start_unix_ns": "…", "end_unix_ns": "…", "direction": "backward" },
  "complete": false,
  "truncation": { "by": "limit", "streams_dropped": 0 },
  "next_cursor": null,
  "provenance": { "instance": "…", "resource": "loki:…", "observed_at_unix_ms": 0, "source_revision": null }
}
```

| Field | Rule |
|---|---|
| `timestamp_unix_ns` | string (nanoseconds exceed JSON-safe integers); Kubernetes and Docker timestamps are converted to ns without rounding claims beyond their precision |
| `stream` | label set for Loki; `{namespace, pod, container}` for Kubernetes; `{container}` for Docker |
| `source` | `stdout`/`stderr` where the provider distinguishes; null for Loki |
| `complete` | true only when the provider returned fewer than `limit` lines and did not indicate more |
| `truncation.by` | `limit`, `bytes`, `time`, `provider`; `streams_dropped` counts streams beyond the stream bound |
| `next_cursor` | `logql-range` only: continuation by adjusting the window edge to the last timestamp (Loki semantics); bound to query, window, direction, connection, config revision |

Errors: base codes; `InvalidInput` for a query the provider rejects with 400 (message is the safe classification, not the provider body); `Timeout` before dispatch only; provider partial results are `complete: false` with `truncation.by: provider`.

## 4. Rules

- Native query preserved: `query` is passed to Loki unchanged; the contract does not parse LogQL. A configured query scope (allowed label selectors) is enforced by the adapter before dispatch (`docs/design.md:500`).
- Time window mandatory and bounded: `end - start` ≤ configured maximum (first-profile default 24 h); `end` defaults to now.
- Ordering: `direction` is honored; within one stream, lines are ordered by timestamp; across streams, order is by timestamp with ties unordered. The contract does not claim global order across streams beyond timestamp.
- Bounds are enforced in this order: line bytes, line count, total bytes, stream count; each records itself in `truncation`.
- Selection authority: `kubernetes-pod-logs` admits only configured namespaces (allowlist before dispatch, as `resources.list` today); `docker-container-logs` admits only configured containers or labels.
- No live follow: `follow`/tail-streaming is a `sessions` profile, not this contract.
- Cache: log reads are not cached by default; a cached read must carry the original observation time and is only served for an identical (connection, query, window, direction, limit).

## 5. Limits (first-profile defaults, to be measured)

| Bound | Value | Source |
|---|---|---|
| Lines per read | 1,000 | old projection |
| Streams per read | 500 | old projection |
| Line bytes | 8 KiB | old projection |
| Total bytes | 128 KiB for pod/container logs; response limit for Loki | `workloads.rs:1127` |
| Window | 24 h | new |
| Cursor TTL | 300 s | base |

## 6. Conformance scenarios (`docs/design.md:988`)

- Loki fixture returns 3 streams × 600 entries; `limit` 1000 → 1,000 lines, `truncation.by: limit`, `complete: false`, `next_cursor` present and re-readable.
- Fixture line of 20 KiB → `line_truncated: true`, `line` is an 8 KiB UTF-8-safe prefix.
- Window exceeding the maximum → `InvalidInput`, no dispatch.
- Loki 400 (bad LogQL) → `InvalidInput`; the error message contains no provider body text.
- Kubernetes fixture for namespace outside allowlist → `Forbidden`, zero requests.
- Kubernetes fixture with `timestamps=true` lines → each line has a ns timestamp; a line without a timestamp is reported with `timestamp_unix_ns: null` rather than invented.
- Tenant header configured → present on the fixture request; caller input cannot set or override it.

## 7. Compatibility

- New contract; no old wire contract for logs existed beyond operation results. The old `kubernetes.pod.logs` `text` blob is not preserved; a facade may join lines.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `LogLine`, `LogRead` types | `crates/connectors-contracts/src/lib.rs` |
| Cursor helper for window-edge continuation | `crates/connectors-sdk/src/lib.rs:161` |
| Configured extra headers on `HttpCapability` (tenant header) | `crates/connectors-host/src/http.rs:11-25` (`HttpConfig`) |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `OperationDeclaration.profile` in `{logql-range, kubernetes-pod-logs, docker-container-logs}` | value |
| Streams, lines | not entities; payload |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Redaction | host filter, opt-in, flagged per line; not a contract guarantee |
| Window maximum | 24 h |
| Docker `until` support | included if the vendor reference confirms it at authoring |
