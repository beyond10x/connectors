# Historical secret-scan baseline

Gitleaks 8.30.1 reports 69 findings represented by 68 exact fingerprints in the repository's
pre-hardening history. The current tree has no finding. Review classified the historical material
as imported examples, generated connector descriptors, vendored research, or a conformance-test
sentinel; none is runtime credential material issued to B10x.

| Historical source | Findings | Disposition |
| --- | ---: | --- |
| `connectors.lock` across commits `82f6a80a2741`, `c585c1ccb3ac`, `748b50dad534`, and `455989616610` | 54 | Generated/imported connector examples; the current generated lock is scrubbed. |
| Vendored `docs/research/vendor/nango-providers.yaml` at `a0873885d0b1` | 11 (10 fingerprints) | Upstream provider templates/examples retained only in history. |
| Imported Zendesk OpenAPI descriptions at `82f6a80a2741` | 2 | Documentation examples, absent from the current descriptions. |
| Connector specification conformance fixture at `c78434fe6a31` | 1 | Deliberate non-live test sentinel, absent from the current fixture. |
| Browser completion rejection fixture at `93672c5e9d` | 1 | Deliberate opaque capability-shaped URL used only to prove malformed fragment refusal; not an issued credential. |

The root `.gitleaksignore` names only those exact commit/path/rule/line fingerprints. It does not
disable a detector or exempt a current path, so any new match still fails the security workflow.
The workflow scans complete Git history with redaction enabled and a checksum-pinned scanner.

If provenance changes or any material is later shown to have been live, remove its fingerprint,
rotate or revoke it at the issuing system, purge it under the incident process if required, and
record the incident separately. Never add a broad rule or path allowlist to make this gate pass.
