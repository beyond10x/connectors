# GitLab personal access token entry

The native decoder and validation helper are implemented in
[auth.rs](../../../src/auth.rs) and exercised by
[provider interpretation tests](../../../tests/auth.rs). They are not yet wired
to CLI acquisition, durable custody or supervised invocation.

This adapter-owned `gitlab.pat` profile uses `static_entry`, purpose
`delegated_user`, subject `user`, and the existing `http-bearer` capability with
the fixed `PRIVATE-TOKEN` header (no added `Bearer` prefix). It accepts an existing
legacy personal access token. It creates or rotates no provider token. This
document specifies the native binding; connection publication still requires the
shared [acquisition](../../../../../contracts/auth/acquisition/v1alpha1/semantics.md),
[custody](../../../../../contracts/auth/custody/v1alpha1/semantics.md) and
[evidence](../../../../../contracts/auth/evidence/v1alpha1/semantics.md) rules.

## Protected input and target

The sole entry document is `{"token":"…"}`. It is one UTF-8 JSON object, at most
64 KiB, with no duplicate/unknown fields or trailing document. The token is
1–8192 visible ASCII bytes; it is never trimmed. Native parsing accepts no URL,
identity, scope or expiry assertions from entry. Terminal entry constructs this
same document privately. The Rust entry type has no Debug/Serialize and erases
its owned temporary buffers; transport and caller buffers have their own owners.

The host fixes the admitted GitLab API base, TLS configuration, adapter instance,
project allowlist and connection ownership before capture. The provider auth
implementation receives the same scoped HTTP capability pinned to the candidate
that would be installed for business reads. Redirects, proxies, ambient credentials
and mutable credential rereads cannot replace that capability. Generic host and
business input cannot select an identity URL or authentication header.

## Baseline validation

Using that exact capability, read `GET /user`, then
`GET /personal_access_tokens/self`, relative to the fixed API v4 base. GitLab
documents that `self` describes the token authenticating the request in its
[personal access tokens API](https://docs.gitlab.com/api/personal_access_tokens/#self-inform).
The [current-user API](https://docs.gitlab.com/api/users/#retrieve-the-current-user)
supplies the authenticated user's numeric identity. Both reads are mandatory;
a successful project read cannot replace either observation.

Require positive numeric IDs, `state: active`, equal user IDs, `active: true`,
`revoked: false`, a present native scope list and a present `expires_at` field.
Explicit null means no provider expiry. A non-null expiry is an exact date at
midnight UTC, as specified by [GitLab token expiration](https://docs.gitlab.com/user/profile/personal_access_tokens/#access-token-expiration).
Missing fields, malformed dates, negative IDs, duplicate JSON keys, overlarge
responses and a different user refuse. Additive provider fields are ignored after
strict JSON decoding; they confer no authority.

The baseline requires `read_api`. A native `api` grant also supplies `read_api`
for this profile; retain the actual grants and add that documented implication.
Scopes are bounded to 64 distinct nonempty strings of at most 256 visible ASCII
bytes. Granular token scopes have different semantics and cannot satisfy this
profile by omission or inference. The selected GitLab writes additionally require
`api`; token grants never replace host approval, operation permission or project
allowlist checks. Scope meanings follow
[GitLab's access-token scopes](https://docs.gitlab.com/user/profile/personal_access_tokens/#personal-access-token-scopes).

Each response is limited to 64 KiB. The two-read validation budget is 30 seconds
total and must be enforced by its execution owner. Evidence is valid for at most
60 seconds from the start of the first read, capped by known token expiry. Reject
a backwards clock, elapsed budget or expired evidence before returning validation.
The execution owner must recheck at publication and final dispatch against the
same generation, fixed binding and private fence. Native validation itself grants
no publication, repair, invocation or secret deletion authority.

401 means invalid credential, 403 means permission denied, 429/5xx or transport
failure means unavailable, and an unexpected response means invalid response.
No failure retries a token exchange or changes the provider target. Safe errors
contain no native response, entry contents or private credential reference.

## Typed ownership and verification

The independent [auth model](../../../spec/ess/domains/auth.yaml) declares the
entry, native observations, validation value and closed failures. Shared
AuthProfile/Connection/CredentialGeneration/CustodyVersion identities remain
independently owned; no duplicate native entity or cross-root import is added.
Provider interpretation tests must prove identity disagreement, missing/unknown
scope evidence, UTC boundary expiry, malformed responses, interrupted/unavailable
validation and exact reuse of the supplied HTTP capability. Full CLI custody and
restart acceptance remains a separate required runtime result.
