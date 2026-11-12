---
format: aep.planning-md/3
id: story:path-parameter-dot-segments-are-refused
kind: story
status: draft
title: A catalog path parameter holding a dot segment or a slash is refused before any request
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

Found by the adversary of story:hubspot-owners-and-line-items (2026-11-12), pre-existing in the
shared engine. A path parameter value holding `/` is sent as one `%2F`-encoded segment:
`objects.list {"objectType":"../owners/2026-09"}` sends `/crm/objects/2026-09/..%2Fowners%2F2026-09`
(`crates/connectors-catalog/src/template.rs:601`). The engine's own download path refuses a segment
that decodes to `/`, because it lands outside `path_prefix` on a server that decodes it
(`adapters/catalog/src/lib.rs:1977`, `:3644`); path parameters get no such check. Whether an upstream
decodes `%2F` was not observed. The request stays a GET with the same credential.

## Acceptance

- A path parameter value that is `.` or `..`, or that holds `/` or decodes to one, is refused before
  any request, for every catalog operation; the adversary's test
  `adversary_a_dot_segment_record_id_never_leaves_the_objects_path` (kept beside this story's
  branch) passes.
