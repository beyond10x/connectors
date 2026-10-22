# Confluence REST v1 binding of datasource.feed/v1alpha1

**Not shipped.** This binding is not in the shipped `confluence-v1` selection
([providers/confluence-v1/operations.json](../../../providers/confluence-v1/operations.json) has no
`feed`), so an instance of the `confluence-v1` connector declares neither `feed.containers` nor
`feed.items`
(`adapters/catalog/tests/confluence_v1_feed.rs::the_shipped_confluence_v1_selection_declares_no_feed`).
Every shipped selection round-trips through a connector definition
(`crates/connectors-catalog/tests/definition_import.rs`), and the connector definition format cannot
yet carry this declaration: it has no next-link continuation (`url_parameter`), no time query
(`position` with `query`), no `utc_offsets`, no `url` or `url_base`, and it takes the items'
container from a path parameter, where this binding names the space in the CQL query. The binding
stays out of the shipped selection until it does.

**Profile:** `confluence-pages/1`. **Status:** declared as data in the test fixture
[tests/feed/confluence/feed.json](../../../tests/feed/confluence/feed.json) and realized by the
catalog engine
([contracts/catalog/v1alpha1/semantics.md, section 3.3](../../../../../contracts/catalog/v1alpha1/semantics.md#33-declared-feed-bindings));
no Rust of its own. Held to the family's `feed-binding` suite in five profile zones by
`crates/connectors-build/src/confluence_feed_conformance.rs`, with the result under
[Conformance](#conformance). Checked against Confluence shapes written from the public REST v1
reference, as the authored contract
(`adapters/atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml`) is; none was recorded
from a running Confluence, and the binding has not been read against one.

This file is the profile statement the family asks of a binding
([datasource.feed/v1alpha1](../../../../../contracts/datasources/feed/v1alpha1/semantics.md)).
Everything not stated here is the catalog engine's rule for every declared feed.

## Capabilities

| capability | value | why |
|---|---|---|
| `deletions` | `not-observed` | a trashed page leaves the search without a record ([Deletions](#deletions)) |
| `kind` | `fixed-word`, `kind_word: space` | every container is a space, global or personal |
| `revision` | `opaque` | a page's version number ([Items](#items-pages)) |
| `visibility` | `all-private` | no space field says who else may read a space ([Visibility](#visibility-every-space-is-private)) |

## Containers: spaces

A container is a Confluence space. `feed.containers` lists the current spaces the user can see
(`GET /wiki/rest/api/space?status=current&limit=<limit>`), global and personal alike, since no
`type` is sent (decided 2026-10-21). One v1 page is read per call, with `limit` at most 100;
Confluence may answer fewer. The next page continues at the `start` that the answer's `_links.next`
carries, and an answer without `_links.next` is the end. A next link without a `start` is
`unavailable`. The offset is a position, not a key: a space created, archived or deleted during a
walk shifts the spaces after it, so the walk can list one twice or not at all. The family promises
no snapshot. `status=current` leaves out archived spaces.

| field | Confluence field |
|---|---|
| `id` | the space's `key` (`DOCS`, `~<account id>` for a personal space), not its numeric `id` |
| `name` | `name` |
| `kind` | always `space` |
| `visibility` | always `private` (below) |

The key is the id because both reads take it: the space lookup by `spaceKey` and the CQL `space`
clause. Whether a space's key can change is not established.

`feed.items` reads any space the user can see, listed or not. It first reads the space
(`GET /wiki/rest/api/space/{spaceKey}`), and Confluence's 404 answers `not_found`.

## Visibility: every space is private

The declaration carries no visibility rule, so every listed space is `private` (decided
2026-10-21). A space's `type` (`global`, `personal`) says who owns it, not who may read it. Anonymous
access and space permissions are not fields of the listing. `private` never overstates who can read
a space's pages.

Confluence holds no direct conversations, so no container is `direct` and none is excluded on that
ground. A personal space is a named space of one user. It is listed and read like a global one, and
it is `private` like every space. Material the user may not read is left out by Confluence, not by
the binding. The authored contract says the listing returns "the spaces the account can see"; that
the search likewise returns only pages the credential can view, page restrictions included, is the
binding's expectation and has not been checked against a site. A page the credential can read is an
ordinary item of a private space; the family has no per-item field for its restrictions.

## Items: pages

An item is one page of the space, read with `GET /wiki/rest/api/content/search`
(`searchContentByCQL`), with `expand=history,version` only. A first read sends
`space = "<key>" and type = page order by lastmodified asc` as `cql`; a resume sends
`space = "<key>" and type = page and lastmodified >= "<minute>" order by lastmodified asc`. Each
value is a quoted CQL string, never spliced raw. Blog posts, comments, attachments and other
content types are not items. Confluence answers at most `limit` pages a request and may answer
fewer. While another page follows, `_links.next` carries the next request's `cursor`, which is read
from that link and form-decoded. One call reads up to 10 pages, until it holds `limit` pages it has
not returned.

| field | Confluence field |
|---|---|
| `id` | the page's content `id` (`"1001"`) |
| `revision` | `version.number`, as text (`"4"`) |
| `created_at` | `history.createdDate` (`2026-08-01T08:00:00.000Z`) |
| `updated_at` | `version.when` (`2026-09-20T10:00:00.000Z`) |
| `author` | `history.createdBy.accountId`, with `displayName` as `display_name`: the page's creator, not its last editor |
| `body` | `title`, representation `text/plain` |
| `url` | `_links.webui` under the answer's `_links.base` (`https://<site>/wiki/spaces/DOCS/pages/1001`) |
| `parent` | null |
| `deleted` | always `false` |

**Revision.** The version number. Each published edit makes a new version, and with it a new
revision. Two edits within one minute give two revisions, and both are returned. A change that makes
no version, such as a comment or a label, gives no new revision. Whether such a change moves
`lastmodified` is not established. If it does, the search orders the page by its new
`lastmodified` while its `version.when` stays old, and every read of the space, a first read
included, answers `unavailable` (out of `updated_at` order) until the page gets a new version.

**Times.** As Confluence writes them; the fixtures write UTC (`…Z`). `history` is not in the
authored contract, which names only the fields the shipped reads rely on, so `created_at` and
`author` rest on the fixture's shape alone.

**Body.** The title. The page body (`body.storage`) is XHTML storage format, not plain text, and 100
bodies at the 64 KiB item ceiling would pass the 3 MiB page ceiling. It is not expanded.

**No `parent`.** Not read. The search is not asked for `ancestors`.

## Watermark and first read

The watermark is a time watermark on `version.when` at minute precision. v1 offers no change cursor,
and the fixtures declare a CQL date literal to the minute. It holds the last returned page's
`version.when`, the kept instant, whose minute is the kept minute, and the `(id, revision)` pairs
returned at that instant. Pages
are asked for oldest `lastmodified` first, and an answer whose `version.when` runs out of that order
is `unavailable`. The watermark is scoped to the profile, the catalog instance and the space.

Not established, since no site was read: that CQL accepts `order by lastmodified asc`, that a
`lastmodified` literal names a minute and no finer, and that a page's `lastmodified` is its
`version.when`. The fixtures declare one form, and the simulated Confluence answers it. Whether
the search cursor is keyed or positional is not established either. Under a positional cursor, a
page versioned between two searches of one call moves the pages after it one place forward, so one
of them is not returned while the read reports `complete` and its watermark passes the page.

A first read sends no `lastmodified` clause and reaches every page of the space the user may see,
from the oldest modification forward.

A resume sends the kept minute less 12 hours, written as UTC wall time: a kept minute of 09:00 UTC
sends `"2026-10-05 21:00"`. That is the declaration's `utc_offsets` (`least: -720`,
`greatest: 840`). In every profile zone from UTC-12:00 to UTC+14:00, Confluence's filter then starts
at or before the kept minute, so no page changed after it is skipped. A page re-read before the kept
instant is dropped, since the watermark already holds it; one at it is skipped by its pair. Order
is checked from 26 hours before the kept minute, the earliest start the zones allow, and each
request of a resume asks for 100 pages.

## Profile zone

The binding is not told the zone Confluence reads the literal in, and reads no user setting. It
assumes:

- Confluence reads `lastmodified >= "<minute>"` as wall time in one zone whose offset at that minute
  lies from UTC-12:00 to UTC+14:00, the extremes of the current offsets in the IANA time zone
  database (`Etc/GMT+12`, `Pacific/Kiritimati`). Which zone it is does not matter within that
  range, and it may change between reads. That it is the user's zone is the binding's expectation;
  the authored contract does not state it.
- Each time Confluence writes carries its offset and is read at it.

A zone outside the range breaks the guarantee. One behind UTC-12:00 starts the filter after the
kept minute and silently skips the pages changed in between. One ahead of UTC+14:00 starts it
earlier than the order check allows, and the resume answers `unavailable` when a page changed in
those extra minutes.

## Index lag

Not established. Whether a page shows up in the CQL search as soon as its version is written is
not stated in the authored contract, which offers nothing the binding could send for
read-after-write consistency. A page that shows up in the search only after a read has returned one
changed later is, at the next resume, before the kept instant and dropped as held: it is skipped
until it changes again. No bound on that lag is known, and none was measured.

## Deletions

Not observed. A deleted page goes to the space's trash and leaves the search, and a purged one
leaves no record at all, so the binding never reports a tombstone and never `deleted: true`. A page
the search stops returning for another reason, such as a restriction added for the user, goes
unreported too. A consumer that needs removals reconciles another
way, outside this family.

## Ceilings and cost

`limit` 1 to 100 (the bound the `confluence-v1` selection sets on each list); watermark and
listing cursor 16 KiB; per-item body 64 KiB; serialized page 3 MiB; provider deadline as the generic
read (30 s). `feed.containers` makes one Confluence request; `feed.items` the space lookup and at
most 10 searches. A first read asks for `limit` pages a request.

A resume re-reads, before the kept minute, 12 hours plus the zone's offset: none at UTC-12:00,
12 hours at UTC, 26 hours at UTC+14:00. When its 10 pages of 100 hold no page it has not returned
(about 1,000 pages changed in that window), it answers
`unavailable` ("more items changed in the re-read window than the pages one read may request
hold"), and stays so for that watermark. Starting again with a first read does not get past it:
once that read's watermark is inside the same window, its resume answers the same, and the pages
after it are not returned while that many pages stay changed in the window.

How many pages share the kept minute does not grow the watermark: it holds the pairs of the pages
at the kept instant only. Pages whose `version.when` is one millisecond are bounded by the 16 KiB
watermark ceiling: 866 with five-digit ids and 638 with ten-digit ones, measured with the
fixture's 27-byte instance name and 4-byte space key; a longer instance name or space key holds
fewer. Past that, the read whose watermark would pass the ceiling answers `unavailable` ("the
watermark would exceed its ceiling; the container id is too long or too many items share one
instant"), after the pages earlier reads returned, none skipped.

## Errors

As the engine maps them: Confluence 400 (a query it refuses), 409, 412 and 422 are
`invalid_input`; 404 and 410 `not_found`; 401, 403, 429 and 5xx as the generic read table; any
other status or a body that is not JSON `unavailable`. Once the space lookup has answered, a 403,
404 or 410 from the search that is not a quota refusal is `not_found`, with the same message as
for a space the user cannot see. A
watermark not issued for this profile, instance and space is `stale_cursor` only after the search
answered: the binding reads it as a first read and discards what it lists. A watermark whose shifted
minute falls outside the years 0000 to 9999 is `stale_cursor` the same way, only after the search
answered.

## Which binding serves the feed

Only REST v1. REST v2 lists a space's pages with no time parameter and no change cursor, so it has
nothing a resume could start from, and no v2 binding of this family exists. Two configurations reach
this one, and neither ships it today:

- an instance of the `confluence-v1` connector, once its selection carries the `feed` block;
- an instance of `confluence` whose `bindings` name a `v1` alternative
  ([One instance for either API](../../../../../docs/catalog-confluence.md#one-instance-for-either-api)),
  with a selection file for that alternative that carries the block. A connection the token kind
  binds to v1 then declares `feed.containers` and `feed.items` and reads them. After a validation
  binds it to v2, `feed.items` is refused as `not_found` before any request.

`adapters/catalog/tests/confluence_binding.rs::a_connection_bound_to_v1_reads_the_feed_and_one_bound_to_v2_does_not`
holds the second case over a selection file the test writes.

## What the declaration cannot express

Each is an extension of the catalog declaration (section 3.3), not Confluence code:

- reading the zone Confluence reads the literal in, which would narrow the 12 to 26 hours a resume
  re-reads, and so the ceiling above;
- visibility from a space's permissions, which the listing does not carry;
- a page body in storage format, which is not plain text;
- a page's parent, from its ancestors;
- deletions, which the search does not report at all.

## Conformance

`the_confluence_v1_feed_binding_passes_the_scenarios_its_profile_declares_in_every_zone` runs the
suite this profile's capabilities select against this declaration, the committed `confluence-v1`
bundle and a simulated Confluence answering in the shapes of `adapters/catalog/tests/feed/confluence/`,
once in each of the profile zones UTC-12:00, UTC-09:30, UTC, UTC+05:45 and UTC+14:00: all 28 of the
scenarios the family defines in each, all passing, since an opaque revision leaves none out. The
suite holds an expected tombstone to the item never being held deleted, each listed `kind` to
`space`, and each container stored `public` to being listed `private`; the run names each change.
The simulation writes times in UTC and reads the literal in the profile zone. It lists a suite
container stored private as a personal space and a public one as a global space, and it holds no
direct conversation.

`adapters/catalog/tests/confluence_v1_feed.rs` holds the exact requests, the field mapping over
those shapes, the listing's continuation and a next link without its `start`, the search's cursor
pages, the resume in every zone from UTC-12:00 to UTC+14:00 in quarter hours, a new version within
the kept minute, the resume that answers `unavailable` at the ceiling, 900 pages changed within one
minute all returned once, the pages one millisecond holds at the watermark ceiling, a watermark
before the year 0000 answered only after the search, a space Confluence does not show, and that
neither Confluence selection declares a feed.
