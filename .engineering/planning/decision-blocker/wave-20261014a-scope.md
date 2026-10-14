---
format: aep.planning-md/3
id: decision-blocker:wave-20261014a-scope
kind: decision-blocker
status: cleared
title: 'What wave 20261014a delivers: headless custody and incident-history reads'
relations:
- blocks: story:mounted-secret-custody
- blocks: story:grafana-irm-history-reads
- blocks: story:catalog-opsgenie-history-reads
- blocks: story:aep-pin-0713
- serves: vision:independent-contract-adapters
- blocks: story:windows-sidecar-retirement-admission
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T02:55:04Z", actor: "human:timo", revision: 2}
---
## Question

What wave 20261014a delivers after 0.50.0: headless credential custody, the two incident-history
read providers, and the planning tool pin.

## Options

| option | what | cost |
|---|---|---|
| A | Four units in one wave: story:mounted-secret-custody, story:grafana-irm-history-reads, story:catalog-opsgenie-history-reads (the `GenieKey ` prefix composed in the catalog, off connectors-host) and story:aep-pin-0713. One implementor per provider unit in its own tree, an adversary on mounted custody and a security review beside it; the pin is done on the integration branch. | Three trees of package gates, about 5-6G of target each (measured on the 0.50.0 wave's host and CLI packages: about 4.7G). |
| B | Mounted custody alone; the providers in the next wave. | One tree; the incident-history reads wait a day. |

Recommended: A. The provider units share no source file with mounted custody; the shared files
below are kept by the coordinator.

Default on silence: A.

## Decided

Option A, 2026-10-14: wave 20261014a delivers the four units on the integration branch
`wave/20261014a`, one pull request, then a release.

## Units

| unit | tree | branch | base | stage |
|---|---|---|---|---|
| story:mounted-secret-custody | conn-u-mounted | impl/mounted-secret-custody | cec189f27 | dispatched |
| story:grafana-irm-history-reads | conn-u-irm | impl/grafana-irm-history-reads | cec189f27 | dispatched |
| story:catalog-opsgenie-history-reads | conn-u-opsgenie | impl/catalog-opsgenie-history-reads | cec189f27 | dispatched |
| story:aep-pin-0713 | conn-w20261014a | wave/20261014a | cec189f27 | coordinator |

Each unit builds into its tree's own `target/`; scratch is the wave's scratch root, one directory
per unit.

## Shared files, kept by the coordinator at integration

- CHANGELOG.md
- crates/connectors-docs/src/status.rs and the generated status page and data
  (website/data/status.json, website/docs/status.md), regenerated once on the integrated head
- contracts/auth/capability/v1alpha1/semantics.md (both provider units add a paragraph)

Units write a patch for these files into their scratch directory instead of editing them.

## Commits this wave makes

One commit per unit (or a few on the unit's branch), the merges of each unit into
`wave/20261014a`, the pin commit, the closing planning-store commit, the pull request into `main`
and its merge, and the release commit and tag. Nothing else.

## Left out

- Jira writes: held while the operator's own Jira branches exist in this repository.
- story:state-directory-on-mounted-volume: depends on mounted custody.
- story:incident-call-lookup and story:incident-call-traces: drafts; the lookup's shape (one
  composite operation or two reads) is open.
- story:gmail-attachment-read-flaky-under-load: test evidence only, no code.
