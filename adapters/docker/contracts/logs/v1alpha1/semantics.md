# Docker container logs/v1alpha1

**Status:** proposed native binding, not implemented. Implements the shared
[datasource.logs/v1alpha1](../../../../../contracts/datasources/logs/v1alpha1/semantics.md)
obligations as profile `docker-container-logs`.

## Selection

```json
{ "target": {"container_name":"/api"}, "streams": ["stdout", "stderr"], "since_unix_s": 1788822000, "until_unix_s": 1788825600, "tail_lines": 200, "max_bytes": 131072 }
```

Target is exactly `{container_id:<64 lowercase hex characters>}` or `{container_name:<exact canonical name beginning with />}`; short IDs and path/query expressions are invalid. Streams is a nonempty duplicate-free stdout/stderr subset, default both. Tail defaults to 200 (integer 1–1,000); max_bytes defaults to 131,072 (integer 1–131,072). Since is required; until defaults once to the receiver's current whole epoch second. Times are nonnegative integers convertible to signed-64-bit nanoseconds, with since < until and difference <=86,400 seconds. A TTY target requires both streams because its provider output is combined. All profile inputs are closed; no follow, previous-container, TLS override or arbitrary provider options are accepted.


## Admission and provider trace

Docker first admits one bounded metadata inspect against the configured daemon and exact requested ID/name. It fixes the returned full ID, validates the exact name when requested, configured ID/name/label scope, supported logging driver and TTY mode. Candidate metadata lookup is separately admitted and may inspect an object that does not qualify for log retrieval; no log body or raw inspect/environment result is disclosed. Outside-scope candidates are Forbidden before the log GET. The subsequent GET uses the resolved full ID, never an alias; known replacement/revocation before dispatch or disclosure refuses the result. Name/label membership is the inspect observation, not an atomic guarantee against concurrent renaming/relabelling.

Docker sends timestamps=true, follow=false, finite tail, selected stdout/stderr and integer since/until. The v1.56 endpoint promises native since/before-until selection; the profile exposes that integer-second selector, without a nanosecond selection or continuation promise. Returned timestamp precision does not strengthen selection precision. Non-TTY output uses the documented eight-byte multiplex framing with validated stream type/reserved bytes/length; TTY output is raw combined text. Inspect selects the decoder: the logs endpoint does not set Content-Type. Unsupported driver/framing is Unavailable, not empty success.


## Lines and result

Selection is `{kind:docker-seconds-tail,since_unix_s,until_unix_s,tail_lines,streams}`,
with `order:provider`. Each line's stream is `{container_id}` using the resolved
full ID. Source is stdout/stderr for non-TTY framing and null for TTY combined
output. Provenance identifies the fixed admitted daemon/container; source_revision
is null.

Frame boundaries are not line boundaries. TTY uses one text buffer; non-TTY output keeps a separate buffer for each selected stdout/stderr channel, so interleaved frames never concatenate text from different streams. Split on LF and preserve empty occurrences. Emit a completed line when its LF arrives; this line-completion order is the selected provider order, not reconstructed wall-clock order. At clean EOF, emit each nonempty unterminated remainder in order of its last received byte position. For example, stdout `hel`, stderr `error\n`, stdout `lo\n` yields stderr `error` then stdout `hello`, never a mixed line.

Remove a valid representable RFC3339/RFC3339Nano prefix and its separating space; otherwise preserve the text and report a null timestamp. Keep the first tail_lines observed occurrences and at most 8 KiB per line after optional redaction, while enforcing the source-byte ceiling on the unredacted stream. At a receiver byte cutoff, retain valid UTF-8 prefixes of pending remainders in their last-byte order, mark each incomplete line truncated and report incomplete. An incomplete trailing UTF-8 scalar at this deliberate cutoff is withheld; malformed bytes before that boundary fail. A cutoff inside a payload whose complete frame header/length was already validated may produce this partial result; a truncated header, invalid header or unexpected EOF before a declared frame ends is Unavailable. No allocation is sized blindly from a declared frame length, and no cutoff manufactures EOF. This profile returns next_cursor:null; native tail/source-byte saturation or a local count omission is terminal partial.


## Completeness, bounds and errors

A clean full response below the receiver byte cap, without provider partial
indication, establishes source exhaustion only when the selected driver binding
proves that the native tail did not saturate. Native tail units and decoded LF
occurrences are separate counts; the binding must establish their actual mapping.
The Engine endpoint's tail wording and arbitrary frame
boundaries alone do not establish that correspondence for driver messages/chunks
or multiline payloads. If the binding cannot establish the necessary native
count/exhaustion mapping, refuse the read as Unavailable instead of inferring
exhaustion from LF count or clean EOF. This is a binding proof prerequisite, not
a promise supplied by the current inspect response or source archive.

More than tail_lines decoded occurrences is not itself a provider-protocol error:
a driver message/frame may contain multiple lines. Validate and locally retain
the first tail_lines under the selected bounds, while using the verified native
mapping for any source-exhaustion inference. Native tail/source-byte saturation
remains conservative partial. A reached local count cap with no actual omission
does not defeat separately established native exhaustion. Interrupted or unknown
EOF proves no exhaustion.
Line-content clipping remains separate from occurrence omission; use the shared
cause/count rules.

The local slice is the first tail_lines occurrences in the line-completion order
defined above, including EOF/cutoff remainders at their specified last-byte
positions. This keeps the bounded projection a prefix of the selected native
response, including at a source cutoff. The native tail request selects source
history; a partial local prefix makes no additional guarantee of containing the
latest tail_lines decoded lines. For example, a verified exhausted response of
two native messages containing 300 decoded lines at tail_lines=200 returns the
first 200 decoded occurrences, page_limit, exactly 100 dropped occurrences and
no cursor. If native saturation or a source cutoff also occurs, those causes
coexist and the full dropped-occurrence total becomes unknown.

For this profile, provider_limit is present when the verified native tail mapping
cannot rule out source saturation. Page_limit is present when more than tail_lines
decoded occurrences were observed and the local output cap omits the remainder;
it does not imply that the provider's native tail unit saturated.
source_bytes is present when max_bytes unredacted text bytes were
consumed. Stop at that exact text ceiling without an EOF lookahead. Line_bytes is
present iff an emitted line was clipped, including valid pending channel prefixes.
All applicable causes appear in shared order when limits coincide. The page_limit
remainder is omitted, not retained, and supplies no cursor; response_bytes is not
emitted. No stream-group cap
or supported provider partial extension is selected. Framing/transport/frame-count
or public-result overflow and an unknown partial indication refuse instead of
becoming successful truncation. Occurrences_dropped is null under native tail/source
saturation; when the verified native mapping establishes exhaustion, count locally
omitted decoded occurrences exactly. The fixed
container stream has zero group-selection losses, so stream_groups_dropped is
zero. Content clipping alone leaves both dropped totals zero.

Text is bounded by max_bytes <=128 KiB; independently cap encoded and decoded
stream bytes including framing at 256 KiB, frame count at 4,096, inspect encoded
and decoded response at 1 MiB, headers at 16 KiB per response, and complete public
JSON at 4 MiB. One admitted inspect plus one full-ID logs GET share the same
15 s provider / 20 s outer deadline, including connects <=5 s. No identity probe,
refresh, retry or follow is silently added.

Deadline exhaustion is Timeout, never clean EOF or byte-cutoff success. Other
malformed, oversized or interrupted observations are Unavailable except for the
explicit validated streaming cutoff. Use safe base errors without raw provider
errors, private inspect data, origins or credentials in diagnostics.

## Evidence and obligations

Official Engine API v1.56 ContainerLogs/ContainerAttach sources are retained in the
[provider evidence](../../../../../docs/evidence/restart-visibility-20260908/provider-evidence.md).
This profile's native interpretation and future fixtures belong here. Required
fixtures cover exact name/ID and label admission, replacement, supported drivers,
TTY/raw and multiplexed decoding, interleaved frames, partial headers/payloads,
UTF-8 cutoffs and timeout. These are specification scenarios, not executed results.
The [scenario record](evidence/20260909/scenarios.md) records the corresponding
manual observations and their F12 finding coverage.
