---
title: Atlassian
sidebar_position: 7
description: Native document contracts and a typed model for Jira and Confluence; the runtime reads go through the catalog provider.
---

# Atlassian

Native document contracts for Jira and Confluence.

The authored document profile describes query dialects, document content, versioning and bounded
results. Provider authentication and mapping remain separate from shared access semantics.

This directory has native contract and ESS sources and no runnable Atlassian adapter. Jira Cloud
and Confluence Cloud reads run today through the [catalog provider](../catalog/index.md) from
their pinned REST documents, not through these profiles.

## Native contract reference

- [Atlassian documents](./contracts/documents.md)
- The [typed native model](./model/index.md), generated from `adapters/atlassian/spec/ess`
