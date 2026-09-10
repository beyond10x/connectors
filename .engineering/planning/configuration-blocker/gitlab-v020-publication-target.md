---
format: aep.planning-md/1
id: configuration-blocker:gitlab-v020-publication-target
kind: configuration-blocker
status: open
title: The GitLab v0.2.0 source publication destination is missing
relations:
- blocks: release-plan:gitlab-v020
revision: 1
---
## Missing input

The operator authorized cutting source release v0.2.0, including commit, annotated tag and push, but this checkout has no publication remote. Its only configured remote, ess-recovery, is a local recovery repository and does not establish public distribution. The destination URL has been requested in the side conversation and has not been supplied.

## Clears this blocker

Supply the intended source repository URL, or an explicit instruction that the existing local recovery repository is the intended final destination. Inspect its branch/tag state and use the already-authorized bot publication path without overwriting unrelated history or existing tags. This is missing destination information, not a request to reapprove the release.

## Independent work

Complete release metadata, verification, local commit/tag and recovery publication. Preserve the active primary checkout; its later clock work is outside this release. The release stays incomplete until its intended destination is established and the required remote references are verified.
