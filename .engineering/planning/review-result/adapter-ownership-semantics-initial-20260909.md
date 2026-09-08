---
format: aep.planning-md/1
id: review-result:adapter-ownership-semantics-initial-20260909
kind: review-result
status: active
title: Initial native contract ownership and preservation review
relations:
- reviews: story:extractable-adapter-contract-ownership
revision: 1
---
needs-revision

story:extractable-adapter-contract-ownership — ACO-S-01 (P2): The shared compatibility matrix still normatively repeats native datasource payload and continuation rules, so retain generic wire support/refusal dispositions there and put native compatibility authority beside each adapter binding. — contracts/service/compatibility.md:144

Rows 144–146 still specify Confluence storage/Jira fields/null omission/CQL resume, Loki nanosecond selection/retained paging/saturation, and Prometheus result types/sample encoding/reserved profiles. The section calls itself a complete disposition matrix and says it covers the linked families' entire public shapes (:131–133); the new native documents in turn call service compatibility authoritative, including adapters/loki/contracts/logs/v1alpha1/semantics.md:164 and adapters/prometheus/contracts/series/v1alpha1/semantics.md:95. These are current normative requirements, not historical evidence or simple index links. An adapter-native schema/continuation change therefore still needs a second shared edit, contrary to adapters/README.md's declared ownership rule. Keep selected wire-version compatibility, explicit reader/profile support, limits and refusal obligations shared; link to the native owner for exact payloads and algorithms. Preserve the selected native semantics while removing their second authority.

story:extractable-adapter-contract-ownership — ACO-S-02 (P2): The media document was relocated into compositions while retaining native SIP and RTVBP authority, so move each independent adapter's binding, configuration and operation semantics into its own adapter directory and leave only pairings and integration behavior in the composition. — docs/compositions/media-session.md:70

This document explicitly names independent connectors.sip and connectors.rtvbp services, but still owns their contract/profile selection (:33–55), operation map (:59–68), selected sip.dial effect/ready-result table (:70–92), auth/configuration (:94–118), provider aperture/library choices and native realization obligations. Shared operations §4.1 still depends on its selected SIP dial binding (contracts/operations/v1alpha1/semantics.md:142), and shared sessions :87 repeats the selected dial interpretation. Moving the file from docs/adapters to docs/compositions did not give those native contracts an extractable owner. Keep shared sessions/media and genuinely shared protocol definitions at root; retain bridge supervision, concrete SIP/RTVBP pairing and equivalence tests in composition. Move the actual native bindings into specification-only adapters/sip and adapters/rtvbp directories without inventing runtime packages. Generic operation effect-versus-readiness rules can remain shared without making this composition the source of native success evidence.

story:extractable-adapter-contract-ownership — ACO-S-03 (P2): The SQL extraction moved the HTTP CA trust policy into the SQL adapter, so restore configured-CA replacement and empty-bundle refusal in the shared HTTP configuration contract and scope the SQL copy to SQL. — adapters/sql/contracts/reads/v1alpha1/semantics.md:25

The moved paragraph now says a configured CA bundle replaces public trust roots for SQL and HTTP providers and empty bundles are refused (:25–26). Before the split this lived in shared service v1alpha1 and applied to HTTP as well as SQL. The current shared HTTP paragraph now says only verified TLS by default and optional configured CA (contracts/service/v1alpha1/semantics.md:67); it does not select replacement rather than augmentation or preserve empty-bundle refusal. A search of current shared contracts and adapter contracts/designs found no other statement of those guarantees. An extracted HTTP adapter with only its declared shared dependencies should not need SQL's native read contract to learn which trust roots it uses. This is an introduced ownership/semantic-preservation regression, even though runtime code is unchanged.

I reviewed the current dirty shared datasource/discovery/auth-evidence/operation/service contracts and the relocated Loki, Kubernetes, Docker, Atlassian, Prometheus, Grafana and Alertmanager contracts/designs/models, with the adapter index, extraction rules, relevant prior ownership audit and current composition documents. The native LogQL scope/range/order/multiplicity/retention rules, pod and Docker decoding selections, Atlassian document/CQL binding, Kubernetes permission and conditional restart values, Docker lifecycle intent, and closed discovery vocabularies remain represented under their owning adapter. Monitoring child auth/configuration is now owned by each child; generic header placement is shared; parent recognition does not import child implementation. Atlassian intentionally remains a combined adapter with distinct Jira and Confluence bindings.

Exact upstream adoption, dependency packaging, Cargo workspace replacement and standalone builds are explicitly remaining prerequisites, so this review does not report them as falsely completed. It also does not claim the in-progress F12/F14 datasource semantics or their native authoring prerequisites are fixed. I found no defensible new loss of Grafana configured-source identity: the detailed old HTTPS/physical-cluster paragraph is Kubernetes-specific in context. Historical evidence and informative owner links were not treated as dependencies merely because they name an adapter.

This was a read-only semantic review apart from this separate local report. No tracked edits, planning mutations, Python, provider calls or runtime/gate execution were performed. I did not read the other reviewer's report. Root is updating mechanical links/planning and was notified of these findings during the audit; this report records the initial reviewed state, before review of those corrections.

```findings
- file: contracts/service/compatibility.md
  line: 144
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "ACO-S-01 (P2): The shared compatibility matrix still normatively repeats native datasource payload and continuation rules, so retain generic wire support/refusal dispositions there and put native compatibility authority beside each adapter binding."
- file: docs/compositions/media-session.md
  line: 70
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "ACO-S-02 (P2): The media document was relocated into compositions while retaining native SIP and RTVBP authority, so move each independent adapter's binding, configuration and operation semantics into its own adapter directory and leave only pairings and integration behavior in the composition."
- file: adapters/sql/contracts/reads/v1alpha1/semantics.md
  line: 25
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "ACO-S-03 (P2): The SQL extraction moved the HTTP CA trust policy into the SQL adapter, so restore configured-CA replacement and empty-bundle refusal in the shared HTTP configuration contract and scope the SQL copy to SQL."
```
