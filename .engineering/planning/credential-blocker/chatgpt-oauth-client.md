---
format: aep.planning-md/3
id: credential-blocker:chatgpt-oauth-client
kind: credential-blocker
status: cleared
title: No registered OAuth client id for the ChatGPT sign-in
relations:
- blocks: story:chatgpt-subscription-connector
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-25T21:11:18Z", actor: "human:timo", revision: 3}
---
## What is missing

An OAuth client id under which Connectors may run the ChatGPT sign-in and refresh a ChatGPT plan grant.

## Facts

- The Codex CLI's client id, `app_EMoamEEZ73f0CkXaXp7hrann`
  (`openai/codex@970111f6cb62cc02fe36a82ea7d71b37a2889c97` `codex-rs/login/src/auth/manager.rs#L1784`), is
  registered to Codex. I found no OpenAI text that permits or forbids another client using it.
- OpenAI's Sign in with ChatGPT, <https://developers.openai.com/siwc/quickstart>, read 2026-10-25
  (verbatim): "Sign in with ChatGPT is currently available to selected commercial partners through a limited
  trial." and "ChatGPT plan usage is available to all open-source partners and selected private clients."
  The open-source flow "registers your client and issues OAuth credentials".

## What clears it

The operator registers a client through Sign in with ChatGPT (or decides to configure the Codex CLI's id
for personal use) and names the scopes the registration grants. The client id is configuration
(`oauth2_client_id`), never a committed value.

`story:chatgpt-subscription-connector` is implemented and tested against a fake server without it; only a
live connection waits on it.

## Decision

Decided 2026-10-25 by the operator, Timo, relayed verbatim with its date by the directing session
`b10x-chef-orchestrator`: the request "it must be possible in connectors lib, to do a regular oauth2 thing,
for anthropic claude code AND for codex/chatgpt. this way a token and refresh token would be stored in
connectors" and, on being asked again, "We made the decission long ago already, I am sick of having to
re-decide over and over again".

The client id is the instance's configuration (`oauth2_client_id`), set by the operator for their own use;
none is committed. The story is implemented and tested against a fake server; registering a Sign in with
ChatGPT client stays open to the operator and blocks nothing in this repository.
