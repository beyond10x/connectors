---
format: aep.planning-md/3
id: story:the-owner-contract-names-every-test-variable-a-debug-cli-passes-to-the-owner
kind: story
status: draft
title: The owner contract names every test variable a debug CLI passes to the owner
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

`contracts/cli/v1alpha1/owner.md` line 80 says: a CLI built with debug assertions whose
environment sets `CONNECTORS_TEST_OWNER_PARENT` passes that variable, alone, to an owner it starts
on Unix.

The code passes more. Read at main 4bd8259f, `crates/connectors-host/src/local/owner/transport.rs`,
the loop in `spawn` after `.env_clear()`: seven names in a debug build
(`CONNECTORS_TEST_OWNER_PARENT`, `CONNECTORS_TEST_OAUTH_FOLLOW`,
`CONNECTORS_TEST_OWNER_START_DELAY_MS`, `CONNECTORS_TEST_VERIFICATION_LOG`,
`CONNECTORS_TEST_ACQUISITION_MS`, `CONNECTORS_TEST_PUBLICATION_FAILS_ONCE`,
`CONNECTORS_TEST_PUBLICATION_REPLY_LOST_ONCE`).
story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build adds
every variable whose name starts with `CONNECTORS_TEST_OVERRIDE_`.

Found by the adversary read of that story's change (review-result:debug-override-adversary-pass-1,
finding 5). It existed before that change.

## Change

The sentence of the contract says what a debug build passes: the named test variables and the
prefix family, and that a release build passes none of them. Whether the contract lists the seven
names or points at the function that picks them (`owner_environment`) is decided when the story is
planned.

## Acceptance

Open until the story is planned. One statement holds already: a reader of
`contracts/cli/v1alpha1/owner.md` finds no sentence that says the CLI passes one test variable
alone.

## Scope

- `contracts/cli/v1alpha1/owner.md`: the paragraph at line 78.

## Not in this story

- Any change of what the CLI passes.
