# Tavily profile `tavily/2026-10` of datasource.websearch/v1alpha1

**Status:** implemented in `adapters/tavily/src/lib.rs`; protocol tests in `adapters/tavily/tests/protocol.rs`.

This profile binds the [websearch family](../../../../../contracts/datasources/websearch/v1alpha1/semantics.md)
to Tavily's API as the pinned OpenAPI document describes it
(`../../../upstream/tavily-openapi.json`, SHA-256
`b84132a5df9fa476cc38aac79083a28085ea2356e379af34f323975d640b91db`, retrieved 2026-10-05). Server
`https://api.tavily.com/`. Every operation is a read Tavily serves as a POST; the adapter sends it
through `AuthenticatedHttp::post_json`, whose paths its composition fixes to `search`, `extract` and
`crawl`. `map`, `research`, `usage` for business reads, `feedback` and `logs` are not exposed.

## Credential

Profile `tavily.api-key`, scheme `http_bearer`: the protected entry `{"api_key": "<key>"}` is sent as
`Authorization: Bearer <key>`. The connection is checked by `GET /usage`, which spends no search
credit. Tavily exposes no account identifier: the identity recorded for a connection is the kind
`tavily.api-key` and, where Tavily reports one, the account's plan name. Two keys are therefore not
told apart by identity, and a repair cannot detect that a key belongs to another account.

## Mapping

| family | Tavily |
|---|---|
| `search.query` | `POST /search` `query` |
| `search.max_results` (1–20) | `max_results` |
| `search.content: none` / `snippet` (default) | `include_raw_content: false`; `content` is null or the result's `content` |
| `search.content: full` | `include_raw_content: "markdown"`; `content` is `raw_content`, else the snippet |
| `search.time_range`, `topic`, `include_domains`, `exclude_domains`, `country`, `language` | the parameters of the same names |
| always sent | `search_depth: "basic"`, `include_published_date: true` |
| result `url`, `title`, `published`, `score` | `url`, `title`, `published_date` (an ISO date kept as is, an RFC 2822 GMT instant converted to RFC 3339, anything else null), `score` (rendered as a decimal string) |
| result `description` | the result's `content`, Tavily's snippet of the page |
| `fetch.urls` (1–20) | `POST /extract` `urls`, with `format: "markdown"`, `extract_depth: "basic"` |
| page `url`, `content` | `results[].url`, `results[].raw_content`; `title` is null, Tavily gives none |
| failure `url`, `reason` | `failed_results[].url`; `reason` is always `provider_refused`, and Tavily's `error` text is not passed on |
| `crawl.url`, `limit` (1–100), `max_depth` (1–5), `max_breadth` (1–100), `select_paths`, `exclude_paths`, `allow_external`, `instructions` | `POST /crawl`, the parameters of the same names, with `format: "markdown"`, `extract_depth: "basic"`, `timeout: 150` |

A result or page without a `url` is dropped. Every other Tavily parameter (`search_depth` values
other than `basic`, `topic: finance`, `include_answer`, `include_images`, `chunks_per_source`,
`auto_parameters`, `exact_match`, date windows, `select_domains`, `include_usage`) is refused by name:
the input schema admits only the family's fields.

## Completeness and truncation

- `search` is complete when Tavily returned `max_results` results; fewer is `provider_limit`, because
  Tavily never states that it has no more.
- `fetch` is complete when every requested URL is in `pages` or `failed`.
- `crawl` is complete when it returned fewer pages than `limit`; reaching `limit` is `result_limit`.
- A content longer than 131,072 bytes is clipped on a UTF-8 boundary (`content_bytes`). Items past a
  serialized result of 3 MiB are omitted (`response_bytes`), and the result is then incomplete.

## Errors and costs

| Tavily status | family error |
|---|---|
| 400, 422 | InvalidInput |
| 401 | Unauthorized |
| 403 | Forbidden |
| 429 | RateLimited |
| 432 (plan limit), 433 (pay-as-you-go limit) | Capacity |
| 5xx | Unavailable |
| anything else, or a malformed body | UpstreamProtocol |

No Tavily error text, request body or key enters an error. A `search` costs one search credit at
`basic` depth; a `fetch` and a `crawl` cost extract credits per page Tavily reads (Tavily's
published pricing, not checked by this adapter). The transport waits at most 160 s; Tavily bounds a
crawl at 150 s.
