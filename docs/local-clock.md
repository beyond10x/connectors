# Check the local approval clock

`connectors approvals clock-check --adapter forge` checks the explicitly configured
authenticated time sources. It returns a bounded UTC observation without starting
the owner or adapter, reading credentials or writing metadata. Approval issuance
and business writes still require their own admission and dispatch integration.

The operator must trust the selected sources' UTC/radius assertions, that they are
run by independent operators, and establish a local monotonic timer frequency
bound. A successful signature or clock-check does not prove any of these. Leave
the clock unconfigured when those assurances are unavailable; ordinary read
workflows need no approval clock.

Add a top-level table to the existing private local configuration. It names two to
eight Roughtime sources and a `quorum`: the answer is formed only where at least
`quorum` of them agree. This example uses the four public sources of the Roughtime
ecosystem list, each run by a different operator; their addresses and keys were
checked on 2026-10-14 (the addresses are the IPv4 answers of the names in the
comments, since the client does no DNS). Recheck each source's current identity,
address and version before use. The example's 10,000 ppm (1%) is an explicit
assumed maximum, including hardware error and Linux timer adjustments; it is not a
calibration result or an automatic default for other machines.

```toml
[approval_clock]
format = "roughtime-quorum/1"
quorum = 2
max_rate_error_ppm = 10000

[[approval_clock.sources]]   # Cloudflare, roughtime.cloudflare.com
address = "162.159.200.123:2003"
public_key = "0GD7c3yP8xEc4Zl2zeuN2SlLvDVVocjsPSL8/Rl/7zg="
version = "roughtime-0x8000000c"

[[approval_clock.sources]]   # int08h, roughtime.int08h.com
address = "51.81.57.55:2002"
public_key = "AW5uAoTSTDfG5NfY1bTh08GUnOqlRb+HVhbJ3ODJvsE="
version = "roughtime-0x8000000c"

[[approval_clock.sources]]   # Netnod, roughtime.se
address = "192.36.143.134:2002"
public_key = "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI="
version = "roughtime-0x8000000c"

[[approval_clock.sources]]   # time.txryan.com
address = "148.113.255.71:2002"
public_key = "iBVjxg/1j7y1+kQUTBYdTabxCppesU/07D4PMDJk2WA="
version = "roughtime-0x8000000c"
```

Each source's `version` is the one Roughtime wire version it is asked in and must
answer in. The one admitted version is `roughtime-0x8000000c` (the draft-19 test
version); any other value, `roughtime-1` included, is refused as an unknown
version, because a version ships only with its verifier and a recorded reply and no
source answered RFC 10049's version `0x00000001` on 2026-10-14. There is no
negotiation. No two sources may share an address or a key. Three sources at quorum
two tolerate one lying source; four at quorum two tolerate one lying and one
unreachable source at once. A quorum equal to the number of sources tolerates no
liar.

On 2026-10-14 only int08h (signed radius 5 s) and time.txryan.com (3 s) answered;
Cloudflare and roughtime.se were unreachable at `roughtime-0x8000000c`. With two
reachable sources this example is a quorum of two of two: it answers, but it
tolerates no outage and no lying source until a third source answers. A verified
source counts toward the quorum at any radius; only the combined answer is capped at
eight seconds, so int08h's 10-second interval still counts and the pair's answer is
about six seconds wide.

```sh
connectors approvals clock-check --adapter forge --output json
```

Success contains `adapter` and `observation`, whose fields are
`configuration_sha256`, `lower_unix_ms`, `upper_unix_ms` and `sources`. Width is at
most eight seconds. `sources` has one line per configured source, in configuration
order: its `index`, `address`, `result` (`agreed` when its interval meets the answer,
`outlier` when it is disjoint from it, or why it was not counted: `unreachable` or
`unverified`) and, when its reply
verified, its signed radius in `radius_ms`. An `outlier` does not stop the answer;
it names the key to examine. This output is a historical observation; copying it
into a file or a later request cannot supply approval clock evidence.

The command asks every source once, in parallel, at its configured numeric address,
under one shared two-second deadline, with no DNS, automatic retry, server/key
discovery or fallback to the machine's wall clock. When no answer forms, the command
answers an operational `unavailable` refusal whose `clock_refusal` names why and
whose `clock_sources` lists each source:

| `clock_refusal` | What happened | `next_action` and what to do |
|---|---|---|
| `below_quorum` | Fewer than `quorum` sources verified. | `check_configuration` if any line is `unverified`: check that source's key and version; else `retry_explicitly`: the others were `unreachable`, run the check again later. |
| `split` | The verified sources disagree: no instant lies in `quorum` of their intervals. | `choose_clock_source`: a source is wrong; find it from the lines and retire its key. |
| `too_imprecise` | The combined answer is wider than eight seconds. A single source is never too imprecise: it counts at any radius. | `choose_clock_source`: select more precise sources. |

A line's `result` is `unreachable` (no verified reply within two seconds, or sending
or receiving failed), `unverified` (the reply did not parse, or its nonce, version,
signatures or Merkle path did not match the source's key and configured version)
or, on a refusal, `verified` (counted, at any radius).

An uncertain local timer, or a check close to UTC midnight, refuses the same way
without a `clock_refusal`. Missing or invalid configuration produces a
configuration refusal. Listing, setup checks and existing status commands do not
make time requests.

## Upgrading from `roughtime-clock/1`

The single-source format `roughtime-clock/1` (one `address` and `public_key`) is
retired. A configuration that still carries it loads, and every command that does
not use the clock works, but `clock-check`, approval issuance and approved writes
refuse with `invalid_configuration`, `next_action` `check_configuration` and
`configuration_rule` `approval_clock_format_retired`. Rewrite the table as
`roughtime-quorum/1` above: move the old source into a `[[approval_clock.sources]]`
entry with its `version`, and add at least one more independent source. A single
source is never read as a quorum. The new table has a new configuration digest, so
a consumer that retained the earlier clock selection rechecks it.

## Bounds

Internally, each source's authenticated sample lasts at most 30 seconds, with
network delay, timer-rate error and outward rounding included in every interval;
once any counted source's window passes, the clock refuses rather than dropping it.
Samples are kept only in the acquiring process; restart/fork requires new samples.
Suspend or uncertain timer continuity invalidates them. Without a leap-announcement
binding, estimates also refuse around UTC midnight instead of extrapolating through
a possible repeated/skipped second. See the [clock contract](../contracts/service/clock.md)
for exact bounds, the quorum rule and the separate source/hardware trust assumptions.

The native Rust client verifies each source at its configured version against its
configured Ed25519 root. It has no ecosystem server-list management or malfeasance
reporting. Linux x86_64 and Rust 1.88 are the selected platform. No executable clock
helper or new production dependency is used.
