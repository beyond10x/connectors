---
format: aep.planning-md/3
id: approval-record:connectors-container-image-publication
kind: approval-record
status: approved
title: Publish the container image ghcr.io/beyond10x/connectors on each release tag (#238)
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-08T22:33:26Z", actor: "human:timo", revision: 2}
---
## Stop

Pull request #238 (`build/container-image`): each release tag `v<version>` also publishes the OCI
image `ghcr.io/beyond10x/connectors:<version>` for `linux/amd64` and `linux/arm64`. AGENTS.md
§ Releases says a container-registry publication needs its own requested scope; this record is
that scope.

## Decision

The operator, 2026-11-08, relayed by the coordinating session of the clean-main pass: option (a),
approve publishing, "nothing remains open because it publishes a container image".

## Scope

- Merge main into `build/container-image` (no rebase), resolve `CHANGELOG.md`, green CI, merge
  through the bot App.
- No release is cut with this merge. The first push to `ghcr.io/beyond10x/connectors` happens on
  the next release tag, through `release-binaries.yml`'s `image` job.
- After that first push a person makes the package public in the organization's package
  settings and checks that the package is linked to this repository.
