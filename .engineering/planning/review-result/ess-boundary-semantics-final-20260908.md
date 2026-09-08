---
format: aep.planning-md/1
id: review-result:ess-boundary-semantics-final-20260908
kind: review-result
status: active
title: Shared ESS boundary independent semantics review, final approval
relations:
- reviews: story:shared-ess-provider-boundary
revision: 1
---
approve

I independently rechecked both corrections and their current ownership/mapping references. No remaining findings in this boundary review.

ESB-S-01 is resolved. The Loki-owned model at contracts/adapters/loki/ess/domains/reads.yaml:6 preserves LogDirection, LogScopeEquality and LogRangeSelection with their original fields and explicit native parsing/admission obligations. Its manifest declares only connectors_loki.reads. The shared datasource domain now retains collection/page facts and decisions without these native selection types; contracts/datasources/logs/v1alpha1/semantics.md:203 identifies the owning model and explicitly distinguishes the pod/container selections. contracts/ess-boundary.json includes LogRangeSelection and LogScopeEquality as native terms. No stale shared-qualified references to the moved types remain in current ESS, contract or design sources outside historical evidence.

ESB-S-02 is resolved. contracts/adapters/grafana/ess/domains/discovery.yaml:12 now declares its own RecognitionProvider with exactly prometheus, loki and alertmanager, consistent with the selected mapping in contracts/discovery/resources/v1alpha1/semantics.md:156. The model records declared recognition and null recognition/candidate for unknown plugins without importing the Kubernetes vocabulary. Shared ResourceRecognition continues to carry an opaque provider identifier.

The adapter index, logs ownership section and records ownership section consistently distinguish adapter-owned native vocabulary from shared facts/references. The shared root does not import the adapter roots. Existing permission-target, discovery-partition and mutation-preparation splits continue to preserve their typed provider coordinates and explicit UNMAPPED obligations. These documents do not claim that ESS shape validation implements a provider parser, runtime admission, public codec or cache.

This approval covers story:shared-ess-provider-boundary's semantic correction only. It does not approve the unfinished datasource stories. This recheck was read-only apart from this separate local report; the initial report remains unchanged. Root reported that the full Rust gate with --msrv passed; I did not independently rerun that gate or inspect another reviewer's report.

```findings
[]
```
