# Local execution audit binding

The host provides a private SQLite implementation of the
[execution audit contract](../contracts/service/audit.md) in
`crates/connectors-host/src/local/audit.rs`. It is a prerequisite for governed
GitLab writes. It does not yet change CLI response audit status or enable any
provider write. Caller policy, approval spending, credential admission and the
complete dispatch coordinator remain separate implementation work.

## Storage and identity

The port uses the existing private local metadata authority, lifecycle lock,
SQLite WAL and full synchronization. Admitted audit anchoring alone installs
migration five, including earlier recognized migrations needed for attempt
references. Ordinary setup retains version three; passive reads never migrate.
Mutation preparation continues to work on versions four and five. Migration
bytes one through four, authority UUID and retained connection identities are
unchanged.

Each host owner allocates a UUID as its opaque public `audit_ref`. The public
identity is the exact pair `{instance, audit_ref}`. The private key follows the
contract's literal `connectors.execution-audit-record/v1` tag, two unsigned
big-endian 32-bit UTF-8 byte lengths and values, then unpadded base64url encoding.
No normalization or delimiter interpretation occurs; decoding refuses alternate
or trailing representations. The private key is never a public correlation id.

An immutable anchor records only host-verified bounded audit facts. The typed
aggregate has no field for raw input, provider output, credentials, approval
proofs or native target details. Its optional connection and attempt references
must exist and agree with the instance and with each other. They grant no use
authority and do not own deletion. Revocation preserves these references.

The selected contract bounds apply to UTF-8 bytes. The stored JSON aggregate
plus private key is limited to 4 KiB. Per-instance capacity is configured from
1 through 100,000 records; lowering it retains existing records and permits their
final append while refusing new anchors. There is no expiry, deletion or eviction
API. Foreign keys have no delete cascade. Timestamp fields encode host-observed
Unix milliseconds as nonnegative signed 64-bit integers; they describe audit
events and supply no trusted authorization, expiry or monotonic-order proof.

## Acknowledgements and recovery

`Store::anchor` accepts facts from trusted host resolution and allocates the ref.
An admitted execution requires an admission-stage anchor, verified principal and
activity; invoke also requires an operation. Describe has no operation or attempt.
Early-refusal fields may be present only when independently verified at that hop;
their provenance remains the coordinator's responsibility. The storage port is
an internal Rust embedding API, not a request admission codec.

Only definite commit acknowledgement of an admitted anchor returns an opaque,
non-Clone, nonserializable `Admission`. Early refusal returns a reference alone.
Failure or acknowledgement uncertainty returns neither a reference nor an
execution receipt. Internal lookup can observe a committed record whose response
was lost, but cannot reconstruct its original receipt.

`Store::confirm` consumes that receipt and compares its original process, exact
anchor facts and metadata authority with the retained unfinalized record. This
confirms audit only. The eventual coordinator must enforce current caller policy,
approval, credential and connection authority, and the mutation gate separately
before provider or leaf dispatch. No store method sends or retries provider work.

After observing its local outcome, the owner allocates a final-observation UUID
before the first append and retains the exact bounded outcome, safe code and
timestamp. `Store::append` borrows those fields and atomically appends at most one
observation. Repeating the exact UUID and semantic fields returns the retained
acknowledgement; changing any field or the UUID conflicts. The first final fact
is immutable. Unknown business effect can still be a fully recorded observation.

Append is acknowledged separately from anchor, mutation settlement and provider
work. A failure leaves the caller's live result intact. A lost acknowledgement
may have committed the observation; exact internal lookup and append retry can
recover only audit knowledge. Gateway and execution-hop records remain separate
even when their opaque public refs happen to match. Lookup never interprets
unavailable, malformed or an older uninstalled schema as absent history.

## Verification boundary

Real SQLite fixtures cover concurrency, rollback and ambiguous acknowledgements,
four abrupt process exits, exact identity and instance isolation, immutable final
observations, capacity, byte bounds, reference retention and migration continuity.
Simulated effects test the receipt ordering only. Dedicated provider evidence and
full production dispatch integration remain required by the owning GitLab work.
Commands and input identities are retained in the
[verification receipt](evidence/local-execution-audit-20260910/README.md).
