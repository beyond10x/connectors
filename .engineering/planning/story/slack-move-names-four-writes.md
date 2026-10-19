---
format: aep.planning-md/3
id: story:slack-move-names-four-writes
kind: story
status: draft
title: The Slack move guide enables the five writes the shipped selection carries
relations:
- serves: vision:independent-contract-adapters
revision: 3
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

## Update after 0.54.0

The shipped bot selection now carries five writes: the four above and `files.delete`
(`docs/catalog-slack.md` lines 13 and 73-75; the `files.delete` row at line 219). *Limits*
(line 923) already says five writes. The move section (line 593, "enables the two writes") still
enables two, so what remains of this story is that section: read "five writes" for "four" in the
acceptance above, with the move enabling all five and its output pasted from a fresh run.
