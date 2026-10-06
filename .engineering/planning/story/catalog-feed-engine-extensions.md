---
format: aep.planning-md/3
id: story:catalog-feed-engine-extensions
kind: story
status: draft
title: The catalog feed engine reads the list shapes real providers use
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-feed-engine
- depends_on: story:feed-profile-capabilities
scope:
- confidence: inferred
  path: adapters/catalog/spec/ess
- confidence: inferred
  path: adapters/catalog/src/feed.rs
- confidence: inferred
  path: contracts/catalog/v1alpha1/semantics.md
revision: 2
---
## Outcome

The catalog feed engine reads the list shapes real providers use, so Jira, GitLab and Confluence bind the feed family by declaration alone.

## Why

Wave 20261006d (2026-10-06): the Jira, GitLab and Confluence bindings each stopped on the §3.3 declaration shape of `contracts/catalog/v1alpha1/semantics.md` (lines 173-178 name the gaps). The GitLab unit extended the engine in its tree; that patch is kept in the coordinating session's worktree archive `connectors-w4-gitlab`.

## Work (spec-first: the catalog ESS model, then §3.3, then `adapters/catalog/src/feed.rs`)

1. Container and item paging: offset paging (`startAt` plus records, ending on a declared `last` flag such as Jira's `isLast`); a last-key cursor (continue after the last record's key; GitLab's `id_after`); a cursor read from a query parameter of a returned next URL (Confluence v2 `_links.next`, relative or absolute); an absent `next` ends the list.
2. Fixed container kind (`kind_word`).
3. A query-language template for items (Jira JQL, Confluence CQL) with a container slot and a time slot. Values are escaped by the language's quoting rules, never spliced raw; the template is checked at load. This needs one security review.
4. Instants: accept `±hhmm` offsets (Jira's `.000+0000`) and minute-precision filters. A minute filter re-reads the overlap minute, pages within it, and drops what the watermark already holds by `(id, revision)`, so no item is lost or repeated however many fall in one minute.
5. Relative item URLs resolved against the provider's base.

## Acceptance

- Fixture providers for each new shape pass the feed suite (as `story:feed-profile-capabilities` declares their capabilities): a JQL-templated time list with minute precision and more than one page inside one minute; an offset-paged container list; a last-key cursor list; a next-URL cursor list ending by an absent `next`.
- A template value holding the language's quote and escape characters reaches the provider escaped, and a container id cannot add a clause (adversary cases).

## Depends on

`story:feed-profile-capabilities` (the suite must test what each profile declares).
