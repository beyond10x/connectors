# GitHub through the catalog provider

The catalog provider reads, from GitHub's REST API, what waits on the
authenticated user (review requests, assigned issues and pull requests, and
notifications) and public repository metadata (an organization's repositories,
one repository, its releases, its README and single files as text). Every
operation is a read: nothing is written, and no notification is marked as
read. Nothing here is GitHub-specific code: the pinned REST description is
compiled into a bundle, a reviewed selection set exposes ten reads, and the
engine described in [the catalog provider guide](local-catalog-provider.md)
binds and sends them. Configuration, connection, approval and invocation work
exactly as described there; this page covers what differs for GitHub.

## Source and bundle

The pinned source is
[`adapters/github/upstream/api.github.com.yaml`](../adapters/github/upstream/README.md):
GitHub's published OpenAPI description of `api.github.com` from
[github/rest-api-description](https://github.com/github/rest-api-description)
release `v2.1.0` (MIT licensed; the licence is vendored beside it), upstream
SHA-256 `4a5394a12ecd1dd0f386ab0eddc133befca3ab831f56f902d25767d64c41684c`,
with example credentials and email addresses redacted by
`connectors-build redact` (rule `upstream-redaction/4`, described in the source
README); the redacted SHA-256 is `5e9edc27c465626b020eb6d9b4d5892e93b4e96f8c7af53ff952ac59b48c0df9`. The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider github \
  --source adapters/github/upstream/api.github.com.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile github.token
```

It carries all 859 operations of the pinned description;
one is reported unsupported (`repos/upload-release-asset`, whose own `servers` override the document's), and no shipped read is among them.
`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle or index a
fresh run would not reproduce byte for byte.

## The shipped selection set

[`adapters/catalog/providers/github/operations.json`](../adapters/catalog/providers/github/operations.json)
exposes ten reads and nothing else. `adapters/catalog/tests/github.rs` pins
this exact id list and each id's `operationId` and path in the pinned
description, so a renamed or dropped id, or a source operation that moved,
fails the gate.

Every list returns one page per call; the caller sends `page` (from 1) and
`per_page` and walks on. GitHub's `Link` header is not read: a walk ends on a
page with fewer items than `per_page`. The provider returns `status`, `body`
and `provenance`; `body` is GitHub's JSON answer, or the file's text for
`readme.raw` and `file.raw`.

| id | pinned `operationId` | request | paging | end condition | time filter |
|---|---|---|---|---|---|
| `review_requests.list` | `search/issues-and-pull-requests` | `GET /search/issues` with `q` fixed to `is:pr is:open review-requested:@me` | `per_page` (1 to 100), `page` | a page shorter than `per_page`, or the 1,000th result | `since`, appended to `q` as `updated:>=<since>` |
| `issues.assigned` | `issues/list` | `GET /issues` with `filter=assigned` and `state=open` fixed: issues and pull requests | `per_page` (1 to 100), `page` | a page shorter than `per_page` | `since`: updated at or after |
| `notifications.list` | `activity/list-notifications-for-authenticated-user` | `GET /notifications`, unread unless `all=true` | `per_page` (1 to 50), `page` | a page shorter than `per_page` | `since`: updated after |
| `user.me` | `users/get-authenticated` | `GET /user` | single item | n/a | none |
| `repos.list` | `repos/list-for-org` | `GET /orgs/{org}/repos`, with `type`, `sort`, `direction` | `per_page` (1 to 100), `page` | a page shorter than `per_page` | none |
| `repo.get` | `repos/get` | `GET /repos/{owner}/{repo}`: `description`, `topics`, `homepage`, `stargazers_count`, `pushed_at`, `archived`, `visibility` | single item | n/a | none |
| `releases.list` | `repos/list-releases` | `GET /repos/{owner}/{repo}/releases`: `tag_name`, `name`, `published_at`, `body` | `per_page` (1 to 100), `page` | a page shorter than `per_page` | none |
| `release.latest` | `repos/get-latest-release` | `GET /repos/{owner}/{repo}/releases/latest` | single item | n/a | none |
| `readme.raw` | `repos/get-readme` | `GET /repos/{owner}/{repo}/readme`, with `ref`, answered as text | single item | n/a | none |
| `file.raw` | `repos/get-content` | `GET /repos/{owner}/{repo}/contents/{path}`, with `ref`, answered as text | single item | n/a | none |

- **`review_requests.list`** sends GitHub's issue search with a query the
  selection fixes (`query_fixed`): `q` is not caller input, and an input
  carrying it is refused before any request. The only caller inputs besides
  paging are `since`, an ISO 8601 date (`2026-10-01`) or date-time
  (`2026-10-01T00:00:00Z`), which the engine appends to `q` as
  `updated:>=<since>` (`query_folds`), and nothing else: `sort` and `order`
  are withheld. A `since` of any other form is refused as `invalid_input`, so
  it cannot add a qualifier to the search. The search serves at most the first
  1,000 results of a query, whatever its `total_count` says: GitHub answers a
  page beyond them with `422`, refused as `invalid_input`. A walk therefore
  ends at a short page or once `page × per_page` reaches 1,000;
  `adapters/catalog/tests/github.rs` walks ten full pages of 100 and is
  refused the eleventh. The search API has its own rate limit (30 requests a
  minute for an authenticated user, GitHub's REST documentation).
- **`issues.assigned`** fixes `filter=assigned` and `state=open`. GitHub's
  `/issues` lists pull requests too; a pull request carries a `pull_request`
  member. `labels`, `sort`, `direction` and the other declared filters are
  accepted by name.
- **`notifications.list`** reads without marking anything as read; `all=true`
  includes notifications already read, `participating=true` keeps those in
  which the user participates, `before` bounds the other end.
- **`readme.raw`** and **`file.raw`** decode the base64 `content` of GitHub's
  JSON answer (`embedded_text`) and return the file as text. A file over
  1 MiB (by its `size`, before decoding) is refused as `capacity`: GitHub
  serves no `content` above 1 MB in this form. A file that is not UTF-8 or
  holds a NUL byte is refused as `unsupported`. A directory, a symlink or a
  submodule answers something other than an encoded file and is refused as
  `upstream_protocol`. The engine sends `path` as one path segment, so a
  nested path such as `docs/guide.md` is sent as `docs%2Fguide.md`; whether
  GitHub resolves that form has not been checked against the live API.

Every other query parameter the pinned description declares for an operation
is accepted by name; one it does not declare is refused before any request.

### Requests and versions

Each request carries `Authorization: Bearer <token>` and the transport's
`Accept: */*`, which GitHub answers with its default JSON media
type, and no `X-GitHub-Api-Version` header, so GitHub's default REST API
version applies: `2022-11-28` (GitHub, "API Versions", read 2026-10-17).

### Rate limits

Each selection declares GitHub's rate-limit headers (`rate_limit_headers`). A
`403` or `429` answer whose `x-ratelimit-remaining` is `0` is refused as
`rate_limited`, whatever its message, with `retry_after_seconds` the time until
the Unix time in `x-ratelimit-reset`, or the `Retry-After` delay when GitHub
sends one (its secondary limits do). The provider sends such a read once more
only when that wait fits before the invocation deadline
([Limits](local-catalog-provider.md#limits)), so a reset minutes away is
returned to the caller, never waited for or retried in a loop. Every other
`403` is `forbidden`.

## Authentication

A GitHub token sent as `Authorization: Bearer <token>`, under the profile
`github.token`. The identity read is `GET /user` (`users/get-authenticated`),
whose `id` names the credential's subject (`github.user`).

- A fine-grained personal access token reads public repositories with no
  permission granted. Review requests and assigned issues of private
  repositories need read access to them (`Issues`, `Pull requests` and
  `Metadata` read). The notifications API does not accept fine-grained
  tokens: `notifications.list` needs a classic token with the
  `notifications` scope (or `repo`), GitHub's REST documentation says.
- A classic personal access token needs `notifications` for
  `notifications.list` and `repo` for private repositories; public reads need
  no scope.

The profile declares no `scopes` read and no `minimum_scopes`, so connecting
does not check a token's scopes; a read the token may not perform is refused
as `forbidden` or `not_found` when it is sent.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "github",
  "provider": "github",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://api.github.com",
  "auth": {
    "profile": "github.token",
    "header": "Authorization",
    "bearer": true,
    "label": "GitHub token",
    "identity": {"path": "user", "kind": "github.user", "subject_pointer": "/id"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/github/operations.json"
}
```

## Set up a local install

Download the archive for your platform, and `SHA256SUMS`, from the release
page (<https://github.com/beyond10x/connectors/releases>), check it and
unpack it:

```sh
sha256sum -c --ignore-missing SHA256SUMS
tar -xzf connectors-<version>-x86_64-unknown-linux-gnu.tar.gz
```

Write the configuration above to an owner-only file, with the absolute paths
of the unpacked `adapters/catalog/generated/bundles` and
`adapters/catalog/providers/github/operations.json` (or of a checkout), then
bind the provider to the local CLI as
[the catalog provider guide](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
describes, with `profiles = ["github.token"]` and the ten ids above as its
`operations`. Connect with the credential document `{"token": "<token>"}`:

```sh
connectors --output json connections connect --adapter github --profile github.token --credential-prompt
connectors --output json operations describe --adapter github --operation review_requests.list
```

## Limits

- Verified against a local HTTPS fixture only
  (`adapters/catalog/tests/github.rs`), with synthetic records under
  `example.com`: the exact request of each read with its time filter, bearer
  header, `Accept` and no version header, the returned body, a two-page walk
  of each list to a short page, the search's 1,000-result cap, the rate-limit
  refusal and the identity read. Nothing has been run against the live API.
- No anonymous profile: the catalog provider's local host requires a
  credential for every connection, so the public reads also run with a token.
  GitHub allows 60 unauthenticated requests an hour and 5,000 with a token.
- The pinned description is GitHub's last tagged release of the repository
  (`v2.1.0`, 2022-10-25); GitHub publishes later descriptions only on its
  default branch. Each shipped operation is present in it unchanged.
- The engine parses and re-serialises the body, so it is returned as equal
  JSON, not as GitHub's exact bytes.
