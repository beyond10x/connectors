# GitLab binding of datasource.feed/v1alpha1

**Profile:** `gitlab-merge-requests/1`. **Status:** declared as data in
[providers/gitlab/operations.json](../../../providers/gitlab/operations.json) (`feed`) and
realized by the catalog engine
([contracts/catalog/v1alpha1/semantics.md, section 3.3](../../../../../contracts/catalog/v1alpha1/semantics.md#33-declared-feed-bindings));
no Rust of its own. Held to the family's `feed-binding` suite by
`crates/connectors-build/src/gitlab_feed_conformance.rs`, with the result under
[Conformance](#conformance). Checked against recorded GitLab shapes; not yet read from a running
GitLab.

This file is the profile statement the family asks of a binding
([datasource.feed/v1alpha1](../../../../../contracts/datasources/feed/v1alpha1/semantics.md)).
Everything not stated here is the catalog engine's rule for every declared feed.

## Capabilities

| capability | value | why |
|---|---|---|
| `deletions` | `not-observed` | GitLab deletes a merge request outright ([Deletions](#deletions)) |
| `kind` | `fixed-word`, `kind_word: project` | GitLab's project record carries no word for itself |
| `revision` | `update-time` | a merge request carries no version; its `updated_at` is the revision ([Items](#items-merge-requests)) |
| `visibility` | `all-private` | one field cannot say who may read a project's merge requests ([Visibility](#visibility-every-project-is-private)) |

## Containers: projects

A container is a GitLab project. `feed.containers` lists the projects the token's user is a
member of (`GET /projects?membership=true&order_by=id&sort=asc`), oldest project id first, one
GitLab page per call (`per_page` = `limit`, at most 100). A full page continues after its last
project's id (`id_after`); a shorter page is the end. A project created between two calls with
an id below the last one listed is not listed by that walk; the family promises no snapshot and
no stable membership.

| field | GitLab field |
|---|---|
| `id` | the project's numeric `id`, stable through renames and transfers |
| `name` | `name_with_namespace` |
| `kind` | always `project` |
| `visibility` | always `private` (below) |

`feed.items` reads any project the token can see, member or not: it first reads the project
(`GET /projects/:id`), and a project GitLab does not show the token answers `not_found`.

## Visibility: every project is private

The declaration carries no visibility rule, so every listed project is `private`, whatever
GitLab's own `visibility` says (decided 2026-10-06). A project's merge requests can be
restricted to its members (`merge_requests_access_level: private`) while the project itself is
`public` or `internal`; mapping the project's `visibility` alone would list such a project
`public` although its merge requests are readable only by members. Until visibility can be read
from two fields, `private` is the answer that never overstates who can read a project's items.

GitLab holds no direct conversations: every project is a named space, so none is `direct` and
none is excluded on that ground.

## Items: merge requests

An item is one merge request of the project, in every state (`state=all`, `scope=all`), read
with `GET /projects/:id/merge_requests`. Issues are not items of this profile (below).

| field | GitLab field |
|---|---|
| `id` | `iid`, the merge request's number within its project (`!12` is `12`) |
| `revision` | `updated_at` |
| `created_at`, `updated_at` | `created_at`, `updated_at` as GitLab writes them |
| `author` | `author.id`, with `author.name` as `display_name` |
| `body` | `description`, representation `text/markdown` (GitLab Flavored Markdown); null as empty text |
| `url` | null |
| `parent` | null: merge requests are not threaded |
| `deleted` | always `false` |

**Revision.** GitLab gives no version; the merge request's `updated_at` changes when the merge
request is edited, its description included, so the binding uses it as the revision. A change
that moves nothing in the item's family fields (a title, a state) still gives a new revision.
Two changes within one millisecond give one revision, since GitLab writes `updated_at` to the
millisecond: the second is not told apart from the first.

**No `url`.** The merge request's `web_url` embeds the project's path, which a rename or transfer
changes without changing the merge request's `updated_at`. Mapping it would let one revision
carry two different `url` values, which the family forbids; the binding gives none.

**Precision.** `created_at` and `updated_at` carry milliseconds (`2026-10-06T09:00:00.081Z`).
Merge requests written within one millisecond are one instant to the engine, which skips those
it already returned there.

## Watermark and first read

The watermark is a time watermark on `updated_at` (GitLab keeps no change cursor for merge
requests): the last returned merge request's `updated_at`, sent back unchanged as
`updated_after` (GitLab: "updated on or after the given time", inclusive), with the
`(iid, updated_at)` pairs already returned at that instant, which the next read skips. Merge
requests are asked for oldest `updated_at` first (`order_by=updated_at&sort=asc`); an answer out
of that order is `unavailable`. The watermark is scoped to the profile, the catalog instance and
the project.

A first read sends no `updated_after` and reaches every merge request the project holds, in any
state, from the oldest `updated_at` forward: all of the project's history GitLab still lists.

## Deletions

Not observed. GitLab deletes a merge request outright (project owners and administrators); a
deleted merge request is no longer listed and leaves no record, so the binding never reports a
tombstone and never `deleted: true`. A closed or merged merge request is an ordinary item with a
new revision. A consumer that needs removals reconciles another way, outside this family.

## Ceilings and cost

`limit` 1 to 100 (GitLab's `per_page` cap); watermark and listing cursor 16 KiB; per-item body
64 KiB; serialized page 3 MiB; provider deadline as the generic read (30 s). `feed.containers`
makes one GitLab request and `feed.items` two (the project, then one page of merge requests). A
read asks GitLab for `limit` merge requests plus those already returned at the kept instant; 100
or more merge requests changed within one millisecond answer `unavailable`.

## Errors

As the engine maps them: GitLab 400, 409, 412 and 422 are `invalid_input`; 404 and 410
`not_found`; 401, 403, 429 and 5xx as the generic read table; any other status or a body that is
not JSON `unavailable`. Whatever GitLab answers for a project whose merge requests feature is
disabled or closed to the token is classified the same way.

## Merge requests only, not issues

A project holds both merge requests and issues, but they are two GitLab list endpoints
(`/merge_requests`, `/issues`), a declaration names one item listing, and a connection carries
one binding of the family (the operation ids are fixed). One profile cannot yield both kinds from
one container. Of the two, merge requests are the smaller correct form: issues additionally carry
a per-item privacy flag (`confidential`) that the family has no field for, and an issue can be
moved to another project.

## What the declaration cannot express

Each is an extension of the catalog declaration (section 3.3), not GitLab code:

- the merge request feature's own access level alongside the project's visibility (visibility
  from more than one field), which would let a public project with open merge requests be listed
  `public`;
- issues beside merge requests (an item endpoint per item kind);
- a `url` with a revision derived from the mapped fields, so that a renamed project's links
  change the revision;
- deletions, which GitLab's merge request list does not report at all.

## Conformance

`the_gitlab_feed_binding_passes_the_scenarios_its_profile_declares` runs the suite this
profile's capabilities select against this declaration, the committed bundle and a simulated
GitLab answering in the recorded shapes of `adapters/catalog/tests/feed/gitlab/`: 26 of the 28
scenarios the family defines, all passing. `revision: update-time` leaves out the two that
restore an item at an instant it already carried another revision at
(`connectors.feed.FeedItem/transition/restore/by/connectors.feed.RestoreItem/restored`,
`connectors.feed.RestoreItem/outcome/restored`). In the scenarios kept, the suite holds an
expected tombstone to the item never being held deleted, each listed `kind` to `project`, each
revision to the instant the item took it, and each container stored `public` to being listed
`private`; the run names each change.

`adapters/catalog/tests/gitlab_feed.rs` holds the exact requests (the membership filter
included, which the suite's views do not reach: they hold what a listing contains and excludes,
not that it lists nothing more), the field mapping over the recorded shapes, the millisecond
resume and the listing's continuation.
