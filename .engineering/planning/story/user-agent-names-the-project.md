---
format: aep.planning-md/3
id: story:user-agent-names-the-project
kind: story
status: active
title: The HTTP User-Agent names the project's public repository
summary: connectors/<version> (https://github.com/beyond10x/connectors) for every provider
relations:
- serves: vision:independent-contract-adapters
- blocks: story:catalog-basecamp
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T10:50:29Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T10:50:29Z", actor: "human:operator", revision: 3}
---
# The HTTP User-Agent names the project's public repository

## Need

Basecamp refuses a request whose `User-Agent` names no application contact (400). The host sends
the fixed `connectors/<version>` (`crates/connectors-host/src/http.rs:14`) and a definition cannot
change it.

## Acceptance

- Every provider request carries `connectors/<version> (https://github.com/beyond10x/connectors)`.
- A test that asserted the old value fails first and then pins the new one.
- The release notes say every provider sees the new header.
