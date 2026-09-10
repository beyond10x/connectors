---
format: aep.planning-md/1
id: credential-blocker:gitlab-runtime-sandbox
kind: credential-blocker
status: open
title: Dedicated GitLab sandbox target and protected credential source are missing
relations:
- blocks: story:persistent-gitlab-journey
withholds: test_result
revision: 1
---
## Missing prerequisite

The operator-approved persistent GitLab journey requires a dedicated provider sandbox, an exact allowed project and protected entry of that sandbox's existing credential. The session requested the sandbox URL/project and an owner-only credential-file path. No response or new sandbox credential has been supplied as of this observation.

The installed 0.7.0 Connectors has credential presence, but its configuration and credentials are a separate deployment and are explicitly excluded from automatic migration by initiative:complete-local-connectors. No secret value was read into conversation and no sandbox identity was inferred from that installation. The previous repository's local service fixtures are not proof that a dedicated live GitLab sandbox is currently authorized or available.

## Impact and clearing evidence

This withholds the live-provider acceptance evidence for story:persistent-gitlab-journey. It does not prevent implementation or deterministic local fixture tests, and is not a claim that the whole initiative cannot progress.

Clear after the operator supplies the dedicated URL, project and an admitted protected source (path only in conversation), or explicitly authorizes a concrete equivalent sandbox setup. Verify that target through the new runtime when custody and owner supervision exist. Local substitutes or specification validation cannot clear the missing live-provider acceptance.
