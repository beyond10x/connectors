---
format: aep.planning-md/3
id: story:approved-writes-on-a-headless-linux-host
kind: story
status: draft
title: Approved writes on a headless Linux host that starts unattended
summary: Mounted read-only custody cannot hold the approval key; the distribution's Secret Service is not qualified
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
# Approved writes on a headless Linux host that starts unattended

## Need

An unattended agent host (headless Ubuntu 24.04, x86_64, user-level install, no desktop session)
reads through Connectors but cannot make any approved write, such as a bot connection posting to
Slack. Approval stays required: this story asks for a supported custody of the approval signing
key on such a host, not for a write that skips approval.

## Reported (0.72.0, in a VM and on a real host; not reproduced here)

- The credential store is `mounted_secret` (read-only files, mode 400). Two Slack connections
  connect and read (`auth.test`, `conversations.history`).
- `invoke <slack bot instance> chat.postMessage` answers `approval_required`.
- `approvals key-init` answers `custody_read_only`: the read-only store refuses the approval
  signing key.
- Secret Service is admitted only for the qualified daemon build, so the distribution's
  gnome-keyring is refused.

## Verified in source (main at 8cad82e3031)

- `custody_read_only` is the refusal for a read-only custody binding
  (`crates/connectors-host/src/local/registry.rs:83`).
- A mounted binding is qualified only as a read-only mount
  (`crates/connectors-host/src/local/keyring/custody.rs:291-303`); the platform binding needs the
  qualified implementation, not availability alone (`apps/connectors/src/local.rs:755-759`).

## Options to weigh (none chosen)

- An approval key custody that works beside the mounted store, for example a key delivered as one
  more mounted read-only file and only read, with the key's issuance kept outside the host.
- A qualified Secret Service build for Ubuntu 24.04, with the same durable-write, restart and
  retirement evidence the current one has.

## Acceptance

- On a headless Ubuntu 24.04 host with no desktop session, started unattended, an approved
  `chat.postMessage` through a bot connection succeeds; without a valid approval it is still refused.
- The chosen custody has a security review recorded before it is accepted.
- The setup guide names the steps for such a host.
