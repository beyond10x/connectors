# auth.profile/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** auth. Sibling documents: [connection](../../connection/v1alpha1/semantics.md), [acquisition](../../acquisition/v1alpha1/semantics.md), [custody](../../custody/v1alpha1/semantics.md), [capability](../../capability/v1alpha1/semantics.md), [evidence](../../evidence/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `auth.profile/v1alpha1` |
| Nature | declaration, not runtime behavior: the part of an adapter specification that says what credentials the provider accepts, for what purpose, how they are acquired, and what they grant |
| Referenced by | operations (`requires_auth`), configuration (`auth_profiles`), `auth.connection` (`auth_profile`), `auth.acquisition` (flow selection), `auth.capability` (which capability a connection exposes), `auth.evidence` (what to check) |

Design responsibility one of four: "Describe required authentication — provider auth profile, referenced by operations/configuration" (`docs/design.md:524-529`). Profiles are provider-owned declarations with shared flow implementations (`docs/design.md:537`).

## 2. Old evidence and disposition

Counts over `../connectors/providers/*.toml` (`rg` on 2026-09-08): schemes `bearer` 50, `basic` 6, `signing` 3 (twilio, slack, stripe); `entry = "connect_session"` 11; grants `authorization_code`+`refresh_token` 4, `authorization_code` 1, `client_credentials` 1, `password`+`refresh_token` 1 (babelforce); `subject = "app"` 11, `subject = "user"` 10.

| Old surface | Source | Disposition |
|---|---|---|
| `[[auth]]` blocks: `name`, `scheme`, `subject`, `env`/`user_env`, `entry`, `description`, `[auth.oauth2]` with `endpoint`, `authorize_path`, `token_path`, `scopes`, `grants` | `../connectors/providers/jira.toml:140-297` | preserve the vocabulary; `env`/`user_env` become secret references in configuration, not profile fields |
| `default_auth` listing acceptable credential sets per operation with required scopes per credential | `providers/jira.toml:217-222` | preserve as `requires_auth` on operations: a list of alternatives, each naming a profile and required granted scopes (`docs/design/09-…:46-72` in the old repo: requirements attach scopes to the credential that owns them) |
| Slack token roles: bot, user, app-level distinct; app-level never a per-tenant installation token | `docs/design.md:539` | preserve as `purpose` |
| Endpoint URLs, PKCE, token shapes inferred from another provider | `docs/design.md:539` | remove: every profile cites its own vendor source |
| Kubernetes exec credential plugin gated by `allow_exec_auth = false` | `../connectors/crates/integration-kubernetes/src/local.rs:311-315` | preserve as an `acquisition: exec_plugin` profile that is disabled unless configuration enables it |
| Subscription credential custody: custody without use, attempt-bounded leases | `../connectors/docs/design/16-subscription-credential-custody.md`, `17-attempt-bounded-subscription-credential-leases.md` | defer; not needed by the six areas in scope |

## 3. Types

A profile declaration inside an adapter specification:

```json
{
  "id": "jira.user_oauth",
  "purpose": "delegated_user",
  "subject": "user",
  "scheme": "http_bearer",
  "acquisition": {
    "flow": "oauth2_authorization_code",
    "authorize_url": "https://auth.atlassian.com/authorize",
    "token_url": "https://auth.atlassian.com/oauth/token",
    "pkce": "none",
    "refresh": "rotating_or_static: per vendor source",
    "grants": ["authorization_code", "refresh_token"],
    "registration": { "kind": "confidential_client", "redirect": "host_callback" }
  },
  "scopes": { "requestable": ["read:jira-work", "write:jira-work", "offline_access", "read:me"], "minimum": ["read:jira-work"] },
  "identity": { "kind": "atlassian_account", "from": "token_response_or_me_endpoint" },
  "revocation": { "supported": true, "url": "…" },
  "capabilities": ["http-bearer"],
  "evidence": ["scope_check", "identity_check"],
  "sources": ["https://developer.atlassian.com/cloud/jira/platform/oauth-2-3lo-apps/"]
}
```

Closed vocabularies:

| Field | Values |
|---|---|
| `purpose` | `service_account`, `delegated_user`, `app_level`, `inbound_verification`, `transport_identity`, `trunk_registration` |
| `subject` | `app`, `user`, `none` (inbound verification, transport identity) |
| `scheme` | `http_bearer`, `http_basic`, `http_signing`, `mtls`, `socket_peer`, `exec_plugin`, `sip_digest`, `session_authority` |
| `acquisition.flow` | `static_entry`, `static_config`, `oauth2_authorization_code`, `oauth2_client_credentials`, `oauth2_password` (declared for compatibility only; refused by default), `workload_identity`, `exec_plugin`, `host_issued` |
| `capabilities` | names from `auth.capability` |
| `evidence` | names from `auth.evidence` |

Operation reference:

```json
{ "id": "issue.create", "requires_auth": [ { "profile": "jira.user_oauth", "scopes": ["write:jira-work"] }, { "profile": "jira.api_token" } ] }
```

## 4. Rules

- A profile is provider-owned and reviewed: `sources` must cite the vendor page for URLs, PKCE, token response shape, refresh behavior, and revocation. The compiler refuses a profile whose `acquisition` names an OAuth flow without `authorize_url`/`token_url` and a source.
- A profile declares; it never contains a credential value, a secret reference, or a redirect secret. Registration secrets (client id/secret) are configuration bound at the host through `auth.custody`.
- `requires_auth` alternatives are evaluated at execution against the selected connection's profile and granted scopes (`auth.evidence`); description metadata never authorizes (`docs/design.md:358`).
- `purpose` differences are semantic, not cosmetic: an `app_level` credential cannot satisfy a `delegated_user` requirement even when the scheme matches (`docs/design.md:539`).
- Profiles with `subject: none` (`inbound_verification`, `transport_identity`) attach to an instance or ingress, not to a per-user connection.
- `oauth2_password` is refused unless configuration explicitly enables it; it exists only to document the one old provider that uses it.
- A profile change is a specification change: it changes the descriptor revision and requires review of every connection that references it.

## 5. Limits

| Concern | Rule |
|---|---|
| Profiles per adapter | bounded by the spec schema (first-profile default 16) |
| `requires_auth` alternatives per operation | bounded (default 8) |
| Scope strings | bounded length, provider syntax not interpreted |

## 6. Conformance scenarios

- Compiler: a profile without `sources` → refused; OAuth flow without URLs → refused; an operation naming an unknown profile → refused.
- Runtime: operation requiring `write:jira-work` invoked over a connection whose granted scopes are `read:jira-work` → `insufficient_scope`, no dispatch.
- Purpose mismatch: connection on an `app_level` profile used for a `delegated_user` requirement → `Forbidden`, no dispatch.
- Descriptor output contains profile ids, purposes, schemes, and flows but no URL secret, client secret, or credential reference.

## 7. Compatibility

- Old `[[auth]]` blocks translate mechanically except `env`/`user_env` (become configuration credential references) and `entry` (becomes `acquisition.flow: static_entry`).
- `signing` scheme providers (twilio, slack, stripe) are out of the six areas; `http_signing` is reserved in the vocabulary so they translate later without a new version.

## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| Spec kind gains `auth_profiles` and per-operation `requires_auth` | `spec-kinds/adapter/v1/schema.json`, `v2/schema.json`; validation in `crates/connectors-spec/src/lib.rs` |
| Descriptor exposes profiles (safe subset) | `crates/connectors-core/src/lib.rs:70-78` |
| Configuration schema import for profile registration secrets | `crates/connectors-host/src/credentials.rs` (`CredentialRef`) |

## 9. ESS entities

| Entity / value | Notes |
|---|---|
| `AuthProfile` (identity: adapter id + profile id), value fields as above, lifecycle `Declared` | owned by `AdapterSpecification` (`ess/domains/declarations.yaml`) |
| `OperationDeclaration.requires_auth` | list of `{profile, scopes}` values |
| Registration (client id/secret) per deployment | belongs to `ServiceConfiguration`, secret material UNMAPPED into custody |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether scopes are opaque strings or provider-parsed | opaque; equality and subset checks only |
| Where the Atlassian `cloud_id` site selection lives (old `base_url = "https://api.atlassian.com/ex/jira/{cloud_id}"`, `providers/jira.toml:206`) | connection attribute filled during acquisition from the accessible-resources endpoint, cited in the adapter document |
| `oauth2_password` presence | declared, refused by default |
