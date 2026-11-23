---
format: aep.planning-md/3
id: story:setup-check-names-missing-provider-with-instances
kind: story
status: draft
title: setup check names a missing catalog-provider executable when an instance is configured
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 7

7. A configured instance hides the diagnostic for a missing catalog-provider executable. With an empty configuration, setup check identified the missing companion. With a valid short-form instance and the same missing binary, it exited 2 with generic invalid_configuration before listing prerequisites. The same configured fixture passed all seven checks with the complete release installed.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/contracts/cli/v1alpha1/semantics.md#L397
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local.rs#L743

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- With one short-form instance configured and the catalog-provider binary absent, `setup check` lists the missing companion instead of exiting 2 with `invalid_configuration`.
