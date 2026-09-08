# Atlassian source supplement — 2026-09-09

This append-only supplement preserves six official CQL pages previously present
only in the coordinator's read-only research cache. The
[native manifest](provider-source-hashes.json) records original URLs, exact
uncompressed SHA-256 values and byte lengths; each `vendor/*.gz` archive uses
`gzip -n`. Every archive was decompressed and compared byte-for-byte with that
cache, and manifest hashes were checked (all matched). No page was refetched;
the retention date is not an invented retrieval timestamp or immutable vendor
release. These are documentation captures, not upstream parser/build dependencies.

| Source | What it supports; what it does not |
|---|---|
| cql-keywords / cql-operators | Boolean grouping, ordering, comparison/membership/text operators used by the selected structural grammar; no local parser correctness proof |
| cql-fields / cql-functions | Native field/function meaning stays with the provider; the receiver grammar does not become a semantic field whitelist |
| advanced-searching-using-cql | Native CQL query and result behavior; no live snapshot or current body-membership guarantee |
| performing-text-searches-using-cql | Exact-phrase examples escape each inner quote once in raw CQL; public JSON and URL encoding are separate layers |

The selected [CQL contract](../../cql.md) fixes receiver bounds, preserves admitted
predicate/order bytes and conjoins trusted page/space constraints. Those receiver
selections are distinct from vendor assertions.

## PageBulk and storage evidence

The existing immutable
[provider manifest](../../../../../../../docs/evidence/datasource-semantics-20260908/provider-source-hashes.json)
retains the full official Confluence v2 source at
`https://dac-static.atlassian.com/cloud/confluence/openapi-v2.v3.json?_v=1.8516.103`,
SHA-256 `451377c5a598ee8155acc11b611404f309bed4a4292ea87f88ed3bfed38fa0a8`.
The [derived schema facts](provider-schema-facts.json) were extracted with `jq`
from those bytes; JSON pointers identify their original locations. They are an
inspection extract, not a second editable vendor schema or generated adapter.

`/paths/~1pages/get/parameters` declares subtype filtering with values page/live,
along with id, space-id, status and body-format. `PageBulk` has no required list;
its subtype property is nullable. `BodyBulk.storage` references `BodyType`, whose
value/representation properties are strings without a required list.

The selected receiver consequently requires usable identity/status/scope/body
data, obtains ordinary-page selection from the mandatory filter when subtype is
absent/null, and rejects a contradictory subtype. For storage, the fixed requested
format plus returned storage key identifies the representation; an optional
returned marker must agree. This is an explicit binding selection based on the
published query/schema, not evidence that multiple provider calls are atomic or
that a particular deployment always filters correctly. A binding unable to verify
those guarantees cannot advertise the profile.

The original Confluence v1/Jira OpenAPI, reconciliation documentation, historical
provider declarations and independent review supplements remain unchanged in the
original [datasource evidence intake](../../../../../../../docs/evidence/datasource-semantics-20260908/provider-evidence.md).
Exact upstream build-source/license adoption and standalone packaging remain
separate prerequisites; these research archives do not claim them complete.
