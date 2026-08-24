---
id: S-066
title: "The transport refusal names its class"
pillar: Platform
status: ready
design: ../design/15-a-zero-configuration-endpoint-plane.md
epic: endpoint-plane
areas: [integrations]
---

# The transport refusal names its class

## Goal

S-065's live diagnosis (rev 16) pinned `grafana_dashboards_list` to an upstream-transport
failure that is specific to that operation's resolved request — `grafana-datasources-list`
succeeds on the same direct route, credential and origin every reconcile tick. The final
discriminator (timeout vs connect vs TLS vs body-read) is discarded at the transport mapping
(`integration-monitoring`, the `.map_err(|_| UpstreamFailure::Transport)` sites), so the last
step of the diagnosis is blind.

## Acceptance

- The transport arm of the refusal log carries an error class derived from the transport
  error (timeout / connect / tls / body-read / other) — never the error's Display string,
  which can embed the URL.
- With the class visible, the live `grafana_dashboards_list` failure is diagnosed on dev and
  fixed here if it is a document/resolution defect (e.g. a response-size or timeout bound the
  dashboards list trips), or handed to provisioning with the concrete evidence recorded.
- The alertmanager 403 stays with provisioning (S-065 Progress) — out of scope here.
