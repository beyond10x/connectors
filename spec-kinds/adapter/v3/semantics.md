# Isolated write generation

`connectors.adapter/v3` supplies the generator for the next GitLab slice. Its
frontend and generated runtime are exercised by an independent consumer fixture;
no production adapter selects v3 yet. The closed v1/v2 readers and v2 GET mapping
remain intact; old-reader refusal tests precede activating a v3 input.

The v3 root retains v2's read `operations`, `mappings`, pinned upstream source and
configuration schema and adds a required `writes` array. Each write entry contains
an ordinary operation descriptor and a separate write mapping. Its operation is
local-only, required-approval Write by this version's definition; there is no
field to opt into unguarded public dispatch. Duplicate IDs across read/write sets
are refused. Generated `descriptor.json` remains the read projection;
`private-descriptor.json` contains both sets with a separately derived revision.

The initial write mapping is intentionally bounded: method `put`, exact pinned
upstream operation/path, path/query parameters using the existing typed parameter
binding, and a JSON-object body map. Each body member has a tagged source:
`input` names one required scalar input property; `string`, `boolean` and
`integer` carry a constant of that exact JSON type. No arbitrary object, null,
array, interpolation, raw JSON, dynamic method or destination is supported.
Required local/upstream names and types must agree. Unknown properties, optional
input extraction, missing upstream requirements and incompatible scalar schemas
are generator refusals, not runtime guesses. The pinned upstream schema is read
without vendor refresh; source identity and reduced upstream artifacts are retained.

The initial codec subset is closed objects with every property required. Inputs
have string, signed 64-bit integer and boolean properties. Outputs additionally
support nested closed objects and explicit `anyOf` pairs of one supported type
and null. Arrays, schema references, arbitrary unions and unsupported keywords
are refused. String length/pattern and `date-time` assertions, integer bounds,
scalar `enum`/`const`, and descriptive title/description are retained. Validation
checks the full declared schema, including date-time formats; integer decoding
also refuses values outside the concrete signed range. An object has at most
256 fields; output lowering permits at most 256 object types per operation and
16 recursive levels. Generated names must remain distinct across the write set.

Upstream scalar unions admit only distinct, unconstrained string/integer/boolean
branches. Source constraints require exact input mappings; constants are checked
against their source schema. Context-bound path/query values have type String
and require an unconstrained source that admits strings. Integer-only sources
need a typed integer input mapping. Required upstream parameters/body members
cannot be omitted. Optional omitted members remain explicit in the coverage report.

Generation emits strict typed input/output values and a separate prepared-write
request. Native preflight runs before preparing the final immutable mapped
request; it cannot dispatch. Execution consumes that request and a non-clone
authenticated write capability once, then invokes the native finish binding.
The existing authenticated GET interface and generated public Adapter::invoke
do not gain write dispatch. A prepared request retains no host metadata or proof;
host composition holds its credential capability until private commit. The
operation's native owner decides which responses establish applied/refused/unknown.
Transport errors, invalid replies or generic retry middleware never imply a safe
repeat. There is no automatic write retry, including auth refresh or redirects.
`WriteOutcome::Applied(Result<T>)` keeps known effect application separate from
safe-result validation. A result validation failure preserves Applied with an
error. Refused and Unknown retain the native owner's classifications.

Generate and check both descriptor projections, strict codecs, mappings, retained
source identities and Rust outputs from the owning v3 input. Two isolated
generations must agree. A typed GET transport must be unable to execute the
generated write, and legacy invoke must refuse the write ID. Generation itself
does not establish approval, provider authorization or runtime acceptance.

`connectors-spec --generate` and the build executor dispatch explicitly by format.
The bundle includes `write-ess/`, `write-rust/`, `writes.rs`, the private descriptor,
reduced upstream source and write coverage, alongside the unchanged read projection.
The existing manifest envelope records the full authored input digest, pinned
source/tool identities and every portable output. Temporary ESS recovery anchors
are excluded. Owned-output, drift and symlink rules apply to both projections.

The executable fixture in `crates/connectors-spec/tests/write_generation.rs`
generates two isolated bundles, compares the v2 projection, builds a separate
consumer with the repository's locked dependency versions, and runs effect,
schema and ownership tests. Compile-fail consumers prove GET/write separation and
single-use requests/capabilities. An additional import fixture checks the selected
GitLab merge operation against its existing vendor pin; it performs no provider I/O.
