# 13 — Shared Secrets custody

Status: implemented for the hosted runtime.

This note amends designs 02 and 07 for the shared Secrets service. Connectors continues to own provider exchange, refresh, provider-side revocation, and the mapping from a connection to credential references. Secrets owns encrypted bytes, versions, local lifecycle, and custody audit records.

## Runtime selection

`[secrets]` and `[vault]` are mutually exclusive runtime stores. Credential-bearing integrations require exactly one complete store. The Secrets form is value-free:

```toml
[secrets]
enabled = true
origin = "http://secrets:8080"
token_file = "/var/run/secrets/tokens/secrets-token"
```

The token is a projected Kubernetes service-account token, read again for every request so rotation does not require a restart. The exact audience and service-account-to-tenant grant live in deployment configuration. Optional `ca_file` adds a private TLS root.

The adapter maps `CredentialRef` into a tenant plus authority namespace and a reversible structured key. Values occur only in bounded JSON bodies. Errors, URLs, metrics, and logs carry references at most. Put-only prepared generations use one atomic Secrets transaction; the existing prepared-outcome journal retains protocol recovery semantics.

`SecretStore::put_owned` is additive and defaults to `put`. Subscription custody overrides it through the remote adapter so the subject verified at connect time becomes resource ownership. Provider refresh reuses the subject retained in the short-lived lease.

## Migration

`connectors-secrets-migrate --config <file>` accepts 1–128 exact tenant/authority scopes, a source Vault configuration, a destination Secrets configuration, and an optional owner subject. It lists references, skips existing destinations, streams each value from source to destination in memory, prints counts only, and never deletes or mutates Vault.

Run migration only while Connectors is stopped. Verify destination counts and runtime readiness, cut the runtime to `[secrets]`, then keep Vault unchanged for an operator-approved rollback window. Removal of the old Vault deployment is a separate destructive operation and is never part of this command.
