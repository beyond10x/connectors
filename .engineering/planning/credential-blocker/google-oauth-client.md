---
format: aep.planning-md/3
id: credential-blocker:google-oauth-client
kind: credential-blocker
status: open
title: No Google OAuth desktop client exists for the operator's account
relations:
- blocks: story:catalog-google-live-deck-read
revision: 2
---
## What is missing

A Google OAuth client of type "Desktop app" for the operator's account, and the path to its
downloaded client JSON on this machine (mode 0600, outside every repository).

## Who can clear it

The operator, in the Google Cloud console:

1. Create a project; enable the Google Slides, Drive, Calendar and Gmail APIs.
2. OAuth consent screen: user type External, publishing status Testing, the operator's account
   added as a test user.
3. Credentials → Create OAuth client ID → Desktop app; download the JSON; `chmod 600` it.
4. Tell the coordinator the file's path.

## What it withholds

Every live observation of `story:catalog-google-live-deck-read`. Fixture work, the gate and every
other story in `epic:google-workspace-reads` are not blocked.
