# Local bounded clock

The approval and retention clocks require an interval containing current UTC;
[delegation](delegation.md#5-time-deadlines-and-durable-uniqueness) limits approval
interval width to 8,000 ms. This binding supplies that interval from a quorum of
explicitly trusted Roughtime sources. A signed timestamp authenticates a source's
assertion; it does not prove that the source or the local hardware keeps correct
time.

## Configuration and trust

The optional `approval_clock` table in local configuration selects format
`roughtime-quorum/1`, a `quorum`, `max_rate_error_ppm` in 1..10,000 and two to
eight `[[approval_clock.sources]]`. Each source names one numeric UDP `address`,
an exact canonical base64 Ed25519 `public_key` and the one wire `version` it
speaks. The one admitted version is `roughtime-0x8000000c` (the draft-19 test
version); a version ships only with its verifier and a recorded signed reply, so
any other value, `roughtime-1` included, is refused as an unknown version. `quorum` is two to eight and at
most the number of sources. No two sources share an address or a key (an
IPv4-mapped IPv6 address is its IPv4 address): two entries
for one key are one operator counted twice. There is no ambient server, DNS/key
discovery, default key or fallback to SystemTime. The configured owner admits the
sources' UTC uncertainty assertions and a bound on the local Linux
CLOCK_MONOTONIC frequency error, including oscillator and kernel adjustments.
The rate bound is a deployment assertion, not a value measured or certified by
this client; selecting an arbitrary value does not qualify a machine. That the
sources are run by independent operators is a deployment assertion too, which
the client cannot check. A deployment without those assurances must leave this
profile unavailable.

```toml
[approval_clock]
format = "roughtime-quorum/1"
quorum = 2
max_rate_error_ppm = 10000

[[approval_clock.sources]]
address = "192.0.2.10:2002"
public_key = "<44-character base64 Ed25519 root key>"
version = "roughtime-0x8000000c"

[[approval_clock.sources]]
address = "198.51.100.20:2002"
public_key = "<another source's key>"
version = "roughtime-0x8000000c"
```

The retired single-source format `roughtime-clock/1` (one `address`, one
`public_key`) still parses, so the rest of the configuration loads, but every use
of the clock refuses it as `invalid_configuration` naming the configuration rule
`approval_clock_format_retired`; it is never read as a one-source quorum. Any other
format string, or a malformed quorum table, refuses the whole configuration.

Each source must include UTC/leap-second uncertainty in its signed radius. The
answer contains true UTC whenever at least `quorum` counted sources tell the truth;
one lying source among honest ones cannot move it off true time, only widen it up to
the cap. `quorum` or more colluding sources can set a false answer, and a quorum
equal to the number of sources tolerates no liar. This is not the Roughtime
ecosystem's malfeasance-reporting or automatic trust-update client. The owning
operator must retire compromised/stale source keys. There is no defense against a
compromised local kernel, time namespace changes, VM snapshot/time rollback or a
timer whose configured rate bound does not hold.

Configuration is non-secret, private-owner-checked TOML. Its complete canonical
digest, source order included, identifies the clock selection. A sampled clock grants no key, caller,
approval or dispatch authority; the consuming coordinator must retain/recheck its
current clock trust selection alongside its current policy before admitting use.
The clock client itself adds no SQLite migration, persistent clock cache or
independent identity. A consumer keeps its own anti-regression floor: the
mutation ledger records the highest admitted `lower_unix_ms` in `mutation_clock`
and refuses an observation below it. The connection registry's `registry_clock`
floor guards wall-clock time, not this clock. Both floors are
`connectors.clock.LocalClockFloor` records with distinct owner identities
(`ess/domains/clock.yaml`, `docs/local-er-metadata.md`).

### Registry clock floor

The registry floor's durable value is a private file beside the metadata lock
(`connectors.clock.RegistryClockFloorFile`), read and replaced only under the
metadata lifecycle lock. A registry transaction refuses a wall-clock sample below
the higher of that file and the recorded `LocalClockFloor`, and writes the file
before it records anything else. The recorded floor advances only to a sample more
than 60,000 ms past it. Without a readable file the floor is the recorded value
plus 60,000 ms, which is at least every sample ever admitted; a file restored to an
older value can admit at most 60,000 ms of regression.

## Authenticated acquisition

Each source is asked at its configured version only, in the wire format of
[Roughtime draft 19](https://datatracker.ietf.org/doc/html/draft-ietf-ntp-roughtime-19),
using seconds for MIDP/MINT/MAXT/RADI; the request carries that version as VER.
There is no negotiation and no downgrade. A fresh 32-byte cryptographic nonce and
server-key commitment per source bind a 1,024-byte request. All sources are asked
in parallel, each over its own connected UDP socket; one shared two-second deadline
starts after every socket is connected and covers every send, receive and
verification, so the total network wait does not grow with the number of sources.
A source with no verified reply by the deadline is `unreachable`. A response is no
larger than the request. Numeric endpoints avoid an unbounded resolver step. There
are no automatic network retries.

Before accepting a response, verify exact framing, bounded/ordered unique tags
and offsets, required field widths, response TYPE, echoed NONC, signed VER equal
to the source's configured version, bounded strictly ordered VERS including that
version, positive RADI,
both Ed25519 signatures, the inclusive delegated-key time interval, and the
Merkle path using the first 32 bytes of SHA-512 over the complete request packet, including
zero remaining index bits. Unknown tags and advertised versions have no authority;
ignore them within the packet bounds while verifying the original signed bytes.
Malformed, mismatched, unavailable, oversized or late observations establish no
clock. Never return packet contents, parser/OS error text or a forged time in a
failure. Acquisition happens outside metadata transactions.

## Local continuation and bounds

The estimate is a non-serializable process-bound value, valid while every counted
source is within 30 seconds of its own monotonic sample preceding nonce creation.
The counted set is fixed at acquisition: a later reading never adds a source or
drops one whose window passed; it refuses. A new process or restart must
acquire new evidence. It holds no socket after acquisition. Each `Clock::now`
performs bounded local clock reads and checked arithmetic only, with no network,
metadata or blocking synchronization.

Each local reading brackets CLOCK_MONOTONIC between two CLOCK_BOOTTIME samples.
Refuse a bracket wider than 1 ms, backward readings, or a change in the inferred
BOOT minus MONOTONIC offset outside the original bracket enlarged by 1 ms.
This invalidates estimates across suspend/resume without relying on the suspended
clock's rate. Include 3 ms of additional continuation uncertainty to cover the
accepted offset/bracketing tolerance; timer resolution above 1 ms refuses.
The configured rate bound applies to the non-suspended monotonic elapsed time.

On macOS the same rule holds with CLOCK_UPTIME_RAW in place of CLOCK_MONOTONIC
and CLOCK_MONOTONIC_RAW in place of CLOCK_BOOTTIME: both read one unadjusted
time base, and only the first stops while the machine sleeps, so their offset
moves only by time asleep. Darwin's CLOCK_MONOTONIC is subject to frequency and
time adjustments and is not used. The rate bound then covers the oscillator
alone.

On Windows the same rule holds with `QueryUnbiasedInterruptTimePrecise` in place
of CLOCK_MONOTONIC and `QueryInterruptTimePrecise` in place of CLOCK_BOOTTIME:
both read the interrupt time in 100 ns units, and only the first excludes the
time the machine sleeps, so their offset moves only by time asleep.
`QueryUnbiasedInterruptTime` and `QueryInterruptTime` advance only at each timer
interrupt, about 15.6 ms by default, and are not used.

Let S_i and R_i be source i's monotonic readings before its nonce creation and
after its reply's receipt, N the current reading, p the admitted parts-per-million
rate error, and q=1,000,000. For measured elapsed d, conservative real elapsed
bounds are `max(0, d - 2 ms) * q / (q + p)` and `(d + 2 ms) * q / (q - p)`, rounded
outward. With source i's signed midpoint M_i and radius A_i, its interval is
`[M_i-A_i + elapsed_lower(N-R_i) - 3 ms, M_i+A_i + elapsed_upper(N-S_i) + 3 ms]`.
The complete request/response delay is therefore included; no symmetric-delay
assumption or best-case network subtraction is permitted.

A source is counted when its reply verified, whatever its own radius or interval
width; no single source is too imprecise, and only the combined answer is capped.
A counted source whose signed values give no interval (M_i below A_i + 3 ms, or an
endpoint past the representable range) contributes none and is an `outlier`; it
never refuses the acquisition or a later reading on its own.
Fewer than `quorum` counted sources
refuses `below_quorum`. The answer is the hull of the instants that at least
`quorum` counted intervals contain (closed intervals), rounded outward to Unix
milliseconds: its lower bound is the earliest such instant, its upper bound the
latest. If no instant lies in `quorum` counted intervals the acquisition refuses
`split`. Every interval is convex, so if at least `quorum` counted intervals contain
true UTC the answer does too, whatever the other intervals are; the guarantee needs
no per-source width bound, and a wide or lying interval can only widen the answer.

Each counted source's conservative elapsed upper bound from S_i must remain at most
30,000 ms. Every answer must be within 0..9007199254740991 Unix ms, ordered and at
most 8,000 ms wide; a wider answer refuses `too_imprecise`. Expiry, overflow,
uncertain continuity, excessive delay or clock-read failure returns
unavailable, never expiry proof for a retained key.
Without an admitted leap-announcement source, do not extrapolate across any UTC
midnight: the answer's lower and upper bounds must lie in the same UTC day, at
least one second after its start and strictly more than one second before its end.
No single source's interval is the origin, so an outlier on another UTC day or a
wide counted source reaching across midnight never refuses the answer. This conservative daily refusal window covers both positive and
negative leap seconds without claiming leap-calendar authority; see
[NIST's UTC explanation](https://www.nist.gov/pml/time-and-frequency-division/leap-seconds-faqs).
This bound is conditional on the explicitly admitted source and timer assumptions;
deterministic tests establish the calculation, not those physical assumptions.

## CLI and verification

`connectors approvals clock-check --adapter <alias>` resolves the configured owner
and adapter, checks the configured clock and returns the adapter alias, clock
configuration digest, current interval and one line per configured source. It
creates no metadata, reads no secret,
starts no owner/adapter and issues no approval. Unconfigured/invalid clock selection
is a configuration refusal; authentication, transport or time-bound failure is a
safe operational refusal. Its output is an observation, never reusable evidence
for a later operation. Listing and existing setup/status commands do not query it.

### Refusals

Every observation and every quorum refusal carries one line per configured source,
in configuration order (`connectors.clock.SourceReport`): its zero-based `index`, its
`address`, its `result` and, exactly when its reply verified, its signed radius in
`radius_ms`.

| `result` | Meaning |
|---|---|
| `agreed` | counted; its interval meets the answer |
| `outlier` | counted; its interval is disjoint from the answer, so it is wrong or its operator's assertion failed |
| `verified` | counted; no answer formed (a refusal) |
| `unreachable` | no verified reply by the shared deadline, or a send or receive error |
| `unverified` | a reply that does not parse, carries another nonce or version, or fails either signature, the delegation interval or the Merkle path |

An acquisition that forms no answer names one combined refusal
(`connectors.clock.Refusal`), each with its own next action. None changes a bound
above: the deadline, the width, the versions and the trusted sources stay as
configured.

| Refusal | When | Next action |
|---|---|---|
| `below_quorum` | fewer than `quorum` counted sources | `check_configuration` if any source is `unverified`; else `retry_explicitly` |
| `split` | no instant lies in `quorum` counted intervals | `choose_clock_source`: a source is wrong; find it from the per-source lines and retire its key |
| `too_imprecise` | the combined answer is wider than 8,000 ms (never a single source) | `choose_clock_source`: select more precise sources |

An `outlier` on a successful observation is not a refusal: the answer still holds
under the quorum, and the line names the key to examine. A local timer, continuity,
overflow or UTC-midnight refusal names no refusal and no source lines. No
observation or refusal carries a source's own interval, the request or the reply
bytes.

Verification must cover independent signed response fixtures and an explicit live
quorum check; nonce/key/version/framing/Merkle substitutions; unknown metadata;
oversized, unavailable and delayed peers; exact interval/expiry/rate/overflow
boundaries; process restart/fork and suspend/clock discontinuity; and production
CLI checks with unchanged read-only lifecycle and no service start or secret access.
A live exchange establishes interoperability and the observed signed bound only.
