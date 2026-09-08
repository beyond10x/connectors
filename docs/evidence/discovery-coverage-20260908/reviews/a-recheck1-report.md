# Discovery coverage and host composition — reviewer A first recheck

**Verdict: needs revision. Residual findings: P0 0 / P1 0 / P2 3 / P3 0.** This is the intermediate normative review; the final typed/textual evidence packet is not yet ready. Initial findings DC-A-01–08 have substantive responses, with the remaining contradictions below. E07/E14/E32 remain explicitly outside closure.

Forty inputs from the revised working-tree packet were frozen before corrections under `sources/`, with SHA256 in `source-hashes.json`. Later working-tree edits are excluded. Planning snapshots are workflow observations, not a final lifecycle assertion. No other reviewer output was inspected. No source/planning mutation or runtime test was performed by this reviewer.

## DC-A-R2-01 — P2 — Downstream rules still prohibit the selected per-partition partial behavior

**Sources:** resources:109–115, versus auth/evidence:132, service compatibility:155, Kubernetes adapter:69; Grafana adapter:134.

The new discovery owner explicitly permits a complete comparable partition to confirm its own absent rows even when another partition makes the overall view incomplete. The unchanged auth/evidence wording prohibits withdrawing unseen resources from an incomplete observation, the compatibility summary says incomplete coverage cannot establish discovery withdrawal, and Kubernetes repeats the blanket prohibition. A complete a plus denied b scan therefore has inconsistent instructions about a's previously known absent object.

The new Grafana paragraph separately says cap/failure/filtered visibility retains only historical unknown evidence, while resources:112 permits trustworthy newly collected positives to be published as observed in a partial view. The adapter summary should not suppress that selected positive-evidence branch or imply stale history is fresh. The older resource conformance rename/type-change trace at :151 should also state the complete comparable scan prerequisite for the asserted withdrawal.

**Required correction:** align auth/evidence, compatibility and adapter summaries with the selected distinction: incomplete partitions provide no absence; an independently complete, comparable partition may prove absence only inside its exact scope, even in a partial view. Permission coverage by itself still proves no provider exhaustion. Say that retained unobserved rows are historical while trustworthy current positives may remain observed under the owner's rule. Keep F08 unknown-authorization pre-resource refusal intact and leave E07/E14/E32 open.

## DC-A-R2-02 — P2 — The merge rule can downgrade terminal withdrawn evidence to stale

**Sources:** resources:109–112,117,123,139–144.

The denied-subset row says to retain prior rows as stale, and the partial/failure row says to retain unobserved prior rows as stale. Those prior rows can already be withdrawn. Section 4.3 makes a positively withdrawn incarnation terminal and requires any later reappearance to receive a new id. The generic stale-retention rule can erase that stronger fact during an incomplete subsequent scan, or at least expose a misleading nonterminal classification.

**Required correction:** state merge precedence explicitly: a retained withdrawn incarnation keeps its withdrawn classification/terminal fact through later denied, capped, unavailable or not_scanned partitions; it is never demoted to stale or renewed into observed. Only bounded eviction removes its row, and lost verifiable continuity still prohibits reuse of its old id. Fresh positives for a reappearing incarnation receive the new id even if the retained old row remains visible. Add a trace for complete withdrawal followed by denied/failed scan and then same-UID reappearance.

## DC-A-R2-03 — P2 — Deadline-limited partial publication does not identify which deadline must remain current

**Sources:** resources:112,127,131,142; ess/domains/discovery.yaml:52–62; service compatibility's separate provider/execution ceilings.

The scan table promises a partial classified view when a deadline cap stops provider work. Publication captures the original deadline and requires rechecking the captured facts, while PublicationFacts contains deadline_current. Scan work names the original provider deadline. If that same expired deadline must remain current at publication, the promised deadline-capped partial publication is impossible; if a different execution/finalization deadline is intended, it is not identified. An implementer could instead silently extend the expired provider budget to finish/publish.

**Required correction:** distinguish the provider-work deadline from the outer execution/publication/response deadline. State exactly when provider sends stop, whether already trustworthy positives may publish while the outer deadline remains valid, and what happens once that final deadline expires. No extra provider send, hidden renewed budget or success after the selected outer boundary is permitted. Type/comment the publication deadline fact consistently and add both boundary traces. Alternatively select refusal rather than successful partial publication when the only supported deadline expires; the document must choose one coherent rule.

## Resolved initial requirements and retained boundaries

- DC-A-01: exact collection key, whole-selection equality, per-partition comparability and disclosure-scoped views now prevent narrowing/filtering from proving unrelated disappearance.
- DC-A-02: provider collection coverage, authorization coverage and public-page completion are separately described; object/request/byte/concurrency bounds and truthful final partial pages are selected. R2-01/R2-03 address remaining cross-document/deadline inconsistencies.
- DC-A-03: one logical host metadata owner, predecessor CAS, attempt/fence binding, acknowledged atomic view/index publication, ambiguity observation and non-reusable restart epochs are specified. No backend transaction is claimed implemented.
- DC-A-04: observation generation, private material generation and route evidence revision are separated. Revalidation is an explicitly admitted same-target host action; retained stale history cannot authorize it. List-services permission remains distinct from exact proxy permission. R2-02 preserves terminal facts during retention.
- DC-A-05: stable incarnation requires reviewed private identity/type/semantic-target equality; title-only rename differs from changed destination/port/authority boundary. Missing provider equality proof is an advertisement/validation gate, not inferred from UID/type.
- DC-A-06/07: composition.md names the concrete executable as the construction/injection owner, with a correct dependency/port diagram. The generic host accepts constructed ports without importing adapters. The profile requires one operating-system process, preserves standalone direct adapters, and refuses missing/cross-process/nested/unsupported routes without a new wire proxy or loader.
- DC-A-08: persistent observation/route/composition ownership remains explicitly unmodeled. Thirteen ESS value declarations record selected shapes without claiming executable reduction, atomicity, installed ports, new entities or a Connection.parent edge; F03's similarly named gateway value is correctly distinguished.

## Evidence status

The review checked normative descriptions, cross-family consistency, source ownership and the new ESS value declarations. It did not run ESS/runtime tests or inspect the other reviewer. The forthcoming final evidence must distinguish schema acceptance and authored expectations from actual provider exhaustion, cache/clock checks, metadata CAS/recovery, route equality, port injection or process behavior. No final approval is asserted before that packet and these corrections are independently reviewed.
