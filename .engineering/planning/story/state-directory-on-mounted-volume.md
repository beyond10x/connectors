---
format: aep.planning-md/3
id: story:state-directory-on-mounted-volume
kind: story
status: draft
title: A state directory under a setgid or sticky mounted volume, for read-only root filesystems
relations:
- depends_on: story:mounted-secret-custody
- serves: vision:independent-contract-adapters
revision: 1
---
## Problem

A container with `readOnlyRootFilesystem: true` has writable storage only on mounted volumes: an
`emptyDir` or a persistent volume. Their root directory is group-writable (Kubernetes applies
`fsGroup` with the setgid bit), and today every ancestor of the state directory must be owned by
root or the effective user and not group- or other-writable, so such a volume is refused as a
state directory's ancestor. `story:mounted-secret-custody` keeps that rule and documents a
writable image layer instead.

## Outcome

A state directory may sit under a mount-point ancestor that is sticky or setgid and
group-writable, when the final state directory is `0700`, owned by the effective user and created
by this process. Every other ancestor rule is unchanged.

## Acceptance

- A state directory under a setgid, group-writable `emptyDir`-like ancestor, itself `0700` and
  owned by the process, opens; the same layout with a final directory that is group-writable or
  owned by another user is refused with the existing code.
- The container setup guide shows the `readOnlyRootFilesystem: true` variant, with the commands
  run.

## Not in scope

Any change to how credentials are admitted (`story:mounted-secret-custody`).
