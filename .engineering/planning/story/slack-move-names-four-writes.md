---
format: aep.planning-md/3
id: story:slack-move-names-four-writes
kind: story
status: draft
title: The Slack move guide enables the four writes the shipped selection carries
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed

In release 0.51.0, `docs/catalog-slack.md` contradicts itself:
- *The shipped selection set* (line 66) lists four writes: `chat.postMessage`, `conversations.open`,
  `chat.update` and `chat.delete`.
- *Moving a local install off a credential-only adapter* (line 404 on) enables two writes. Its
  configuration diff permits only those two ids, and its revision hash (`45d968e5…`) was printed
  against the twelve-operation selection that existed before edit and delete shipped.
- *Limits* (line 702) still says message edit and delete are not selected, and that only two
  writes are.

## Acceptance

- The move section enables all four writes. Its `--print-local-bootstrap` revision and every
  output it shows come from a fresh run against the shipped `operations.json`, pasted from that
  run.
- *Limits* names the writes the shipped selection carries, and no longer lists edit and delete
  as unselected.
- The website pages that render or link the guide stay consistent; the generated pages are
  regenerated, never edited.
