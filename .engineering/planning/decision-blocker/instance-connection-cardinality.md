---
format: aep.planning-md/3
id: decision-blocker:instance-connection-cardinality
kind: decision-blocker
status: cleared
title: Nobody has decided whether an instance holds at most one connection, which ones count, and whether that is enforced
relations:
- blocks: epic:connector-vocabulary
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-16T11:47:08Z", actor: "human:timo", revision: 4}
---
## Question

Instance -> Connection: how many connections may one instance hold, which ones count, and is the
limit enforced on existing aliases that already hold more than one?

## What conflicts

- `architecture-decision-record:connector-vocabulary` decision 3: an instance "holds at most one
  connection".
- `ess/domains/auth_bindings.yaml`, entity `connectors.auth_bindings.Connection`, relation `instance`
  (`references connectors.declarations.ServiceConfiguration`, `cardinality: one`, `via: instance_id`)
  bounds only the Connection side; nothing bounds how many Connections reference one instance. The
  same entity's notes (line 190) speak of "the instance holds a connection, revoked ones
  included" and of "every other connection of the instance" (line 172): several connections per
  instance, revoked ones retained, are the modelled state today.
- `--connection` is a required selector on `connections describe/repair/revalidate/revoke/launch`
  (`ess/domains/cli.yaml` lines 879-980), which only makes sense when an instance holds several.

## Options

1. At most one non-revoked connection; revoked ones are history. Connect on an instance with a live
   or stale connection is refused; `--connection` becomes optional.
2. At most one connection ever; a new consent needs a new instance.
3. The ADR sentence is descriptive (the normal case) and nothing is enforced; `--connection` stays.

Any of 1-2 also needs an answer for an existing alias holding two non-revoked connections when it
migrates (`migration-plan:connector-vocabulary` keeps connection ids byte for byte).

## What waits on it

No story enforcing the limit was drafted. The drafted stories keep today's cardinality and the
`--connection` selector unchanged.

## Decision

Option 1, decided by the operator on 2026-10-16: at most one non-revoked connection per instance;
revoked connections stay as history. Connect on an instance that holds a `Live` or `Stale`
connection is refused; `--connection` becomes optional. An existing alias that holds more than one
non-revoked connection keeps all of them through migration (connection ids stay byte for byte), and
a new connect on it is refused until the extra connections are revoked. On the machine this was
decided on, no alias holds more than one non-revoked connection (`connectors connections list
--adapter <alias>` for each of 8 aliases, 2026-10-16).
