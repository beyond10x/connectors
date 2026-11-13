# Pinned HubSpot sources

Two unmodified HubSpot OpenAPI documents, both from HubSpot's public specification
collection, `HubSpot/HubSpot-public-api-spec-collection` at commit
`55d9bcaaf1c208516f11a33aa43fb6bba7aac823`. Neither the documents nor the collection
repository declare a licence, and neither document declares terms of service; no
licence text is vendored.

## CRM Objects

`crm-objects-2026-09.json` is the HubSpot CRM Objects API `2026-09` OpenAPI document,
retrieved on 2026-09-30:

- [Exact source](https://raw.githubusercontent.com/HubSpot/HubSpot-public-api-spec-collection/55d9bcaaf1c208516f11a33aa43fb6bba7aac823/PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json),
  path `PublicApiSpecs/CRM/Objects/Rollouts/424/2026-09/objects.json`.
- SHA-256: `1cb7ca32e7a9ff224838f8c1f02a99104e509d651fc2524dfd16f77fdc9be9f9`
- Size: 203,904 bytes.
- OpenAPI `3.0.1`; `info.version` `2026-09`; server `https://api.hubapi.com`.

## Owners

`crm-owners-2026-09.json` is the HubSpot Crm Owners API `2026-09` OpenAPI document,
retrieved on 2026-11-12 from the same commit:

- [Exact source](https://raw.githubusercontent.com/HubSpot/HubSpot-public-api-spec-collection/55d9bcaaf1c208516f11a33aa43fb6bba7aac823/PublicApiSpecs/CRM/Crm%20Owners/Rollouts/146888/2026-09/crmOwners.json),
  path `PublicApiSpecs/CRM/Crm Owners/Rollouts/146888/2026-09/crmOwners.json`.
- SHA-256: `f7bd5bb7859a3f822d0b3d9b148321620b25393f319a1a7d800eb04d73eff7e3`
- Size: 16,389 bytes.
- OpenAPI `3.0.1`; `info.version` `2026-09`; server `https://api.hubapi.com`.
- Two operations, both `GET`: `/crm/owners/2026-09` (list, scope `crm.objects.owners.read`
  or `automation`) and `/crm/owners/2026-09/{ownerId}` (read one).

A catalog bundle has one source, the CRM Objects document. The two owner reads enter
that bundle through `hubspot-owners.amendments.json`, bound to the CRM Objects digest
above: each entry is an added read that names the Owners document's own `operationId`,
path and parameters and cites that document's URL. `adapters/catalog/tests/hubspot.rs`
holds each entry equal to the Owners document (operation id, path, and every
parameter's name, location, requirement, type and allowed values) and asserts that the
document declares nothing else. The amendments leave both vendored documents as
published.

## Records

`hubspot-source-hashes.json` records each document's digest and length for its
`vendor/*.json.gz` archive, a gzip copy of the same bytes; the repository gate's
archived upstream source check re-derives both from those archives.
`adapters/catalog/tests/hubspot.rs` asserts that this file, the manifest record, this
README and the committed bundle index name the digests.

The catalog provider compiles the CRM Objects document and the amendments into
`../../catalog/generated/bundles/hubspot.bundle.json`; the reviewed read-only
selection set is `../../catalog/providers/hubspot/operations.json`, described in
[the HubSpot guide](../../../docs/catalog-hubspot.md). HubSpot does not endorse
this adapter. Refreshing a source means replacing that file, its archive, both
records and the bundle together; refreshing the Owners document also means checking
the amendments against it.
