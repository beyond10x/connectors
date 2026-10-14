---
format: aep.planning-md/3
id: decision-blocker:mounted-secret-custody-trust
kind: decision-blocker
status: cleared
title: Nobody has decided which file owners, state directories and version pins mounted-secret custody admits
relations:
- blocks: story:mounted-secret-custody
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T20:03:39Z", actor: "human:timo", revision: 3}
---
## Question

Three design points of `story:mounted-secret-custody` are security trade-offs the approved custody design does not settle. Each is answered before the story leaves draft.


### Q1. Group-readable, root-owned items (the Kubernetes default for non-root pods).
A non-root pod can read a Secret only as `0440 root:<fsGroup>`. Admitting that widens readers to
every process in the pod holding `fsGroup`.

- (a) Admit group read only when `st_gid` is held by the process, the file is root-owned, there
  are no write or other bits, and the mount is read-only.
- (b) Strict owner-only (`0400`, owned by the effective UID). Kubernetes then needs an init
  container that copies into a memory volume, which today's ancestor rule also refuses, so (b)
  implies Q2(b).
- (c) Strict owner-only and require the job to run as UID 0.

*Recommendation: (a).* It is the platform's native delivery and adds no copy. The pod is the
trust boundary either way.

### Q2. Writable state under `readOnlyRootFilesystem: true`.
Today every ancestor of the state directory must be owned by root or the effective UID and not
group- or other-writable. An `emptyDir` or a PVC root with `fsGroup` fails that.

- (a) Keep the rule. The documented setup requires a writable container layer with an
  image-owned `0700` directory.
- (b) Admit a sticky or setgid mount-point ancestor when the final state directory is `0700`,
  owned by the effective UID and created by this process.
- (c) (a) now, (b) as a follow-up story.

*Recommendation: (c).* It ships the consumer's case without loosening filesystem admission in
the same change.

### Q3. How a mounted version is pinned (what the metadata store records about the bytes).

- (a) An HMAC-SHA-256 of the bytes under a random key held in an owner-only file in the state
  directory. This resists offline guessing of low-entropy passwords from a leaked database
  alone.
- (b) A plain SHA-256. It is simpler, but a leaked database allows offline guessing of a weak
  password.
- (c) No pin; always serve the current bytes. This breaks the immutable-version rule and uses
  rotated bytes under old evidence.

*Recommendation: (a).*

## Decision

Decided 2026-10-13, within the approved headless-custody design:

- **Q1 (a).** A secret file readable by group is admitted only when the group is one the process
  holds, the file is owned by root, it has no write bits and no other bits, and it sits on a
  read-only mount. This is how Kubernetes mounts a Secret for a non-root pod. Cost, stated in the
  design: every process in the pod that holds that group can read the file.
- **Q2 (c).** Today's ancestor rule stays: the documented container setup gives the state
  directory a writable image layer. Admitting a sticky or setgid mount ancestor when the final
  directory is `0700` and owned by the process is a separate story,
  `story:state-directory-on-mounted-volume`.
- **Q3 (a).** A mounted version is pinned as an HMAC-SHA-256 of its bytes under a random key held
  in an owner-only file in the state directory.
