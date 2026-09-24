---
format: aep.planning-md/2
id: story:catalog-cli-journeys
kind: story
status: draft
title: Re-run the production CLI mutation journeys with the catalog provider as the child
relations:
- decomposes: initiative:complete-local-connectors
- derived_from: epic:retire-native-gitlab-adapter
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The production CLI journeys that proved the local owner's approval spend,
mutation settlement, lost-response handling, owner crash with same-key
observation, background recovery and post-effect revocation run again with the
catalog provider as the adapter child, and their records are retained as
evidence.

## Scope

The native GitLab adapter carried these journeys as ignored tests under
`adapters/gitlab/tests/local_runtime/` (`cli_journey.rs`, `guarded_merge.rs`,
`background_recovery.rs`, `ci.rs`, `merge_requests.rs`, `mr_validation.rs`,
deleted in `story:remove-native-gitlab-adapter`; last at commit 8dac09e). They
needed a qualified GNOME keyring, `dbus-daemon`, a signing clock fixture and a
built production CLI, and ran for evidence rather than in the gate. Port the
host-mechanics journeys to `adapters/catalog/tests/local_runtime/` with the
catalog provider as the child, the shipped selection set and a fake TLS GitLab
that answers the identity and scope probes; the native read semantics they
also checked (caps, cursor partitions, bounded traces, pinned validation) have
no catalog form and are not ported.

## Acceptance

- Each ported journey runs with `CONNECTORS_TEST_CLI` and the fixture custody,
  and passes; the run is recorded under `docs/evidence/`.
- The mutation ledger states, provider request counts and refusal codes the
  native journeys asserted hold with the catalog provider as the child.
