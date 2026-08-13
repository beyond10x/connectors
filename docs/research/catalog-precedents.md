# Catalog-as-text precedents, mined

**Measured 2026-08-13** against the artifacts vendored under [vendor/](vendor/) (provenance in
[vendor/provenance.toml](vendor/provenance.toml)), compared with our own canonical connector
document schema (`connector-document.schema.json`, currently in the flux-connectors checkout,
migrating here). Companion narrative: [unified-api-platforms.md](unified-api-platforms.md) §4.

## The corpus

| Artifact | Shape | Scale |
|---|---|---|
| Nango `providers.yaml` | one YAML file, auth + runtime metadata as data, **no operations** | **957 providers** at pinned commit |
| Airbyte `manifest.yaml` (source-pokeapi) | one YAML per connector, read-streams as data, interpreted by a generic runtime | ~hundreds of manifest-only connectors upstream |
| Apideck `connector.yml` | OpenAPI spec of a catalog-as-queryable-API (metadata, coverage matrices) | ~14 unified categories |
| Pipedream `create-issue.mjs` | registry component as **code** (props + `run()`) | thousands of components |
| Ours (`catalog/*.catalog.json`) | one canonical JSON document per provider, operations included, compiled to an offset-indexed pack | 55 providers · 70 services · **835 operations** · 53 events · 5 channel bindings |

## Nango field statistics (the closest precedent)

Auth modes across 957 providers:

```
310 API_KEY        95 BASIC          4 OAUTH1           2 BILL
291 OAUTH2         60 TWO_STEP       4 JWT              1 each: TBA, SIGNATURE,
 98 OAUTH2_CC      22 MCP_OAUTH2     2 NONE               INSTALL_PLUGIN, CUSTOM,
                    2 MCP_OAUTH2_GENERIC                   AWS_SIGV4, APP
```

Field adoption (count of providers declaring each):

```
918 proxy                419 connection_config      54 webhook_routing_script
281 verification         160 retry                  41 post_connection_script
 67 alias                 33 paginate               24 token_response_metadata
```

Three readings of these numbers:

1. **The data-shaped core is nearly universal** — `proxy` base URLs on 96% of entries,
   credential-verification probes on 29%, typed per-connection config on 44%. These earn their
   place in a text catalog.
2. **The script escape hatches are rare** — webhook routing (5.6%) and post-connection scripts
   (4.3%) are the only places Nango leaves data for code. That is the boundary to design
   declaratively: our channel bindings already express verification matrices, discriminators and
   payload maps as data for the five bindings that exist; the general webhook-routing rule
   grammar is the gap to close before scaling inbound events across providers.
3. **The agent era is arriving in their catalog** — 24 `MCP_OAUTH2(_GENERIC)` entries did not
   exist in earlier snapshots. And `AWS_SIGV4` exists as a first-class auth mode: exactly the
   scheme the historical AWS plugin-retirement plan needed, with a working
   precedent for its declaration shape.

## Gap table: what they express that our catalog cannot yet

| Feature | Where seen | Assessment for our schema |
|---|---|---|
| `verification` probe (endpoint + expected outcome to validate a credential) | Nango (281×) | We have top-level `verify` per connector — confirm it covers per-service probes and failure classification; Nango's shape is simpler and battle-tested |
| `token_response_metadata` (extract extra OAuth-response fields into connection metadata, e.g. Slack `incoming_webhook.url`, `bot_user_id`) | Nango (24×) | **Adopt.** No equivalent; today such values would be lost at acquisition time |
| `alias` (provider template inheritance) | Nango (67×) | Consider a constrained variant for vendor families (e.g. `gitlab` vs `gitlab-self-hosted`); full inheritance fights review-equals-execution — an aliased entry's effective surface is not its diff |
| `retry` driven by named rate-limit headers (`at`/`after`) | Nango (160×) | Our `quirks.rate_limit` is representable but nothing declares one yet; adopt Nango's minimal header-name shape as the first supported form |
| Declarative pagination interpreted by a generic runtime (cursor/offset/link) | Nango (33×), Airbyte (paginator strategies) | We have `quirks.pagination` (landed C-536); Airbyte's strategy taxonomy is the reference for growing it |
| `connection_config` regex-validated user fields interpolated into base URLs | Nango (419×) | We have `config` fields with `binds` + endpoint slots — equivalent power; keep |
| Incremental sync cursors, partition routers, transformations, record selectors | Airbyte | Sync/ELT machinery — **out of scope v1** (we are invoke+events, not a data pipeline); their requester/error-handler split is still instructive |
| Catalog served as queryable API with coverage matrices | Apideck `connector.yml` | We already serve `catalog.json` + explorer; coverage matrices (which operations per provider vs category) are a cheap, high-value projection to add |
| MCP-shaped auth (`MCP_OAUTH2`) | Nango (24×) | Watch; likely the shape of "provider is itself an MCP server" — do not model prematurely |

## Gap table: what our catalog expresses that none of them do

| Feature | Ours | Why nobody else has it |
|---|---|---|
| **Operations as reviewable data** — method, URL template, parameter placement, body encoding, constant headers, closed template vocabulary (literals, `{var}`, `{"$param": name}`, nothing else) | 835 operations | Nango keeps operations as TypeScript; Pipedream as JS components; Airbyte declares read-streams only. Declaring *invocable writes* as closed, total data is our core differentiator |
| **Risk vocabulary per operation** — `risk` (ordered), `idempotency`, `direction`, `semantic_effects` | every operation | Only sighting anywhere: Pipedream's MCP-style `annotations` (`destructiveHint`/`readOnlyHint`/`openWorldHint` — three booleans, no ordering, no grant system consuming them). Ours feeds a grant admission system; theirs feeds a UI hint |
| **`expose` curation** — catalogued/addressable vs projected-to-models | 835 catalogued, ~9 projected for the largest provider | Nobody else distinguishes "callable" from "belongs in a model's context"; the meta-tools pattern (Composio) is the runtime workaround for not having it |
| **Review-equals-execution chain** — committed canonical documents, per-provider hashes in a lockfile, byte-identical offset-indexed pack, digest verified before serving a record | `connectors.lock` + `catalog.pack` | Platforms serve their catalog from a database; none can say "the bytes a human reviewed are the bytes the runtime executes" |
| **Credential surface as data** — scheme, acquisition, placement, subject, hazard, complete OAuth2 spec; registration *requirement* never a value (`client_id` is deployment config) | `auth` section | Closest is Nango's auth templates, which stop at the OAuth dance; placement/subject/hazard (what the secret touches, what may see it) exist nowhere else |
| **Channel bindings as data** — transport, verification matrix, discriminator, delivery-id, payload maps, closed event sets | 5 bindings | Nango's equivalent is a named script |
| **Provider authority as reverse-DNS address** leading every credential path | `authority` | Platforms use display keys; nobody derives storage/permission addressing from vendor identity |

## Consequences for the domain model and schema

1. Keep the canonical-document + pack + lockfile architecture unchanged — it is ahead of the
   field, not behind it.
2. Fold in, roughly in order of cost/benefit: `token_response_metadata`, header-name rate-limit
   retry, per-service verification probes, coverage-matrix projection.
3. Design the declarative webhook-routing rule grammar (verify + attribute) before scaling
   inbound events beyond the five existing channel bindings; keep a script escape hatch off the
   table until a real provider defeats the grammar (Nango data point: 94% never needed one).
4. `AWS_SIGV4` and client-certificate schemes have working declaration precedents; the aws and
   kubernetes migration waves (flux-roadmap 0024) should crib Nango's field shapes.
5. Do not adopt: template inheritance (fights reviewability), sync/ELT machinery (out of scope),
   catalog-in-a-database (forfeits review-equals-execution).
