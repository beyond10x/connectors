# Explicit private bounded reads

`connectors-private/3` selects a read-only native execution binding. Its ready
handshake echoes that exact selection and the existing closed Bootstrap shape,
containing only read requirements. An executable must opt in explicitly before
the owner transfers any credential. There is no negotiation, fallback or implicit
upgrade from private/1 or private/2. Those codecs retain their meaning.

The inherited authenticated owner/child channel, executable pin, incarnation,
framing, secret custody and supervised process lifecycle are unchanged. Version 3
retains legacy validate and stop controls, but refuses legacy invoke and every
write control. This selection supplies no extended mutation support.

The closed `invoke_bounded` control contains `request_id`, `operation`, `revision`,
`partition` and `budget`. The request id is a canonical nonnil UUID. `budget` is
the `connectors.transport.PrivateReadBudget` value: `deadline_ticks` (original
Linux CLOCK_MONOTONIC nanoseconds), `execution_ms`, `provider_ms`, `input_bytes`,
`result_bytes`. All are positive integers. Maximums are 40,000 ms execution,
30,000 ms provider, 262,144 input bytes and 4,194,304 result bytes; provider time
cannot exceed execution time. Unknown fields, invalid values and unsupported
selections refuse, never silently clip. The parent establishes the original
deadline before owner admission; passing or reconstructing the value in another
thread or process never starts a new execution interval. A future deadline more
than its declared execution interval away is invalid. All processes are in the
same boot and time namespace; there is no public or remote deadline authority.

These private byte allowances bound the payload. The public receiver must budget
the complete service request/result envelope separately before advertising its
limits. Private/3 support alone does not establish conformance to that binding,
curated semantics, approval behavior or current connection/operation admission.

The parent checks the selection and input before writing any secret. The child
checks read effect, descriptor revision, selectors, limits and input schema before
native invocation. Native composition receives the unchanged original budget.
It establishes one provider cutoff at the start of its provider sequence, bounded
by the remaining execution deadline, and passes that same cutoff through token
exchange, credential resolution and all provider requests. The connect timeout
is at most five seconds inside the remaining provider interval. No retry or
refresh behavior is added. Result validation, encoding and both channel directions
remain inside the original execution deadline. Oversized results refuse without
truncation. An in-flight timeout or broken exchange terminates and reaps the exact
child; a deadline already expired before transfer refuses without provider work.

The catalog executable opts into this read binding and shares one selected HTTP
cutoff between its OAuth token client and business client. Other executables
remain unsupported until they implement the same obligations. Production MCP
serving and owner selection of this binding are separate integration work.
