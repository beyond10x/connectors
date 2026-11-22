---
format: aep.planning-md/3
id: story:mutation-stores-have-a-retention-path
kind: story
status: draft
title: Attempt, audit and redemption stores have a retention path before their lifetime cap
tags:
- cr-005
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-005 (S3): Attempt, audit and redemption stores are lifetime caps with no recovery path.

- Citations: `crates/connectors-host/src/local/mutations.rs:152-153`; `mutations/types.rs:189` (10,000); `owner/mutation.rs:340`; `audit.rs:120-121`; `approvals/spend.rs:93-102`; `server/v1alpha2.rs:381-389`.
- What happens: after 10,000 writes to one instance every later write is refused `capacity`, permanently. On the served host a full audit store refuses reads too and answers `unavailable`.
- Status: verified. The cap is documented as deliberate (`docs/local-mutation-ledger.md:99-103`), so this is a risk, not a deviation.
- Fix: a retention command and an early warning; answer `capacity` on the served path.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-005. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
