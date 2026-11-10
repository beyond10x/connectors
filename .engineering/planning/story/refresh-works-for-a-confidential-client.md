---
format: aep.planning-md/3
id: story:refresh-works-for-a-confidential-client
kind: story
status: draft
title: Refresh works for a confidential OAuth2 client, or connect refuses one
tags:
- cr-006
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Defect

Code review finding CR-006 (S3): Refresh cannot work for a confidential OAuth2 client (latent).

- Citations: `crates/connectors-host/src/local/owner/refresh.rs:374` (`Client::new(declaration, Some(&entry.client_id), None, &https)`); `crates/connectors-oauth2/src/client.rs:107-114`; `crates/connectors-host/src/local/oauth2_grant.rs:467-516`.
- What happens: connect succeeds, the first refresh fails. No shipped declaration is confidential today.
- Status: verified.
- Fix: keep the client secret in custody and pass it, or refuse confidential declarations at connect.

- Review's story note: new (`story:google-consent-on-the-oauth2-core`, draft, plans one).

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-006. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- A test shows the finding on the base (red) and the fix makes it green; where no test can make it red, that evidence is recorded and nothing is changed.
