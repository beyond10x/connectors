---
format: aep.planning-md/3
id: story:parity-aws-reads
kind: story
status: draft
title: AWS CloudWatch, logs, EC2, EKS, RDS and S3 reads
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

AWS reads: CloudWatch metrics and logs, EC2, EKS, RDS and S3 listings, so Connectors covers the fluxplane `aws` plugin (`docs/fluxplane-plugin-parity.md`, section `aws`).

## Operations

`aws.test`, `aws.inspect`, `aws.cloudwatch.metrics`, `aws.logs.groups`, `aws.logs.query`, `aws.logs.tail`, `aws.ec2.instances`, `aws.eks.clusters`, `aws.rds.instances`, `aws.s3.buckets`, `aws.s3.objects`

0 calls in Claude Code and Codex session transcripts from 2026-09-09 to 2026-10-08 (recount of
2026-10-08: `fluxplane-plugin operation invoke|call aws …` sites in Bash tool calls, each tool call
counted once).

## Surface

A new provider. Needs an AWS Signature Version 4 auth profile, which the shared credential contracts do not have; that profile is modelled first.

## Scheduling

Ordered last by call count. Not placed in a wave until a session uses the plugin or the operator
asks for it.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives.
- The parity page row of each operation moves to covered, with the Connectors operation named.
