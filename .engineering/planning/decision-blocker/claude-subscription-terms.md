---
format: aep.planning-md/3
id: decision-blocker:claude-subscription-terms
kind: decision-blocker
status: cleared
title: 'Anthropic''s terms forbid third-party storage of Claude.ai tokens: build the Claude subscription profile or not'
relations:
- blocks: story:claude-subscription-connector
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-25T21:11:18Z", actor: "human:timo", revision: 3}
---
## Question

May Connectors store and refresh a Claude subscription (claude.ai) OAuth grant and hand its access token to
local callers?

## Why it is open

Anthropic's published terms for Claude Code,
<https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use>, read 2026-10-25
(verbatim): "Anthropic does not permit third-party developers to offer Claude.ai login into their own
applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their users.
Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens —
sign-in to a Claude account must complete through Anthropic's own flow." and "Anthropic reserves the right
to take measures to enforce these restrictions and may do so without prior notice." The same page permits
"an end user from signing in to the unmodified Claude Code binary with their own Claude subscription".

The request (Timo, 2026-10-25, verbatim) names Claude Code: "for anthropic claude code AND for
codex/chatgpt".

## Options

- **A (recommended).** No Claude subscription profile. Anthropic access goes through a Console API key
  (an existing static token profile), and a person who wants their plan runs the unmodified `claude`
  binary, which signs in itself; `connections launch` can start it as a pinned consumer.
- **B.** Build `story:claude-subscription-connector` for the operator's own single-user use, accepting the
  enforcement risk the terms name; nothing ships a client id by default and the guide quotes the terms.
- **C.** Ask Anthropic (the page links "contact sales") for a permitted route, and hold the story until
  then.

Decider: operator

## Decision

Decided 2026-10-25 by the operator, Timo, relayed verbatim with its date by the directing session
`b10x-chef-orchestrator`: the request "it must be possible in connectors lib, to do a regular oauth2 thing,
for anthropic claude code AND for codex/chatgpt. this way a token and refresh token would be stored in
connectors" and, on being asked again, "We made the decission long ago already, I am sick of having to
re-decide over and over again".

**Option B.** `story:claude-subscription-connector` is built for the operator's own single-user use. The
guide quotes the terms verbatim with their URL and the date read; no client id is shipped by default
(`oauth2_client_id` is configuration). The operator accepted the enforcement risk the terms name by
asking for it.
