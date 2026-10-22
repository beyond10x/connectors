---
format: aep.planning-md/3
id: story:custody-keeps-out-of-the-session-keyring
kind: story
status: draft
title: Custody keeps out of the operator's session keyring
summary: Tests never write the session keyring, removing a store deletes its items, the collection is the default alias
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Outcome

Tests and conformance runs never write the operator's session keyring, removing a store removes its custody items, and custody addresses its collection through the Secret Service `default` alias instead of a hard-coded `login` path.

## Why

Observed 2026-10-22 by another session, read-only over D-Bus with no secret value read: the operator's `/org/freedesktop/secrets/collection/login` held 347 items. 175 were "Connectors approval signing key" (format `approval-signing-custody/1`) and 164 "Connectors credential" (format `custody/1`), each with its own `connectors.authority`. They were created 2026-09-29 to 2026-10-22, and none was ever deleted. Over 19 days the keyring daemon grew to 2.1G resident and 2.3G swap and needed SIGKILL to restart. The item count is the suspected cause; it is not proven.

`crates/connectors-custody/src/secret_service/mod.rs:30` hard-codes `LOGIN_COLLECTION` as `/org/freedesktop/secrets/collection/login`. Every writer on the session bus therefore lands in the operator's real keyring.

## Work

1. Find every test, conformance run and example that reaches the session bus's Secret Service. Route each to a private daemon (the disposable Secret Service `confluence_binding.rs` already uses) or to a test double. Add a guard that fails a test run which would open the session bus's Secret Service.
2. Removing a store, an instance or a connection deletes the custody items it owns, and an approval signing key goes with the store that issued it. A failed delete is reported and named; it is never silently left.
3. Address the collection as `/org/freedesktop/secrets/aliases/default`. That is the alias the Secret Service specification and libsecret use for the user's chosen persistent collection. It resolves to `login` on a stock GNOME keyring, does not assume a collection name other desktops (KWallet's Secret Service, KeePassXC) do not use, and lets a private daemon serve the alias without the operator's collection. Keep reading items already stored under `login` when the alias resolves elsewhere: a migration lists them and moves nothing silently.
4. One command lists the custody items of this machine that no store names, and removes them only on an explicit flag.

## Acceptance

- A test run whose environment points at the session bus Secret Service fails before any write, naming the variable.
- Removing a store with N custody items leaves 0 items carrying its authority, observed on a private daemon.
- Custody writes reach the collection behind the `default` alias, and a private daemon that serves only that alias gets them.
- The orphan listing names every item whose authority no store holds, and removes none without the flag.

## Inventory 2026-10-22

Read-only, no secret value read. The authority is the `local_authority.authority_id` row of a store's `metadata.sqlite3` (`crates/connectors-host/src/local/metadata.rs:20,45,405-406`). 68 stores on the machine name 51 authorities. The keyring's 339 custody items carry 309 authorities, and only 4 of those (33 items) are named by a store that still exists. Of the other 305 authorities, 304 hold exactly one item and one holds two; they were created 2026-09-29 to 2026-10-22. That one-item-per-authority pattern fits test or probe runs that each made a fresh store; which processes wrote them is not established.
