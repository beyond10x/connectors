---
format: aep.planning-md/1
id: nosuchkind:probe
kind: nosuchkind
status: archived
title: Probe artifact created in error, kept as a record
refs:
- provider: legacy
  reference: S-074
relations:
- derived_from: epic:sources
revision: 4
---
## What this is

An artifact created by mistake on 2026-09-04, kept because the store's journal is tamper-evident and
a deleted document is a defect `aep artifact validate` reports.

While demonstrating an argument-order bug in the migration driver, the agent assumed
`aep artifact new` would refuse an undeclared kind and used one as a supposedly inert probe. It is
not refused: a kind that declares no lifecycle permits every status and every move, so the probe
became a real document. Removing it with `rm` was then caught:

> nosuchkind:probe was deleted: its log ends at event nosuchkind:probe@1#0~9b631d3316fff614 and the
> store holds no document — nothing is physically deleted through a command, so this was `rm`.

The document was never in version control, so it was recreated and retired through its ladder
instead. It is `archived` and appears in no active view.

## Why it is not deleted

Nothing in the planning store is deleted. The lesson is worth more than the tidiness: the planning
skill says never to create an artifact in order to look at one, and this is what happens when that
rule is broken.
