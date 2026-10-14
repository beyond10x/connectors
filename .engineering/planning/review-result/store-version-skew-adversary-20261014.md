---
format: aep.planning-md/3
id: review-result:store-version-skew-adversary-20261014
kind: review-result
status: active
title: Adversary review of the store version skew refusal
relations:
- reviews: story:store-version-skew-refusal
revision: 1
---
## Adversary review of the store version skew refusal

Reviewed on 2026-10-14: `story:store-version-skew-refusal` on `unit/store-version-skew` at
`5de9e7d6b`, by running cases against the built CLI and host. The failing cases are in
`apps/connectors/tests/store_version_skew_adversary.rs` (888d1780a); the fixes are 24af72e49
(specification), 22150f96e (host), f7ed1b67d and 21897f37e (tests).

| # | finding | outcome | case |
|---|---|---|---|
| 1 | A writer marker whose version the binary cannot parse (`1.0.0-rc.1`, an edited store) answered `metadata_unavailable` with `retry_status` on every command. | fixed: versions order by SemVer precedence, pre-releases included; a version that still cannot be ordered under another model digest is `metadata_schema_newer` with `store_version = "unrecorded"` | `adversary_an_unparseable_marker_version_is_not_a_retryable_unavailability` |
| 2 | A store a newer binary migrated past this binary's level was refused before the marker was read, as `metadata_unavailable` with `retry_status`. | fixed: a level above the binary's, in a database with this project's application id, is `metadata_schema_newer`, `unrecorded`; a digest mismatch or a replay failure stays `metadata_unavailable`, as nothing proves a newer writer caused it | `adversary_a_store_a_newer_binary_raised_past_this_level_is_refused_by_name` |
| 3 | The model digest covered generated invariant names and messages, so a generator change alone refused an older binary with identical records. | fixed: the digest covers entity names, identities, lifecycles, field schemas and sorted invariant assertions only | `the_model_digest_covers_record_shapes_not_generated_text` |
| 4 | An unlocked observation write is not re-checked against a concurrent newer writer that raised the marker after the open. | no change: the write records only its own clock and never lowers the marker | none |

Held: every command that opens the store passes the check; `0.9` orders below `0.10`; the marker
rides in the same batch as the records it describes; the metamorphic relation that ignores the
marker on a migrated store hides no difference; the failure's JSON fields are the same on the
direct and the owner path.
