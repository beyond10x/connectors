# Connect Confluence Cloud

Confluence is served generically from the catalogue — a `[[catalog]]` row and a credential in the
store, with no per-provider Rust and no `[confluence]` configuration section. It shares an Atlassian
cloud tenant with Jira, so it shares the cloud id and, usually, the account.

## Which token kind you hold, and which base URL it needs

| what you have | mechanism | how it travels | base URL that accepts it |
|---|---|---|---|
| An API token you minted at `id.atlassian.com` under your own account | `confluence.api_token` | Basic `email:token` | the Cloud gateway |
| An API token minted **by a service account** (organization-issued, no person behind it) | `confluence.service_api_token` | Bearer | the Cloud gateway |

The base URL for both, including Confluence's `/wiki` prefix:

```text
https://api.atlassian.com/ex/confluence/<cloud id>/wiki
```

Your cloud id is the UUID your own site reports at
`https://<your-site>.atlassian.net/_edge/tenant_info`. It is the same value your Jira connection
uses — see [Connect Jira Cloud](connect-jira.md), which states at length why the `*.atlassian.net`
site host is not the route.

**The reads address `rest/api`, not `api/v2`.** Confluence publishes both. Measured on one tenant on
2026-09-04 with a service-account API token, under the gateway base above:

| path | result |
|---|---|
| `/rest/api/space?limit=100&type=global` | HTTP 200, 22 spaces |
| `/api/v2/spaces` | HTTP 401 |

The v2 surface is not reachable by that credential kind, so this connector's reads do not use it. A
consequence worth knowing: the v1 surface addresses a space by its **key** (`ENG`), not by the
numeric id the v2 surface used, and it pages by offset rather than by an opaque cursor — so unlike
before, a caller can actually walk past the first page.

## Configuration

A service-account token, which is what an integration that will be deployed should carry:

```shell-session
connectors connect confluence \
  --as confluence.service_api_token \
  --set cloud_id=11111111-2222-3333-4444-555555555555 \
  --credential-file ~/.config/b10x/atlassian/confluence.token
```

A personal API token instead, which is Basic and therefore has two halves — the token goes to the
store, the account email is configuration:

```shell-session
connectors connect confluence \
  --as confluence.api_token \
  --set cloud_id=11111111-2222-3333-4444-555555555555 \
  --set email=you@example.com
```

Either writes a row like this, and never a credential value:

```toml
[[catalog]]
provider = "confluence"
grant_ref = "grant:confluence:local"
initiation = "platform"
allow_writes = false
credential = "confluence.service_api_token"
operator_approved = true

[catalog.endpoints]
cloud_id = "11111111-2222-3333-4444-555555555555"

# Only for the Basic mechanism. Keyed by the credential the account name joins, not by the
# configuration field's own name.
[catalog.usernames]
"confluence.api_token" = "you@example.com"
```

`connectors auth status` reports both halves. A Basic credential whose account name is missing reads
`stored-without-user-half` rather than `stored`, because a token that cannot be joined is a token
that cannot be sent.

**The same Atlassian token is stored twice.** Jira and Confluence are separate connectors with
separate authorities, so one token pasted for Jira is not visible to Confluence and must be supplied
again. That is the addressing rule, not an oversight: the alternative would be one connector holding
two products' credentials under a single address.

## What it reads

`confluence-space-list` is the `verify` probe — it takes no required argument and needs nothing
configured beyond the cloud id and the credential. Pass `type=global` to leave out the one-per-member
personal spaces, and `limit` up to Confluence's maximum of 100.

`confluence-page-get` returns the page **body**, in Confluence storage format — XHTML-like markup,
not Markdown — when it is called with `expand=body.storage,version`. That parameter is declared
`const` and ought to be pinned; the document generator drops `const` and `enum` from a published
input schema, so it has to be sent by the caller. The same is true of `type` on the two list reads.
Without `expand` the page comes back with no readable content at all, which is what this connector
could only ever do before 2026-09-04.

`confluence-page-create` and `confluence-comment-add` still address the `api/v2` surface and are
therefore **not callable** with a token the gateway serves. Rewriting them needs the v1 write shapes
grounded in the vendor's own document rather than from memory; until then this connector reads.
