# 16 — Subscription credential custody

Status: proposed 2026-08-25. Backs the `subscription-custody` epic (S-070..S-074), with the
`oauth-consolidation` epic (S-069) as its prerequisite.
Timo's goal, verbatim intent: a person signed in to the platform clicks **Connect Claude-Code**,
completes the flow, and the resulting credential is persisted under *their* user, reachable from
the web UI and the `connectors` CLI alike.

This document answers what that can and cannot mean, and names the one schema change it needs.

## What the vendor actually permits (measured, 2026-08-25)

The literal request — take a Claude Code OAuth token and call the Anthropic API with it — is not
available to us, on three independent grounds:

- **No client to register as.** Anthropic operates no third-party OAuth client registration. The
  Claude Code client id `9d1c250a-e61b-44d9-88ed-5944d1962f5e` is hardcoded to Claude Code.
- **Server-side enforcement since January 2026.** A subscription OAuth token presented outside
  Claude Code and Claude.ai is refused with `This credential is only authorized for use with Claude
  Code and cannot be used for other API requests`.
- **Policy since February 2026.** Subscription OAuth is Claude Code and Claude.ai only; third-party
  products, the Agent SDK included, must authenticate with a Console API key.

Two consequences follow and are not negotiable by design choice:

1. **The API path is an API key.** `anthropic.api_key`, already declared, is the only credential
   this platform may spend against `api.anthropic.com`.
2. **There is no identity in an Anthropic credential.** No token of any kind carries a profile or
   email; `/v1/organizations/me` returns an organization id, type and name, and requires an Admin
   key. Anthropic cannot be an identity provider for us, so Google SSO stays and the original
   "read the email, replace the login" half of the request is withdrawn.

## The distinction this document introduces: custody without use

A Claude Code subscription token is still worth holding. A person who has one wants the platform
to run *Claude Code* on their behalf, and the credential has to live somewhere addressable,
per-user, and revocable. What must never happen is connectors spending it — that is the act the
vendor refuses and ADR 0014 assigns elsewhere.

So the two verbs separate:

- **Custody** — issuing an address, storing the value, rotating it, reporting presence, revoking
  it. This is connectors' owned responsibility under platform ADR 0032, and nothing in the harness
  is built for it: harness has no multi-tenant store, no HTTP surface, and one operator.
- **Use** — placing the value on an outbound request. This stays with the harness adapter, per
  platform ADR 0014.

Splitting them is what lets the credential be held here without contradicting either ADR. It is
also the reason the amendment to ADR 0014 is narrow: it admits custody, and restates that use is
unchanged.

## The schema consequence

A custody-only provider cannot be expressed today. `crates/connector-spec/src/provider/validation.rs:30`
refuses a declaration carrying neither `[spec]` nor `[[operations]]`:

> declares neither `[spec]` nor any `[[operations]]`, so it describes no operations at all.

That refusal is pinned verbatim by `tests/golden/nothing-to-generate.error`, and it is correct for
every provider that exists — all 64 describe an external surface. Two ways around it are available
and both are wrong:

- **Point `[spec]` at a document and select nothing.** Legal — `catalog-build/src/seam.rs:741-743`
  states that a spec with nothing selected is a connector with no operations, not an error. But
  there is no Anthropic document to point at for this surface, and authoring one to select nothing
  from is exactly the fabrication AGENTS.md step 2 forbids.
- **Declare one honest operation.** Defeats the point: the credential becomes spendable from here.

The change is therefore a declared kind, not a workaround:

```toml
custody_only = true
```

A provider that sets it declares a credential and nothing else. The loader's obligations invert:

| Field | Ordinary provider | `custody_only` provider |
|---|---|---|
| `[spec]` / `[[operations]]` | one is required | **both refused** |
| `base_url` | required, non-empty | **refused** — there is no request to build |
| `[[service]]` | as declared | **refused** — a service exists to carry operations |
| `verify` | optional | **refused** — verification is a request |
| `[[auth]]` | as declared | **required**, at least one |

Every one of those is a refusal rather than a silent allowance, so the kind cannot be used to
smuggle a half-declared ordinary provider past review. The invariant belongs in the one
consolidated file, `crates/catalog-build/tests/main/catalog_invariants.rs`, parameterised over
every provider that sets the flag — AGENTS.md bans a per-provider test file.

## The two providers

**`anthropic`** is unchanged in identity, authority and auth names. `catalog_invariants.rs:1226`
pins its auth list to `["anthropic.api_key", "anthropic.admin_key"]` and asserts no Claude Code or
undocumented OAuth authority reaches it; that assertion stays true and stays valuable. The only
addition is `entry = "connect_session"` on the API key, so the value can arrive through a hosted
Connect Session instead of a hand-edited file. Its declared `verify = "anthropic-models-list"`
becomes the validity ping the UI reports.

**`claude-code`** is new, `custody_only`, `authority = "com.anthropic.claude-code"`. The authority
leads every credential path this provider will ever own and cannot be repointed, so it is chosen
as permanent per AGENTS.md step 3. The credential arrives by `entry = "connect_session"`: the
person runs `claude setup-token` on their own machine, the vendor's own tooling performs the
browser flow, and the printed token is pasted into a single-purpose session. We never drive the
vendor's OAuth, never hold a client id, and never see the browser leg — which is precisely why
this shape survives the vendor's enforcement while a redirect-based one would not.

The separate provider id is what keeps the anthropic invariant honest. Extending `anthropic` would
have required weakening a test someone wrote deliberately, twice.

## Why not the subscription OAuth shape

`OAuth2Spec::public_client` and `OAuth2Spec::token_endpoint` exist in the IR and no shipped
provider uses them. `crates/connector-spec/tests/main/oauth_token_endpoint.rs:1-13` records why
they were added: *"Anthropic's subscription flow authorizes on `claude.ai` and redeems its token on
`platform.claude.com`, which one endpoint cannot express."* The declaration surface for this flow
was built and the acquisition authority was withheld on purpose.

That judgement is reaffirmed here, now with the vendor's own enforcement behind it. The fields stay
unused. If Anthropic ever opens third-party registration, they are what a declaration would use,
and `auth_archetypes.rs:355-460` already states the terms: an `authorization_code` grant must
declare operator-level, non-secret `oauth.client_id` and `oauth.redirect_uri` configuration, every
grant must declare `token_path`, and `public_client = true` exempts only the client secret.

## Rules carried over

Custody is ordinary custody; nothing here earns an exception.

- Per-user storage is `CredentialRef::for_instance(tenant, authority, instance_id, service, leaf)`,
  addressed through `integration_catalog::credential_address` and no second copy of that rule —
  `connectors-console/src/enrol.rs:227` records what the second copy cost last time.
- Values commit through a prepared transaction with crash recovery, never a point write.
- Non-credential response fields land in Connection metadata, never the credential store
  (design 01 § open questions, the `token_response_metadata` invariant).
- Connect Session invariants hold unchanged: single-purpose, short-lived, never returns credential
  material to its creator, terminal event names the connection id and nothing else.
- No `client_id`, secret, or credential-shaped example appears in the catalog.

## Open questions

1. **Does `custody_only` want to be a provider-level flag or a credential-level one?** A provider
   that is *mostly* ordinary but holds one unspendable credential is imaginable and not yet needed.
   The flag is provider-level until a second case argues otherwise.
2. **Rotation.** A `claude setup-token` token is a one-year credential with no refresh. Presence is
   reportable; expiry is not, without spending it. Whether the platform warns on age, and on what
   basis, is deferred to the story that gives it a consumer.
3. **Who reads it.** The harness adapter is the intended and only reader. The mechanism that binds
   that — a reader allowlist on the authority, or convention plus review — is not settled here and
   blocks nothing in this document.
