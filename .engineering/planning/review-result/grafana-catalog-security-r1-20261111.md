---
format: aep.planning-md/3
id: review-result:grafana-catalog-security-r1-20261111
kind: review-result
status: active
title: Security review pass 1 on the Grafana catalog and proxy reads
relations:
- reviews: story:catalog-grafana-reads
- reviews: story:catalog-grafana-datasource-proxy-reads
revision: 1
---
# Security review pass 1: Grafana catalog reads and data-source proxy reads

Reviewed the working tree of both units applied uncommitted on 47166ae5d8 in the review tree
`grafana-review`. Cases in `adapters/catalog/tests/grafana_security_review.rs`: 10 cases, 7 green,
3 red.

Invariants held: the token is read from the mounted item and sent only as Bearer, in no target,
answer or log; the allow-list refuses every caller spelling tried before any request and before the
credential read, and bounds only the operations it names; only reads are selectable; over-limit
answers are refused as capacity, also when chunked; the vendored source matches its pin everywhere
it is named.

Not held: a uid carrying an encoded `/` or `\` is sent in front of the fixed proxy sub-path
(`sr_a_uid_segment_never_decodes_to_a_separator`); a repeated key in `catalog.allowed_values`
turns a non-empty list into admit-all (`served::sr_start_refuses_a_malformed_allow_list`,
`served::sr_a_repeated_key_does_not_open_the_allow_list`).

```findings
[
{"file":"crates/connectors-catalog/src/template.rs","line":722,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a proxy-read uid holding / or \\ is sent as a %2F/%5C segment in front of the fixed proxy sub-path, which the engine's own download check refuses as an escape on a decoding server"},
{"file":"adapters/catalog/src/served.rs","line":206,"category":"integrity","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a repeated operation or input key in catalog.allowed_values is accepted with the last one winning, so an empty repeat silently turns a non-empty allow-list into admit-all"}
]
```
