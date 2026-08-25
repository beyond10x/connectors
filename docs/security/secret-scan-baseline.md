# Historical secret-scan baseline

Gitleaks 8.30.1 reports 91 findings represented by 90 exact fingerprints in the repository's
history. The current tree carries no credential material. Review classified the historical material
as imported examples, generated connector descriptors, vendored research, or a conformance-test
sentinel; none is runtime credential material issued to b10x.

| Historical source | Findings | Disposition |
| --- | ---: | --- |
| `connectors.lock` across commits `82f6a80a2741`, `c585c1ccb3ac`, `748b50dad534`, and `455989616610` | 54 | Generated/imported connector examples; the current generated lock is scrubbed. |
| `foundation/connectors/connectors.lock` at monorepo layer commit `ba0cc3a74d3c` | 4 | Deterministic SHA-256 artifact digests changed by the canonical schema path; none is credential material. |
| Vendored `docs/research/vendor/nango-providers.yaml` at `a0873885d0b1` | 11 (10 fingerprints) | Upstream provider templates/examples retained only in history. |
| Imported Zendesk OpenAPI descriptions at `82f6a80a2741` | 2 | Documentation examples, absent from the current descriptions. |
| Connector specification conformance fixture at `c78434fe6a31` | 1 | Deliberate non-live test sentinel, absent from the current fixture. |
| Browser completion rejection fixture at `93672c5e9d` | 1 | Deliberate opaque capability-shaped URL used only to prove malformed fragment refusal; not an issued credential. |
| `connectors.lock` across commits `4520cf47f7c6`, `cc6c7d97d559`, `b56d12b5086f`, `fe6233da32d6`, and `181c144775f8` | 18 | Deterministic SHA-256 artifact digests on a line whose key names the vendor, so `bitbucket`/`discord`/`newrelic`/`sentry` beside 64 hex characters matches a vendor-token rule. Verified by recomputation: `sha256sum catalog/<provider>.catalog.json` equals each flagged value. |

Regenerating `connectors.lock` moves its line numbers, so a rebuild produces new fingerprints for
the same reviewed material rather than reusing the old ones. They accumulate per commit. That is
the cost of fingerprint scoping and it is the right cost: a rule or path allowlist would have made
this gate quiet about a real secret landing in the same file.

The combined component and migration `.gitleaksignore` files name only those exact
commit/path/rule/line fingerprints. They do not disable a detector or exempt a current path, so any
new match still fails the local security gate. The gate scans complete Git history with redaction
enabled and a checksum-pinned scanner.

If provenance changes or any material is later shown to have been live, remove its fingerprint,
rotate or revoke it at the issuing system, purge it under the incident process if required, and
record the incident separately. Never add a broad rule or path allowlist to make this gate pass.
