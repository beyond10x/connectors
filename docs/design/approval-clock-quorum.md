# Design: the approval clock quorum

Status: design, 2026-10-14; sources named 2026-10-14. Model: `ess/domains/clock.yaml`. Not yet implemented.

## Decision

The approval clock moves from one Roughtime source to several Roughtime sources
combined by a quorum, with a width cap of 8,000 ms instead of 4,000 ms. The quorum
replaces the single-source check; there is no single-source mode afterwards.

## What changes

Current behaviour this design replaces:

| Today | Where |
|---|---|
| One `address`, one `public_key`, format `roughtime-clock/1` | `crates/connectors-host/src/local/clock.rs:77-84`, `:97`; `contracts/service/clock.md:11-13` |
| One exchange, 2 s deadline, no retry | `clock.rs:25`, `:124-167`; `clock.md:58-60` |
| Observation at most 4,000 ms wide | `clock.rs:27`, `:242`; `clock.md:113-115` |
| The approval proof clock refuses a width over 4,000 ms | `crates/connectors-host/src/local/approvals/proof.rs:90-100`; `contracts/service/delegation.md:145`; `docs/local-approval-binding.md:56-57` |
| The mutation ledger refuses a width over 4,000 ms | `crates/connectors-host/src/local/mutations.rs:463-471`; `docs/local-mutation-ledger.md:75-76` |
| Clock validity checked when the whole configuration loads | `crates/connectors-host/src/local/config.rs:258-262` |
| "An explicitly trusted single-source binding" | `clock.md:21-23`; `docs/local-clock.md:52-54` |

All three width checks move to 8,000 ms in the same change; a clock that answers
8,000 ms while the proof or ledger still refuses above 4,000 ms is a regression.

## Sources

The quorum's sources are the four public Roughtime servers of the Roughtime
ecosystem list (`ecosystem.json` in `github.com/cloudflare/roughtime`, read
2026-10-14), each run by a different operator. Four sources with quorum two tolerate
one lying and one unreachable source at once (see Threat model):

| Source | Operator | Name the address was resolved from | Root public key |
|---|---|---|---|
| 0 | Cloudflare | `roughtime.cloudflare.com:2003` | `0GD7c3yP8xEc4Zl2zeuN2SlLvDVVocjsPSL8/Rl/7zg=` |
| 1 | int08h (Roughenough) | `roughtime.int08h.com:2002` | `AW5uAoTSTDfG5NfY1bTh08GUnOqlRb+HVhbJ3ODJvsE=` |
| 2 | Netnod | `roughtime.se:2002` | `S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI=` |
| 3 | time.txryan.com | `time.txryan.com:2002` | `iBVjxg/1j7y1+kQUTBYdTabxCppesU/07D4PMDJk2WA=` |

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

The addresses are numeric because the host does no DNS; they are the names'
IPv4 answers on 2026-10-14 and are rechecked, with the keys, before use. The keys and
addresses are this document's and the live test's (`independent_live_quorum`); the
host ships no default source.

Probe of 2026-10-14 (one request per source and version, 3 s timeout, from the
client's own request and verifier):

| Source | At `0x8000000c` | With VER `0x00000001` |
|---|---|---|
| Cloudflare | no reply | no reply |
| int08h | 420-byte reply, verifies | no reply |
| Netnod roughtime.se | no reply (it answered on 2026-09-10) | no reply |
| time.txryan.com | 460-byte reply, verifies | no reply |

So int08h and time.txryan.com speak `roughtime-0x8000000c` today. Every source is
configured at `roughtime-0x8000000c`, the one version the client verifies.
Cloudflare and roughtime.se stay listed but were unreachable at that version on
2026-10-14, and neither answered VER `0x00000001` either, so no reply at RFC
10049's version could be recorded and that version is not offered (see
"Version"). The live check of 2026-10-14 verified int08h with a 5,000 ms signed
radius and time.txryan.com with a 3,000 ms one: both count (see "Width cap"), and
their combined answer is within the cap. With those two sources reachable the
quorum is two of two: an answer, but it tolerates no outage and no lying source
until a third source answers (see Threat model).

- **How many.** Two to eight sources (`connectors.clock.Configuration`), `quorum` two
  to eight and at most the number of sources. Three sources with quorum two is the
  smallest configuration that tolerates one lying source; four with quorum two
  tolerates one lying and one unreachable source at once (see Threat model).
- **What identifies one.** Its root public key is its trust identity; its address
  is where it is asked. No two sources may share an address or a key: two entries
  for one key are one operator counted twice, which defeats the quorum. Each check
  stays the one `Configuration::resolved` applies today (`clock.rs:86-115`), per
  source. In output a source is named by its zero-based position in `sources` and
  its address. The configuration digest (`clock.rs:119-121`) covers the whole
  table, source order included, so reordering sources is a new clock selection.
- **Independence.** That the sources are run by independent operators is a
  deployment assertion, like the rate bound (`clock.md:15-19`). The host cannot
  check it and does not claim it.
- **Version.** Each source names the one wire version it speaks
  (`connectors.clock.RoughtimeVersion`). The request carries it as VER; a reply
  verifies only if its signed VER equals it and its VERS lists it, as
  `clock/protocol.rs:151-165` does today for the fixed `0x8000000c`
  (`protocol.rs:14`). There is no negotiation and no downgrade. One version is
  admitted: `roughtime-0x8000000c`, the draft-19 test version the client verifies,
  which int08h and time.txryan.com answer. A version ships only together with its
  verifier and a recorded signed fixture. RFC 10049's version `0x00000001` has
  neither: no source answered it on 2026-10-14, so no reply could be recorded, and a
  configuration naming `roughtime-1` is refused as an unknown version. A further
  version is added as an enum variant once a source answers it and its reply is
  recorded. The field is required with no default, so the digest always names the
  version.
- **No discovery.** No DNS, no server list, no default key, no fallback to the
  system clock, unchanged from `clock.md:13-14`.

## Acquisition and timeouts

All sources are queried in parallel, each from its own connected UDP socket with
its own fresh nonce, one thread per source (`std::thread::scope`; no new dependency).
Sockets are bound and connected first, as today (`clock.rs:127-133`); then one shared
deadline of 2,000 ms starts, and every send, receive and verification must finish
before it. A source that has not delivered a verified reply by the deadline is
`unreachable`. There are no retries. The total network wait is therefore 2,000 ms
whatever the number of sources, the same bound as today.

Sequential querying is rejected: its total time grows with the number of sources
(up to 16 s for eight), and a late source would hold the whole check.

Each source keeps its own monotonic stamps: S_i read before its nonce is created,
R_i read after its reply arrives, as `clock.rs:135` and `:148` do for one source.
Each pair passes the continuity check of `clock.rs:150`, and every later reading is
checked against every counted S_i (`clock.rs:213`).

## Quorum rule

Let N be the current monotonic reading, p the rate bound, M_i and A_i source i's
signed midpoint and radius, and `elapsed_lower`/`elapsed_upper` the conservative
bounds of `clock.md:105-107`. In nanoseconds, source i's interval at N is

```
l_i = M_i - A_i - 3 ms + elapsed_lower(N - R_i)
u_i = M_i + A_i + 3 ms + elapsed_upper(N - S_i)
```

exactly today's interval (`clock.md:107-109`, `clock.rs:226-241`), per source.

1. **Count.** At acquisition, a source is *counted* when its reply verified,
   whatever its own radius or interval width. No source is `too_imprecise` on its
   own: only the combined answer is capped (step 5). The counted set is fixed at
   acquisition; later readings never add or drop a source.
2. **Below quorum.** Fewer than `quorum` counted sources refuses `below_quorum`.
3. **Coverage.** For an instant x, c(x) is the number of counted sources with
   l_i <= x <= u_i (closed intervals). The answer is the hull of the instants that
   at least `quorum` counted intervals contain:

   ```
   L = min { x : c(x) >= quorum }      (always some l_j)
   U = max { x : c(x) >= quorum }      (always some u_k)
   answer = [floor(L / 1 ms), ceil(U / 1 ms)]
   ```

   Computed by sorting the 2k endpoints, starts before ends at equal values, and
   sweeping a counter: L is the start at which the counter first reaches `quorum`,
   U the last end at which it falls from `quorum` to `quorum - 1`.
4. **Split.** If no instant has c(x) >= quorum, the acquisition refuses `split`.
5. **Width.** U - L, rounded outward, over 8,000 ms refuses `too_imprecise`.
6. **Day and range.** The overflow bound and the UTC-midnight rule (`clock.rs:242-254`)
   apply to the answer, with the earliest counted source's origin lower bound
   (M_i - A_i - 3 ms) as the origin. These refusals name no cause.
7. **Validity.** The answer is available while every counted source's
   `elapsed_upper(N - S_i)` is at most 30,000 ms (`clock.rs:222-225`). Each later
   `Clock::now` recomputes steps 3 to 6 at its own N, over the counted set, with no
   network; any refusal there is `ClockUnavailable` as today (`clock.rs:262-266`).

**Why it holds.** Every interval is convex. If at least `quorum` counted intervals
contain true UTC t, then c(t) >= quorum, so L <= t <= U: the answer contains t. That
is the whole guarantee, and it needs no assumption about the other intervals, nor
any bound on a counted interval's width: a wide honest interval still contains t,
and a wide lying one can only widen the answer, which the cap of step 5 refuses.
So a source counts at any width.

**Why not the alternatives.**
- *Intersection of the counted intervals* excludes t as soon as one lying interval
  overlaps the honest ones without containing t, even when every other source is
  honest.
- *Marzullo's maximum-agreement interval* returns the region most intervals share.
  Colluding or coincident wrong intervals can out-vote t's region, and the region it
  returns can exclude t while t's own coverage still meets the quorum.
- The hull of the quorum-covered instants is the conservative choice: it may be
  wider than either, and width is what the cap and `too_imprecise` absorb.

**Example**, quorum 2, intervals in ms relative to a common origin:

| Counted intervals | c >= 2 on | Answer | Width | Result |
|---|---|---|---|---|
| [0, 2000], [500, 2500], [9000, 11000] | [500, 2000] | [500, 2000] | 1,500 | answer; third source `outlier` |
| [0, 2000], [1500, 3500], [3000, 5000] | [1500, 2000], [3000, 3500] | [1500, 3500] | 2,000 | answer; all `agreed` |
| [0, 2000], [5000, 7000], [10000, 12000] | nothing | none | - | `split` |
| [0, 6000], [5000, 11000], [10000, 16000] | [5000, 6000], [10000, 11000] | [5000, 11000] | 6,000 | answer |
| [0, 7000], [6000, 13000], [12000, 19000] | [6000, 7000], [12000, 13000] | [6000, 13000] | 7,000 | answer |
| [0, 4000], [3000, 7000], [8000, 12000], [11000, 15000] | [3000, 4000], [11000, 12000] | [3000, 12000] | 9,000 | `too_imprecise` |

## Width cap: 8,000 ms

The cap bounds the combined answer, never a single source. A verified source counts
toward the quorum at any width (Quorum rule, step 1): the guarantee needs no
per-source bound, and the answer, the hull of the quorum-covered instants, is
usually much narrower than its widest counted source. With quorum two of two it is
the intersection of the two intervals, at most as wide as the narrower.

A source's interval at acquisition is about `2A + 6 ms + (d + 2 ms) q/(q - p)` for
round trip d, and it grows by about `2p/q` of the elapsed time over the 30 s
validity (600 ms at 10,000 ppm). Roughtime draft 19 signs RADI in whole seconds
(`clock.md:54-56`), so A is at least 1 s:

| Radius | At acquisition, d = 2,000 ms, 10,000 ppm | After 30 s | 4,000 ms cap | 8,000 ms cap |
|---|---|---|---|---|
| 1 s | 4,028 ms | 4,628 ms | refused near the deadline | admitted |
| 2 s | 6,028 ms | 6,628 ms | always refused | admitted for the whole 30 s |
| 3 s | 8,028 ms | 8,628 ms | always refused | admitted at acquisition if d <= ~1,950 ms |

The table is the width of an answer formed from such sources alone. Under the
single-source check a source with a 2 s radius could never be used, and one with a
1 s radius only on a fast path; a quorum needs several sources and cannot pick only
the most precise one. 8,000 ms admits answers from radius-2 sources for the whole
validity window. The live check of 2026-10-14 is the case the per-source bound got
wrong: int08h signed a 5 s radius (an interval of about 10 s, over the cap) and
time.txryan.com a 3 s one; capping each source counted only one and refused
`below_quorum`, while their intersection is about 6 s wide and is admitted.

The ceiling is the delegation profile. The issuer sets `iat = floor(midpoint)` and
`nbf = iat - 5` (`delegation.md:145`, `proof.rs:213-217`), and its own interval must
satisfy `nbf <= lower`. With width W, `lower = midpoint - W/2` and
`midpoint - iat` is in [0, 1 s), so the issuer's own check holds for every W up to
10,000 ms. 8,000 ms (±4 s) keeps a full second of margin under that limit. A wider
cap would need a change to the proof profile itself, which this design does not make.

Consequences for `delegation.md:145`, which changes to "at most eight seconds (at
most ±4 s)" in the same change:
- *Delivery time left.* Issued at true time t, `exp = iat + 25 >= t - W/2 + 24`. A
  receiver whose interval contains t + τ has upper bound at most t + τ + W, so
  `upper < exp` holds for every τ < 24 - 1.5 W: at least 18 s at 4,000 ms, at least
  12 s at 8,000 ms, of the 25 s delivery span. The 295 s approval span keeps at
  least 282 s.
- *Not-before across hosts.* `nbf <= lower` on a different receiving host was
  already not guaranteed: `nbf <= t + W/2 - 5` against a receiver lower bound down
  to t + τ - W, so a fresh proof can refuse for τ < 1.5 W - 5: up to 1 s at
  4,000 ms, up to 7 s at 8,000 ms. Whether the 5 s margin should move is the
  delegation contract's question, not this design's.

## Refusals

A per-source result (`connectors.clock.SourceResult`) and, when no answer forms, one
combined refusal (`connectors.clock.Refusal`). Both extend the single-source causes
of the refusal-cause change (commit `0ec7ccd1a`: `unreachable`, `unverified`,
`too_imprecise`, with `clock_refusal`, `clock_radius_ms` and the next action
`choose_clock_source`).

Per source, in configuration order, on every observation and every quorum refusal
(`connectors.clock.SourceReport`: `index`, `address`, `result`, `radius_ms` when the
reply verified):

| `result` | Meaning |
|---|---|
| `agreed` | counted; its interval meets the answer |
| `outlier` | counted; its interval is disjoint from the answer, so it is wrong or its operator's assertion failed |
| `verified` | counted; no answer formed (a refusal) |
| `unreachable` | no verified reply by the shared deadline, or a send or receive error (single-source meaning unchanged) |
| `unverified` | a reply that does not parse or verify against this source's key and version (unchanged) |

There is no per-source `too_imprecise`: the single-source cause becomes the combined
refusal below, because a verified source counts at any width.

Combined, as the CLI `unavailable` refusal at `stage = observation`:

| Refusal | When | `next_action` |
|---|---|---|
| `below_quorum` | fewer than `quorum` verified (counted) sources | `check_configuration` if any source is `unverified`; else `retry_explicitly` |
| `split` | no instant lies in `quorum` counted intervals | `choose_clock_source`: a source is wrong; find it from the per-source lines and retire its key |
| `too_imprecise` | the combined answer is wider than 8,000 ms; the only `too_imprecise`, never a single source | `choose_clock_source`: select more precise sources |

In the CLI `Failure`, `clock_refusal` carries the combined refusal and a new
`clock_sources` carries the per-source lines; the single `clock_radius_ms` moves into
the per-source `radius_ms`. A local timer, continuity, overflow or UTC-midnight
refusal names no cause and keeps `check_configuration`, as in `0ec7ccd1a`. No
refusal or observation carries a source's own interval, the request or the reply.
An `outlier` on a successful observation is not a refusal: the answer still holds
under the quorum guarantee, and the line tells the operator which key to examine.

`ess/domains/cli.yaml` is not changed here: the refusal-cause change already rewrites
the same `Failure` fields and is not on `main`. The implementation applies
`clock_refusal: Optional<connectors.clock.Refusal>` and
`clock_sources: Optional<List<connectors.clock.SourceReport>>` on top of it.

## Migration from `roughtime-clock/1`

Refused, with an upgrade message; never read as a one-source configuration. A
single source cannot meet a quorum of two, and accepting it would keep the
single-source trust silently under a new name.

The refusal is scoped to the clock. `approval_clock` is parsed as either shape, so a
retired `roughtime-clock/1` table no longer fails the whole configuration at load
(`config.rs:258-262`), which would stop reads that never use the clock. Every use of
the clock (paths under `crates/connectors-host/src/local/`: `approvals clock-check`
in `apps/connectors/src/local.rs:422-434`; approval issuance, `owner/approval_issuance.rs:85`,
`:358`; maintenance, `owner/maintenance.rs:109`, `:143`; mutation execution and
recovery, `owner/mutation/execution.rs:85`, `owner/mutation/recovery.rs:48`) refuses
`invalid_configuration` at `stage = configuration` with `check_configuration` and a
message naming `roughtime-quorum/1` and `docs/local-clock.md`. Any other format
string, or a malformed quorum table, stays a whole-configuration refusal as today.
The new shape has a new digest, so a consumer retaining the earlier clock selection
sees a changed selection and rechecks it (`clock.md:29-32`).

## Threat model

The answer contains true UTC whenever at least `quorum` counted sources tell the
truth. Within that:

| Event | Effect |
|---|---|
| One lying source (valid signature, false time), the others honest and reachable, `quorum <= sources - 1` | Cannot move the answer off true time, at any width: it counts like every verified source, and the guarantee needs no per-source bound. Signing a wide radius or a false time can only widen the answer, past the cap at worst, causing `too_imprecise` (denial of service); it shows as `outlier` when its interval is disjoint. A liar whose interval overlaps the answer is undetectable and harmless. With the two sources reachable on 2026-10-14 (int08h radius 5 s, time.txryan.com radius 3 s) the quorum is two of two, so this row does not apply: no outage and no lying source is tolerated until a third source answers. |
| One unreachable source | Availability only: an answer while `quorum` others are counted, else `below_quorum`. |
| One lying and one unreachable source | Safe only if `quorum` honest sources remain counted (four sources, quorum two). With three sources and quorum two, the liar and the remaining honest source form the quorum and a liar overlapping it can set a false answer. |
| `quorum` or more colluding liars | A false answer. Out of scope: the sources' independence is a deployment assertion. |
| Network attacker | Cannot forge or replay (signature, fresh nonce per source). Can drop (`unreachable`, `below_quorum`) and delay; delay is inside every interval, so it widens rather than shifts (`too_imprecise`). |
| Quorum equal to the number of sources | Every source must answer, and one liar overlapping the others can shift the answer. Allowed, documented as not tolerating a liar. |

Unchanged and still out of scope (`clock.md:24-27`): a compromised local kernel, time
namespace changes, VM snapshot or rollback, a timer outside its rate bound, and key
retirement, which stays the operator's.

## Conformance scenarios

Host (`crates/connectors-host/src/local/clock/tests.rs`, fixtures from
`tests/fixtures/clock/server.rs`, one fixture server per source):

- `quorum_answer_is_hull_of_quorum_coverage`: the six example rows above, exact bounds.
- `quorum_contains_truth_with_one_disjoint_liar`: three sources, one disjoint, answer
  contains the fixture time; the liar is `outlier`.
- `quorum_overlapping_liar_cannot_exclude_truth`: three sources, liar overlaps both
  honest intervals without containing t; answer contains t.
- `quorum_below_quorum_names_each_source`: fewer counted than the quorum; each other
  line `unreachable` or `unverified`, a wide source `verified`; next action by
  precedence.
- `quorum_split_refuses`: no instant covered twice; `split`, `choose_clock_source`.
- `quorum_combined_width_over_cap_refuses`: the last example row; `too_imprecise`.
- `quorum_wide_source_counts_and_only_the_answer_is_capped`: a radius-5 s source
  (wider than 8,000 ms) and a radius-3 s source answer at quorum two, both `agreed`;
  two radius-5 s sources form an answer over 8,000 ms, refused `too_imprecise` with
  both lines `verified`.
- `quorum_width_cap_boundary`: answers of exactly 8,000 ms admitted, 8,001 ms refused.
- `quorum_shared_deadline`: one source silent, total acquisition under 2,000 ms plus
  verification; that source `unreachable`.
- `quorum_version_mismatch_is_unverified`: a reply signed with another VER, or VERS
  without the configured version, is `unverified`; no second request is sent.
- `quorum_configuration_refusals`: one source; nine sources; quorum 1; quorum above the
  source count; duplicate address; duplicate key; missing `version`; an unknown
  version, `roughtime-1` included.
- `quorum_counted_set_is_fixed`: a later `Clock::now` past one source's 30 s window
  refuses; it never drops that source and answers from the rest.
- `legacy_single_source_format_refuses_only_the_clock`: a `roughtime-clock/1` table
  loads the configuration; `clock-check` and approval issuance refuse
  `invalid_configuration` with the upgrade message; a connection read still works.

CLI (`apps/connectors/tests/local_cli.rs`): `clock_check_reports_each_source`,
`clock_check_split_refusal`, `clock_check_below_quorum_refusal`,
`clock_check_legacy_format_refusal`, each against the generated contract fixtures.

Width consistency (`approvals/tests.rs`, `owner/mutation/tests.rs`):
`proof_clock_admits_8000_ms`, `ledger_clock_admits_8000_ms`, each refusing 8,001 ms.

Live (ignored, explicit): `independent_live_quorum`, replacing
`independent_live_source` (`clock/tests.rs:198`), against the four sources of "Sources"
at quorum two.
