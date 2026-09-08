# Kubernetes log scenarios — manual F12 audit

These eight cases apply the [native contract](../../semantics.md) manually.
They cover the native portions of LD-B-02/03 and LD-A-03; no pod reader or
permission/transport fixture was executed.

| ID | Input or failure | Required observation |
|---|---|---|
| K01 | Namespace already outside admitted allowlist | Forbidden with zero provider calls. |
| K02 | No fresh permission evidence for selected pod/container | At most one exact pods/log permission request, then one admitted GET; both consume the original 15 s/20 s budgets. |
| K03 | since_seconds=3600, tail_lines=200 | Provider-relative request with timestamps=true, follow=false, previous=false and combined output; no local absolute end, limitBytes or TLS bypass. |
| K04 | Timestamped line followed by untimestamped line and empty LF line | Preserve provider order and the empty occurrence. Only a valid representable timestamp prefix is removed; other timestamps are null without sorting. |
| K05 | Clean EOF with a nonempty unterminated remainder, below caps | Emit that remainder; complete may be true. A 20 KiB line clips to 8 KiB with line_bytes and can still be entry-complete. |
| K06 | Tail and byte ceilings coincide, final emitted prefix clipped | provider_limit,source_bytes,line_bytes in shared order; complete:false, no cursor, occurrences_dropped:null, stream_groups_dropped:0. |
| K07 | Source cutoff splits a UTF-8 scalar versus malformed earlier UTF-8 | Withhold only the incomplete trailing scalar at the deliberate cutoff; malformed earlier bytes refuse Unavailable. A timeout is Timeout, never a valid cutoff. |
| K08 | Transport/header/public-result overflow, unexpected interruption, revocation before disclosure | Overflow/interruption refuses Unavailable; current revocation refuses disclosure. No hidden retry or fabricated complete empty logs. |

The exact PodLogOptions rationale is retained in
[provider-sources.md](provider-sources.md). Native selectors/decoder/authority
remain unimplemented; shared ESS log facts do not execute these decisions.
