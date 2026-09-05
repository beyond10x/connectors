---
format: aep.planning-md/1
id: story:gitlab-group-and-project-creation
kind: story
status: draft
title: Create GitLab groups and projects through admitted operations
summary: Close the provisioning gap that forces callers out of Connectors when creating a repository namespace and project.
relations:
- derived_from: epic:catalog-adoptions
revision: 1
---
## Context

A repository relocation on 2026-09-05 required creating a private GitLab group and an empty
project beneath it. The installed personal-local CLI returned no operation from
`connectors operation search --query gitlab-project-create --limit 25`; its broader GitLab
search exposed reads only. The operator explicitly authorized a separate GitLab CLI to finish.

This is a provisioning gap, not a claim that Connectors supports no GitLab writes. At source
revision `65abcd54bbc4cf4b4ed7330acc362701a0bcdfe3`, `providers/gitlab.toml:323` and `:397`
declare group/project discovery, while `:571`, `:704`, `:898`, and `:945` already declare
issue, merge-request, branch, and commit creation. Group/project creation is absent. Distinguish
catalog presence from operation admission when diagnosing the installed deployment.

## Acceptance

An authorized caller can discover, describe, and invoke catalogued GitLab group and project
creation through `connectors`, create a private group (including a subgroup when requested) and
an empty project under it, and read both back through the existing operations, with the existing
credential, grant, approval, audit, and upstream-error handling enforced.

## Scope

- Cited: `providers/gitlab.toml` owns GitLab request, credential, and effect declarations;
  `catalog/gitlab.catalog.json` is generated and must be regenerated with its lock records.
- Cited: `ess/system/domains/catalog.yaml:119` already models Operation;
  `ess/system/domains/connection.yaml:101` models Connection; and
  `ess/system/domains/runtime.yaml:219` models InvokeOperation. Extend that existing surface.
- Inferred implementation work: apply the repository's source-provenance rules to the missing
  vendor endpoints, select their operations, and test request construction plus admission and
  failure behavior. No new domain entity or standalone provisioning client is needed.
- Existing credentials stay inside Connectors. A read-only credential must not acquire write
  authority. Creates are not silently retried after ambiguous upstream outcomes.
- Path collisions and upstream permission failures remain visible; an existing destination must
  not be adopted as the newly created result without an explicit verification step.
- Project transfer, deletion, archiving, member management, and general GitLab write coverage are
  outside this issue.

## Verification

Add meaningful coverage for group/subgroup creation, project namespace selection, empty-repository
initialization, explicit visibility, insufficient authority, name collision, and an ambiguous
create response. Verify both catalog selection and discoverability through the deployed CLI.
Run the catalog regeneration/check and the affected runtime gates; include one end-to-end
provisioning exercise against an explicitly designated GitLab test namespace.

## Vendor References

Official references inspected on 2026-09-05:

- https://docs.gitlab.com/api/groups/#create-a-group
- https://docs.gitlab.com/api/projects/#create-a-project

They document the group and project creation endpoints. Their request fields, authority
requirements, and returned identities must be captured through the repository's provenance
workflow during implementation, rather than inferred from the reproduction above.
