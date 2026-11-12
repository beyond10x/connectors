---
title: Reconnect a connection
sidebar_position: 8
description: What a stale connection is and how to publish a new credential on the same connection, for every provider.
lede: A credential the provider refuses makes the connection stale; reconnect publishes a new one on the same connection and the same identity.
source: docs/local-connection-registry.md, docs/catalog-google-oauth.md, contracts/cli/v1alpha1/semantics.md
---

# Reconnect a connection

Every provider page under [Connectors](../connectors/index.md) ends its "Reconnect when the
connection is Stale" section with the provider's own cause. The steps are the same for every
provider.

## Stale

When the provider stops honouring the stored credential (revoked, rotated, expired), the
connection turns `stale`. An invoke answers `next_action: repair_connection`; revalidation cannot
help and no retry renews it. Publish a new credential on the same connection.

## Find the revision

```sh
connectors --output json connections list --connector <connector> --instance <instance> \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
```

## Reconnect

```sh
connectors connections reconnect --connector <connector> --instance <instance> \
  --expected-revision "<revision from the list>" <credential option>
```

The credential option is the one that connected the instance: `--credential-prompt`,
`--credential-file <owner-only file>`, `--credential-stdin` or `--credential-env <NAME>`, which
reads the credential from an environment variable. For a Google connector it is the
same client file, and the consent runs again; for a sign-in connector (ChatGPT, Claude) it signs in
again, by browser or with `--device`.

Reconnect replaces the credential on the same connection. A credential of another account or user
is refused as an identity mismatch (`identity_mismatch`) and the connection keeps its entry. A provider that cannot tell two accounts
apart says so on its page.
