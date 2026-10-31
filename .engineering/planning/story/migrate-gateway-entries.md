---
format: aep.planning-md/3
id: story:migrate-gateway-entries
kind: story
status: draft
title: setup migrate moves Jira and Confluence entries behind the Atlassian gateway
tags:
- d298
relations:
- decomposes: epic:connector-vocabulary
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`setup migrate` moves a legacy Jira or Confluence catalog entry that reaches the site through the
Atlassian gateway (`request_prefix`, today kept as `gateway_prefix`) to an instance of the `jira`
or `confluence` connector, keeping its instance id and revision.

## Context

Split from `story:migrate-writes-the-short-instance-form` in wave D298. A declaration can carry
`api_path`, and with it an endpoint's path becomes the request prefix
(`adapters/catalog/src/local.rs` `instance_configuration`). But an endpoint that already ends in
the `api_path` is refused, so every documented site-form instance (`docs/catalog-jira.md`,
`docs/catalog-confluence.md`) would be refused once the declaration gains one. A rule for site
endpoints, or a separate gateway connector, has to be decided first.

## Acceptance

- A gateway-form legacy Jira entry and Confluence entry migrate with equal revisions.
- A documented site-form instance derives as before.
