---
format: aep.planning-md/3
id: story:setup-migrate-holds-the-configuration-lock
kind: story
status: draft
title: setup migrate --apply writes the configuration under the endpoint lock
tags:
- cr-007
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-007 (S3): `setup migrate --apply` writes outside the configuration lock.

- Citations: `apps/connectors/src/local/migrate.rs:734-738`; `apps/connectors/src/local/endpoint.rs:545-554` ("every writer of this file holds the endpoint lock").
- What happens: a `connections connect --endpoint` between migrate's compare and rename is overwritten; the connect continues for an instance the file no longer names.
- Status: verified.
- Fix: take the same lock in `migrate::publish`.

- Review's story note: new (`story:endpoint-publish-holds-one-writer` covered endpoint against endpoint only).

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-007. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
