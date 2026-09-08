# Docker log scenarios — manual F12 audit

These ten cases apply the [native contract](../../semantics.md) manually.
They cover LD-B-02/03 and LD-A-03 without executing an inspect/logs request or
framing decoder. The earlier official Engine API archive remains unchanged.

| ID | Input or failure | Required observation |
|---|---|---|
| D01 | Short ID, name/path expression, empty/duplicate stream set or out-of-range seconds | InvalidInput before provider dispatch. Default until resolves once at whole-second precision. |
| D02 | Admitted exact-name inspect resolves full ID but label/name scope fails | Metadata may be retrieved under separate admission; Forbidden before logs; no inspect/env or foreign metadata disclosure. |
| D03 | Name resolves full ID, then alias is replaced | Logs targets the fixed full ID. Known replacement/revocation refuses; no new alias lookup or claim of atomic name/label membership. |
| D04 | TTY versus non-TTY target | TTY requires both streams and raw combined decoding. Non-TTY validates the eight-byte framing from inspect-selected mode, not Content-Type. |
| D05 | stdout hel, stderr error+LF, stdout lo+LF across frames | Emit stderr error then stdout hello; channel buffers never mix. |
| D06 | Two unterminated channel remainders at clean EOF | Emit nonempty remainders in last-byte order. Timestamp prefixes are removed only when valid/representable; null timestamps are not sorted. |
| D07 | Simultaneous native-tail/local-occurrence/text caps with valid pending payload prefixes | provider_limit,page_limit,source_bytes,line_bytes when all apply; complete:false, no cursor; unknown dropped occurrences under native/source saturation and zero fixed-container group loss. |
| D08 | Deliberate cutoff within validated frame payload versus truncated header/unexpected frame EOF | Valid cutoff may release scalar-aligned pending prefixes; truncated/invalid header or unexpected frame EOF is Unavailable. Never allocate from unchecked declared length. |
| D09 | Frame-count/framing/inspect/header/result ceiling, deadline, or changed authority | Budget overflow is Unavailable; expired deadline is Timeout; current authority refuses dispatch/disclosure. Inspect+GET share the original budgets and add no retry. |
| D10 | Clean EOF has fewer LF occurrences than tail, but driver tail counts messages/chunks without a verified mapping; alternatively a verified mapping establishes native exhaustion while decoded lines exceed the local cap | Missing proof is Unavailable; LF count alone proves no exhaustion. Valid multi-line messages are not protocol errors. When native exhaustion is established but the local cap omits lines, report page_limit and the exact observed dropped count, without provider_limit or cursor. |

Line clipping alone may coexist with complete:true and zero occurrence loss.
These observations do not establish an implemented decoder or live-daemon behavior.
In particular, the archived Engine reference alone does not prove the driver-tail
mapping needed by D10; that remains an explicit native binding prerequisite.
