---
format: aep.planning-md/3
id: story:catalog-feed-engine-extensions
kind: story
status: active
title: The catalog feed engine reads the list shapes real providers use
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:catalog-feed-engine
- depends_on: story:feed-profile-capabilities
scope:
- confidence: inferred
  path: adapters/catalog/spec/ess
- confidence: cited
  path: adapters/catalog/spec/ess/domains/feed.yaml
- confidence: cited
  path: adapters/catalog/src/feed.rs
- confidence: inferred
  path: adapters/catalog/src/lib.rs
- confidence: inferred
  path: adapters/catalog/tests/feed/
- confidence: inferred
  path: adapters/catalog/tests/feed_engine.rs
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-build/src/catalog_feed_conformance.rs
- confidence: inferred
  path: crates/connectors-docs/
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T21:05:12Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-21T21:05:13Z", actor: "human:timo", revision: 9}
---
## Outcome

The catalog feed engine reads the list shapes real providers use, so Jira, GitLab and Confluence bind the feed family by declaration alone.

## Why

Wave 20261006d (2026-10-06): the Jira, GitLab and Confluence bindings each stopped on the §3.3 declaration shape of `contracts/catalog/v1alpha1/semantics.md` (lines 173-178 name the gaps). The GitLab unit extended the engine in its tree; that patch is kept in the coordinating session's worktree archive `connectors-w4-gitlab`.

## Work (spec-first: the catalog ESS model, then §3.3, then `adapters/catalog/src/feed.rs`)

Revised 2026-10-07: the last-key cursor and the fixed container kind moved to `story:gitlab-feed-binding`, the only binding that needs them; this story keeps the shapes Jira and Confluence need.

1. Container and item paging: offset paging (`startAt` plus records, ending on a declared `last` flag such as Jira's `isLast`); a cursor read from a query parameter of a returned next URL (Confluence v2 `_links.next`, relative or absolute); an absent `next` ends the list.
2. A query-language template for items (Jira JQL, Confluence CQL) with a container slot and a time slot. Values are escaped by the language's quoting rules, never spliced raw; the template is checked at load. This needs one security review.
3. Instants: accept `±hhmm` offsets (Jira's `.000+0000`) and minute-precision filters. A minute filter re-reads the overlap minute, pages within it, and drops what the watermark already holds by `(id, revision)`, so no item is lost or repeated however many fall in one minute.
4. Relative item URLs resolved against the provider's base.

## Acceptance

- Fixture providers for each new shape pass the feed suite (as `story:feed-profile-capabilities` declares their capabilities): a JQL-templated time list with minute precision and more than one page inside one minute; an offset-paged container list; a next-URL cursor list ending by an absent `next`.
- A template value holding the language's quote and escape characters reaches the provider escaped, and a container id cannot add a clause (adversary cases).

## Depends on

`story:feed-profile-capabilities` (the suite must test what each profile declares).

## Scope

Derived 2026-10-21 by `story-scoper` at e26481df09. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `adapters/catalog/src/feed.rs` — cited (Work 1-4)
- **Model:** `adapters/catalog/spec/ess/domains/feed.yaml` (CursorPaging 50, TimeFilter 93, Items 130-150) — cited
- **Contract:** `contracts/catalog/v1alpha1/semantics.md` §3.3 rows 174 (time) and 177 (url), "cannot express" list 184-185, which this story closes — cited
- **Also likely:** `adapters/catalog/src/lib.rs:1096-1100,1544` (`Feed::new` receives path segments only, so item 4 needs the origin) — inferred
- **Also likely:** `adapters/catalog/tests/feed/` fixture declarations and `api.json`; `crates/connectors-build/src/catalog_feed_conformance.rs` (`Simulated` at 115-274 must answer the new shapes); `adapters/catalog/tests/feed_engine.rs` — inferred
- **Generated:** `crates/connectors-docs` contract reference, regenerated — inferred
- **Would collide with:** `story:jira-feed-binding` and `story:confluence-feed-binding` (land after this one)
- **Safety fact:** `Declaration` and `Mark` are `deny_unknown_fields` (`feed.rs:244`, `:1067`) and GitLab is the only shipped `position: time` binding, so new fields are optional with serde defaults and minute precision is a declared opt-in — cited, unproven

### Corrections to Work, read against the tree on 2026-10-21

| Work item | the story says | the tree says | this story builds |
|---|---|---|---|
| 1 containers | offset paging, `startAt` plus records, ending on `isLast` | Jira's non-deprecated project list `GET /rest/api/3/project/search` is offset-paged with `isLast` (upstream `jira-platform-v3.json`) | offset paging for container lists, as written |
| 1 items | offset paging | Jira items page by a body token `nextPageToken` on `/rest/api/3/search/jql` (`adapters/catalog/tests/jira.rs:404-409`); `Feed::by_time` (835-909) sends one request and has no continuation | a time-list continuation by a body token or a next-URL parameter, within one call |
| 1 next URL | a cursor read from a query parameter of a returned next URL | Confluence v1 `getSpaces` (`start`) and `searchContentByCQL` (`cursor`) ride in `_links.next` (`adapters/catalog/tests/confluence_v1.rs:421,451`); `containers` sends the whole value (`feed.rs:729-732`) | read one named query parameter from the next URL; never follow the URL |
| 2 template | JQL and CQL | CQL exists only on the `confluence-v1` provider; the v2 bundle has no search path | a query-language template, quoting per language, checked at load |
| 3 instants | `±hhmm` and minute filters | Jira writes `.000+0000` (`jira.rs:297`), refused by `instant` (`feed.rs:1294`) | as written |
| 4 relative URLs | resolve against the provider base | Jira's `self` is absolute; Confluence `_links.webui` is relative (`confluence_v1.rs:378`); the origin is absent from `Feed` | as written |

Security review points (Work item 2): a container id reaches JQL/CQL text (`feed.rs:773`, schema only `minLength 1` at `:1187`); the time literal is formatted from the parsed instant (`:635`), never spliced from text; the next-URL value is read as one named parameter and never followed (`:729-732`).

Not established in the tree: the JQL and CQL minute literal and zone rule; whether CQL accepts `order by lastmodified asc`. The fixtures declare one form each; no live provider is read in this story.
