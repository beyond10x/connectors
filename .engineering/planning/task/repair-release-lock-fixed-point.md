---
format: aep.planning-md/1
id: task:repair-release-lock-fixed-point
kind: task
status: implemented
title: Repair the release lockfile fixed point
summary: Regenerate every satellite workspace lock after a release version change and prove the tag gate from a clean checkout.
relations:
- decomposes: story:broker-read-only-git-fetch-sessions
revision: 4
---
<!-- Starting point for a `task` artifact, seeded by `aep artifact new task <name>`.
     No frontmatter here on purpose: the `---` block is written by the CLI from the id, kind, status
     and relations you gave it, and a second copy in this file would be the one that went stale.
     Delete the italic guidance as you fill each section. -->

# Task: <name>

## What

*The concrete unit of work, in the active voice. One task, one thing — if this needs an "and", it is
two tasks or it is a story.*

## Why

*Which story or specification this serves. The `decomposes` or `implements` relation carries the
edge; this says what would be lost by dropping the task.*

## Done When

*The observable condition that ends it. A task whose completion is a matter of opinion is one that
gets marked done twice and finished never.*

## Notes

*Anything the next person needs and cannot derive: the file to start in, the command to run, the
trap somebody already fell into.*
