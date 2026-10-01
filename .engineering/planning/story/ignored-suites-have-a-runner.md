---
format: aep.planning-md/3
id: story:ignored-suites-have-a-runner
kind: story
status: draft
title: The 35 ignored suites have a command that runs them
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: docs/development.md
revision: 2
---
## Defect

35 tests carry `#[ignore]` (`git grep '#\[ignore'` on `origin/main` `74fac9f1f`): 8 need a
qualified disposable GNOME Secret Service, 8 a built production CLI plus that Secret Service, 3
GNOME Keyring 50.0 with dbus-daemon, 2 a Kubernetes or PostgreSQL sandbox, 3 are subprocess entry
points, 1 is a timing measurement, the rest similar. No task, gate step or CI job runs them; the
last recorded `--ignored` run is in `review-result:adversary-settlement-pass-2-20260912`
(14 passed). Whether the remaining ones pass today is unknown.

## Change

One `connectors-build` subcommand runs every ignored suite whose prerequisites are present and
names each one it skipped together with the missing prerequisite. Subprocess entry points stay
ignored and are excluded by name.

## Scope

- `crates/connectors-build/src/main.rs` — inferred: subcommands are declared there.
- `docs/development.md` — inferred.

## Acceptance

- On a machine with the documented prerequisites, the subcommand runs every ignored suite except
  the subprocess entry points and exits non-zero if one fails.
- Without a prerequisite, it reports each skipped suite and the prerequisite by name and runs the rest.
- `docs/development.md` lists the prerequisites per family and the command.
- One run's output on the operator's machine is recorded as evidence.
