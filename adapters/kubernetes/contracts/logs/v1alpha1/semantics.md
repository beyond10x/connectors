# Kubernetes pod logs/v1alpha1

**Status:** proposed native binding, not implemented. Implements the shared
[datasource.logs/v1alpha1](../../../../../contracts/datasources/logs/v1alpha1/semantics.md)
obligations as profile `kubernetes-pod-logs`.

## Selection

```json
{ "namespace": "x", "pod": "api-0", "container": "api", "since_seconds": 3600, "tail_lines": 200, "max_bytes": 131072 }
```

Namespace/pod and optional container are validated literal Kubernetes path components. Defaults: since_seconds=86,400, tail_lines=200, max_bytes=131,072. Integer bounds: 1–86,400, 1–1,000 and 1–131,072 respectively. An omitted container uses the provider's single-container default; ambiguity is a safe failure, never fan-out.


Input is closed: no cursor, follow, previous-container, TLS override, absolute
upper endpoint or arbitrary provider options. The profile does not promise an
absolute time interval or archival continuity.

## Admission and provider trace

Pod admission checks configured namespace and the exact `(get,"","v1","pods",namespace,pod,"log")` target under the [Kubernetes permission binding](../../auth/v1alpha1/semantics.md). Make one finite log GET with follow=false, previous=false, timestamps=true, selected container/sinceSeconds/tailLines and combined output. Omit provider limitBytes: the pinned API says it may return slightly fewer/more bytes and split the last line, so it does not prove exact exhaustion. Enforce max_bytes while consuming instead. Never send insecureSkipTLSVerifyBackend. SinceSeconds is relative to the provider's request-time clock and available pod history; no upper endpoint or portable exact absolute interval is promised. Do not invent a local absolute window or sort null-timestamp lines into one.


## Lines and result

Selection is `{kind:kubernetes-relative-tail,since_seconds,tail_lines}`, with
`order:provider`. Each line's stream is `{namespace,pod,container}` (container is
null when omitted); `source` is null because provider output is combined.
Provenance identifies the fixed admitted source and pod; source_revision is null.

Use one text buffer. Split on LF, preserve empty occurrences and emit completed
lines in arrival order. At clean EOF emit a nonempty unterminated remainder.
Remove a valid representable RFC3339/RFC3339Nano prefix and its separating space;
otherwise preserve the text and use timestamp_unix_ns:null. Do not sort timestamps
or invent precision. Keep the first tail_lines occurrences and at most 8 KiB per
line after admitted optional redaction, with independent redacted/line_truncated
flags. Enforce the unredacted source byte limit while consuming.

At a deliberate receiver byte cutoff, retain a valid UTF-8 prefix of the pending
remainder, mark it truncated and report incomplete. Withhold an incomplete trailing
scalar at that cutoff; malformed earlier bytes fail. An unexpected interruption
does not establish EOF. No cutoff manufactures exhaustion.

## Completeness, bounds and errors

Return next_cursor:null. Clean full EOF below the requested tail and receiver byte
caps, without provider partial indication or local omission, establishes exhaustion
of the selected available history. Reaching either cap remains conservatively
partial. Line clipping is distinct from occurrence omission and can coexist with
complete:true; use the shared cause/count rules.

The source text ceiling is max_bytes <=128 KiB; independently bound both encoded
and decoded transport bytes to 256 KiB, headers to 16 KiB and the full public JSON
response to 4 MiB. One exact-target permission query is permitted when fresh
evidence is absent, followed by at most one log GET. Both consume the same 15 s
provider / 20 s outer deadline, including connects <=5 s. No activation/identity
probe, refresh, extra retry or live follow is silently added.

Safe base errors apply. Deadline exhaustion is Timeout, never clean EOF or cutoff
success. Invalid or interrupted source observations are Unavailable except for the
explicit valid receiver cutoff. Current binding/revocation fences govern dispatch
and disclosure; no raw provider errors or private data enter diagnostics.

## Evidence and obligations

The predecessor `kubernetes.pod.logs` selectors and 128 KiB bound are recorded at
`../connectors/crates/integration-kubernetes/src/workloads.rs:1120-1145`.
The pinned v1.35.0 PodLogOptions source and limits are in the
[provider evidence](../../../../../docs/evidence/datasource-semantics-20260908/provider-evidence.md).
The old text blob is deliberately projected into lines; a compatibility facade may
join them, but that is an explicit adapter binding.

Required fixtures: out-of-allowlist namespace makes zero requests; exact pods/log
permission precedes GET; timestamped and untimestamped lines preserve order;
empty lines, unterminated EOF, split UTF-8, tail/byte caps, timeout and current
revocation remain distinct. These are specification scenarios, not test results.
Native selectors, decoding and containment remain unimplemented binding obligations.
