# Proposed isolated write generation

`connectors.adapter/v3` is selected for the next GitLab slice. It is not yet an
implemented frontend. The closed v1/v2 readers and v2 GET mapping remain intact;
old-reader refusal tests must precede activating a v3 input.

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

Generate and check both descriptor projections, strict codecs, mappings, retained
source identities and Rust outputs from the owning v3 input. Two isolated
generations must agree. A typed GET transport must be unable to execute the
generated write, and legacy invoke must refuse the write ID. Generation itself
does not establish approval, provider authorization or runtime acceptance.
