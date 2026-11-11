---
format: aep.planning-md/3
id: approval-record:wave-grafana-proxy-reads
kind: approval-record
status: draft
title: Non-interactive wave readings for the Grafana data-source proxy reads
tags:
- non-interactive
relations:
- decides: story:catalog-grafana-datasource-proxy-reads
revision: 1
---
Non-interactive wave, session 2c881569-7f3b-4a58-ae7a-367874f446ae, brief relayed by `b10x`.
The readings of approval-record:wave-grafana-catalog-reads hold for this story too; this record
adds the one reading specific to it.

## Readings taken

1. Stop rule. The brief stops this story if the definition format cannot fix the proxy sub-path
   per operation. The pinned OpenAPI 3 document declares only
   `GET /datasources/proxy/uid/{uid}/{datasource_proxy_route}` (`datasourceProxyGETByUIDcalls`).
   A scratch definition with `source.amendments` carrying three `add_operation` entries
   (`/datasources/proxy/uid/{uid}/api/v1/query`, `.../api/v1/query_range`,
   `.../loki/api/v1/query_range`) lowered with exit 0: 317 operations, 0 unsupported, the three
   paths fixed per operation. The format can fix the sub-path, so the story continues and no
   `upstream-blocker` is filed.
2. Amended paths lowered without the `/api` prefix the document's own paths carry
   (`/api/datasources/...`). The recorded-fixture tests assert the exact request path sent, which
   settles which form the amendment must use.
3. Order. The story edits the same selection, bundle, docs and tests as
   story:catalog-grafana-reads; it starts from the integration branch after that story merges.
