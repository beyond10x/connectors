# Auth model closure verification

Specification: core-model-closure-20260909. Unit base:
`8a5cf563fc717fd4b23e4dd7d470d0c967f39461`; branch
`specs/model-auth-20260909`. This is specification/model authoring only.

The four owning contracts select immutable qualified AuthProfile records,
Connection Live/Revoked administrative identity, private initial binding allocation,
Acquisition one-use consumption/Completing and terminal retention, and scoped
CustodyVersion identity with guarded byte reclamation. Four entities are added in
[auth_bindings.yaml](../../../ess/domains/auth_bindings.yaml). No backend, callback
codec, new provider profile or current public reader implementation is added.
The [sixteen scenario dispositions](scenario-audit.md) are manual textual/model
checks, separate from compiler executions below.

Executed with pinned ESS 0.20.0 at
`/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess`, with TMPDIR
inside the task tree's `.local/model-closure-20260909/tmp`.

| Check | Executed result |
|---|---|
| `ess specify validate --path ess` before edits | exit 0; 14 files valid; [log](baseline-validate.log) |
| `ess specify compile --path ess --out <scratch>/baseline.json` before edits | exit 0; 218 declarations; [log](baseline-compile.log) |
| Unit validate/compile with new domain registration | exit 0; 15 files, 252 declarations; [validation](unit-validate.log), [compilation](unit-compile.log) |
| Unit plus exact available coordinator reference patches | exit 0; 15 files, 252 declarations; [validation](unit-joins-validate.log), [compilation](unit-joins-compile.log) |
| Existing model compatibility inspection with jq | Eleven existing identities, lifecycles and old field types preserved; every old named type unchanged; [result](identity-type-check.json) |
| Repeated canonical compilation | byte-identical `unit-joins.json` and `unit-joins-repeat.json` with cmp; [repeat log](unit-joins-repeat.log) |
| `ess generate --path <unit> --kind schema --out <scratch>/join-schema` repeated | exit 0; 266 artifacts each; recursive diff reports no changed bytes; [first log](join-schema.log), [repeat](join-schema-repeat.log) |
| Wrong instance type fixture | intentional isolated String→Uuid mutation; exit 1, type_mismatch against existing ServiceConfiguration; [full refusal](negative-instance-type.log) |
| Missing consume causation fixture | intentional removal of ConsumeAcquisition's move; exit 1, missing_causation and consequent unreachable wrong-state branch; [full refusal](negative-consume-causation.log) |
| Shared-file patch and whitespace | `git apply --check <scratch>/shared-joins.patch` and `git diff --check` exit 0 against the unit base |

The unit tree is a scratch copy of the **real existing ESS root**, with this auth
file, domain registration and the proposed available coordinator joins. It uses
no guessed/stub entity. `ess/system.yaml` and the existing domains are unchanged in
the tracked worker checkout. Thus these results are **unit** validation, not a
claim that the tracked partial checkout or the final integrated root has passed.
The new cross-worker `Connection.mediated_route` field is an explicit coordinator
patch pending its real discovery type; it is not included in these unit counts.

Coordinator patch adds the auth domain registration; a CredentialGeneration→Connection
reference through the existing String connection_ref; an optional authored
OperationDeclaration.requires_auth value; and updates stale owner comments. Existing
CredentialGeneration fields, identity and Captured lifecycle remain unchanged.
No generation-to-custody foreign key pins secret bytes or mutates immutable captures.
DispatchAdmission keeps its existing generation link without duplicate binding fields.

The schema projections are private model artifacts in scratch, not public codecs
or committed/generated adapter bundles. Native Optional projection still means
omission, not the public protocol's required-null fields. No OpenAPI serving surface
is selected by this domain, and no OpenAPI generation/HTTP execution claim is made.

Actual durable uniqueness/consumption, truthful acknowledgements, field assignment,
profile qualification and exact identity equality, current authority/time, custody
byte capture/erasure, valid-use/retirement fencing, generation publication and
revocation/final-dispatch atomicity remain explicit UNMAPPED binding predicates.
The owning prose now selects ownership/cardinality/lifecycle; these execution
obligations are not deferred decisions about what those relations mean. The
coordinator runs the combined Rust boundary/gate and obtains independent review.
No runtime tests or full build were run by this unit.
