# Independent namespace-reference check

Reviewed the official [Kubernetes ResourceAttributes reference](https://kubernetes.io/docs/reference/kubernetes-api/definitions/resource-attributes-v1-authorization/) on 2026-09-08 using the web reader. The final packet already recorded the Connectors documentation-lookup capability gap; this check used that same official fallback source. No live cluster or provider API was contacted.

The namespace description states: `"" (empty) means "all" for namespace scoped resources from a SubjectAccessReview or SelfSubjectAccessReview`.

This supports the corrected distinction between an all-namespaces Service SSAR and a genuinely cluster-scoped resource; resource/verb and the complete private binding still determine the exact target. It supplies no permission, live API compatibility, selector normalization or provider completeness proof.

The old design was independently read from exact Git object 81459ac42ddd518d3942f4b079841e9e0ed6efc8, docs/design/10-local-kubernetes-context-and-resource-discovery.md lines73–79. That historical source already selects an empty configured namespace list only with an allowed cluster-wide review. Its exact bytes are preserved as auxiliary/old-design10.md and auxiliary-hashes.json. The note is a review record, not a byte-for-byte snapshot of the web page.
