---
title: Catalog provider
sidebar_position: 2
description: The providers the catalog provider serves from pinned API documents, each with its authentication and how far it has been verified.
---

# Catalog provider

`connectors-catalog-provider` loads a digest-verified bundle built from a pinned API document and
exposes a reviewed selection of its operations through the local CLI, with no Rust per endpoint.
Reads are one bound GET; writes are one bound POST, PUT, PATCH or DELETE under the host's
approval, audit and attempt controls, with an optional guard declared as data.
[The catalog provider](../../../concepts/catalog-provider.md) explains the model, and
[Build a catalog bundle](../../../guides/build-a-catalog-bundle.md) builds one.

## Providers

| Provider | Operations | Authentication | Verified against | Guide |
|---|---|---|---|---|
| GitLab | 18 reads, 4 approved writes, the merge-request feed | personal access token in a header | a live GitLab | [GitLab](../gitlab/index.md) |
| Jira Cloud | issue search by JQL, one issue, comments, changelogs, transitions, creatable issue types, user search, attachment content; a guarded transition run | HTTP basic, profile `atlassian.basic` | a live Jira through the API gateway (search); local fixtures (the rest) | [Jira guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-jira.md) |
| Confluence Cloud | pages changed since a cutoff, a space's pages, one page with its body, a page's comments | HTTP basic, profile `atlassian.basic` | local fixtures | [Confluence guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-confluence.md) |
| HubSpot CRM | one object type's records, paged, and one record by id | private-app access token | local fixtures | [HubSpot guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-hubspot.md) |
| Zendesk Support | tickets, users and organizations changed since a start time, one of each by id, a ticket's comments | OAuth client credentials (`zendesk.oauth`) or an API token (`zendesk.basic`) | local fixtures; no live account yet | [Zendesk guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-zendesk.md) |
| Google Drive | files, export and changes; guarded metadata writes | OAuth refresh token from browser consent | local fixtures | [Drive guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-drive.md) |
| Google Slides | presentations and pages; guarded `batchUpdate` | OAuth refresh token | local fixtures | [Slides guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-slides.md) |
| Google Calendar | calendar lists and events; guarded event writes | OAuth refresh token | local fixtures | [Calendar guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-calendar.md) |
| Gmail | messages, threads, history and labels; guarded drafts, no direct `messages.send` | OAuth refresh token | local fixtures | [Gmail guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-gmail.md) |
| Runpod | `pod.create`, `pods.list`, `pod.terminate` | API key as a bearer token; the identity is the configured connection | a local fixture | [Runpod guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-runpod.md) |
| Slack | conversations, users, the workspace, custom emoji and the token's identity; files and their content; posting a message and opening a direct message under approval; on a second connection with a user token, the conversations, history and threads that user can read, the users and message search | bot token as a bearer token, identity from `auth.test`; a user token for the user-token reads and search | a local fixture | `docs/catalog-slack.md` in the repository |

Google consent and the shared OAuth client setup are in the
[Google OAuth guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-oauth.md).
Jira and Confluence share one Atlassian profile, so one account and API token serve both.

## Provider notes

- **Zendesk** and **Slack**: authentication, paging, the failure answers and the writes under approval are on the [Zendesk](../../../connectors/zendesk.md) and [Slack](../../../connectors/slack.md) connector pages.
- **HubSpot** has no updated-since filter yet.
- **Runpod** writes are unguarded required-approval mutations. A create without a definite answer
  is `unknown` and is never sent again. `pod.create` admits fifteen body keys, among them
  `dockerStartCmd`, `dockerEntrypoint` and `networkVolumeId` (since 0.37.0); any other key is
  refused as `invalid_input` before a request.
- **Binary reads** (Slack file download, Jira `attachment.content`): the limits, redirects and the `hosts` a connection must admit are on the [Slack](../../../connectors/slack.md) and [Jira](../../../connectors/jira.md) connector pages.
- A connection follows a rebuilt bundle or selection on its next `connections revalidate`
  without credential re-entry, when its provider authority, profile and identity are unchanged.
  A profile can declare an `access` read that validation makes, so a credential that identifies
  its holder but cannot read is refused as `insufficient_scope`; Confluence declares one.

## Contracts

The provider realizes the shared [catalog contract](../../contracts/catalog.md)'s generic HTTP
profile. Its native feed binding is the [GitLab feed profile](./contracts/feed.md). The catalog
index service, which would list and locate precompiled bundles, is specified and not
implemented; directly configured providers need no index, and catalog information grants no
installation or execution authority.
