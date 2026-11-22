---
format: aep.planning-md/3
id: story:install-cache-refuses-a-symlinked-entry
kind: story
status: draft
title: The definitions install cache refuses a symlinked entry, and its test can fail
tags:
- cr-012
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-012 (S3): Install cache accepts a symlinked entry; its test cannot fail.

- Citations: `crates/connectors-definitions-install/src/store.rs:209`, `:229-230`; `crates/connectors-definitions-install/src/adversary_tests.rs:258-279`.
- What happens: `root.join("")` gives a trailing slash, so `symlink_metadata` follows a link to a 0555 directory. The test asserts only inside `if let Ok(again)`. Needs write access to the cache root.
- Status: inferred.
- Fix: `symlink_metadata(entry)` on the root; test with a 0555 target and assert `Err`.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-012. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
