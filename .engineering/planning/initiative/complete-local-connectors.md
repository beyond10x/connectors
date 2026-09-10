---
format: aep.planning-md/1
id: initiative:complete-local-connectors
kind: initiative
status: active
title: Complete the local Connectors CLI with reusable MCP support
relations:
- informed_by: specification:core-model-closure-20260909
- informed_by: specification:recent-agent-adapter-usage-20260909
- informed_by: epic:mcp-contracts
- informed_by: story:kubernetes-spec-service
- serves: vision:independent-contract-adapters
revision: 3
---
## Outcome and authority

Implement the operator-approved local Connectors product through seven successive working runtime milestones. Connectors owns product authority; ../mcp owns reusable protocol mechanics. This record carries the implementation plan supplied on 2026-09-10. It does not close the completed specification milestones or treat them as runtime proof.

Linux x86_64 and Rust 1.88 are the initial platform. SQLite owns non-secret metadata, with versioned migrations, WAL and full synchronization. The admitted OS Secret Service collection owns credentials. Existing installed Connectors configuration and credentials are not automatically migrated. Local native credential entry is protected; native SaaS OAuth onboarding is excluded. MCP browser OAuth and refresh are included.

## Delivery sequence

1. Persistent GitLab: fresh setup, protected entry, saved connection, existing read, CLI and owner restart, reuse without re-entry. story:persistent-gitlab-journey owns this first delivery.
2. Complete local management across GitLab, Kubernetes and SQL, including repair/revoke races, exact process ownership and durable stop suppression. Integrate story:kubernetes-spec-service after the first persistent journey.
3. Expand ../mcp with resources, prompts, servers, progress/cancellation, consumer-owned stdio and explicit OAuth one-use exchange/publication seams. Adopt AEP there, preserve its primary documentation workflow edit, verify and publish source commits through verified Atlas bot authority.
4. Pin that exact published MCP revision here and implement outbound and inbound tools/resources/prompts over stdio and Streamable HTTP, with version-specific 2026-07-28 and explicit 2025-11-25 interoperability, caller isolation and authenticated local verification of the cloud-capable server profile.
5. Complete engineering reads: GitLab CI/MRs, PostgreSQL/MySQL, Jira JQL, Slack threads, Confluence collection, Loki/Prometheus/Grafana and Kubernetes diagnosis.
6. Complete approval/audit/attempt/dispatch/idempotency handling, selected SaaS writes, Kubernetes changes/exec/copy/tunnels, Docker workflows and recorded Helm history/rollback. SQL remains read-only.
7. Prove the full incident journey, sandbox cleanup, Rust 1.88 and existing tools-only MCP/Harness compatibility, deterministic generation and two isolated reproducible distributable builds; update support documentation.

Acceptance is docs/recent-adapter-usage-20260909.md C01-C17 and C20-C22 plus the supplied MCP interoperability and deterministic failure cases. Every selected workflow requires runtime evidence. Fixtures do not satisfy missing dedicated-provider sandbox evidence.

## Specification ownership and dependencies

contracts/cli/v1alpha1/semantics.md and ess/domains/cli.yaml own the existing local surface. docs/design.md section 31 inventories distinct logical metadata owners and atomic groups. ess/domains/auth_bindings.yaml, credentials.yaml, credential_evidence.yaml, declarations.yaml and connection_admission.yaml supply existing typed homes; native profiles belong to adapters. The implementation must model and review missing semantics before decomposing them. Later milestones remain ordered outcomes here until their contractual gaps are resolved, not invented runtime stories.

The existing epic:mcp-contracts remains the Connectors MCP contract owner. The Kubernetes generation story and tooling-blocker:kubernetes-driver-protocol-loading retain their existing meaning; paid driver runs are not required. Existing specification milestones remain historical evidence.

## Verification and publication

Use task-owned TMPDIR under .local/tmp and two Cargo jobs. Run the affected ESS validation, generation/drift/conformance, website checks where affected, and the required Connectors gate with MSRV. MCP uses cargo xtask gate; recheck remote policy before publishing already locally verified source. Keep timestamped receipts separate from deterministic payloads. Reuse evidence only when its relevant inputs are unchanged.

Connectors publication and cloud deployment are excluded, as are GitHub/AWS adapters, optional MCP extensions, Atlas registration/delivery wiring and unrelated Harness repinning. Publish only verified MCP source commits and consume their exact revision. Keep Connectors local. No new bare recovery repository. Follow the explicit single-agent direct-checkout rule here; use managed worktrees and leases for MCP. Final integration requires clean Connectors main, published MCP goal commits and no task-owned linked worktrees.

## Current evidence and open prerequisites

Initial Connectors main is clean. Production apps/connectors/src/main.rs has only compatibility describe/invoke/serve; the grouped parser in apps/connectors-cli-contract is generated and its standalone default handler is unavailable. No persistent metadata authority or production Secret Service binding exists in this checkout. The installed 0.7.0 CLI is a separate product baseline and is not migrated.

Dedicated GitLab sandbox target and protected credential-file path have been requested; no secret is requested in conversation. The other provider sandboxes, Secret Service durability qualification and independent MCP interoperability remain required evidence. They are not assumed from installed credential presence. This is an interactive run with no approval bypass records.
