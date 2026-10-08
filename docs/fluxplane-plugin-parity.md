# fluxplane-plugin parity

**2026-10-07.** Connectors is to replace `fluxplane-plugin` in the operator's Claude Code and
Codex sessions. To do that it needs functional parity with the fluxplane operations those
sessions actually use. This page compares every declared fluxplane operation and CLI verb with
what the Connectors local CLI serves today. It weights each gap by the calls made since
2026-09-09 and groups the gaps into implementable units and waves.

Sources:

- **fluxplane inventory.** A working file generated on 2026-10-07 from the installed plugin
  state, the plugin skill references and the session transcripts since 2026-09-09. It is not in
  this repository. It is cited as `fluxplane inventory §N`.
  - It holds 25 plugins and 301 declared operations, 109 of them used, with 6,356 calls to
    declared operations.
  - A call is one invocation site in one tool call. It is an attempt: a site inside a loop
    counts once, and success is not checked (fluxplane inventory §1 *Limits*).
- **Connectors.** Commit `fef5e36f7`, source release v0.31.0. The operation selections are
  unchanged since `ba8968e3e0`.
  - Catalog operations are cited as `adapters/catalog/providers/<provider>/operations.json:<line>`.
    A table cell that shows only `(:<line>)` refers to that plugin's file, which its section
    names.
  - Native operations are cited as `adapters/<adapter>/spec/adapter.json:<line>`.
- **Recheck, 2026-10-08.** Against source release v0.35.0 (`aedd89aa4`) the selected operation
  ids are the same 80 as at `fef5e36f7` (every `adapters/catalog/providers/*/operations.json` and
  `adapters/*/spec/adapter.json`), so no verdict below changes. The call counts are still those
  of 2026-10-07; they were not recounted.
- **Recount, 2026-10-08 18:03 UTC.** Calls from 2026-09-09 to 2026-10-08, read from the Claude
  Code and Codex session transcripts on this machine: every `fluxplane-plugin operation
  invoke|call <plugin> <operation>` site in a shell tool call, each tool call counted once by its
  id. Declared and undeclared operation names both count. The tables below keep the 2026-10-07
  counts; the stories of `epic:fluxplane-plugin-parity` are ordered by this recount.

  | plugin | calls | verdict |
  |---|---:|---|
  | gitlab | 2,541 | covered |
  | jira | 1,415 | covered |
  | slack | 710 | missing |
  | sql | 663 | partial only |
  | grafana | 394 | missing |
  | loki | 334 | missing |
  | kubernetes | 81 | covered |
  | homer | 33 | missing |
  | confluence | 13 | covered |
  | alertmanager | 2 | missing |
  | prometheus | 2 | missing |
  | the other 11 missing plugins | 0 | missing |
  | **total** | **6,191** (148 operation names) | |

## Summary

| verdict | declared operations | calls | used operations (calls > 0) | calls |
|---|---:|---:|---:|---:|
| covered | 29 | 2,659 | 20 | 2,659 |
| partial | 23 | 1,273 | 16 | 1,273 |
| missing | 249 | 2,424 | 73 | 2,424 |
| **total** | **301** | **6,356** | **109** | **6,356** |

Some calls used operation names that the plugin does not declare (fluxplane inventory §4). They
are mapped to the operation they most likely meant in [Used but not declared](#used-but-not-declared):

- 122 calls are mapped from §4a and §4b, and 2 more from §4d.
- 33 of them land on covered capabilities. The other 91 are added to the gap units.

Per plugin (operations / calls):

| plugin | declared | used | calls | covered | partial | missing |
|---|---:|---:|---:|---|---|---|
| gitlab | 64 | 38 | 2,670 | 15 / 2,183 | 8 / 206 | 41 / 281 |
| jira | 21 | 13 | 1,364 | 3 / 440 | 1 / 354 | 17 / 570 |
| slack | 30 | 16 | 704 | 0 / 0 | 0 / 0 | 30 / 704 |
| sql | 6 | 6 | 703 | 0 / 0 | 6 / 703 | 0 / 0 |
| grafana | 20 | 7 | 425 | 0 / 0 | 0 / 0 | 20 / 425 |
| loki | 5 | 4 | 370 | 0 / 0 | 0 / 0 | 5 / 370 |
| kubernetes | 24 | 14 | 72 | 6 / 31 | 6 / 10 | 12 / 31 |
| homer | 8 | 5 | 32 | 0 / 0 | 0 / 0 | 8 / 32 |
| confluence | 15 | 3 | 13 | 4 / 5 | 0 / 0 | 11 / 8 |
| prometheus | 8 | 2 | 2 | 0 / 0 | 0 / 0 | 8 / 2 |
| alertmanager | 5 | 1 | 1 | 0 / 0 | 0 / 0 | 5 / 1 |
| asterisk | 8 | 0 | 0 | 0 / 0 | 0 / 0 | 8 / 0 |
| aws | 11 | 0 | 0 | 0 / 0 | 0 / 0 | 11 / 0 |
| clock | 0 | 0 | 0 | — | — | — |
| docker | 44 | 0 | 0 | 0 / 0 | 0 / 0 | 44 / 0 |
| duckduckgo | 1 | 0 | 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| git | 6 | 0 | 0 | 0 / 0 | 0 / 0 | 6 / 0 |
| ollama | 7 | 0 | 0 | 0 / 0 | 0 / 0 | 7 / 0 |
| openai | 3 | 0 | 0 | 0 / 0 | 0 / 0 | 3 / 0 |
| opsgenie | 8 | 0 | 0 | 0 / 0 | 0 / 0 | 8 / 0 |
| sleep | 1 | 0 | 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| system | 1 | 0 | 0 | 0 / 0 | 0 / 0 | 1 / 0 |
| tavily | 1 | 0 | 0 | 1 / 0 | 0 / 0 | 0 / 0 |
| vision | 2 | 0 | 0 | 0 / 0 | 0 / 0 | 2 / 0 |
| websearch | 2 | 0 | 0 | 0 / 0 | 2 / 0 | 0 / 0 |
| **total** | **301** | **109** | **6,356** | **29 / 2,659** | **23 / 1,273** | **249 / 2,424** |

Per plugin, the verdict of the plugin as a whole: **missing** when no declared operation is
covered or partial, otherwise the best verdict any of its operations reaches.

| verdict | plugins | count |
|---|---|---:|
| covered | gitlab, jira, confluence, kubernetes, tavily | 5 |
| partial only | sql, websearch | 2 |
| missing | slack, grafana, loki, homer, prometheus, alertmanager, asterisk, aws, docker, duckduckgo, git, ollama, openai, opsgenie, sleep, system, vision | 17 |
| no operations declared | clock | 1 |

## Verdict rules

- **covered:** a Connectors operation or CLI verb gives the same capability against the same
  provider. The row names it.
  - A difference in presentation, such as Atlassian Document Format instead of Markdown, is
    noted in the gap column but does not lower the verdict.
- **partial:** the same provider and capability family, but a narrower capability. The gap
  column says what is missing: a write, a filter, a database engine or a field.
- **missing:** nothing in Connectors.

An operation with 0 calls still gets a verdict. A `*.test` operation is covered by
`connections revalidate` (`apps/connectors/spec/cli.yaml:375`) wherever Connectors has a
connection to that provider. That command re-runs the profile's identity probe.

## gitlab

Source: fluxplane inventory §2 gitlab.

Connectors serves GitLab through the catalog provider, `adapters/catalog/providers/gitlab/operations.json`
(line numbers below). The bundle carries all 1,847 operations of the pinned GitLab source
(README.md:43-44), so every missing GitLab row is a selection that has not been made.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `gitlab.pipeline.list` | 579 | 27 | 2026-10-06 | `pipelines.list` (:19) | covered | — |
| `gitlab.job.list` | 387 | 24 | 2026-10-07 | `pipeline.jobs` (:24) | covered | — |
| `gitlab.mr.show` | 370 | 32 | 2026-10-06 | `merge_request.get` (:17) | covered | Takes a project id and an iid; there is no `PROJECT!IID` shorthand. |
| `gitlab.repository.file.show` | 226 | 24 | 2026-10-06 | `file.get` (:10) | covered | Content comes back base64-encoded, as GitLab sends it. No `max_bytes` bound and no binary detection. |
| `gitlab.mr.create` | 184 | 21 | 2026-10-06 | `merge_request.create` (:55) | covered | Needs the source-branch head `sha` for the guard, and an approval. |
| `gitlab.mr.list` | 122 | 23 | 2026-10-05 | `merge_requests.list` (:14) | covered | Project-scoped only. It is not established whether fluxplane also lists across projects. |
| `gitlab.repository.commit.list` | 81 | 17 | 2026-10-06 | `commits.list` (:44) | covered | — |
| `gitlab.mr.merge` | 77 | 10 | 2026-10-06 | `merge_request.merge` (:70) | partial | The guard requires `mergeable` and a succeeded pinned head pipeline, so it refuses merge-when-pipeline-succeeds and projects without a pipeline. |
| `gitlab.compare` | 76 | 19 | 2026-09-30 | `repository.compare` (:48) | covered | No per-file diff bound. |
| `gitlab.search.blobs` | 75 | 12 | 2026-10-06 | — | missing | Code search is not selected (`/projects/:id/search`, scope `blobs`). |
| `gitlab.repository.tag.list` | 71 | 23 | 2026-10-03 | `tags.list` (:35) | covered | — |
| `gitlab.mr.update` | 60 | 12 | 2026-10-04 | `merge_request.update` (:62) | partial | The guard requires state `opened`, so reopening a closed merge request is refused. |
| `gitlab.mr.changes` | 58 | 13 | 2026-10-03 | `merge_request.get` (:17) + `repository.compare` (:48) | partial | No merge-request diffs read (`/merge_requests/:iid/diffs` is not selected). The diffs can only be had by comparing the request's `diff_refs`. |
| `gitlab.project.list` | 45 | 12 | 2026-10-03 | `projects.list` (:32) | covered | — |
| `gitlab.mr.discussion.list` | 43 | 11 | 2026-10-06 | — | missing | Merge-request discussions are not selected. |
| `gitlab.project.show` | 36 | 19 | 2026-10-03 | `project.get` (:5) | covered | — |
| `gitlab.repository.tree` | 35 | 13 | 2026-10-06 | — | missing | Repository tree is not selected. |
| `gitlab.mr.note.create` | 20 | 10 | 2026-10-03 | — | missing | The merge-request note write is not selected. |
| `gitlab.pipeline.retry` | 19 | 5 | 2026-10-05 | — | missing | Pipeline retry is not selected. |
| `gitlab.release.create` | 16 | 3 | 2026-10-02 | — | missing | The release write is not selected. |
| `gitlab.repository.tag.create` | 16 | 5 | 2026-09-20 | — | missing | The tag write is not selected. |
| `gitlab.pipeline.cancel` | 13 | 5 | 2026-10-06 | — | missing | Pipeline cancel is not selected. |
| `gitlab.repository.commit.create` | 12 | 2 | 2026-09-20 | — | missing | The commit write is not selected. |
| `gitlab.project.create` | 11 | 7 | 2026-10-02 | — | missing | The project write is not selected. fluxplane also resolves the group namespace by path. |
| `gitlab.repository.file.update` | 6 | 2 | 2026-09-20 | — | missing | The file write is not selected. |
| `gitlab.release.list` | 5 | 3 | 2026-10-02 | `releases.list` (:38) | covered | — |
| `gitlab.repository.tag.show` | 5 | 3 | 2026-09-28 | `tags.list` (:35) | partial | No single-tag read. `tags.list` with `search` finds the tag. |
| `gitlab.branch.create` | 4 | 1 | 2026-09-20 | — | missing | The branch write is not selected. |
| `gitlab.release.show` | 4 | 3 | 2026-10-02 | `releases.list` (:38) | partial | No single-release read. |
| `gitlab.branch.delete` | 3 | 2 | 2026-10-01 | — | missing | Branch delete is not selected. |
| `gitlab.pipeline.create` | 3 | 2 | 2026-10-06 | — | missing | The CI trigger is not selected. |
| `gitlab.release.link.list` | 2 | 1 | 2026-10-02 | `releases.list` (:38) | partial | Links appear only inside each release's `assets`. There is no per-release link list. |
| `gitlab.deployment.list` | 1 | 1 | 2026-09-28 | `deployments.list` (:50) | covered | — |
| `gitlab.environment.list` | 1 | 1 | 2026-09-28 | — | missing | Environments are not selected. |
| `gitlab.mr.discussion.reply` | 1 | 1 | 2026-09-24 | — | missing | The discussion write is not selected. |
| `gitlab.mr.discussion.resolve` | 1 | 1 | 2026-09-24 | — | missing | The discussion write is not selected. |
| `gitlab.release.update` | 1 | 1 | 2026-09-29 | — | missing | The release write is not selected. |
| `gitlab.repository.tag.delete` | 1 | 1 | 2026-09-20 | — | missing | Tag delete is not selected. |
| `gitlab.branch.delete_merged` | 0 | 0 | - | — | missing | Not selected. |
| `gitlab.ci.variable.create` | 0 | 0 | - | — | missing | CI variables are not selected. |
| `gitlab.ci.variable.delete` | 0 | 0 | - | — | missing | CI variables are not selected. |
| `gitlab.ci.variable.update` | 0 | 0 | - | — | missing | CI variables are not selected. |
| `gitlab.index.build` | 0 | 0 | - | — | missing | A fluxplane-local index. Connectors keeps no index. |
| `gitlab.issue.create` | 0 | 0 | - | `issue.create` (:53) | covered | Unguarded: the same approved input sent twice opens a second issue. |
| `gitlab.issue.list` | 0 | 0 | - | `issues.list` (:7) | partial | Project-scoped. fluxplane also lists across accessible projects. |
| `gitlab.issue.note.create` | 0 | 0 | - | — | missing | Issue notes are not selected. |
| `gitlab.issue.note.list` | 0 | 0 | - | — | missing | Issue notes are not selected. |
| `gitlab.issue.show` | 0 | 0 | - | `issues.list` (:7) | partial | No single-issue read. `issues.list` with `iids[]` finds the issue. |
| `gitlab.issue.update` | 0 | 0 | - | — | missing | Issue update is not selected. |
| `gitlab.mr.approve` | 0 | 0 | - | — | missing | Merge-request approval is not selected. |
| `gitlab.mr.diff.lines` | 0 | 0 | - | — | missing | Typed diff-line parsing is composition done on the fluxplane side. |
| `gitlab.mr.discussion.create` | 0 | 0 | - | — | missing | The discussion write is not selected. |
| `gitlab.release.delete` | 0 | 0 | - | — | missing | Release delete is not selected. |
| `gitlab.release.link.create` | 0 | 0 | - | — | missing | Release-link writes are not selected. |
| `gitlab.release.link.delete` | 0 | 0 | - | — | missing | Release-link writes are not selected. |
| `gitlab.release.link.update` | 0 | 0 | - | — | missing | Release-link writes are not selected. |
| `gitlab.repository.archive` | 0 | 0 | - | — | missing | A binary archive. The catalog reads only JSON or text responses (`adapters/catalog/src/lib.rs:77`). |
| `gitlab.repository.changelog.add` | 0 | 0 | - | — | missing | The changelog API is not selected. |
| `gitlab.repository.changelog.generate` | 0 | 0 | - | — | missing | The changelog API is not selected. |
| `gitlab.repository.file.create` | 0 | 0 | - | — | missing | The file write is not selected. |
| `gitlab.repository.file.delete` | 0 | 0 | - | — | missing | File delete is not selected. |
| `gitlab.snippet.create` | 0 | 0 | - | — | missing | Snippets are not selected. |
| `gitlab.snippet.delete` | 0 | 0 | - | — | missing | Snippets are not selected. |
| `gitlab.test` | 0 | 0 | - | `connections revalidate` (identity probe) | covered | — |

Connectors also serves four GitLab reads that fluxplane does not declare: `pipeline.get` (:22),
`job.get` (:27), `job.trace` (:29) and `branch.get` (:12). The sessions tried two of them under
undeclared names, `gitlab.pipeline.show` and `gitlab.repository.branch.show` (see
[Used but not declared](#used-but-not-declared)).

## jira

Source: fluxplane inventory §2 jira.

Connectors catalog provider: `adapters/catalog/providers/jira/operations.json`. The pinned source
`adapters/atlassian/upstream/jira-platform-v3.json` carries `getIssue`, `createIssue`,
`editIssue`, `deleteIssue`, `addComment`, `updateComment`, `deleteComment`, `getTransitions`,
`doTransition`, `linkIssues`, `getCreateIssueMetaIssueTypes`, `getEditIssueMeta`, `findUsers`,
`getAttachmentContent`, `addAttachment` and `removeAttachment`. None of them is selected.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `jira.issue.show` | 354 | 49 | 2026-10-07 | `issues.search` (:5) | partial | No single-issue read (`getIssue` is not selected). JQL `key = <KEY>` returns the issue. Bodies stay in ADF, not Markdown. |
| `jira.issue.search` | 310 | 34 | 2026-10-06 | `issues.search` (:5) | covered | JQL only. Rich-text fields come back as ADF, not Markdown. |
| `jira.issue.comment.add` | 141 | 24 | 2026-10-05 | — | missing | `addComment` is not selected. Its body would be ADF; nothing converts Markdown. |
| `jira.issue.comment.list` | 130 | 20 | 2026-10-05 | `issue.comments` (:7) | covered | Bodies come back as ADF, not Markdown. |
| `jira.issue.transition.list` | 127 | 20 | 2026-10-01 | — | missing | `getTransitions` is not selected. |
| `jira.issue.create` | 115 | 24 | 2026-10-06 | — | missing | `createIssue` is not selected. There is no read-back check for fields Jira dropped. |
| `jira.issue.transition.run` | 109 | 18 | 2026-10-01 | — | missing | `doTransition` is not selected. It would run by transition id only: no by-name run and no walk to a target status. |
| `jira.issue.link.add` | 40 | 11 | 2026-09-28 | — | missing | `linkIssues` is not selected. |
| `jira.issue.edit` | 26 | 10 | 2026-09-24 | — | missing | `editIssue` is not selected. |
| `jira.issue.create_meta` | 5 | 5 | 2026-10-06 | — | missing | `getCreateIssueMetaIssueTypes` is not selected. |
| `jira.issue.delete` | 3 | 1 | 2026-09-16 | — | missing | `deleteIssue` is not selected. |
| `jira.issue.attachment.get` | 2 | 1 | 2026-09-17 | — | missing | A binary download (`getAttachmentContent`). The catalog reads only JSON or text responses. |
| `jira.user.search` | 2 | 2 | 2026-09-28 | — | missing | `findUsers` is not selected. |
| `jira.index.build` | 0 | 0 | - | — | missing | A fluxplane-local index. |
| `jira.issue.attachment.add` | 0 | 0 | - | — | missing | A multipart upload (`addAttachment`). It is not established whether the catalog can send a non-JSON request body. |
| `jira.issue.attachment.delete` | 0 | 0 | - | — | missing | `removeAttachment` is not selected. |
| `jira.issue.attachment.list` | 0 | 0 | - | — | missing | Attachments come with `getIssue`, which is not selected. |
| `jira.issue.comment.delete` | 0 | 0 | - | — | missing | `deleteComment` is not selected. |
| `jira.issue.comment.edit` | 0 | 0 | - | — | missing | `updateComment` is not selected. |
| `jira.issue.edit_meta` | 0 | 0 | - | — | missing | `getEditIssueMeta` is not selected. |
| `jira.test` | 0 | 0 | - | `connections revalidate` (identity read `myself`, `docs/catalog-jira.md:84`) | covered | — |

## slack

Source: fluxplane inventory §2 slack.

Connectors has no Slack provider: there is no `adapters/slack` directory and no catalog
selection. The 2026-09-09 baseline already lists Slack as uncovered
(`docs/recent-adapter-usage-20260909.md`, U12). The gap column names the Slack Web API method
that would back each operation.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `slack.thread` | 225 | 50 | 2026-10-06 | — | missing | No provider. Would be backed by `conversations.replies`. |
| `slack.message.send` | 132 | 30 | 2026-10-05 | — | missing | No provider. Would be backed by `chat.postMessage`, plus `conversations.open` for a direct message. |
| `slack.message.list` | 92 | 34 | 2026-10-05 | — | missing | No provider. Would be backed by `conversations.history`. |
| `slack.search` | 73 | 19 | 2026-10-05 | — | missing | No provider. Would be backed by `search.messages`, which needs a user token. |
| `slack.user.list` | 41 | 17 | 2026-10-05 | — | missing | No provider. Would be backed by `users.list`. |
| `slack.channel.list` | 40 | 20 | 2026-10-05 | — | missing | No provider. Would be backed by `conversations.list`. |
| `slack.file.upload` | 24 | 10 | 2026-09-29 | — | missing | No provider. Would be backed by `files.getUploadURLExternal` and `files.completeUploadExternal`: a binary upload. |
| `slack.message.edit` | 20 | 8 | 2026-10-03 | — | missing | No provider. Would be backed by `chat.update`. |
| `slack.info` | 15 | 8 | 2026-09-25 | — | missing | No provider. Would be backed by `auth.test` and `team.info`. |
| `slack.file.download` | 11 | 8 | 2026-09-29 | — | missing | No provider. A binary fetch of `url_private`. |
| `slack.message.delete` | 10 | 6 | 2026-09-29 | — | missing | No provider. Would be backed by `chat.delete`. |
| `slack.file.delete` | 7 | 4 | 2026-09-29 | — | missing | No provider. Would be backed by `files.delete`. |
| `slack.file.info` | 7 | 2 | 2026-09-29 | — | missing | No provider. Would be backed by `files.info`. |
| `slack.file.list` | 3 | 1 | 2026-09-29 | — | missing | No provider. Would be backed by `files.list`. |
| `slack.test` | 3 | 3 | 2026-09-26 | — | missing | No provider. Would be backed by `auth.test`. |
| `slack.emoji.list` | 1 | 1 | 2026-09-28 | — | missing | No provider. Would be backed by `emoji.list`. |
| `slack.bookmark.add` | 0 | 0 | - | — | missing | No provider. Would be backed by `bookmarks.add`. |
| `slack.bookmark.delete` | 0 | 0 | - | — | missing | No provider. Would be backed by `bookmarks.remove`. |
| `slack.bookmark.edit` | 0 | 0 | - | — | missing | No provider. Would be backed by `bookmarks.edit`. |
| `slack.bookmark.list` | 0 | 0 | - | — | missing | No provider. Would be backed by `bookmarks.list`. |
| `slack.channel.join` | 0 | 0 | - | — | missing | No provider. Would be backed by `conversations.join`. |
| `slack.channel.mark-read` | 0 | 0 | - | — | missing | No provider. Would be backed by `conversations.mark`. |
| `slack.download` | 0 | 0 | - | — | missing | No provider. A binary fetch of `url_private`. |
| `slack.index.build` | 0 | 0 | - | — | missing | A fluxplane-local index. |
| `slack.mentions` | 0 | 0 | - | — | missing | No provider. Would be backed by `search.messages`, plus classification done on the fluxplane side. |
| `slack.presence.get` | 0 | 0 | - | — | missing | No provider. Would be backed by `users.getPresence`. |
| `slack.presence.set` | 0 | 0 | - | — | missing | No provider. Would be backed by `users.setPresence`. |
| `slack.reaction.add` | 0 | 0 | - | — | missing | No provider. Would be backed by `reactions.add`. |
| `slack.reaction.remove` | 0 | 0 | - | — | missing | No provider. Would be backed by `reactions.remove`. |
| `slack.unreads` | 0 | 0 | - | — | missing | No provider. The backing method is not established. |

## sql

Source: fluxplane inventory §2 sql.

Connectors native adapter: `adapters/sql/spec/adapter.json`. It is PostgreSQL only (README.md:112).

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `sql.query` | 647 | 24 | 2026-10-06 | `query.read` (:251) | partial | PostgreSQL only. No MySQL or SQLite engine. |
| `sql.table.show` | 32 | 5 | 2026-09-28 | `schema.list` (:139) | partial | PostgreSQL only. Gives column names and native types; no primary keys, foreign keys or nullability. |
| `sql.table.list` | 14 | 4 | 2026-09-28 | `schema.list` (:139) | partial | PostgreSQL only. Tables are derived from column metadata; no views flag and no row estimates. |
| `sql.test` | 5 | 5 | 2026-09-28 | `connections revalidate` (the session is the credential check, `adapters/sql/src/auth.rs:14`) | partial | PostgreSQL only. |
| `sql.database.list` | 4 | 3 | 2026-09-15 | `schema.list` (:139) | partial | PostgreSQL only. Lists the schemas of the one connected database; there is no database listing. |
| `sql.index.list` | 1 | 1 | 2026-09-16 | `query.read` (:251) against `pg_indexes` | partial | PostgreSQL only. No dedicated index read. |

**Engines.** The fluxplane `sql` plugin (skill reference `sql.md`, version 0.20.0) describes
itself as "Read-only SQL query operations for MySQL, PostgreSQL, SQLite, and compatible
endpoints". Its credential method reads `MYSQL_USERNAME` and `MYSQL_PASSWORD` as well as the
generic `SQL_*` variables. Connectors serves PostgreSQL only.

**Which engine the 647 `sql.query` calls hit: unknown.** The inventory counted operation names
only. It recorded neither the `endpoint_ref` nor the engine. Two indicators point to MySQL, but
this is inference and was not verified:

- The 2026-09-09 baseline observed incident work "against several MySQL/Aurora databases" and
  PostgreSQL only for local fixtures (`docs/recent-adapter-usage-20260909.md`, U06).
- Two calls since then put `mysql` in the verb position (fluxplane inventory §4d).

## grafana

Source: fluxplane inventory §2 grafana.

Connectors has no Grafana runtime. `adapters/grafana` holds only `design.md` and an ESS model.
That design makes Grafana a mediated route: `grafana-datasource-proxy`
(`adapters/grafana/design.md:16`) forwards a child adapter's GET for Loki, Prometheus or
Alertmanager. So a Grafana-proxied query needs both the Grafana route and the child adapter.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `grafana.loki.query` | 320 | 12 | 2026-09-28 | — | missing | A LogQL range query through the datasource proxy. Needs the Grafana route and a Loki adapter. |
| `grafana.prometheus.query` | 49 | 7 | 2026-10-06 | — | missing | An instant PromQL query through the proxy. Needs the Grafana route and a Prometheus adapter. |
| `grafana.datasource.list` | 31 | 12 | 2026-10-06 | — | missing | Datasource discovery is designed but not implemented. |
| `grafana.prometheus.range` | 17 | 3 | 2026-10-06 | — | missing | A range PromQL query through the proxy. |
| `grafana.loki.labels` | 6 | 4 | 2026-09-22 | — | missing | Loki label discovery through the proxy. |
| `grafana.loki.recent_logs` | 1 | 1 | 2026-09-15 | — | missing | Recent Loki logs through the proxy. |
| `grafana.prometheus.rules` | 1 | 1 | 2026-09-23 | — | missing | Prometheus rules through the proxy. |
| `grafana.alerts.active` | 0 | 0 | - | — | missing | Alertmanager through the proxy. |
| `grafana.alerts.silences.create` | 0 | 0 | - | — | missing | An Alertmanager write through the proxy. |
| `grafana.alerts.silences.delete` | 0 | 0 | - | — | missing | An Alertmanager write through the proxy. |
| `grafana.alerts.silences.list` | 0 | 0 | - | — | missing | Alertmanager through the proxy. |
| `grafana.annotation.add` | 0 | 0 | - | — | missing | No Grafana runtime. |
| `grafana.annotation.list` | 0 | 0 | - | — | missing | No Grafana runtime. |
| `grafana.dashboard.get` | 0 | 0 | - | — | missing | No Grafana runtime. |
| `grafana.dashboard.list` | 0 | 0 | - | — | missing | No Grafana runtime. |
| `grafana.datasource.health` | 0 | 0 | - | — | missing | No Grafana runtime. |
| `grafana.folder.list` | 0 | 0 | - | — | missing | No Grafana runtime. |
| `grafana.tempo.search` | 0 | 0 | - | — | missing | No Tempo adapter. |
| `grafana.tempo.trace.get` | 0 | 0 | - | — | missing | No Tempo adapter. |
| `grafana.test` | 0 | 0 | - | — | missing | No Grafana connection. |

## loki

Source: fluxplane inventory §2 loki.

Connectors has no Loki executable. `adapters/loki` holds `design.md`, an ESS model, the
`logql-range` contract and, since 2026-10-08, a library binding of three operations tested
against recorded provider answers (`adapters/loki/contracts/logs/v1alpha1/semantics.md` §11).
The baseline's two limits of the contract (`docs/recent-adapter-usage-20260909.md`, U07),
no metric expressions and no label discovery, are closed by §11. No operation is reachable
through `operations invoke` yet: the local host admits no profile without a credential
(`loki.anonymous`), and `loki.bearer` needs the identity-validation mechanism
`adapters/loki/design.md` leaves open.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `loki.query` | 313 | 8 | 2026-09-30 | — | missing | No connection. The library binds `logs.query_range` (`adapters/loki/spec/adapter.json:52`), unpaged, at most 1,000 lines. |
| `loki.metric` | 40 | 3 | 2026-09-24 | — | missing | No connection. The library binds `logs.query_metric` (`adapters/loki/spec/adapter.json:251`), instant and range. |
| `loki.labels` | 11 | 4 | 2026-09-24 | — | missing | No connection. The library binds `logs.labels` (`adapters/loki/spec/adapter.json:421`), names and values. |
| `loki.test` | 6 | 4 | 2026-09-28 | — | missing | No Loki connection, so no `connections revalidate`. |
| `loki.recent_logs` | 0 | 0 | - | — | missing | No runtime. |

## kubernetes

Source: fluxplane inventory §2 kubernetes.

Connectors native adapter: `adapters/kubernetes/spec/adapter.json`.

- `resources.list` (:156) returns full provider objects. It covers four kinds: pods, services,
  deployments and endpointslices (`adapters/kubernetes/src/lib.rs:66`).
- Each instance configures the namespaces it may read (`adapters/kubernetes/src/lib.rs:55-70`).
- The adapter has no single-object read and no name filter. Its input is `namespace`, `kind`,
  `limit` and `cursor` (`adapters/kubernetes/src/lib.rs:387-393`).
- It advertises no writes and no process execution (README.md:115-116).

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `kubernetes.pod.list` | 17 | 7 | 2026-09-22 | `resources.list` kind `pods` (:156) | covered | Configured namespaces only. |
| `kubernetes.container.list` | 10 | 2 | 2026-09-20 | `resources.list` kind `pods` (:156) | covered | Containers are read from the full pod objects; there is no derived container list. |
| `kubernetes.namespace.list` | 10 | 4 | 2026-09-22 | — | missing | Namespaces are configured, not listed. Namespaces are not a kind. |
| `kubernetes.pod.exec` | 9 | 2 | 2026-09-15 | — | missing | Process execution is not advertised. |
| `kubernetes.cluster.list` | 5 | 5 | 2026-09-18 | `connections list` (`apps/connectors/spec/cli.yaml:320`) | partial | Lists saved Kubernetes connections, not kubeconfig contexts. |
| `kubernetes.pod.logs` | 5 | 1 | 2026-09-15 | — | missing | Specified in `adapters/kubernetes/contracts/logs` but not implemented. |
| `kubernetes.service.list` | 4 | 1 | 2026-09-19 | `resources.list` kind `services` (:156) | covered | Configured namespaces only. |
| `kubernetes.pod.show` | 3 | 2 | 2026-09-15 | `resources.list` kind `pods` (:156) | partial | No single-object read or name filter; the caller pages and matches. |
| `kubernetes.deployment.show` | 2 | 2 | 2026-09-19 | `resources.list` kind `deployments` (:156) | partial | No single-object read or name filter. |
| `kubernetes.portforward.start` | 2 | 1 | 2026-09-21 | — | missing | No port-forward. |
| `kubernetes.portforward.stop` | 2 | 1 | 2026-09-21 | — | missing | No port-forward. |
| `kubernetes.deployment.history` | 1 | 1 | 2026-09-11 | — | missing | ReplicaSets are not a kind. `helm_releases.history` (:550) lists Helm revisions, not rollout revisions. |
| `kubernetes.event.list` | 1 | 1 | 2026-09-11 | — | missing | Events are not a kind. |
| `kubernetes.secret.read` | 1 | 1 | 2026-09-11 | — | missing | No Secret read. The Helm reads disclose redacted projections only. |
| `kubernetes.container.show` | 0 | 0 | - | `resources.list` kind `pods` (:156) | partial | No single-object read. |
| `kubernetes.deployment.list` | 0 | 0 | - | `resources.list` kind `deployments` (:156) | covered | Configured namespaces only. |
| `kubernetes.deployment.restart` | 0 | 0 | - | — | missing | Restart is specified in `adapters/kubernetes/contracts/mutations`; no write is implemented. |
| `kubernetes.deployment.scale` | 0 | 0 | - | — | missing | No write. |
| `kubernetes.endpoint.discover` | 0 | 0 | - | `endpoints.discover` (:268) | covered | — |
| `kubernetes.ingress.list` | 0 | 0 | - | — | missing | Ingresses are not a kind. |
| `kubernetes.node.list` | 0 | 0 | - | `hosts.discover` (:453) | partial | Opt-in. Returns name, addresses and conditions (`adapters/kubernetes/src/lib.rs:580`); no roles, kubelet version or capacity. |
| `kubernetes.portforward.list` | 0 | 0 | - | — | missing | No port-forward. |
| `kubernetes.service.show` | 0 | 0 | - | `resources.list` kind `services` (:156) | partial | No single-object read. |
| `kubernetes.test` | 0 | 0 | - | `connections revalidate` (SelfSubjectReview probe, `adapters/kubernetes/src/auth.rs:16`) | covered | — |

## homer

Source: fluxplane inventory §2 homer.

Connectors has no Homer provider and no adapter directory. `adapters/sip` covers SIP media
sessions, not capture search.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `homer.call.list` | 13 | 3 | 2026-09-24 | — | missing | No provider. |
| `homer.call.show` | 10 | 2 | 2026-09-24 | — | missing | No provider. |
| `homer.test` | 4 | 2 | 2026-09-24 | — | missing | No provider. |
| `homer.call.qos` | 3 | 1 | 2026-09-24 | — | missing | No provider. |
| `homer.search` | 2 | 1 | 2026-09-15 | — | missing | No provider. |
| `homer.alias.list` | 0 | 0 | - | — | missing | No provider. |
| `homer.call.analyze` | 0 | 0 | - | — | missing | No provider. Multi-leg correlation is composition done on the fluxplane side. |
| `homer.pcap.export` | 0 | 0 | - | — | missing | No provider. The export is a binary PCAP. |

## confluence

Source: fluxplane inventory §2 confluence.

Connectors catalog provider: `adapters/catalog/providers/confluence/operations.json`. The pinned
source is Confluence Cloud REST v2 only (`adapters/atlassian/upstream/confluence/README.md`), and
v2 has no CQL search and no user search.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `confluence.page.search` | 8 | 2 | 2026-09-21 | — | missing | No CQL search. It needs the REST v1 source. |
| `confluence.page.show` | 4 | 3 | 2026-09-24 | `page.get` (:11) | covered | The body comes in storage format, not Markdown. |
| `confluence.test` | 1 | 1 | 2026-09-21 | `connections revalidate` (identity probe, `docs/catalog-confluence.md:111`) | covered | — |
| `confluence.attachment.delete` | 0 | 0 | - | — | missing | `deleteAttachment` (v2) is not selected. |
| `confluence.attachment.get` | 0 | 0 | - | — | missing | A binary download. The catalog reads only JSON or text responses. |
| `confluence.index.build` | 0 | 0 | - | — | missing | A fluxplane-local index. |
| `confluence.page.attachment.add` | 0 | 0 | - | — | missing | Upload is a v1 multipart call and is not in the pinned v2 source. |
| `confluence.page.attachment.list` | 0 | 0 | - | — | missing | `getPageAttachments` (v2) is not selected. |
| `confluence.page.comment.add` | 0 | 0 | - | — | missing | `createFooterComment` (v2) is not selected. |
| `confluence.page.comment.list` | 0 | 0 | - | `page.comments` (:13) | covered | Footer comments only. |
| `confluence.page.create` | 0 | 0 | - | — | missing | `createPage` (v2) is not selected. |
| `confluence.page.delete` | 0 | 0 | - | — | missing | `deletePage` (v2) is not selected. |
| `confluence.page.list` | 0 | 0 | - | `space.pages` (:8), `pages.changed` (:5) | covered | Filters by space; `title` is a parameter of `getPages`. |
| `confluence.page.update` | 0 | 0 | - | — | missing | `updatePage` (v2) is not selected. |
| `confluence.user.search` | 0 | 0 | - | — | missing | There is no user search in REST v2. |

## prometheus

Source: fluxplane inventory §2 prometheus.

Connectors has no Prometheus runtime. `adapters/prometheus` holds `design.md` and the
`promql-range` contract. The baseline notes that instant queries are not specified
(`docs/recent-adapter-usage-20260909.md`, U08).

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `prometheus.query` | 1 | 1 | 2026-09-16 | — | missing | No runtime. Instant queries are not specified. |
| `prometheus.test` | 1 | 1 | 2026-09-24 | — | missing | No Prometheus connection. |
| `prometheus.alerts` | 0 | 0 | - | — | missing | No runtime. |
| `prometheus.labels` | 0 | 0 | - | — | missing | No runtime. |
| `prometheus.query_range` | 0 | 0 | - | — | missing | No runtime. A range query is specified but not implemented. |
| `prometheus.rules` | 0 | 0 | - | — | missing | No runtime. |
| `prometheus.series` | 0 | 0 | - | — | missing | No runtime. |
| `prometheus.targets` | 0 | 0 | - | — | missing | No runtime. |

## alertmanager

Source: fluxplane inventory §2 alertmanager.

Connectors has no Alertmanager runtime. `adapters/alertmanager` holds only `design.md`, which
leaves the alert record schema as an open authoring obligation.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `alertmanager.alerts` | 1 | 1 | 2026-09-20 | — | missing | No runtime. |
| `alertmanager.silence.create` | 0 | 0 | - | — | missing | No runtime and no write. |
| `alertmanager.silence.delete` | 0 | 0 | - | — | missing | No runtime and no write. |
| `alertmanager.silence.list` | 0 | 0 | - | — | missing | No runtime. |
| `alertmanager.test` | 0 | 0 | - | — | missing | No Alertmanager connection. |

## asterisk

Source: fluxplane inventory §2 asterisk. Connectors has no Asterisk provider.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `asterisk.ami.ping` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.call.originate` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.channel.hangup` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.channel.list` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.command` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.devicestate.list` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.peer.list` | 0 | 0 | - | — | missing | No provider. |
| `asterisk.queue.status` | 0 | 0 | - | — | missing | No provider. |

## aws

Source: fluxplane inventory §2 aws.

Connectors has no AWS provider (`docs/recent-adapter-usage-20260909.md`, U14). The generic
header credentials do not implement AWS request signing.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `aws.cloudwatch.metrics` | 0 | 0 | - | — | missing | No provider. |
| `aws.ec2.instances` | 0 | 0 | - | — | missing | No provider. |
| `aws.eks.clusters` | 0 | 0 | - | — | missing | No provider. |
| `aws.inspect` | 0 | 0 | - | — | missing | No provider. |
| `aws.logs.groups` | 0 | 0 | - | — | missing | No provider. |
| `aws.logs.query` | 0 | 0 | - | — | missing | No provider. |
| `aws.logs.tail` | 0 | 0 | - | — | missing | No provider. |
| `aws.rds.instances` | 0 | 0 | - | — | missing | No provider. |
| `aws.s3.buckets` | 0 | 0 | - | — | missing | No provider. |
| `aws.s3.objects` | 0 | 0 | - | — | missing | No provider. |
| `aws.test` | 0 | 0 | - | — | missing | No provider. |

## clock

Source: fluxplane inventory §2 clock.

`clock` declares no operations. It is a context provider for the current time only, so there is
nothing to map and no row. The nearest Connectors command is `approvals clock-check`
(`apps/connectors/spec/cli.yaml:207`), and that only checks the approval time source.

## docker

Source: fluxplane inventory §2 docker.

Connectors has no Docker runtime. `adapters/docker` holds only `design.md` and an ESS model.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `docker.build_cache.prune` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.copy_from` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.copy_to` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.create` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.exec` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.inspect.raw` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.list` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.logs` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.prune` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.remove` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.restart` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.run` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.show` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.start` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.stats` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.stop` | 0 | 0 | - | — | missing | No runtime. |
| `docker.container.top` | 0 | 0 | - | — | missing | No runtime. |
| `docker.context.list` | 0 | 0 | - | — | missing | No runtime. |
| `docker.context.show` | 0 | 0 | - | — | missing | No runtime. |
| `docker.events` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.build` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.inspect.raw` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.list` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.prune` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.pull` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.push` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.remove` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.show` | 0 | 0 | - | — | missing | No runtime. |
| `docker.image.tag` | 0 | 0 | - | — | missing | No runtime. |
| `docker.info` | 0 | 0 | - | — | missing | No runtime. |
| `docker.network.create` | 0 | 0 | - | — | missing | No runtime. |
| `docker.network.inspect.raw` | 0 | 0 | - | — | missing | No runtime. |
| `docker.network.list` | 0 | 0 | - | — | missing | No runtime. |
| `docker.network.prune` | 0 | 0 | - | — | missing | No runtime. |
| `docker.network.remove` | 0 | 0 | - | — | missing | No runtime. |
| `docker.network.show` | 0 | 0 | - | — | missing | No runtime. |
| `docker.system.df` | 0 | 0 | - | — | missing | No runtime. |
| `docker.system.prune` | 0 | 0 | - | — | missing | No runtime. |
| `docker.volume.create` | 0 | 0 | - | — | missing | No runtime. |
| `docker.volume.inspect.raw` | 0 | 0 | - | — | missing | No runtime. |
| `docker.volume.list` | 0 | 0 | - | — | missing | No runtime. |
| `docker.volume.prune` | 0 | 0 | - | — | missing | No runtime. |
| `docker.volume.remove` | 0 | 0 | - | — | missing | No runtime. |
| `docker.volume.show` | 0 | 0 | - | — | missing | No runtime. |

## duckduckgo

Source: fluxplane inventory §2 duckduckgo.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `duckduckgo.search` | 0 | 0 | - | — | missing | No DuckDuckGo provider. Web search exists only through Tavily. |

## git

Source: fluxplane inventory §2 git.

Connectors has no local git adapter. The only repository surface is the GitLab-hosted reads.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `git.add` | 0 | 0 | - | — | missing | No local git adapter. |
| `git.commit` | 0 | 0 | - | — | missing | No local git adapter. |
| `git.diff` | 0 | 0 | - | — | missing | No local git adapter. |
| `git.push` | 0 | 0 | - | — | missing | No local git adapter. |
| `git.status` | 0 | 0 | - | — | missing | No local git adapter. |
| `git.tag` | 0 | 0 | - | — | missing | No local git adapter. |

## ollama

Source: fluxplane inventory §2 ollama. Connectors has no Ollama provider.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `ollama.chat` | 0 | 0 | - | — | missing | No provider. |
| `ollama.embed` | 0 | 0 | - | — | missing | No provider. |
| `ollama.generate` | 0 | 0 | - | — | missing | No provider. |
| `ollama.info` | 0 | 0 | - | — | missing | No provider. |
| `ollama.model.list` | 0 | 0 | - | — | missing | No provider. |
| `ollama.model.show` | 0 | 0 | - | — | missing | No provider. |
| `ollama.ps` | 0 | 0 | - | — | missing | No provider. |

## openai

Source: fluxplane inventory §2 openai. Connectors has no OpenAI provider.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `openai.image.generate` | 0 | 0 | - | — | missing | No provider. |
| `openai.model.list` | 0 | 0 | - | — | missing | No provider. |
| `openai.vision.analyze` | 0 | 0 | - | — | missing | No provider. |

## opsgenie

Source: fluxplane inventory §2 opsgenie. Connectors has no Opsgenie provider.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `opsgenie.alert.ack` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.alert.close` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.alert.get` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.alert.list` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.alert.note` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.oncall` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.schedule.list` | 0 | 0 | - | — | missing | No provider. |
| `opsgenie.test` | 0 | 0 | - | — | missing | No provider. |

## sleep

Source: fluxplane inventory §2 sleep.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `sleep` | 0 | 0 | - | — | missing | No local wait operation. |

## system

Source: fluxplane inventory §2 system.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `system.info` | 0 | 0 | - | — | missing | No local system operation. |

## tavily

Source: fluxplane inventory §2 tavily.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `tavily.search` | 0 | 0 | - | `websearch.search` (`adapters/tavily/spec/adapter.json:40`) | covered | Spends Tavily credits. |

## vision

Source: fluxplane inventory §2 vision. Connectors has no vision provider.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `vision.analyze` | 0 | 0 | - | — | missing | No provider. |
| `vision.provider.list` | 0 | 0 | - | — | missing | No provider. |

## websearch

Source: fluxplane inventory §2 websearch.

| fluxplane operation | calls | sessions | last used | Connectors operation | verdict | gap |
|---|---:|---:|---|---|---|---|
| `websearch.provider.list` | 0 | 0 | - | `operations list --family` (`apps/connectors/spec/cli.yaml:410`) | partial | Lists the web-search operations of the configured adapters. Tavily is the only one. |
| `websearch.search` | 0 | 0 | - | `websearch.search` (`adapters/tavily/spec/adapter.json:40`) | partial | One provider (Tavily). No aggregation over providers. |

## CLI verbs

Source: fluxplane inventory §3. Connectors commands are cited from `apps/connectors/spec/cli.yaml`.

| fluxplane verb | calls | sessions | Connectors equivalent | verdict | gap |
|---|---:|---:|---|---|---|
| `operation invoke` | 6,533 | 95 | `operations invoke` (:432) | covered | A write needs `--approval-file` under an approval policy. |
| `operation list` | 646 | 83 | `operations list` (:410) | covered | — |
| `operation describe` | 542 | 69 | `operations describe` (:423) | covered | — |
| `operation search` | 121 | 34 | `operations list --family` (:410) | partial | Filters by family only. No text search over operation names or descriptions. |
| `operation batch` | 0 | 0 | — | missing | No batch invoke. |
| `operation` (no subcommand, mostly `--help`) | 23 | 12 | `operations --help` | covered | — |
| `operation show` (not a fluxplane verb) | 4 | 3 | `operations describe` (:423) | covered | It meant `describe`. |
| `auth status` | 19 | 13 | `connections status` (:364) | covered | — |
| `auth methods` | 1 | 1 | `adapters describe` (:293) | partial | The profile is fixed per instance in the local config. Methods are not listed. |
| `auth connect` | 2 | 1 | `connections connect` (:340) | covered | The credential is taken by file, stdin or a hidden prompt. |
| `auth test` | 1 | 1 | `connections revalidate` (:375) | covered | — |
| `auth disconnect` | 0 | 0 | `connections revoke` (:399) | covered | — |
| `auth` (no subcommand) | 2 | 2 | `connections --help` | covered | — |
| `datasource list` | 4 | 3 | `operations list --family` (:410) | partial | Lists operations by contract family, not datasource entities. |
| `datasource records` / `search` / `search-all` / `get` / `batch-get` / `lookup` / `lookup-all` | 0 | 0 | `operations invoke` on a `datasource.records/v1alpha1` operation | partial | No cross-provider search (`search-all`) and no batch get. |
| `datasource` (no subcommand) | 1 | 1 | — | covered | Help only. |
| `lookup` | 9 | 3 | — | missing | A reverse lookup over fluxplane-local indexes. Connectors keeps no index. |
| `endpoint list` | 149 | 32 | `adapters list` (:284), `connections list` (:320) | covered | — |
| `endpoint get` | 1 | 1 | `adapters describe` (:293), `connections describe` (:331) | covered | — |
| `endpoint save` | 6 | 3 | the per-adapter TOML startup config, then `connections connect` | partial | No CLI verb writes an instance. |
| `endpoint import` | 6 | 2 | — | missing | No endpoint import. |
| `endpoint remove` | 4 | 2 | `connections revoke` (:399) | partial | Revokes the credential; the instance stays in the config. |
| `endpoint discover` | 4 | 2 | Kubernetes `endpoints.discover` through `operations invoke` | partial | Observes EndpointSlices but does not save endpoints. |
| `endpoint health` / `test` / `doctor` | 0 | 0 | `connections status` (:364), `connections revalidate` (:375), `setup check` (:279) | covered | — |
| `endpoint` (no subcommand) | 5 | 5 | — | covered | Help only. |
| `evidence list` / `observe` | 0 | 0 | — | missing | No evidence verb. |
| `incident timeline` | 0 | 0 | — | missing | No incident verb. |
| `monitor connect` / (no subcommand) | 1 | 1 | — | missing | No monitor verb. |
| `index build` / `status` | 0 | 0 | — | missing | No index. |
| `context list` / `build` / `build-all` | 0 | 0 | — | missing | No context providers. |
| `blob put` (and the bare group) | 11 | 4 | — | missing | No blob store, and binary uploads are not supported. |
| `process list` | 1 | 1 | `adapters status` (:300) | covered | `process stop` maps to `adapters stop` (:307). `process logs` (0 calls) has no equivalent. |
| `list` (plugins) | 61 | 18 | `adapters list` (:284) | covered | — |
| `status` | 1 | 1 | `adapters status` (:300) | covered | — |
| `manifest` | 6 | 2 | `adapters describe` (:293) | covered | — |
| `doctor` | 3 | 2 | `setup check` (:279) | covered | — |
| `describe` (plugin) | 3 | 2 | `adapters describe` (:293) | covered | — |
| no verb (`--help`, `--version`, bare) | 100 | 37 | `connectors --help`, `connectors --version` | covered | — |

The following verbs had 0 calls and get no row: the plugin lifecycle verbs `install`, `update`,
`enable`, `disable`, `remove`, `search`, `run`, `upgrade`, `selftest`, `pin`, `unpin`,
`rollback` and `version`, plus `skill install`, `skill refresh` and `dev sync`.

Connectors has no plugin lifecycle. Adapters are declared in local config, and a configuration
upgrade goes through `connections revalidate`.

## Used but not declared

Source: fluxplane inventory §4. Each name is mapped to the declared operation it most likely
meant. When no operation is declared for that capability, the row says so and gives the nearest
one. The unit column (see [Gap units](#gap-units)) shows where a gap call is counted.

| name as written (plugin as written) | calls | sessions | most likely meant | Connectors verdict | unit |
|---|---:|---:|---|---|---|
| `jira.issue.get` (jira) | 27 | 13 | `jira.issue.show` | partial | U03 |
| `gitlab.commit.show` (gitlab) | 6 | 4 | none declared (a single-commit read); nearest is `gitlab.repository.commit.list` | missing (`/repository/commits/:sha` is not selected) | U10 |
| `gitlab.mr.get` (gitlab) | 6 | 2 | `gitlab.mr.show` | covered | — |
| `gitlab.repository.branch.list` (gitlab) | 4 | 3 | none declared (a branch list) | missing (only `branch.get` is served) | U10 |
| `jira.issue.transitions` (jira) | 4 | 4 | `jira.issue.transition.list` | missing | U07 |
| `gitlab.commit.diff` (gitlab) | 3 | 2 | none declared (a single-commit diff); nearest is `gitlab.compare` | partial (`repository.compare` from the parent commit) | U10 |
| `gitlab.mr.note.list` (gitlab) | 3 | 2 | `gitlab.mr.discussion.list` | missing | U11 |
| `gitlab.pipeline.show` (gitlab) | 3 | 3 | none declared (a single-pipeline read) | covered by `pipeline.get` (`adapters/catalog/providers/gitlab/operations.json:22`) | — |
| `jira.search` (jira) | 3 | 1 | `jira.issue.search` | covered | — |
| `slack.thread.replies` (slack) | 3 | 3 | `slack.thread` | missing | U02 |
| `slack.user.info` (slack) | 3 | 2 | `slack.user.list` | missing | U02 |
| `jira.issue.show` (atlassian) | 2 | 1 | `jira.issue.show` | partial | U03 |
| `gitlab.commit.list` (gitlab) | 2 | 1 | `gitlab.repository.commit.list` | covered | — |
| `gitlab.pipeline.job.list` (gitlab) | 2 | 2 | `gitlab.job.list` | covered | — |
| `gitlab.pipeline.jobs` (gitlab) | 2 | 1 | `gitlab.job.list` | covered | — |
| `gitlab.repository.compare` (gitlab) | 2 | 1 | `gitlab.compare` | covered | — |
| `jira.issue.transition` (jira) | 2 | 2 | `jira.issue.transition.run` | missing | U07 |
| `slack.auth.test` (slack) | 2 | 2 | `slack.test` | missing | U02 |
| `alertmanager.alerts.list` (alertmanager) | 1 | 1 | `alertmanager.alerts` | missing | U24 |
| `sql` (fluxplane-plugin) | 1 | 1 | `sql.query` (inferred) | partial | U01 |
| `gitlab.file.show` (gitlab) | 1 | 1 | `gitlab.repository.file.show` | covered | — |
| `gitlab.merge_request.get` (gitlab) | 1 | 1 | `gitlab.mr.show` | covered | — |
| `gitlab.mr.diff` (gitlab) | 1 | 1 | `gitlab.mr.changes` | partial | U11 |
| `gitlab.mr.discussions` (gitlab) | 1 | 1 | `gitlab.mr.discussion.list` | missing | U11 |
| `gitlab.mr.pipelines` (gitlab) | 1 | 1 | none declared (a merge request's pipelines); nearest is `gitlab.pipeline.list` | covered (`merge_request.get` carries `head_pipeline`; `pipelines.list` filters by `ref` and `sha`) | — |
| `gitlab.repository.branch.show` (gitlab) | 1 | 1 | none declared (a single-branch read) | covered by `branch.get` (`adapters/catalog/providers/gitlab/operations.json:12`) | — |
| `gitlab.repository.commit.diff` (gitlab) | 1 | 1 | none declared (a single-commit diff) | partial (`repository.compare` from the parent commit) | U10 |
| `job.list` (gitlab) | 1 | 1 | `gitlab.job.list` | covered | — |
| `mr.create` (gitlab) | 1 | 1 | `gitlab.mr.create` | covered | — |
| `project.show` (gitlab) | 1 | 1 | `gitlab.project.show` | covered | — |
| `grafana.loki.metric` (grafana) | 1 | 1 | `grafana.loki.query` (a metric LogQL query) | missing | U05 |
| `call.list` (homer) | 1 | 1 | `homer.call.list` | missing | U17 |
| `get_issue` (jira) | 1 | 1 | `jira.issue.show` | partial | U03 |
| `issue.get` (jira) | 1 | 1 | `jira.issue.show` | partial | U03 |
| `jira.issue.comment.create` (jira) | 1 | 1 | `jira.issue.comment.add` | missing | U06 |
| `jira.issue.view` (jira) | 1 | 1 | `jira.issue.show` | partial | U03 |
| `k8s.pod.list` (kubernetes) | 1 | 1 | `kubernetes.pod.list` | covered | — |
| `loki.query_range` (loki) | 1 | 1 | `loki.query` | missing | U04 |
| `slack.conversation.list` (slack) | 1 | 1 | `slack.channel.list` | missing | U02 |
| `slack.conversations.history` (slack) | 1 | 1 | `slack.message.list` | missing | U02 |
| `slack.user.lookup` (slack) | 1 | 1 | `slack.user.list` | missing | U02 |
| `slack.user.show` (slack) | 1 | 1 | `slack.user.list` | missing | U02 |
| `sql.endpoints` (sql) | 1 | 1 | the `endpoint list` verb | covered (`adapters list`, `connections list`) | — |
| `slack.message.list` (operation id in the plugin position, §4b) | 6 | 2 | `slack.message.list` | missing | U02 |
| `slack.thread` (§4b) | 4 | 3 | `slack.thread` | missing | U02 |
| `gitlab.repository.commit.list` (§4b) | 2 | 1 | `gitlab.repository.commit.list` | covered | — |
| `jira.issue.show` (§4b) | 2 | 2 | `jira.issue.show` | partial | U03 |
| `jira.issue.get` (§4b) | 1 | 1 | `jira.issue.show` | partial | U03 |
| `confluence.test` (§4b) | 1 | 1 | `confluence.test` | covered | — |
| `gitlab.mr.show` (§4b) | 1 | 1 | `gitlab.mr.show` | covered | — |
| `gitlab.search.blobs` (§4b) | 1 | 1 | `gitlab.search.blobs` | missing | U10 |
| `jira.issue.get` (in the verb position, §4d) | 2 | 1 | `jira.issue.show` | partial | U03 |

In total, 124 calls are mapped: 33 covered and 91 to gap units. The rest of §4 is not
attributed:

- **§4c:** 55 calls whose operation sat in a variable with no literal id in the same command.
- **§4d:** the other verbs that are not in the CLI.
  - `plugin` (12), `call` (5), `connection` (4), `ops` (1) and `gitlab` (1).
  - `mysql` (2): the intent is unknown, but it suggests MySQL use (see [sql](#sql)).
  - Prose words: `not`, `parity` and one quoted variable.

## Gap units

Each unit is one provider capability family that a single story can deliver. A unit's calls are
the declared calls of its missing and partial operations, plus the mapped calls from
[Used but not declared](#used-but-not-declared). Units are ordered by total calls. Ties are
broken by sessions.

The "provider in Connectors" column uses three values:

- **catalog** — a catalog provider exists.
- **native** — a native adapter exists.
- **new** — no runtime exists; any design or ESS model is named.

| unit | name | provider | provider in Connectors | likely surface | operations | declared calls | undeclared calls | total |
|---|---|---|---|---|---|---:|---:|---:|
| U01 | MySQL query and schema reads | sql | native (PostgreSQL only) | Extend the native adapter with a MySQL binding in `adapters/sql`: wire protocol, dialect, typed values. The baseline U06 already calls it required. | `sql.query`, `sql.table.show`, `sql.table.list`, `sql.test`, `sql.database.list`, `sql.index.list` | 703 | 1 | 704 |
| U02 | Slack provider and conversation reads | slack | new | A new catalog provider with an OpenAPI source, if a current Slack Web API document can be pinned (not checked). Otherwise a new native adapter. Needs bot and user token profiles, because search needs a user token. | `slack.thread`, `slack.message.list`, `slack.search`, `slack.user.list`, `slack.channel.list`, `slack.info`, `slack.test`, `slack.emoji.list` | 490 | 22 | 512 |
| U03 | Jira issue reads | jira | catalog | Catalog `operations.json` selection only: `getIssue`, `getCreateIssueMetaIssueTypes`, `findUsers`. | `jira.issue.show`, `jira.issue.create_meta`, `jira.user.search` | 361 | 37 | 398 |
| U04 | Loki LogQL query, metric and labels | loki | new (`adapters/loki`: design, ESS model and `logql-range` contract) | A new native adapter. Metric queries and label discovery need contract additions. | `loki.query`, `loki.metric`, `loki.labels`, `loki.test` | 370 | 1 | 371 |
| U05 | Grafana datasources and Loki through Grafana | grafana | new (`adapters/grafana`: design and ESS model) | A new native adapter: datasource discovery and the `grafana-datasource-proxy` mediated route. Depends on U04 for LogQL. | `grafana.loki.query`, `grafana.datasource.list`, `grafana.loki.labels`, `grafana.loki.recent_logs` | 358 | 1 | 359 |
| U06 | Jira issue writes | jira | catalog | Catalog `operations.json` selection: `createIssue`, `editIssue`, `addComment`, `linkIssues`, `deleteIssue`, with guards to decide. Markdown-to-ADF conversion would be a catalog engine change. | `jira.issue.comment.add`, `jira.issue.create`, `jira.issue.link.add`, `jira.issue.edit`, `jira.issue.delete` | 325 | 1 | 326 |
| U07 | Jira transitions | jira | catalog | Catalog `operations.json` selection: `getTransitions`, `doTransition`. A by-name run or a walk to a target status is composition outside the catalog. | `jira.issue.transition.list`, `jira.issue.transition.run` | 236 | 6 | 242 |
| U08 | Slack message writes | slack | new (after U02) | Same provider surface as U02. | `slack.message.send`, `slack.message.edit`, `slack.message.delete` | 162 | 0 | 162 |
| U09 | GitLab merge-request writes | gitlab | catalog | Catalog `operations.json` selection only: notes, discussion reply and resolve, plus guarded variants of merge (merge-when-pipeline-succeeds) and update (reopen). | `gitlab.mr.merge`, `gitlab.mr.update`, `gitlab.mr.note.create`, `gitlab.mr.discussion.reply`, `gitlab.mr.discussion.resolve` | 159 | 0 | 159 |
| U10 | GitLab repository reads | gitlab | catalog | Catalog `operations.json` selection only: code search, tree, single commit, commit diff, branch list. | `gitlab.search.blobs`, `gitlab.repository.tree` | 110 | 15 | 125 |
| U11 | GitLab merge-request reads | gitlab | catalog | Catalog `operations.json` selection only: merge-request diffs, discussions, notes. | `gitlab.mr.changes`, `gitlab.mr.discussion.list` | 101 | 5 | 106 |
| U12 | Prometheus PromQL, direct and through Grafana | prometheus | new (`adapters/prometheus`: design and `promql-range` contract) | A new native adapter. Instant queries and rules need contract additions. The mediated placement depends on U05. | `grafana.prometheus.query`, `grafana.prometheus.range`, `grafana.prometheus.rules`, `prometheus.query`, `prometheus.test` | 69 | 0 | 69 |
| U13 | Slack files | slack | new (after U02) | A catalog engine change: binary download and a multi-step external upload. | `slack.file.upload`, `slack.file.download`, `slack.file.delete`, `slack.file.info`, `slack.file.list` | 52 | 0 | 52 |
| U14 | GitLab tags and releases | gitlab | catalog | Catalog `operations.json` selection only. | `gitlab.release.create`, `gitlab.repository.tag.create`, `gitlab.repository.tag.show`, `gitlab.release.show`, `gitlab.release.link.list`, `gitlab.release.update`, `gitlab.repository.tag.delete` | 45 | 0 | 45 |
| U15 | GitLab CI/CD writes and environments | gitlab | catalog | Catalog `operations.json` selection only. | `gitlab.pipeline.retry`, `gitlab.pipeline.cancel`, `gitlab.pipeline.create`, `gitlab.environment.list` | 36 | 0 | 36 |
| U16 | GitLab project and repository writes | gitlab | catalog | Catalog `operations.json` selection only. | `gitlab.repository.commit.create`, `gitlab.project.create`, `gitlab.repository.file.update`, `gitlab.branch.create`, `gitlab.branch.delete` | 36 | 0 | 36 |
| U17 | Homer SIP capture reads | homer | new | A new catalog provider, if the Homer API's Swagger document can be pinned (not checked). Otherwise a new native adapter. | `homer.call.list`, `homer.call.show`, `homer.test`, `homer.call.qos`, `homer.search` | 32 | 1 | 33 |
| U18 | Kubernetes reads: namespaces, single objects, events, rollout history | kubernetes | native | Change the native adapter: new kinds (namespaces, events, ReplicaSets) and a single-object read. | `kubernetes.namespace.list`, `kubernetes.pod.show`, `kubernetes.deployment.show`, `kubernetes.deployment.history`, `kubernetes.event.list` | 17 | 0 | 17 |
| U19 | Kubernetes exec and port-forward | kubernetes | native | Change the native adapter: the streaming subresources. The execution profile is deferred (README.md:115-116). | `kubernetes.pod.exec`, `kubernetes.portforward.start`, `kubernetes.portforward.stop` | 13 | 0 | 13 |
| U20 | Confluence CQL search | confluence | catalog | A new pinned OpenAPI source (Confluence REST v1) for the existing catalog provider. | `confluence.page.search` | 8 | 0 | 8 |
| U21 | Kubernetes contexts | kubernetes | native | Change the native adapter and the CLI connection setup: discover kubeconfig contexts. | `kubernetes.cluster.list` | 5 | 0 | 5 |
| U22 | Kubernetes pod logs | kubernetes | native | Change the native adapter: implement the specified `adapters/kubernetes/contracts/logs`. | `kubernetes.pod.logs` | 5 | 0 | 5 |
| U23 | Jira attachment download | jira | catalog | A catalog engine change: binary responses. | `jira.issue.attachment.get` | 2 | 0 | 2 |
| U24 | Alertmanager alerts | alertmanager | new (`adapters/alertmanager`: design only) | A new native adapter. The alert record schema is still unauthored. | `alertmanager.alerts` | 1 | 1 | 2 |
| U25 | Kubernetes Secret read | kubernetes | native | Change the native adapter. It needs a disclosure decision first. | `kubernetes.secret.read` | 1 | 0 | 1 |
| | **total** | | | | 89 operations | **3,697** | **91** | **3,788** |

The units cover exactly the 89 declared operations that have calls and are partial (16) or
missing (73). Their 3,697 declared calls equal the partial (1,273) and missing (2,424) totals in
the [Summary](#summary).

## Waves

Each wave holds at most four units, and the units in one wave touch different providers. A unit
waits for any unit it depends on: U05 after U04, U12 after U05, U08 and U13 after U02.

- **7 waves is the minimum.** 25 units at four per wave need at least 7, and GitLab alone has
  6 units.
- **One unit moved forward.** U18 sits ahead of U17 so that the five Kubernetes units fit into
  those 7 waves.

| wave | units | providers | calls |
|---|---|---|---:|
| W1 | U01 MySQL query and schema reads · U02 Slack provider and conversation reads · U03 Jira issue reads · U04 Loki LogQL | sql, slack, jira, loki | 1,985 |
| W2 | U05 Grafana datasources and Loki through Grafana · U06 Jira issue writes · U08 Slack message writes · U09 GitLab merge-request writes | grafana, jira, slack, gitlab | 1,006 |
| W3 | U07 Jira transitions · U10 GitLab repository reads · U12 Prometheus PromQL · U18 Kubernetes reads | jira, gitlab, prometheus, kubernetes | 453 |
| W4 | U11 GitLab merge-request reads · U13 Slack files · U17 Homer SIP capture reads · U19 Kubernetes exec and port-forward | gitlab, slack, homer, kubernetes | 204 |
| W5 | U14 GitLab tags and releases · U20 Confluence CQL search · U21 Kubernetes contexts · U23 Jira attachment download | gitlab, confluence, kubernetes, jira | 60 |
| W6 | U15 GitLab CI/CD writes and environments · U22 Kubernetes pod logs · U24 Alertmanager alerts | gitlab, kubernetes, alertmanager | 43 |
| W7 | U16 GitLab project and repository writes · U25 Kubernetes Secret read | gitlab, kubernetes | 37 |
| | **total** | | **3,788** |

## Not planned until used

183 operations with 0 calls are partial or missing: 176 missing and 7 partial. They belong to
no unit until a session uses them. The other 9 operations with 0 calls are already covered.

| plugin | count | operations |
|---|---:|---|
| gitlab | 24 | `branch.delete_merged`, `ci.variable.create`, `ci.variable.delete`, `ci.variable.update`, `index.build`, `issue.list` (partial), `issue.note.create`, `issue.note.list`, `issue.show` (partial), `issue.update`, `mr.approve`, `mr.diff.lines`, `mr.discussion.create`, `release.delete`, `release.link.create`, `release.link.delete`, `release.link.update`, `repository.archive`, `repository.changelog.add`, `repository.changelog.generate`, `repository.file.create`, `repository.file.delete`, `snippet.create`, `snippet.delete` |
| jira | 7 | `index.build`, `issue.attachment.add`, `issue.attachment.delete`, `issue.attachment.list`, `issue.comment.delete`, `issue.comment.edit`, `issue.edit_meta` |
| slack | 14 | `bookmark.add`, `bookmark.delete`, `bookmark.edit`, `bookmark.list`, `channel.join`, `channel.mark-read`, `download`, `index.build`, `mentions`, `presence.get`, `presence.set`, `reaction.add`, `reaction.remove`, `unreads` |
| grafana | 13 | `alerts.active`, `alerts.silences.create`, `alerts.silences.delete`, `alerts.silences.list`, `annotation.add`, `annotation.list`, `dashboard.get`, `dashboard.list`, `datasource.health`, `folder.list`, `tempo.search`, `tempo.trace.get`, `test` |
| loki | 1 | `recent_logs` |
| kubernetes | 7 | `container.show` (partial), `deployment.restart`, `deployment.scale`, `ingress.list`, `node.list` (partial), `portforward.list`, `service.show` (partial) |
| homer | 3 | `alias.list`, `call.analyze`, `pcap.export` |
| confluence | 10 | `attachment.delete`, `attachment.get`, `index.build`, `page.attachment.add`, `page.attachment.list`, `page.comment.add`, `page.create`, `page.delete`, `page.update`, `user.search` |
| prometheus | 6 | `alerts`, `labels`, `query_range`, `rules`, `series`, `targets` |
| alertmanager | 4 | `silence.create`, `silence.delete`, `silence.list`, `test` |
| asterisk | 8 | all |
| aws | 11 | all |
| docker | 44 | all |
| duckduckgo | 1 | all |
| git | 6 | all |
| ollama | 7 | all |
| openai | 3 | all |
| opsgenie | 8 | all |
| sleep | 1 | all |
| system | 1 | all |
| vision | 2 | all |
| websearch | 2 | all (partial) |
