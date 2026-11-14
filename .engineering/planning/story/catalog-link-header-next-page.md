---
format: aep.planning-md/3
id: story:catalog-link-header-next-page
kind: story
status: archived
title: A read follows the Link header's next page on the definition's own origin
summary: Only rel=next is exposed; a next URL on another origin is refused
relations:
- serves: vision:independent-contract-adapters
- blocks: story:catalog-basecamp
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T10:50:31Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T10:50:31Z", actor: "human:operator", revision: 3}
- {from: "active", to: "archived", at: "2026-11-14T01:30:44Z", actor: "human:operator", revision: 4}
---
# A read follows the Link header's next page on the definition's own origin

## Need

Basecamp answers a list with a bare JSON array and names the next page only in the `Link` header's
`rel="next"`. A read's answer drops every header (`adapters/catalog/src/lib.rs:1946-1950`) and a walk
takes its next position only from the body (`adapters/catalog/src/walk.rs:261-285`).

## Acceptance

- A definition can ask for the `Link` header's `rel="next"` URL; it is exposed to the walk as the
  next page, and no other response header becomes readable by a definition.
- The next URL is followed only when its scheme, host and port equal the definition's base URL; a
  next URL on another origin is refused, by a case that fails first.
- A list without `rel="next"` ends the walk.
- Every existing definition lowers and behaves byte-equal; the catalog format reference documents
  the field.
