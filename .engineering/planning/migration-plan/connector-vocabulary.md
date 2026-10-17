---
format: aep.planning-md/3
id: migration-plan:connector-vocabulary
kind: migration-plan
status: draft
title: Migrate adapter aliases to connectors and named instances without re-consent
relations:
- derived_from: epic:connector-vocabulary
- informed_by: architecture-decision-record:connector-vocabulary
revision: 1
---
## Goal

Existing installations move from `[adapters.<alias>]` and `--adapter` to connectors and named
instances (`docs/design/connector-vocabulary.md`) without re-entering a credential, re-running a
consent, revalidating a connection or republishing an approval policy.

## What persists and depends on the old names

| Record | Keyed by | Source |
|---|---|---|
| Connection | `instance_id`, `configuration_revision` | `ess/domains/auth_bindings.yaml:105`, `:128` |
| Approval policy | `instance_id`, `configuration_revision`, `descriptor_revision`, `executable_selection`, `clock_configuration_sha256` | `connectors approvals policy-status --adapter slack-bot` output, 2026-10-16 |
| Configuration revision of a catalog instance | digest of `format`, `instance`, `provider`, `bundle_sha256`, `source_sha256`, `api_base`, `auth` without `token_ca_file`, `operations`, and `token_ca_digest`, `feed`, `request_prefix`, `hosts` when present; file paths are not digested | `adapters/catalog/src/local.rs:923-960` |
| CLI JSON output field `adapter` | the alias | `ess/domains/cli.yaml:23-52` |
| Callers passing `--adapter` | the alias | `rg -l -F -- --adapter` under `~/beyond10x`: connectors 86 files, cortex 13, platform 8, ess 8, thalamus 5, llm-gateway 5, loom 4, agentplugins 4, examples 2, zwirn 1 |

Not checked: the Secret Service attributes of custody entries, the mutation ledger and execution
audit keys, and how the non-catalog providers (`slack-credentials`, `tavily`) derive their revision.

## Invariant

A migrated instance keeps its `instance_id` and its `configuration_revision` byte for byte. Then
every record above stays valid and nothing is re-consented, revalidated or republished. The
digest's key names (`instance`, `provider`) stay as they are inside the digest even where the
user-facing word changes, because renaming a key moves every revision.

## Steps

1. **Read both formats.** The host reads `[connectors.<connector>.instances.<instance>]` and the
   legacy `[adapters.<alias>]` in one file. An instance and an alias that resolve to one
   `instance_id` are refused.
2. **Stable ids.** A migrated instance carries its old `instance_id` explicitly. A new instance gets
   an id derived from connector and instance name.
   UNMAPPED: the derived id's format; `instance_id` must pass the selector check at
   `crates/connectors-host/src/local/config.rs:530-533`, and whether `/` passes it is not checked.
3. **Derived fields.** The host derives the native configuration (bundle, selection, auth profile,
   endpoint default, executable) from the installed connector so that the digest input above is
   identical to the legacy entry's.
4. **`connectors setup migrate`.** `--dry-run` prints the new file and, per instance, the
   configuration revision and approval-policy selection before and after; `--apply` writes the file
   and keeps the old one as `config.toml.before-migrate-<timestamp>`. An entry whose revision would
   change is not migrated; the command names the digest field that differs.
5. **CLI names.** `--connector` and `--instance` replace `--adapter`; `connections reconnect`
   replaces `connections repair`. The old flag and verb keep working with one deprecation line on
   standard error, and the JSON output carries `adapter` beside `connector` and `instance`, for a
   deprecation window. UNMAPPED: the window's length, decided in the release plan.
6. **Callers.** The repositories in the table above move to the new flags inside the window; the
   connectors plugin skills in `agentplugins` move in the release that ships step 5.
7. **`Stale`.** No data migration: existing connections are `Live`, and the first refused credential
   moves one to `Stale`.
8. **Read and write instance pairs.** Not migrated automatically. One instance per account changes
   `auth.minimum_scopes` and `requested_scopes`, so its revision differs and it needs a new
   connection and consent. The guide says: add the merged instance, connect it, then revoke the old
   pair. On the machine this plan was written on, the only pair member is `gmail-write`, configured
   and not connected.
9. **Removal.** Reading `[adapters.*]`, `--adapter` and `repair` ends in a later release, after the
   window.

## Rollback

Restore `config.toml.before-migrate-<timestamp>` and run the previous release. While the invariant
holds, the state directory and custody are unchanged by the migration, so the previous release reads
them as before.

## Verification

- Per migrated instance: `--dry-run` shows equal revisions; after `--apply`, `connections list` shows
  the same connection ids with the same state, and `approvals policy-status` shows the same policy
  revision.
- A fixture configuration holding every adapter kind in the repository (catalog, credential
  binaries, MCP, SQL, Kubernetes) migrates with equal revisions.
- An invoke through `--adapter <alias>` and through `--connector … --instance …` reaches the same
  connection during the window.
