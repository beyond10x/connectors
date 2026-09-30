---
format: aep.planning-md/3
id: story:catalog-api-base-gateway-prefix
kind: story
status: active
title: A catalog api_base may carry a gateway prefix in front of the document's paths
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: docs/catalog-confluence.md
- confidence: cited
  path: docs/catalog-jira.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T14:11:04Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-30T14:11:05Z", actor: "human:timo", revision: 4}
---
## Source

Live setup on 2026-09-30 with release 0.18.0. Atlassian service-account API tokens and OAuth 2.0 (3LO) tokens only
work through the API gateway, `https://api.atlassian.com/ex/jira/<cloud id>/…` and `…/ex/confluence/<cloud id>/…`, not
through the site URL. With `api_base = https://api.atlassian.com/ex/jira/<cloud id>/rest/api/3`,
`--print-local-bootstrap` refuses `InvalidConfiguration`: every selected operation's path (`/rest/api/3/…`) must lie
under the `api_base` path (`adapters/catalog/src/lib.rs:259`, `path_within`), and the gateway prefix precedes it.
So neither credential available on the operator's machine can connect Jira or Confluence.

## Acceptance

- The native configuration can state a request prefix in front of the document's paths (for example
  `api_base = https://api.atlassian.com/ex/jira/<cloud id>` with the document's `/rest/api/3/...` paths appended), and
  the base-path check applies to the document's paths after the prefix; a path outside the document base is still
  refused.
- The identity read uses the same prefix; `docs/catalog-jira.md` and `docs/catalog-confluence.md` show the gateway
  form for service-account and OAuth tokens beside the site form.
- A fixture test sends a Jira and a Confluence read through a prefixed base and asserts the exact request path;
  GitLab, Jira and Confluence configurations without a prefix are unchanged (same configuration revision).
