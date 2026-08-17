---
layout: home

hero:
  name: flux-connectors
  text: Governed connector operations
  tagline: Browse reviewed SaaS connector contracts and their safety metadata.
  image:
    src: /brand/mark.svg
    alt: ''
  actions:
    - theme: brand
      text: Browse the catalogue
      link: /explorer
    - theme: alt
      text: Current availability
      link: '#availability'

features:
  - title: One reviewed explorer
    details: Explore generated SaaS connector operations from the committed catalog.
  - title: Know what a call needs
    details: Every operation page shows its typed parameters, request path, credentials, destination hosts, and exact Flux source.
  - title: Limits are part of the contract
    details: Shared availability constraints and operation-specific issues are shown alongside the capability they affect.
---

<script setup>
import { data as catalog } from './data/catalog.data.mts'
</script>

## The Flux catalogue

The component publishes a growing connector catalog. Flux is one prospective client, not the owner
of this catalog or its generic contract.

<CatalogSnapshot :catalog="catalog" />

## What you can evaluate today

The catalogue is useful before live connector execution is enabled. You can inspect:

- a stable connector operation name and plain-language description;
- HTTP method, request path, typed parameters, and published schemas;
- risk and idempotency metadata for approval and retry decisions;
- required credentials and destination hosts;
- shared constraints plus any limitation specific to that operation.

Open the [connector explorer](explorer.md) to compare the current surface or deep-link directly to an
entry.

## Availability {#availability}

> [!CAUTION]
> **The catalogue is preview-only. No connector can make a live API call yet.** Secure credential
> application and tenant configuration still need host support. Do not treat the current modules as
> production-ready integrations.

Some operations also have narrower limitations, including query values that cannot yet be encoded
safely. Freshdesk currently has no credential configuration because publishing the apparent one
would put a secret outside Flux's protection. These conditions are shown on the affected connector
or operation page, where they matter.

The project fails closed: a capability is marked unavailable rather than presented as usable with
an unsafe or incomplete request.

## Follow the project

The source, release history, local build instructions, and contribution workflow live in the
[private monorepo](https://github.com/b10x/b10x) under `foundation/connectors`. The public site stays focused
on the connector catalogue and its user-facing contract.
