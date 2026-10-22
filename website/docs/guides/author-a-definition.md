---
title: Author a definition for an API that ships no catalog
sidebar_position: 5
description: Write a connectors-definition/1 file for an HTTP API that Connectors does not ship, vendor its API document, check it and invoke it without configuring an instance.
lede: A definition names a vendored API document, one auth profile and the operations to expose; the reference lowerer turns it into what the catalog provider runs, and no Rust build is needed for a new API.
source: crates/connectors-catalog/src/definition.rs, spec/domains/definition.yaml and connectors/gitlab.yaml in beyond10x/connectors-definitions, apps/connectors/src/local/definition.rs, contracts/cli/v1alpha1/semantics.md
---

# Author a definition for an API that ships no catalog

When Connectors ships no catalog for an HTTP API, a `connectors-definition/1` file can still
reach it. The file names the API's own OpenAPI document, vendored beside it, and says which of
its operations to expose, under which local names, with which auth profile and which checks
around a write. Everything the document already states (methods, paths, parameters, response
shapes) stays in the document. The reference lowerer, `crates/connectors-catalog/src/definition.rs`,
reads both and writes what the catalog provider runs, so a run of the file goes through the same
admission, approval and audit as a configured instance.

The format is defined in the
[definitions repository](https://github.com/beyond10x/connectors-definitions), in
`spec/domains/definition.yaml` (schema `urn:beyond10x:connectors-definition:1`). Its
`connectors/gitlab.yaml` is the full worked example of the format. This page builds a smaller
one: three GitLab reads, written by hand on 2026-10-18, with the host replaced by
`gitlab.example.com`.

:::note[Not yet run on this page]
The commands below are shown without their output. Each is followed by "Run in a tree; output
pasted in a later revision." A later revision of this page adds one run and its output.
:::

## Lay out the tree

A definitions tree has three directories:

```text
connectors/gitlab-reads.yaml
auth/
upstream/gitlab-reads/openapi_v3.yaml.gz
upstream/gitlab-reads/openapi_v3.amendments.json
upstream/gitlab-reads/LICENSE
upstream/gitlab-reads/README.md
```

- `connectors/<id>.yaml` is the definition. The file stem is the connector id, unless the file
  sets `id:`, and it names the bundle the lowerer builds. The stem, the `id` and a `catalog` are
  plain names: ASCII letters, digits, `-` and `_`.
- `auth/<name>.yaml` holds auth profiles that several connectors share (schema
  `urn:beyond10x:connectors-auth:1`). A profile takes one with `use: <name>`, and every field it
  sets replaces the shared one. A definition with its own profile needs nothing here.
- `upstream/<id>/` holds the vendored API document, gzipped, and what records where it came from.

The definitions repository's rules for `upstream/<id>/` apply to a tree of your own as well:

- the document is a gzip copy of the unmodified file, named `<name>.gz`;
- `source.sha256` in the definition is the SHA-256 of the decompressed bytes;
- `upstream/<id>/README.md` records the file, the URL it was taken from, the retrieval date, the
  size and the license;
- the license text sits beside it as `LICENSE` where the document declares one.

The `README.md` of the definitions repository's `upstream/gitlab/` is a model for that record.
Vendor the document and take its digest:

```sh
mkdir -p upstream/gitlab-reads
gzip -9n < openapi_v3.yaml > upstream/gitlab-reads/openapi_v3.yaml.gz
gunzip -c upstream/gitlab-reads/openapi_v3.yaml.gz | sha256sum
```

Run in a tree; output pasted in a later revision.

A Swagger 2.0 document is named with `swagger:` and a Google Discovery document with
`discovery:`; either also names `projection:`, the file name its OpenAPI projection is recorded
under. An OpenAPI document names neither. `amendments:` is optional. When
named, the file must exist under the tree, be a `connectors-source-amendments/1` document whose
`source_sha256` is the digest of the decompressed document, and list at least one cited
correction; a missing file is refused as `unreadable`, a wrong digest or an empty list as
`invalid_input`. Leave the line and the file out of the worked example if the document needs no
correction.

## Write the file

The worked example, `connectors/gitlab-reads.yaml`:

```yaml
schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/gitlab-reads/openapi_v3.yaml.gz
  sha256: f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530
  amendments: upstream/gitlab-reads/openapi_v3.amendments.json
base_url: https://{hostname}/api/v4
config: {hostname: {type: string, default: gitlab.example.com}}
auth:
  gitlab.pat:
    scheme: header
    header: PRIVATE-TOKEN
    label: GitLab personal access token
    identity: {path: user, kind: gitlab.user, subject: /id}
    scopes: {path: personal_access_tokens/self, pointer: /scopes, minimum: [read_api]}
paging:
  pages: {request: page, next: "header:x-next-page", limit: per_page, max: 100}
operations:
  me.get:
    op: getApiV4User
    description: Read the user the credential belongs to
  project.get:
    op: getApiV4ProjectsId
    description: Read one project by id or url-encoded path
  merge_requests.list:
    op: getApiV4ProjectsIdMergeRequests
    description: List a project's merge requests, one page per call
```

The hand-written trial also carried `catalog: gitlab`. That key reuses an existing catalog
bundle for a second selection; for an API with no catalog, leave it out.

### Base URL and its variables

`base_url` may name variables in braces, here `hostname`, and `config` declares each one. A
variable `config` does not declare is refused as `undeclared_variable`. Config values are strings;
`values` lists the allowed ones.

Give every variable a `default`, and make the base URL `https://`.
`operations invoke --definition` takes the instance's endpoint from the lowered declaration, and
refuses a file whose endpoint is not stated in full or is not `https://` as `invalid_input` at
`arguments`. The instance trusts the platform's roots.

### One auth profile

A profile id is `<provider>.<name>`, and every profile of the file shares one provider prefix,
or the lowerer refuses it as `provider_id`. With more than one profile, mark exactly one
`default: true` (`auth_default` otherwise).

A token profile is `scheme: header` with the `header` it goes in (and an optional `prefix`),
`scheme: bearer` for `Authorization: Bearer`, or `scheme: basic` with an `account_label`. Each
profile has:

- `label`: the credential prompt. Connecting asks `GitLab personal access token:` for the
  example above, with the input hidden.
- `identity`: the read that proves the credential and names who holds it. `path` is the read
  under `base_url`, `kind` the subject's kind, `subject` a JSON pointer into the answer.
  `source: configuration` names the connection itself after the read proves the key, for a key
  that belongs to no user.
- `scopes` (optional): a read and pointer that list the granted scopes, and the `minimum` the
  connection must hold.

An OAuth profile is `scheme: oauth2_refresh`, with `authorize_url` and `token_url`, or
`scheme: oauth2_client_credentials`, with `token_url` only. Take both URLs from the vendor's own
documentation of its OAuth flow, never from another provider's profile, and list the scopes the
consent asks for in `scopes.requested`:

```yaml
auth:
  example.oauth:
    scheme: oauth2_refresh
    label: Example refresh token
    identity: {path: me, kind: example.user, subject: /id}
    scopes: {requested: [read]}
    authorize_url: https://auth.example.com/oauth/authorize
    token_url: https://auth.example.com/oauth/token
```

The definitions repository requires each profile to cite that documentation. The format has no
field for the citation (a key the schema does not name is refused as `unknown_field`), so give it
in the change that adds the file. No client id, secret or token is ever in the file: for an
`oauth2_refresh` profile, connecting reads the OAuth client from the file `--credential-file`
names.

### Operations

Each key under `operations` is the local name a caller invokes; `op` is the document's
`operationId`. Nothing the document states is repeated. `description` is written only where it
says more than the document's summary, which is the description otherwise.

- `effect`: an operation whose method is not `GET` must say `read` or `write`, or it is refused as
  `effect_required`. A `write` on a `GET` operation, or on a download with no `op`, is refused as
  `write_method`. Under an approval policy a write needs an approval (`--approval-file`).
- Bounds: a paging style's `max` bounds its `limit` parameter (here `per_page` from 1 to 100).
  `bounds` adds others: `{per_page: {minimum: 1, maximum: 20}}` for a range, or
  `{scope: {values: [blobs]}}` to hold a string to a set of values.
- `withhold: [<parameter>]` keeps a parameter of the document away from the caller.
- `paging`: when more than one style of the file's `paging` matches an operation, it names one
  (`ambiguous_paging` otherwise).

### A write: body and guard

`body` closes a write's JSON body to the keys it lists, each with its notation:

| Written | The key |
|---|---|
| `string`, `integer`, `boolean` | carries a value of that JSON type |
| `any` | carries any JSON value |
| `string+` | carries a string that must not be empty (`+` only after `string`) |
| `string_list`, `string_list!` | carries an array of strings (`+` is refused; `!` last) |
| `string!`, `string+!`, `integer!` | must be sent (`!` last) |
| `=value` | is fixed: the caller never sends it, and `value` is sent; `=true` and `=false` are booleans, a whole number that fits 64 bits (optional leading `-` or `+`; `=007` is 7) an integer sent as a number, anything else a string |

Any other notation is refused as `body_type`. A write without `body` takes an open body: any
JSON object, sent as given, with only the approval binding it.

A `guard` checks the write before and after it is sent. The definitions repository's
`connectors/gitlab.yaml:120-127` resolves a merge-request discussion this way:

```yaml
  merge_request.discussion.get:
    op: getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId
    description: Read one discussion of merge request noteable_id (its IID) by discussion_id
  merge_request.discussion.resolve:
    op: putApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId
    effect: write
    body: {resolved: boolean}
    guard:
      before: {read: merge_request.discussion.get, expect: {/id: discussion_id, /resolvable: =true}}
      after: {expect: {/id: discussion_id, /resolved: body.resolved}}
    description: Resolve (body.resolved true) or unresolve (false) one resolvable discussion
```

- `before.read` names the read to run first: an operation key of the file, or an `operationId`
  of the document (`unknown_guard_read` otherwise). Its path parameters take the write's input
  of the same name.
- `with` binds a parameter of that read to another input, for example
  `{read: branch.get, with: {branch: body.source_branch}, expect: {/commit/id: sha}}`. A parameter
  the read does not have is refused as `guard_parameter`.
- `expect` maps a JSON pointer into the read's answer to an input reference (`discussion_id`,
  `body.resolved`), a literal (`=opened`), a boolean (`=true`) or `"?"`, which holds when the
  member exists and is not `null`, whatever its value. If one comparison fails, the write is
  refused before anything is sent.
- `after.expect` compares the write's answer the same way; `"?"` there requires what a create
  names, such as Jira's `{/id: "?", /key: "?"}` after `issue.create`. `after.read` reads again instead,
  `after.absent: true` wants that read to answer 404, and `also` lists further reads before the
  write. When the answer does not match, the outcome is unknown: the provider may have applied the write, so read the target before any retry.

## Check it

In the definitions tree, `task check` validates the file against the projected schema and checks
that the vendored document hashes to `source.sha256`. It needs `ess`, `yq` v4 and `task`:

```sh
task check
```

Run in a tree; output pasted in a later revision.

`task check` validates shape only. That each `op` exists in the document, that each variable is
declared, and the rest of this page's rules are the lowerer's. From a Connectors checkout,
`connectors-build definition` runs the lowerer alone, writes the bundle, its index, the
selection and the declaration into `--out`, and names the first refusal:

```sh
cargo run --locked -p connectors-build -- definition \
  --file ../defs/connectors/gitlab-reads.yaml --root ../defs --out target/try/lowered
```

Run in a tree; output pasted in a later revision.

A refused definition writes nothing. The error starts with the refusal's name and a colon, then
says what it found: `unknown_operation` names the operation key and the `op` the document lacks.
The names a hand-written file meets most:

| Refusal | Cause |
|---|---|
| `malformed`, `unknown_field`, `wrong_schema` | not the schema's shape, a key it does not name, or another `schema` |
| `source_hash_mismatch`, `source_not_gzip`, `source_outside_root` | the vendored document does not hash to `source.sha256`, is not `<name>.gz`, or lies outside the tree |
| `source_kind` | a source that names other than one document, a `projection` where none is taken or none where one is, or a document of another kind |
| `source_too_large` | a vendored document that decompresses past 64 MiB |
| `unknown_operation`, `no_operation` | an `op` the document has no operation for, or none given |
| `effect_required` | a method other than `GET` without `effect` |
| `write_method` | `effect: write` on a method other than `POST`, `PUT`, `PATCH` or `DELETE`, or with no `op` |
| `undeclared_variable` | a variable in `base_url` or an OAuth URL that `config` does not declare |
| `auth_default`, `auth_incomplete`, `auth_misplaced`, `provider_id` | the profile rules above |
| `auth_library`, `auth_profile_missing` | `auth_library`: a profile's `use` is not a plain lowercase name of `a-z`, `0-9` and `-`; `auth/<name>.yaml` under the definitions root, which that name resolves to, cannot be read or is not a `urn:beyond10x:connectors-auth:1` file; or the profile it supplies carries `use` or `default`, which only a connector's profile may. `auth_profile_missing`: that file has no profile of the same id |
| `auth_lifetime` | a profile's `evidence_lifetime_ms` outside 1 to 300,000 |
| `unreadable` | a file the definition names (the document or its amendments) is missing or cannot be opened |
| `body_type` | a body notation outside the table above, two body keys naming one path (`x` and `/x`) or one inside another (`/a` and `/a/b`), or a guard reading a body key the body does not admit or fixes |
| `body_enum` | a `body_enums` entry other than distinct non-empty values of a key `body` declares as a `string` the caller sends |
| `unknown_guard_read`, `guard_parameter` | a guard read or binding the file cannot resolve |
| `unknown_paging`, `ambiguous_paging`, `paging_parameter` | a paging style the file does not declare, more than one that matches, or one whose parameters the operation lacks |

The full list is the `Refusal` type in `crates/connectors-catalog/src/definition.rs`.

## Invoke it

`operations invoke --definition <file> --operation <key>` runs one operation of the file with no
configured instance. The first run asks once whether to connect, then prompts with the profile's
`label`:

```sh
connectors operations invoke --definition defs/connectors/gitlab-reads.yaml --operation me.get
connectors operations invoke --definition defs/connectors/gitlab-reads.yaml \
  --operation project.get --input-json '{"id":"group/project"}'
```

Run in a tree; output pasted in a later revision.

The definitions root defaults to the parent of the directory holding the file, so a file in
`defs/connectors/` reads `defs/upstream/` and `defs/auth/`. A file kept anywhere else names its
tree:

```sh
connectors operations invoke --definition drafts/gitlab-reads.yaml --definitions-root defs \
  --operation me.get
```

Run in a tree; output pasted in a later revision.

A file the lowerer refuses is `invalid_input` at `arguments` and leaves nothing behind. The
instance is `gitlab-reads/definition`; the [definition file guide](invoke-a-definition-file.md)
covers the connect question, credential reuse, the cache and a file moved to another host.

## What the format cannot express today

- **Composed writes.** One operation is one request, with its guard reads around it. A write
  that takes two requests, such as an upload address followed by a completion, is two
  operations, each invoked and approved on its own.
- **Login exchange.** The schemes are `basic`, `bearer`, `header`, `oauth2_refresh` and
  `oauth2_client_credentials`. An API that trades a password or key for a session token at a
  login endpoint cannot be described.
- **Fixed values beyond scalars.** `=value` is a string, an integer or a boolean. A fixed array,
  object or null cannot be written.
- **A fixed document with a caller's member.** A nested document cannot be fixed with one member
  left to the caller. Without `body`, a document body is sent as given; nothing types its
  members.
- **A host without a default.** A `base_url` variable without a `default` lowers, but
  `--definition` cannot invoke it.
