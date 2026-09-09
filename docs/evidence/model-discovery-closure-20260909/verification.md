# Discovery and configuration model closure

Specification-only selection under specification:core-model-closure-20260909,
from base `8a5cf563fc717fd4b23e4dd7d470d0c967f39461`. No runtime, provider call,
public codec, planning-store mutation or generated implementation is included.

The owning prose selects one private DiscoveryCollection per exact scope epoch,
owning bounded ResourceObservation records. Snapshot and latest publication
receipt are embedded values; neither creates another persistent owner. Connection
embeds the fixed mediated-route value and its replaceable evidence. Historical
observation coordinates impose no live foreign key. Composition is immutable
admitted configuration, with instance coordinates resolved against existing
ServiceConfiguration identities.

## Unit verification

Pinned compiler: ESS 0.20.0 from the repository's local toolchain. The baseline
real `ess/` validates (14 files), and compiles to 11 entities. For the new unit,
the real authored tree was copied to scratch and only
`connectors.discovery_state` was added to the copied system domain list. No
cross-domain stub was introduced. That registered unit validates (15 files) and
compiles to 13 entities. Every existing entity identity compares equal to its
baseline identity; the two additions are DiscoveryCollection.collection_ref:String
and ResourceObservation.observation_ref:String.

Commands used, with scratch outputs retained in the author worktree:

```sh
ess specify validate --path ess
ess specify compile --path ess --format json --out baseline-ir.json
ess specify validate --path unit/ess
ess specify compile --path unit/ess --format json --out unit-ir.json
ess specify compile --path unit/ess --format json --out unit-ir-repeat.json
cmp unit-ir.json unit-ir-repeat.json
```

The first two commands ran before authoring. The last four ran against the
scratch registered unit. Both successful compilations were byte-identical.
Inspection of compiled IR confirms the single `owns` edge carries the collection
identity on ResourceObservation.collection_ref, and Withdrawn has only a Retired
edge, with no exit from Retired. In an isolated negative copy, changing only that
child ownership carrier from String to Integer makes validation exit 1 with
`type_mismatch` / `ESS-ENTITY-002`, identifying the required String carrier.

Initial authoring validation refused underscore outcome names and then ambiguous
unconditional outcomes. Outcome names were corrected to lower-kebab spelling;
trusted owner-decision inputs and guarded dispositions now disambiguate them.
The passing model does not claim those decisions derive the underlying facts.

## Focused contract cases

These are source/compiled-model checks, not executed storage or provider tests.

| Case | Selected result and evidence |
|---|---|
| Same object in two collection scopes | Independent qualified identities and owned rows; withdrawal in one does not withdraw the other (resources §4.3/§4.6). |
| Positive observation after temporary staleness | Existing incarnation may return Stale → Observed only under the same exact identity/target proof (ResourceObservation lifecycle and resources §4.3). |
| Positive data after confirmed withdrawal | Withdrawn cannot transition to Observed; a new admitted incarnation is required. Compiled lifecycle retains only its retirement edge. |
| Collection retirement or continuity loss | Retired cannot reopen; fresh scope/epoch/identity cannot revive an old cursor or child coordinate (resources §4.6). |
| Old publisher after view retirement or another publication | Private revision and generation comparison refuse the predecessor; row/index/view changes are one owner transaction (resources §4.4/§4.6). |
| Unknown acknowledgement, then superseded/missing receipt | Read-back returns unavailable, never proof of non-commit or authority to republish (resources §4.6, ObservePublication). |
| View expiry versus retained history | Earliest retained-row expiry removes paging/index/receipt access; evidence/cursors remain ≤300 seconds and private history ≤600 seconds from last positive observation. Unexpired history alone gives no route authority. |
| Replacing rows/scopes at capacity | Retired physical records count toward 500 rows and 256 scopes until cleanup; cleanup precedes replacement or new work refuses capacity (resources §4.6/§5). |
| Child points to cleaned historical observation | Child cannot pin or recreate the row; current resolution is unavailable, while the fixed child value may remain (mediated route §4.2). |
| New route evidence versus changed target | Only evidence can advance by same-target CAS; fixed source/target/epoch/owner/access remains unchanged, and changed binding needs a newly admitted child (mediated route §4.1/§4.2). |
| Credentialless direct parent | Optional generation carries no invented credential identity; selected profile must explicitly admit the absence, and missing required evidence still refuses (mediated route §4.2). |
| Composition change, wrong instance revision or lost port | New configuration value requires review; mismatched instance/revision or absent private port refuses readiness. No composition database lifecycle or installation grant is inferred (composition contract). |

## Coordinator joins and limits

The coordinator must register the new domain, add DiscoveryCollection's
`source_connection` reference to the actual
`connectors.auth_bindings.Connection.connection_ref:String`, and insert optional
`connectors.discovery.MediatedRouteBinding` on that real Connection. Exact
unapplied patches accompany the handoff. This unit deliberately omits those auth
joins pending combined-model integration; it is not the final combined gate.
Composition's String instance coordinates need binding resolution rather than a
fabricated entity or unsupported value-field relation.

No selected ownership/lifecycle choice remains blocked. Injective qualified
identity allocation, immutable field assignment, canonical equality, evidence
provenance, actual clocks/capacity cleanup, durable cross-record atomicity,
current authority/CAS fencing, route target equality and installed live ports
remain explicit implementation predicates. ESS types identity/reference shape
and lifecycle causation; it does not execute those obligations. Public discovery
state remains exactly observed/stale/withdrawn; internal Retired is not exported.
