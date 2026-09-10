# GitLab merge-request reads/v1alpha1

This is the MR read portion of C14 and GitLab's changed-record portion of C21.
Completion requires both disposable local CLI evidence and dedicated GitLab
sandbox evidence. The saved PAT read profile, current connection evidence, native
project allowlist and operation policy apply to each call.

## Selection and bounded results

`merge_request.get` selects a configured project and positive project-local `iid`.
It returns one `item` and provenance. `merge_requests.list` selects a configured
project, `state` (`all`, `opened`, `closed`, `locked` or `merged`), inclusive `updated_after`
and `updated_before`, limit 1..100 and an optional cursor. Both input timestamps
use UTC `YYYY-MM-DDTHH:MM:SSZ`, valid calendar dates, with after <= before. The
request uses scope=all, order_by=updated_at and sort=asc, and returns one bounded
page with provenance, next_cursor and complete. No caller checkpoint is stored.

The adapter-owned ESS Observation retains native id/iid/project_id, nullable
source_project_id, target_project_id, title, nullable description, source/target
branches, state, draft, detailed_merge_status, nullable sha/merge_commit_sha/
squash_commit_sha and updated_at. IDs fit positive signed 64-bit integers.
project_id must equal target_project_id and an explicitly numeric selected project;
get also verifies iid. A source project can differ for a fork or be absent after
deletion. Returned timestamps are valid bounded RFC3339 values. Title/branch
strings are nonempty and at most 1024 UTF-8 bytes, description at most 16384 bytes,
state/status at most 64 bytes, timestamps at most 64 bytes. SHAs, when present,
contain exactly 40 or 64 lowercase hex characters. Missing nullable source/sha/
description fields project to null; missing required fields refuse. Unselected
provider fields are discarded. Oversized or inconsistent data fails
upstream_protocol; it is never silently truncated or filtered.

List verifies each item's state when filtered, inclusive update interval,
nondecreasing update times and distinct native IDs and iids within the page.
Cursors bind instance, descriptor revision, authenticated connection partition,
operation, project, state, both timestamps and limit. Numeric continuation must
advance. A full page with no continuation header requires another page; an explicit
empty next-page header or a short page without that header ends this traversal.
Mutable offset pagination is not a consistent snapshot: concurrent updates can
move records between pages, and completion means traversal exhaustion only. A
consumer owns overlap, deduplication and checkpoint policy; failed/partial reads
must not advance its complete-collection checkpoint. This profile supplies no
lossless change feed or multi-provider checkpoint transaction.

## Observation boundary

Native state and detailed_merge_status are bounded open strings. Unknown values,
checking/unchecked statuses and a null SHA remain observations, never success.
`has_conflicts:false` is deliberately not used as mergeability proof. A full MR
head SHA is an observed head, not a pinned read of immutable MR metadata; provenance
source_revision is null. Null merge commit fields do not prove no business effect.
Neither read checks all required CI/approval rules, produces an approval proof,
validates a future write, merges or authorizes retry. The separate
[pinned-head validation](validation.md) operation reports selected checks without
granting a write. Guarded create/update/merge semantics must be modeled and
reviewed separately before their runtime decomposition. Existing shared mutation contracts continue to own approval,
audit, durable attempts, dispatch fences, replay and lost-response uncertainty.

## Source and verification

Mappings use the unchanged GitLab OpenAPI at revision
`2ff8d865e5016b14b724d1c2ce745f8300696192`, SHA256
`f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`.
AEP reverse import succeeds; its full-provider draft leaves entity ownership and
lifecycle questions UNMAPPED. This value projection selects the Basic/MR scalar
fields and introduces no entity graph. The source's list response is incorrectly
shaped as one Basic object; array interpretation is a native finish obligation.
[GitLab MR documentation](https://docs.gitlab.com/api/merge_requests/) checked on
2026-09-10 supplies list filtering and asynchronous merge-status context.

Test selected identity, nullable/deleted-source observations, unknown status,
valid and invalid calendars, fixed windows, date/order/filter mismatches, duplicate
IDs, pagination exhaustion, stale cursors across every selection/connection axis,
schema/project/policy refusal before provider work, malformed/oversized data and
safe provider failures. Prove generated mappings through production CLI persistence
and disposable HTTPS/keyring services. Preserve all eight existing GitLab reads,
Rust 1.88, generation/conformance/boundaries, packaging and website checks. Local
fixtures do not clear the dedicated sandbox blocker or complete C14/C21.
