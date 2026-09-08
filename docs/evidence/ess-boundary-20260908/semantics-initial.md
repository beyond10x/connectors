needs-revision

story:shared-ess-provider-boundary — ESB-S-01 (P2): The shared datasource domain still owns the selected native log scope and range input, so move LogScopeEquality, LogRangeSelection and its direction vocabulary into a Loki-owned model while retaining provider-independent collection/page facts in the shared root. — ess/domains/datasource_reads.yaml:10

LogScopeEquality is the exact label/value predicate used by query_scope.required_equalities and checked against the initial LogQL stream selector (contracts/datasources/logs/v1alpha1/semantics.md:102). LogRangeSelection combines the query, nanosecond interval, direction and provider collection cap of the selected logql-range binding (:33–39, :110); the pod and Docker profiles explicitly use different native selections (:43–57, :84). The generic names hide their present provider ownership. Preserve these typed values in an independently compiled Loki root, document the mapping into shared admission/collection/page facts, and add the identified native terminology to the gate policy so this exact regression is rejected.

story:shared-ess-provider-boundary — ESB-S-02 (P2): The discovery split preserves the closed recognition-provider vocabulary only in the Kubernetes model, leaving Grafana's selected recognition mapping without its own typed vocabulary, so add the Grafana-owned three-provider enum and document its projection into shared ResourceRecognition. — contracts/adapters/grafana/ess/domains/discovery.yaml:5

Before this change, shared RecognitionProvider provided the vocabulary used by ResourceRecognition for both declared discovery profiles. The shared field is now intentionally opaque, and the sole replacement RecognitionProvider belongs to connectors_kubernetes.discovery. The Grafana root models only its profile and collection partition kind, although the selected Grafana mapping is explicitly prometheus/loki/alertmanager with declared confidence (contracts/discovery/resources/v1alpha1/semantics.md:156). Preserve that closed set in the Grafana owner's independent model. Do not import the Kubernetes type or add those values back to the shared enum.

I reviewed all shared ESS domain files, the new adapter roots, the boundary policy and ownership documentation, and the relevant changed contract/adapter references against HEAD dd08cfd60aa43740da21946666dc0fa89c1160dc. The moved permission tuple, conditional Deployment preparation and Docker lifecycle intent retain their original coordinates and explicit UNMAPPED predicates. Shared private permission references are clearly distinguished from the unchanged public target tuple. Discovery partition metadata and provider identifiers remain adapter codec obligations, and the documents do not claim that ESS compilation proves runtime admission or public codec behavior. I found no additional boundary defect in the remaining shared auth, lifecycle, declaration, document-decision or observation values.

This is a read-only semantic review of the initial boundary draft, not approval of the unfinished datasource stories or an execution of the repository gate. I did not modify tracked files, run Python, call providers, mutate planning artifacts or read the other reviewer's report. Root is independently validating the gate and is responsible for preserving this initial report and obtaining review of the corrections.

```findings
- id: ESB-S-01
  artifact: story:shared-ess-provider-boundary
  file: ess/domains/datasource_reads.yaml
  line: 10
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "ESB-S-01 (P2): The shared datasource domain still owns the selected native log scope and range input, so move LogScopeEquality, LogRangeSelection and its direction vocabulary into a Loki-owned model while retaining provider-independent collection/page facts in the shared root."
- id: ESB-S-02
  artifact: story:shared-ess-provider-boundary
  file: contracts/adapters/grafana/ess/domains/discovery.yaml
  line: 5
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "ESB-S-02 (P2): The discovery split preserves the closed recognition-provider vocabulary only in the Kubernetes model, leaving Grafana's selected recognition mapping without its own typed vocabulary, so add the Grafana-owned three-provider enum and document its projection into shared ResourceRecognition."
```
