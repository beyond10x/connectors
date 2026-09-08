# Monitoring tenant-header review disposition

Source: E30, owned by `story:contracts-tenant-header`. Checked on 2026-09-08 against opening commit `6c8ecc2` plus this specification-only diff.

The normative proposed direct representation is `http.extra_headers`, with a receiver-owned tenant header entry. The logs contract and Grafana child example now agree. The former `tenant_header` label is explicitly descriptive, not another accepted key or a claim about an existing legacy file. Current `crates/connectors-host/src/http.rs::HttpConfig` has no extra-header implementation; this remains a future configuration binding.

Textual scenario audit:

| Case | Required result |
|---|---|
| Read direct Loki rules and monitoring child example | Both name `http.extra_headers`; no undocumented alias/mapping |
| Caller supplies a tenant selector or differently cased header | Cannot change admitted provider tenant; closed request/binding refuses unsupported input |
| Credential/transport header collision or case-duplicate configuration | Configuration refused before dispatch |
| Tenant configuration changes | New admitted revision; no cached page/continuation crosses it |
| Grafana-mediated child asks to override tenant | Parent route/datasource retains ownership; unsupported requested binding refused, no direct fallback |

Validation: read both complete normative passages and current `HttpConfig`; checked name/ownership/boundary text, current implementation qualification and all relative links. Result: pass, zero broken links. No HTTP fixture was executed, no header/configuration schema or runtime implementation changed. This correction names an existing proposed configuration value, not a new entity; it introduces no ESS relation/cardinality or implementation decomposition.
