---
format: aep.planning-md/3
id: story:catalog-json-answers-reencoded
kind: story
status: draft
title: Catalog JSON answers are re-encoded while guides say byte-identical
tags:
- from-finding
relations:
- informed_by: review-result:grafana-catalog-adversary-r1-20261111
revision: 1
---
## Finding

The catalog engine parses every 2xx JSON answer and writes it again (`adapters/catalog/src/lib.rs:2612`,
`read_json` into a `Value`): object keys come back sorted and Go's `&`, `<` and `>`
escapes are removed. Several providers' guides and acceptance lines say an answer comes back
"byte-identical" or "unchanged"; their recorded-fixture tests pass because the fixtures are written
in the engine's output form already.

Found by review-result:grafana-catalog-adversary-r1-20261111 while attacking the Grafana reads. The
re-encoding predates that wave; the Grafana guide was reworded to "the same JSON value" instead.

## Outcome

Either a 2xx JSON answer is passed through as the upstream bytes (serde_json `raw_value` is already
enabled through connectors-core), or every guide and acceptance line that says "byte-identical" for
a real answer says "the same JSON value, keys sorted, compact".

## Acceptance

- A recorded-fixture test with upstream key order and Go escapes states which of the two holds, for
  every catalog provider.
- No guide claims byte identity the engine does not deliver.
