---
format: aep.planning-md/3
id: story:oauth-fixed-loopback-redirect
kind: story
status: draft
title: An auth profile pins the loopback redirect's port and path
summary: For OAuth services that admit only a registered loopback redirect, such as Launchpad
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:catalog-basecamp
revision: 1
---
# An auth profile pins the loopback redirect's port and path

## Need

Launchpad, Basecamp's OAuth service, admits only a registered loopback redirect with an explicit
port; Basecamp's own CLI registers `http://127.0.0.1:8976/callback`
(github.com/basecamp/basecamp-cli, `internal/auth/auth.go:72-75`). The host always listens on a
random port, `http://127.0.0.1:<port>` (`crates/connectors-host/src/local/oauth.rs:171-176`).

## Acceptance

- An auth profile may name a fixed loopback port and path; the consent flow listens there and sends
  that redirect URI, and refuses to start when the port is taken.
- A profile that names none keeps the random port; every existing definition behaves byte-equal.

Not built while the first Basecamp version signs in with a personal access token
(story:catalog-basecamp); it is needed with story:launchpad-token-request-parameters for OAuth.
