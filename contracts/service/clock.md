# Local bounded clock

The approval and retention clocks require an interval containing current UTC;
[delegation](delegation.md#5-time-deadlines-and-durable-uniqueness) limits approval
interval width to 4,000 ms. This binding supplies that interval under explicit
infrastructure trust. A signed timestamp authenticates a source's assertion; it
does not prove that the source or the local hardware keeps correct time.

## Configuration and trust

The optional `approval_clock` table in local configuration selects format
`roughtime-clock/1`, one numeric UDP `address`, an exact canonical base64 Ed25519
`public_key`, and `max_rate_error_ppm` in 1..10,000. There is no ambient server,
DNS/key discovery, default key or fallback to SystemTime. The configured owner
admits the source's UTC uncertainty assertions and a bound on the local Linux
CLOCK_MONOTONIC frequency error, including oscillator and kernel adjustments.
The rate bound is a deployment assertion, not a value measured or certified by
this client; selecting an arbitrary value does not qualify a machine. A deployment
without those assurances must leave this profile unavailable.

The source must include UTC/leap-second uncertainty in its signed radius. This
is an explicitly trusted single-source binding, not the Roughtime ecosystem's
multi-server measurement, malfeasance-reporting or automatic trust-update client.
The owning operator must retire compromised/stale source keys. A cryptographically
valid but false source assertion violates the admitted infrastructure assumption.
There is no defense against a compromised local kernel, time namespace changes,
VM snapshot/time rollback or a timer whose configured rate bound does not hold.

Configuration is non-secret, private-owner-checked TOML. Its complete canonical
digest identifies the clock selection. A sampled clock grants no key, caller,
approval or dispatch authority; the consuming coordinator must retain/recheck its
current clock trust selection alongside its current policy before admitting use.
No SQLite migration, persistent clock cache or independent identity is introduced.

## Authenticated acquisition

The selected wire format is the explicit test version `0x8000000c` of
[Roughtime draft 19](https://datatracker.ietf.org/doc/html/draft-ietf-ntp-roughtime-19),
using seconds for MIDP/MINT/MAXT/RADI. No version downgrade is attempted. A fresh
32-byte cryptographic nonce and server-key commitment bind a 1,024-byte request.
The client makes one connected UDP exchange, with a two-second overall deadline
and a response no larger than the request. Numeric endpoints avoid an unbounded
resolver step. There are no automatic network retries.

Before accepting a response, verify exact framing, bounded/ordered unique tags
and offsets, required field widths, response TYPE, echoed NONC, selected signed
VER, bounded strictly ordered VERS including the selected version, positive RADI,
both Ed25519 signatures, the inclusive delegated-key time interval, and the
Merkle path using the first 32 bytes of SHA-512 over the complete request packet, including
zero remaining index bits. Unknown tags and advertised versions have no authority;
ignore them within the packet bounds while verifying the original signed bytes.
Malformed, mismatched, unavailable, oversized or late observations establish no
clock. Never return packet contents, parser/OS error text or a forged time in a
failure. Acquisition happens outside metadata transactions.

## Local continuation and bounds

The estimate is a non-serializable process-bound value, valid for at most 30 seconds
from the monotonic sample preceding nonce creation. A new process or restart must
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

Let S and R be monotonic readings before nonce creation and after receipt, N the
current reading, p the admitted parts-per-million rate error, and q=1,000,000.
For measured elapsed d, conservative real elapsed bounds are
`max(0, d - 2 ms) * q / (q + p)` and
`(d + 2 ms) * q / (q - p)`, rounded outward. With signed midpoint M and radius A,
the current interval is `[M-A + elapsed_lower(N-R) - 3 ms,
M+A + elapsed_upper(N-S) + 3 ms]`, rounded outward to Unix milliseconds.
The complete request/response delay is therefore included; no symmetric-delay
assumption or best-case network subtraction is permitted.

The conservative elapsed upper bound from S must remain at most 30,000 ms.
Every observation must be within 0..9007199254740991 Unix ms, ordered and at most
4,000 ms wide. Expiry, overflow, uncertain continuity, excessive radius/delay or
clock-read failure returns unavailable, never expiry proof for a retained key.
Without an admitted leap-announcement source, do not extrapolate across any UTC
midnight: the original lower bound and current upper bound must lie in the same
UTC day, at least one second after its start and strictly more than one second
before its end. This conservative daily refusal window covers both positive and
negative leap seconds without claiming leap-calendar authority; see
[NIST's UTC explanation](https://www.nist.gov/pml/time-and-frequency-division/leap-seconds-faqs).
This bound is conditional on the explicitly admitted source and timer assumptions;
deterministic tests establish the calculation, not those physical assumptions.

## CLI and verification

`connectors approvals clock-check --adapter <alias>` resolves the configured owner
and adapter, checks the configured clock and returns the adapter alias, clock
configuration digest and current interval. It creates no metadata, reads no secret,
starts no owner/adapter and issues no approval. Unconfigured/invalid clock selection
is a configuration refusal; authentication, transport or time-bound failure is a
safe operational refusal. Its output is an observation, never reusable evidence
for a later operation. Listing and existing setup/status commands do not query it.

Verification must cover independent signed response fixtures and an explicit live
source check; nonce/key/version/framing/Merkle substitutions; unknown metadata;
oversized, unavailable and delayed peers; exact interval/expiry/rate/overflow
boundaries; process restart/fork and suspend/clock discontinuity; and production
CLI checks with unchanged read-only lifecycle and no service start or secret access.
A live exchange establishes interoperability and the observed signed bound only.
