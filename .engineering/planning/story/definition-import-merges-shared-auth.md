---
format: aep.planning-md/3
id: story:definition-import-merges-shared-auth
kind: story
status: draft
title: An import keeps the profiles of a shared auth file it does not ship
summary: definition import rewrites auth/<prefix>.yaml whole from shipped profiles
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
revision: 3
---
## Outcome

`connectors-build definition import` keeps every profile of a shared `auth/<prefix>.yaml` it does not
ship, or its help text and the authoring guide say that import owns those files whole.

## Why later

The second review of release 0.54.0 (2026-10-19, lane s3) proved that an import of `jira` erases a
hand-written `atlassian.oauth` profile from `auth/atlassian.yaml` (scratch test `verify_s3_2.rs:29`,
log `s3-verify-2.log`): `import.rs:2154-2168` writes the library from shipped profiles only, through a
plain `fs::write` (`import.rs:1786-1795`). The authoring guide calls `auth/` files "profiles that
several connectors share" (`website/docs/guides/author-a-definition.md:47-49`), which allows
hand-written entries. Whether import owns the files is a decision, not a patch.

## Acceptance

- Either the import reads the existing library and merges it, shipped profiles winning, with a test
  that a hand-written profile survives; or the help text and the guide state that import rewrites the
  shared auth files whole.
