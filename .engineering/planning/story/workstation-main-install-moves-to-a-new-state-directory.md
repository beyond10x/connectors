---
format: aep.planning-md/3
id: story:workstation-main-install-moves-to-a-new-state-directory
kind: story
status: draft
title: The workstation's main install moves to a current release on a new state directory
relations:
- depends_on: story:settled-read-uses-leave-the-store
- decomposes: epic:zero-open-issues-and-store-defects
revision: 2
---
## Outcome

The workstation's main install of connectors (0.65.2, `~/.config/connectors/config.toml`,
state `~/.local/state/connectors`) answers in seconds again: it runs a current release on a new
state directory, with the old directory kept.

## Decision

The operator, 2026-11-23 about 14:25 CET (decisions delegated by Timo): option C, a new state
directory on a current release, and only

1. after the release that holds story:settled-read-uses-leave-the-store (reads then append
   nothing to the store), and
2. once it is shown that a current release reads stored credentials on this host: 0.73.0,
   tried and rolled back on 2026-11-21, answered `custody_unavailable` for every stored
   credential (it looked for the Secret Service collection file under its D-Bus-escaped name
   `Standard_2dSchl_c3_bcsselbund.keyring`; the file is `Standard-Schlüsselbund.keyring`).

Nothing before that. Then the plan goes to the operator: the new state directory, which
connections need reconnecting and whether any needs Timo's own login, the old directory kept.
Until then the install stays 0.65.2 (decision H5 = A).

## Facts

- Reported 2026-11-23 14:03-14:12 CET by a consumer session: `metadata.sqlite3` is 693 MB,
  every CLI call takes 60 to 140 s; `instances status` and reads answer `timeout` at
  `admission`; `connections revalidate` on `slack/slack-bot` answered `outcome_unknown` twice;
  that instance holds two `pending` connections. Not read by this repository's session.
- A second consumer, about 13:55 CET: the owner and its providers keep the caller's working
  directory (story:owner-and-providers-run-in-root-directory).
- Cause of the size: issue #267, about 78 KB appended per read up to 0.75.0, and every call
  reads the whole store. No release compacts a store.
- 0.74.0 changed the collection-file lookup ("Linux Secret Service custody decodes GNOME's
  escaped collection names", verification-report:connectors-v0740-release-scope).
- Read 2026-11-23 14:27 CET: the side install `~/.config/connectors-jira-read` runs the 0.75.0
  binary with no custody section in its config; its `setup check` answers `keyring` and
  `persistent_custody_qualification` `ready`, and its reads answered that morning (14 s after
  the move to 0.75.0). So 0.75.0 reads a stored credential through the Secret Service on this
  host. Not shown: the same for the main install's own entries, and for 0.76.0 or later.
- Instances of the main install, read 2026-11-21 by another session: `jira/<instance>` (the Jira instance),
  `gitlab/forge`, `gmail/gmail-write`, `slack/slack-bot`, `slack/slack-search`,
  `tavily/tavily`, `zendesk/zendesk`.
- 0.76.0 and later write a store that 0.75.0 and earlier do not open.

## Acceptance

Not written. First steps when condition 1 holds: a read on a side install with the released
binary (condition 2), then the plan for the operator.
