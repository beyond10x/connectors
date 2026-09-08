needs-revision

story:extractable-adapter-contract-ownership — ACO-S-04 (P2): The shared compatibility paragraph still owns Kubernetes's exact authorization target codec and all-namespaces Service selection, so delegate those native details to the Kubernetes auth binding while retaining generic coverage/wire compatibility rules. — contracts/service/compatibility.md:157

The datasource rows targeted by ACO-S-01 are corrected, but the following authorization-coverage paragraph still spells out target{verb,api_group,api_version,resource,namespace,name,subresource} and explicitly fixes all-namespaces Service selection to its F08 tuple. Its link points to shared evidence, which now deliberately delegates the target codec and namespace interpretation to the native Kubernetes binding. This is current normative compatibility prose, not a historical record or illustrative payload. It leaves another shared authority for a provider-owned schema, contradicting the extraction boundary. Keep authorization completeness, supported-reader requirements, error disposition and the distinction from provider exhaustion here; describe target as the selected adapter's declared closed codec, and link adapters/kubernetes/contracts/auth/v1alpha1/semantics.md for the seven-field tuple and namespace semantics. Preserve those concrete rules in the native owner.

The three initial corrections are otherwise present:

- ACO-S-01's datasource rows now define shared wire support/refusal dispositions and explicitly defer native schemas/algorithms to the adapter links (contracts/service/compatibility.md:133–148).
- ACO-S-02's native SIP and RTVBP authority now resides under adapters/sip and adapters/rtvbp. The SIP operation map, selected dial-effect/ready-result table, trunk/auth/configuration, aperture and native media rules are preserved; RTVBP owns its exact profile, authority/upgrade, queue and native configuration obligations. Media composition retains pairing, bridge/device supervision and integration checks, and shared operations points to the native SIP binding.
- ACO-S-03 is resolved: contracts/service/v1alpha1/semantics.md:67–68 restores HTTP configured-CA replacement and empty-bundle refusal, while adapters/sql/contracts/reads/v1alpha1/semantics.md:25–26 is SQL-only.

I found no additional introduced loss in the corrected SIP/RTVBP split or a newly lost Grafana identity guarantee. Existing native F12/F14 semantics, executable provider proofs, exact-source adoption, shared-package selection and standalone builds remain outside this ownership approval. The documents continue to state those prerequisites without claiming a runtime implementation or extracted build.

Reviewed the frozen docs/evidence/adapter-ownership-20260909/reviewed-sources.tar.gz with independently verified SHA-256 0422c7d2a8038bfbe09079a2b728e237e68631441ceee8ddb22098b99e4ad049. Extracted only into the ignored local review directory and verified every file against the adjacent source-hashes.txt with no mismatch. Findings and citations refer to that frozen content, not later working-tree edits. This was a read-only semantic recheck apart from local review artifacts; no tracked edits, planning mutation, Python, provider calls or runtime/gate execution. I did not read the other reviewer's report.

```findings
- file: contracts/service/compatibility.md
  line: 157
  category: semantics
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "ACO-S-04 (P2): The shared compatibility paragraph still owns Kubernetes's exact authorization target codec and all-namespaces Service selection, so delegate those native details to the Kubernetes auth binding while retaining generic coverage/wire compatibility rules."
```
