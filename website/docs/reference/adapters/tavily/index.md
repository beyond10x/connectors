---
title: Tavily
sidebar_position: 5
description: Web search, page fetch and site crawl through the shared websearch family.
---

# Tavily

Search the web, fetch pages and crawl a site through the shared
[websearch family](../../contracts/data/websearch.md).

## Current capabilities

The `tavily` adapter implements `websearch.search`, `websearch.fetch` and `websearch.crawl` as
profile `tavily/2026-10`, mapped from Tavily's pinned OpenAPI document. Every operation is a read
that Tavily serves as a POST; the host admits it through `post_json` for the three fixed paths
only, with no write approval.

Results carry each website's URL, title where Tavily gives one, description, content and
published date, with `complete`, `truncation` and `provenance`. A content longer than 128 KiB is
clipped, and items past a 3 MiB result are omitted. Search and crawl have answered live through
the local CLI; fetch has been exercised against a local fake server only.

## Access and limits

The credential profile `tavily.api-key` holds the API key in Secret Service custody; it is
checked with `GET /usage`, which spends no search credit. Tavily exposes no account identifier,
so two keys are not told apart. Every read spends Tavily credits, and nothing is cached.

The repository's [local Tavily guide](https://github.com/beyond10x/connectors/blob/main/docs/local-tavily.md)
covers the native configuration, connecting with the key on stdin, invoking each operation and
replacing the executable.

## Native contract reference

- [Tavily websearch profile](./contracts/websearch.md)
