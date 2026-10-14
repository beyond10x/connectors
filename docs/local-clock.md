# Check the local approval clock

`connectors approvals clock-check --adapter forge` checks an explicitly configured
authenticated time source. It returns a bounded UTC observation without starting
the owner or adapter, reading credentials or writing metadata. Approval issuance
and business writes still require their own admission and dispatch integration.

The operator must trust the selected source's UTC/radius assertions and establish
a local monotonic timer frequency bound. A successful signature or clock-check
does not prove either physical assumption. Leave the clock unconfigured when
those assurances are unavailable; ordinary read workflows need no approval clock.

Add a top-level table to the existing private local configuration. This example
uses the public [roughtime.se service](https://roughtime.se/), whose address and
key were checked on 2026-09-10. Recheck the source's current identity before use.
The example's 10,000 ppm (1%) is an explicit assumed maximum, including hardware
error and Linux timer adjustments; it is not a calibration result or an automatic
default for other machines.

```toml
[approval_clock]
format = "roughtime-clock/1"
address = "192.36.143.134:2002"
public_key = "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI="
max_rate_error_ppm = 10000
```

```sh
connectors approvals clock-check --adapter forge --output json
```

Success contains `adapter` and `observation`, whose fields are
`configuration_sha256`, `lower_unix_ms` and `upper_unix_ms`. Width is at most four
seconds. This output is a historical observation; copying it into a file or a
later request cannot supply approval clock evidence.

The command makes one UDP request to the configured numeric address, with no DNS,
automatic retry, server/key discovery or fallback to the machine's wall clock.
When no observation results, the command answers an operational `unavailable`
refusal whose `clock_refusal` names the cause:

| `clock_refusal` | What happened | `next_action` and what to do |
|---|---|---|
| `unreachable` | The source did not answer within two seconds, or sending or receiving failed. | `retry_explicitly`: run the check again later; if it persists, check the configured address. |
| `unverified` | The reply did not verify: it did not parse, or its nonce, protocol version, signatures or Merkle path did not match the configured key and protocol. | `check_configuration`: check the configured public key against the source's published key, and that the source serves Roughtime version `0x8000000c`. |
| `too_imprecise` | The reply verified, but its signed radius (`clock_radius_ms`), with the network delay and timer-rate error, makes the interval wider than four seconds. | `choose_clock_source`: configure a source with a smaller radius. |

An uncertain local timer, or a check close to UTC midnight, refuses the same way
without a `clock_refusal`. Missing or invalid configuration produces a
configuration refusal. Listing, setup checks and existing status commands do not
make time requests.

Internally, authenticated samples last at most 30 seconds, with network delay,
timer-rate error and outward rounding included in every interval. They are kept
only in the acquiring process; restart/fork requires a new sample. Suspend or
uncertain timer continuity invalidates the sample. Without a leap-announcement
binding, estimates also refuse around UTC midnight instead of extrapolating through
a possible repeated/skipped second. See the [clock contract](../contracts/service/clock.md)
for exact bounds and the separate source/hardware trust assumptions.

The native Rust client verifies Roughtime test version `0x8000000c` against the
configured Ed25519 root. It is a single trusted-source binding, without ecosystem
server-list management or malfeasance reporting. Linux x86_64 and Rust 1.88 are the
selected platform. No executable clock helper or new production dependency is used.
