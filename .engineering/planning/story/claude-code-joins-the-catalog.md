---
format: aep.planning-md/1
id: story:claude-code-joins-the-catalog
kind: story
status: implemented
title: Claude Code joins the catalog as a custody-only provider
refs:
- provider: legacy
  reference: S-071
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: providers
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-071-claude-code-joins-the-catalog.md:20`. **read**

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

## Context

Give a person's Claude Code subscription credential an owner, a per-user address and a revocation
path, without connectors ever being able to spend it.

Source frontmatter: pillar Catalog · areas [providers, catalog-build] · design `../design/16-subscription-credential-custody.md`. **read**

Source `note:` field, quoted: “New provider id claude-code, authority com.anthropic.claude-code, custody_only. The credential is a claude setup-token output pasted into a Connect Session. Blocked on S-070; the authority is permanent once chosen.”

## Status

`done` in the source. Quoted from `docs/stories/S-071-claude-code-joins-the-catalog.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-071-claude-code-joins-the-catalog.md`, which is not deleted and now names this artifact.

- First written 2026-08-25 · last touched 2026-09-01 · 4 revision(s)
- Legacy id `S-071`, recorded as the reference `legacy:S-071`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
