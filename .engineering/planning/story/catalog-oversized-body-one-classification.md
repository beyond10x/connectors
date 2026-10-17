---
format: aep.planning-md/3
id: story:catalog-oversized-body-one-classification
kind: story
status: draft
title: An over-bound catalog response body has one classification in contract, model and transport
revision: 1
---
## Problem

`contracts/catalog/v1alpha1/semantics.md` §5–6 classifies a provider response body over the response bound as `UpstreamProtocol`, while the catalog transport and the shared model refuse it as `capacity` (the GitLab `file.raw` read shipped in 0.53.0 follows the transport: a file over the 4 MiB bound is refused `capacity`). The contract and the implementation disagree.

## Acceptance

- The contract, the shared ESS model and the transport name one classification for an over-bound response body, and a conformance scenario holds it for a catalog read.
