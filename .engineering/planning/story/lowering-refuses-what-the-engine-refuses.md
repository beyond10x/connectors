---
format: aep.planning-md/3
id: story:lowering-refuses-what-the-engine-refuses
kind: story
status: draft
title: Lowering refuses the effect and method combinations the engine refuses at load
tags:
- cr-011
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-011 (S3): Lowering accepts definitions the engine refuses at load.

- Citations: `crates/connectors-catalog/src/definition.rs:3038-3050` (`(Some(effect), _) => effect`); engine rules at `adapters/catalog/src/lib.rs:1207-1217`, `:1150-1166`.
- What happens: `effect: read` on a POST lowers and then fails at configuration. No approval bypass.
- Status: lowering site verified; engine lines are the reviewer's citation.
- Fix: share the engine's effect/method and unique-id rules.

- Review's story note: new (`story:definition-reader-refuses-what-the-feed-engine-refuses`, active, covers feed rules only).

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-011. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
