---
format: aep.planning-md/3
id: story:git-relay-pre-push-check-sandbox
kind: story
status: draft
title: Run the git relay's pre-push check without the instance's file access
summary: A check that builds or tests pushed code cannot read the mounted forge credential
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:git-smart-http-relay-with-quarantine
revision: 1
---
## Outcome

The git relay's pre-push check runs without the instance's file access, so a check that builds or
tests the pushed tree cannot read the mounted forge credential or another repository's mirror.

Today the check runs as the instance's user with a cleared environment
(`adapters/catalog/src/served/git.rs`, `pre_push_check`). Its environment holds no credential, but
it can read every file the instance reads, the mounted credential item included.
`docs/running-services.md` § Git relay tells operators to configure a check that inspects the
pushed objects and never runs them. This story removes that restriction.

## Acceptance

- `the_pre_push_check_cannot_read_the_forge_token`: a check that tries to read the instance's
  mounted credential item finds it absent, and the push it passes is applied.
- `the_pre_push_check_sees_only_its_quarantine`: a check that lists the work directory sees its
  own quarantine and no mirror of another repository.
- The sandbox is configuration (a confinement profile or a wrapper command), with no organization
  name in source, and a missing sandbox refuses start when the configuration asks for one.

## Origin

Adversary pass on unit C4 (wave-d265-c4), finding 4, judgement: the check runs with the
instance's file access and read the forge token in a probe.
