---
id: S-063
title: "The deployment list pages instead of refusing"
pillar: Platform
status: ready
design: ../design/14-mcp-transport-for-the-hosted-connectors-server.md
epic: mcp-entry
areas: [integrations, server]
---

# The deployment list pages instead of refusing

## Goal

`tool_invoke k8s_deployment_list {namespace: "latest"}` refuses with `result_too_large`
(e2e 2026-08-24): the namespace holds too many deployments for one response envelope, and the
tool exposes no way to page or bound the result. A bounded refusal is correct fail-closed
behaviour, but a list tool that cannot list a busy namespace is not usable.

## Acceptance

- The MCP tool accepts an optional page bound and cursor (mirroring the underlying datasource
  list verb's `limit`/`cursor`), defaulting to a size that fits the envelope.
- A follow-up page is reachable via the returned cursor; `result_too_large` no longer occurs
  for the default page size on the dev cluster's `latest` namespace.
- The tool description names the paging contract.
