---
format: aep.planning-md/3
id: story:served-merge-refusal-live-record
kind: story
status: draft
title: GitHub's real answer to a refused merge is recorded and served as refused
summary: 'D313 follow-up of story:served-merge-refusal-answers-refused: record the live 4xx of a behind-branch merge under a strict ruleset'
tags:
- d298
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

GitHub's actual status and body for a `pull_request.merge` of a branch behind `main` under a
ruleset with `strict_required_status_checks_policy: true` are recorded from a live run or a
reproduction against a scratch repository, and the served instance's answer to it is observed to
be `refused` with that message.

## Context

D282 and D313. `story:served-merge-refusal-answers-refused` maps a definite 4xx refusal of a write
to `refused`, tested against a stub with GitHub's documented 405 and 422 bodies, because no
recording of GitHub's real answer existed when it was implemented (wave D298).

## Acceptance

- A recorded request and answer (status, headers that matter, body) of the refused merge, with the
  repository, ruleset and date, kept as a fixture with its source.
- The served instance answers that recorded refusal with `refused` and the provider's message.
- If the recorded status or body differs from the stub's, the stub is corrected to it.

## Out of Scope

- Updating the branch or retrying on the agent's behalf.
