# Pinned HubSpot CRM Objects source

`crm-objects-2026-09.json` is the unmodified HubSpot CRM Objects API `2026-09`
OpenAPI document, retrieved on 2026-09-30 from HubSpot's public specification
collection:

- [Exact source](https://raw.githubusercontent.com/HubSpot/HubSpot-public-api-spec-collection/55d9bcaaf1c208516f11a33aa43fb6bba7aac823/PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json),
  repository `HubSpot/HubSpot-public-api-spec-collection` at commit
  `55d9bcaaf1c208516f11a33aa43fb6bba7aac823`, path
  `PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json`.
- SHA-256: `1cb7ca32e7a9ff224838f8c1f02a99104e509d651fc2524dfd16f77fdc9be9f9`
- Size: 203,904 bytes.
- OpenAPI `3.0.1`; `info.version` `2026-09`; server `https://api.hubapi.com`.
- Neither the document nor the collection repository declares a licence, and
  the document declares no terms of service; no licence text is vendored.

`hubspot-source-hashes.json` records the same digest and length for
`vendor/crm-objects-2026-09.json.gz`, a gzip copy of the same bytes; the
repository gate's archived upstream source check re-derives both from that
archive. `adapters/catalog/tests/hubspot.rs` asserts that this file, the
manifest record, this README and the committed bundle index name one digest.

The catalog provider compiles this document into
`../../catalog/generated/bundles/hubspot.bundle.json`; the reviewed read-only
selection set is `../../catalog/providers/hubspot/operations.json`, described in
[the HubSpot guide](../../../docs/catalog-hubspot.md). HubSpot does not endorse
this adapter. Refreshing the source means replacing this file, its archive,
both records and the bundle together.
