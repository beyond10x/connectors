# Jira binding of datasource.feed/v1alpha1

**Not shipped.** This binding is not in the shipped Jira selection
([providers/jira/operations.json](../../../providers/jira/operations.json) has no `feed`), so an
instance of the Jira connector declares neither `feed.containers` nor `feed.items`
(`adapters/catalog/tests/jira_feed.rs::the_shipped_jira_selection_declares_no_feed`). Every
shipped selection round-trips through a connector definition
(`crates/connectors-catalog/tests/definition_import.rs`), and the connector definition format cannot
yet carry this declaration: it has no offset listing (`cursor.offset`), no time query (`position` with `query`)
and no `utc_offsets`. The binding stays out of the shipped selection until it does.

**Profile:** `jira-issues/1`. **Status:** declared as data in the test fixture
[tests/feed/jira/feed.json](../../../tests/feed/jira/feed.json) and realized by the catalog engine
([contracts/catalog/v1alpha1/semantics.md, section 3.3](../../../../../contracts/catalog/v1alpha1/semantics.md#33-declared-feed-bindings));
no Rust of its own. Held to the family's `feed-binding` suite in five profile zones by
`crates/connectors-build/src/jira_feed_conformance.rs`, with the result under
[Conformance](#conformance). Checked against Jira shapes taken from the pinned upstream description
(`adapters/atlassian/upstream/jira-platform-v3.json`); none was recorded from a running Jira, and
the binding has not been read against one.

This file is the profile statement the family asks of a binding
([datasource.feed/v1alpha1](../../../../../contracts/datasources/feed/v1alpha1/semantics.md)).
Everything not stated here is the catalog engine's rule for every declared feed.

## Capabilities

| capability | value | why |
|---|---|---|
| `deletions` | `not-observed` | Jira deletes an issue outright ([Deletions](#deletions)) |
| `kind` | `fixed-word`, `kind_word: project` | the pinned upstream does not enumerate the values of a project's `projectTypeKey` |
| `revision` | `update-time` | the issue search returns no version; an issue's `updated` is the revision ([Items](#items-issues)) |
| `visibility` | `all-private` | no project field says who else may read a project's issues ([Visibility](#visibility-every-project-is-private)) |

## Containers: projects

A container is a Jira project. `feed.containers` lists the live projects the user can view, by key
(`GET /rest/api/3/project/search?action=view&status=live&orderBy=key`), one Jira page per call
(`maxResults` = `limit`, at most 100; Jira may answer fewer). `startAt` takes the index of the
page's first project: the next page starts after the projects the answer carried, the answer with
`isLast: true` is the end, and an empty answer that is not the last is `unavailable`. The index is
a position, not a key: a project created, archived or deleted during a walk shifts the projects
after it, so the walk can list one twice or not at all. The family promises no snapshot.

`action=view` lists a project where the user has the *Browse projects* or *Administer projects*
permission for it, or *Administer Jira*. `status=live` leaves out archived projects and those in
the recycle bin; the upstream marks `status` experimental.

| field | Jira field |
|---|---|
| `id` | the project's `id` (`"10000"`), not its `key` |
| `name` | `name` |
| `kind` | always `project` |
| `visibility` | always `private` (below) |

`feed.items` reads any project the user can see, listed or not. It first reads the project
(`GET /rest/api/3/project/{id}`, which needs *Browse projects*), and Jira's 404 ("the project is
not found or the user does not have permission to view it") answers `not_found`.

Two upstream statements bear on the listing, and neither has been checked against a running Jira:

- A project listed only through *Administer projects* or *Administer Jira* fails the lookup, which
  needs *Browse projects*, so `feed.items` answers `not_found` for a container `feed.containers`
  listed. `action=browse` would list exactly the projects the lookup and the search admit. The
  wave decision of 2026-10-21 kept the upstream default, `view`.
- The project search documents a 404 "if no projects matching the search criteria are found".
  `feed.containers` then answers `not_found` rather than an empty complete listing. The simulated
  Jira of the suite answers an empty listing with 200.

## Visibility: every project is private

The declaration carries no visibility rule, so every listed project is `private` (decided
2026-10-21). Jira's `isPrivate` says "the user can't see the project or any associated issues": it
is the calling user's view, not who else on the site can read the project, so it cannot make a
project `public`. `private` never overstates who can read a project's issues.

Jira holds no direct conversations. Every project is a named space, so none is `direct` and none is
excluded on that ground. Jira excludes what the user may not read: the search includes an issue
only where the user has *Browse projects* for its project and, where issue-level security is
configured, permission to view the issue. An issue the credential can read is an ordinary item of a
private project; the family has no per-item field for its security level.

## Items: issues

An item is one issue of the project, read with `GET /rest/api/3/search/jql`
(`searchAndReconsileIssuesUsingJql`), asking for `fields=summary,created,updated,creator` only. A
first read sends `project = "<id>" ORDER BY updated ASC` as `jql`; a resume sends
`project = "<id>" AND updated >= "<minute>" ORDER BY updated ASC`. Each value is a quoted JQL
string, never spliced raw. Jira answers at most `maxResults` issues a page and may answer fewer,
with `nextPageToken` on every page but the last; one call reads up to 10 pages, until it holds
`limit` issues it has not returned.

| field | Jira field |
|---|---|
| `id` | the issue's `id` (`"10001"`), not its `key` |
| `revision` | `fields.updated` as Jira wrote it (`2026-10-06T11:00:00.000+0200`) |
| `created_at`, `updated_at` | `fields.created`, `fields.updated`, as the same instants in UTC (`2026-10-06T09:00:00.000Z`) |
| `author` | `fields.creator.accountId`, with `displayName` as `display_name`; null without a creator |
| `body` | `fields.summary`, representation `text/plain`; null as empty text |
| `url` | null |
| `parent` | null |
| `deleted` | always `false` |

**Revision.** The search returns no version for an issue. Its `updated` changes when the issue is
edited, its summary included, so the binding uses it as the revision. Any change Jira records as an
update gives a new revision, also one that moves no family field (a status, an assignee), and two
changes within one millisecond give one. The revision is the text Jira wrote, offset included, and
the upstream writes top-level timestamps "in the system default user time zone". The same instant
written with another offset is another revision: when that zone changes between two reads, the
issues at the kept minute are returned once more under the new text, nothing else about them
changed.

**Body.** The summary. An issue's `description` is Atlassian Document Format, a JSON value the
engine answers `unavailable` (section 3.3), and `renderedFields` is HTML; neither is read.

**No `url`.** The answer's `self` is the issue's REST address, not a page a reader opens, and the
answer holds no path to the issue's page for the declaration to resolve.

**No `parent`.** Not read. Whether Jira's `parent` always names an issue of the same project, as
the family's `parent` must, is not established from the pinned upstream.

## Watermark and first read

The watermark is a time watermark on `updated` at minute precision (the search offers no change
cursor, and a JQL date literal names a minute). It holds the last returned issue's `updated`
and the `(id, revision)` pairs returned within that minute, the kept minute. Issues are asked for
oldest `updated` first; an answer out of that order is `unavailable`. The watermark is scoped to the
profile, the catalog instance and the project.

A first read sends no `updated` clause and reaches every issue of the project the user may see,
from the oldest `updated` forward. Issues of archived projects are left out, as the search does by
default (`includeArchivedProjects` is not sent).

A resume sends the kept minute less 12 hours, written as UTC wall time: a kept minute of 09:00 UTC
sends `"2026-10-05 21:00"`. That is the declaration's `utc_offsets` (`least: -720`,
`greatest: 840`). In every profile zone from UTC-12:00 to UTC+14:00, Jira's filter then starts at
or before the kept minute, so no issue changed after it is skipped. An issue re-read before the kept
minute is dropped, since the watermark already holds it; one within it is skipped by its pair. Order
is checked from 26 hours before the kept minute, the earliest start the zones allow, and each
request of a resume asks for 100 issues.

## Profile zone

The binding is not told the zone Jira reads the literal in, and never reads the user's `timeZone`
(upstream: "the time zone specified in the user's profile", or the site's default where that is
hidden or unset). It assumes:

- Jira reads `updated >= "<minute>"` as wall time in one zone whose offset at that minute lies from
  UTC-12:00 to UTC+14:00, the extremes of the current offsets in the IANA time zone database
  (`Etc/GMT+12`, `Pacific/Kiritimati`). Which zone it is does not matter within that range, and it
  may change between reads. That it is the user's profile zone is the binding's expectation; the
  pinned upstream does not state it.
- Each time Jira writes carries its offset and is read at it, so the zone Jira writes times in need
  not be the one it reads the literal in.

A zone outside the range breaks the guarantee. One behind UTC-12:00 starts the filter after the kept
minute and silently skips the issues changed in between. One ahead of UTC+14:00 starts it earlier
than the order check allows, and the resume answers `unavailable` when an issue changed in those
extra minutes.

## Index lag

Not established. The upstream says "Recent updates might not be immediately visible in the returned
search results" and offers `reconcileIssues`, at most 50 issue ids, for read-after-write
consistency; the binding does not send it. An issue that shows up in the search only after a read
has returned one updated later is, at the next resume, before the kept minute and dropped as held:
it is skipped until it changes again. The upstream gives no bound on that lag, and none was
measured.

## Deletions

Not observed. Jira deletes an issue outright: a deleted issue is no longer returned by the search
and leaves no record there, so the binding never reports a tombstone and never `deleted: true`. An
issue the search stops returning for another reason, such as a security level the user cannot see,
goes unreported too. A resolved or closed issue is an ordinary item with a new revision. A consumer
that needs removals reconciles another way, outside this family.

## Ceilings and cost

`limit` 1 to 100 (the project search's `maxResults` cap, applied to issues too); watermark and
listing cursor 16 KiB; per-item body 64 KiB; serialized page 3 MiB; provider deadline as the generic
read (30 s). `feed.containers` makes one Jira request; `feed.items` the project lookup and at most
10 searches. A first read asks for `limit` issues a page.

A resume re-reads, before the kept minute, 12 hours plus the zone's offset: none at UTC-12:00,
12 hours at UTC, 26 hours at UTC+14:00. When its 10 pages of 100 hold no issue it has not returned
(about 1,000 issues changed in that window, as a bulk edit of that many can), it answers
`unavailable` ("more items changed in the re-read window than the pages one read may request
hold"), and stays so for that watermark. A consumer then starts again with a first read, which
returns every issue again.

## Errors

As the engine maps them: Jira 400 (a search Jira refuses), 409, 412 and 422 are `invalid_input`;
404 and 410 `not_found`; 401, 403, 429 and 5xx as the generic read table; any other status or a
body that is not JSON `unavailable`. Once the project lookup has answered, a 403 from the search
that is not a quota refusal is `not_found`, as for a project the user cannot see. A watermark not
issued for this profile, instance and project is `stale_cursor` only after the search answered:
the binding reads it as a first read and discards what it lists. A watermark whose shifted minute
falls outside the years 0000 to 9999 is `stale_cursor`.

## What the declaration cannot express

Each is an extension of the catalog declaration (section 3.3), not Jira code:

- reading the zone Jira reads the literal in, which would narrow the 12 to 26 hours a resume
  re-reads, and so the ceiling above;
- `reconcileIssues` with the ids a read returned, which would close the index lag above for those;
- visibility from a field that says who on the site can read a project, which `isPrivate` does not;
- a description in Atlassian Document Format as a body;
- an issue's page as `url`, which the answer gives only as a key;
- deletions, which the search does not report at all.

## Conformance

`the_jira_feed_binding_passes_the_scenarios_its_profile_declares_in_every_zone` runs the suite this
profile's capabilities select against this declaration, the committed bundle and a simulated Jira
answering in the shapes of `adapters/catalog/tests/feed/jira/`, once in each of the profile zones
UTC-12:00, UTC-09:30, UTC, UTC+05:45 and UTC+14:00: 26 of the 28 scenarios the family defines in
each, all passing. `revision: update-time` leaves out the two that restore an item at an instant it
already carried another revision at
(`connectors.feed.FeedItem/transition/restore/by/connectors.feed.RestoreItem/restored`,
`connectors.feed.RestoreItem/outcome/restored`). In the scenarios kept, the suite holds an expected
tombstone to the item never being held deleted, each listed `kind` to `project`, each revision to
the instant the item took it, and each container stored `public` to being listed `private`; the run
names each change. The simulation reads the literal and writes times in the same zone.

`adapters/catalog/tests/jira_feed.rs` holds the exact requests, the field mapping over those
shapes, the listing's continuation, the resume in every zone from UTC-12:00 to UTC+14:00 in quarter
hours, a written zone other than the read one, a written zone that changes between reads, the
resume that answers `unavailable` at the ceiling, and that the shipped selection declares no feed.
