---
id: S-071
title: "Claude Code joins the catalog as a custody-only provider"
pillar: Catalog
status: done
priority:
design: ../design/16-subscription-credential-custody.md
epic: subscription-custody
areas: [providers, catalog-build]
note: "New provider id claude-code, authority com.anthropic.claude-code, custody_only. The credential is a claude setup-token output pasted into a Connect Session. Blocked on S-070; the authority is permanent once chosen."
---

# Claude Code joins the catalog as a custody-only provider

## Goal

Give a person's Claude Code subscription credential an owner, a per-user address and a revocation
path, without connectors ever being able to spend it.

## Acceptance

- [x] `providers/claude-code.toml` declares `id = "claude-code"`,
      `authority = "com.anthropic.claude-code"`, `custody_only = true`, and exactly one credential
      with `entry = "connect_session"`.
- [x] The authority `com.anthropic.claude-code` differs from `anthropic`'s `com.anthropic.api`, and
      a catalogue invariant asserts no two providers share an authority — the authority leads every
      credential path this provider will ever own and is never repointed (AGENTS.md, adding a
      connector, step 3).
- [x] The declaration carries **no** `client_id`, no secret, no credential-shaped example, and no
      reference to `claude.ai/oauth` or `platform.claude.com/v1/oauth`. We do not drive the vendor's
      OAuth; `claude setup-token` does, on the person's own machine.
- [x] `providers/anthropic.toml` is untouched. Its invariant at
      `crates/catalog-build/tests/main/catalog_invariants.rs:1226` — auth exactly
      `["anthropic.api_key", "anthropic.admin_key"]`, no Claude Code authority in the API connector
      — still passes unmodified. A separate id is what keeps that assertion honest.
- [x] A parameterised invariant asserts `claude-code` exposes zero operations and zero services, so
      nothing in connectors can spend the credential.
- [x] Build and commit as one unit: `catalog build` → `catalog diff` clean twice → `catalog check`.

## Progress
- 2026-09-01 — `claude-code` now joins the generated catalog as a distinct custody-only provider.
  The parameterized authority and zero-surface invariants cover it, and the Anthropic API provider
  remains byte-for-byte untouched.
- 2026-09-01 — [S-077](S-077-claude-subscriptions-connect-with-pkce-and-refresh.md) supersedes the
  original paste-only acquisition constraint after the installed provider client demonstrated a
  public-client PKCE and refresh contract. The catalog remains custody-only and exposes no request
  surface.

## Notes

- Blocked on [S-070](S-070-a-provider-can-hold-a-credential-it-cannot-spend.md): the declaration
  kind does not exist yet.
- Authority for admitting this at all: platform ADR 0056 (2026-08-25), which partially supersedes
  ADR 0014 for custody and separates
  custody from use. Use stays with the harness adapter; this story is custody only.
- Vendor constraint, measured 2026-08-25 and the reason the shape is paste-only: Anthropic operates
  no third-party OAuth client registration, and since January 2026 refuses a subscription token
  presented outside Claude Code and Claude.ai with `This credential is only authorized for use with
  Claude Code and cannot be used for other API requests`. A redirect-based acquisition would be
  both unregisterable and refused.
- `OAuth2Spec::public_client` and `OAuth2Spec::token_endpoint` stay unused. They exist for this
  vendor's flow (`crates/connector-spec/tests/main/oauth_token_endpoint.rs:1-13`) and the
  acquisition authority was withheld on purpose; design 16 reaffirms that.
