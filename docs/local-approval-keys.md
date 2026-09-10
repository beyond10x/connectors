# Local approval-signing keys

The local owner can create and manage Ed25519 approval-signing keys in qualified
Secret Service custody. SQLite stores public keys, immutable identity history and
publication state. Key management does not issue approval proofs or enable provider
writes; those still require caller admission, qualified time and dispatch binding.

After local setup and adapter configuration, initialize and inspect an issuer:

```sh
connectors approvals key-init --adapter forge
connectors approvals key-status --adapter forge --output json
```

The first initialization needs no expected revision. Status returns the issuer,
audience, current revision and a bounded list of public key IDs, public keys and
states. Before initialization, the optional issuer member is absent. Status starts
no adapter and reads no signing seed. The configured adapter
executable need not be running for any key-management command.

Use the exact UUIDs returned by status for subsequent management:

```sh
connectors approvals key-rotate --adapter forge \
  --expected-revision "$revision" --expected-key "$active_key"
connectors approvals key-recover --adapter forge \
  --expected-revision "$revision" --candidate "$candidate_key"
connectors approvals key-revoke --adapter forge \
  --expected-revision "$revision" --expected-key "$active_key"
connectors approvals key-retire --adapter forge \
  --expected-revision "$revision" --key "$retired_key"
```

Refresh status after each decision. Rotation stages a candidate before storing its
seed and publishes it only after durable custody acknowledgement. Until publication,
the old key stays active. After publication, the new key is active and the old key
is retired, even if the CLI response was lost. Rotation immediately invalidates the
old key for further proof use; there is no overlap window.

Recovery confirms the exact staged seed and synchronizes custody again. A missing
seed cannot be recovered by generating another one; cancel that candidate with
`key-retire` and explicitly initialize or rotate again. A locked keyring must be
unlocked outside this CLI before protected key use can succeed.

Revocation retires both the current key and any pending replacement. To initialize
again, supply the new revision to `key-init --expected-revision "$revision"`.
Retirement removes only exact noncurrent material. An uncertain deletion remains
`retiring`; retry that exact key using the revision from status. Deleted identity
history remains, and the 128-key lifetime bound refuses additional allocation.

The [issuer contract](../contracts/service/approval-issuers.md) specifies the
publication and reclamation rules. The [proof/spend binding](local-approval-binding.md)
requires consumers to combine the current-key lease with independent caller and
subject authorization. Credentials and signing keys occupy different custody
purposes, including when their private UUID coordinates happen to match.
