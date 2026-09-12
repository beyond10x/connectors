# Kubernetes Helm release reads/v1alpha1

**Status:** implemented native binding. Shared wire envelopes, pages and
admission remain in
[service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md).

A Helm release is stored in the cluster as one Secret per revision. This binding
reads four things from those Secrets — the revision history, the revisions
currently marked deployed, the values recorded for one revision and that
revision's rendered manifest — and nothing else. It installs, upgrades,
uninstalls and rolls back nothing, renders and lints no chart, and reaches no
registry. Every shape it relies on is pinned line by line in the
[provider source evidence](evidence/20260912/provider-sources.md) against Helm
v4.3.0 and v3.22.0, which agree on every field read here.

## Not a resource kind

`resource_kinds` stays the closed enum `pods`, `services`, `deployments`,
`endpointslices`. A release read is its own operation with its own admitted
target; it is not reachable through `resources.list`, it does not add a kind a
caller may select, and the configured kinds do not affect it in either
direction. There is no general Secret read: the only objects these operations
address are release Secrets selected by Helm's own `owner=helm` plus `name`
labels, or one exact object named `sh.helm.release.v1.<release>.v<revision>`.

## Selection

| Operation | Input | Provider request |
|---|---|---|
| `helm_releases.history` | `{namespace, release, limit, cursor?}` | `GET .../namespaces/<ns>/secrets?labelSelector=owner=helm,name=<release>&limit&continue` |
| `helm_releases.status` | `{namespace, release, limit, cursor?}` | the same with `,status=deployed` |
| `helm_releases.values` | `{namespace, release, revision, limit}` | `GET .../namespaces/<ns>/secrets/sh.helm.release.v1.<release>.v<revision>` |
| `helm_releases.manifest` | `{namespace, release, revision, limit}` | the same single object |

`namespace` must be inside the configured namespace scope. `release` must match
Helm's own release-name rule, at most 53 bytes; a name Helm would refuse is
`invalid_input` before any request rather than a lookup that cannot succeed.
`limit` is 1–100 for the collection reads and 1–500 for the projections.
`revision` is an integer in 1..=2,147,483,647 — Helm's own `Version int` domain
— naming one stored object. The bound is enforced by this binding and not only
by the host's input validation: a bound that only an outer layer checks is not
enforced by the layer that publishes it.

Input is closed. There is no all-namespaces mode, no cross-release listing, no
label selector a caller supplies and no free-form field selector. `helm list`
without a namespace is the separate admitted scope of
[auth §4.4](../../auth/v1alpha1/semantics.md); several per-namespace allows do
not synthesize it.

## Revision records

Each item of `helm_releases.history` and `helm_releases.status` is read from one
Secret's own name, type and labels. The payload is not decoded for these two
operations, so a revision record never touches stored release content.

```json
{"source_secret":"sh.helm.release.v1.api.v3","source_revision":"73",
 "namespace":"fixture","release":"api","revision":3,"status":"deployed",
 "created_at_unix_s":null,"modified_at_unix_s":1757000300}
```

`source_secret` is the exact object each observation came from. `status` is one
of Helm's nine values; a tenth string refuses as `upstream_protocol` rather than
being folded into `unknown`. `revision` comes from the `version` label and must
be canonical positive decimal, and the object's name must equal the key those
labels imply — an object whose name and labels disagree is not a record this
binding will read. `created_at_unix_s` and `modified_at_unix_s` come from label
writes on separate Helm code paths, so either may be absent; absent means not
recorded, never zero.

An object carrying Helm's labels whose type is not `helm.sh/release.v1` refuses
the page. It is deliberately not skipped: Helm's own driver logs and skips an
undecodable row, which is sound for a client that reports nothing about
completeness, and unsound here, where a skipped row would leave `complete:true`
claiming a history it did not observe.

`helm_releases.status` reports every revision the store marks deployed and does
not choose among them. More than one is possible — Helm's own source calls that
a concurrently corrupted store and resolves it by sorting — so an empty result
means no deployed revision and a result of length two means exactly that.

## Disclosure

Recorded values routinely carry credentials, and a rendered manifest contains
the bodies of every Secret the release applied. Neither is disclosed.

`helm_releases.values` returns one item per node of the recorded value
structure: its dotted path, its JSON shape and a SHA-256 over the canonical JSON
of its value. `helm_releases.manifest` returns one item per document of the
rendered text: its position, its UTF-8 byte length **as stored**, and a SHA-256
over exactly those stored bytes. Nothing is stripped from a document — Helm's
writer terminates every document with a newline, including the last
(`action.go:358,476` in v4.3.0, `action.go:183` in v3.22.0, both pinned), so a
document whose trailing newline were removed would not reproduce under
`sha256sum` and the final document would be framed differently from every
other. No stored scalar and no manifest byte appears in either result, at any
depth, for any shape. There is no configuration, input or
mode that discloses one; the projection **is** the disclosure, and a caller that
needs the literal is asking for something this binding does not do.

The two digests deliberately cover different bytes, because they exist for
different things. `content_digest` exists to be **reproduced**: a reader holding
the rendered document must get the same value from `sha256sum`, so it covers the
document exactly as stored between its separators and nothing else. `value_digest` exists to be **compared**:
a recorded value is a JSON value, not a byte string, so it covers the canonical
JSON of that value and two equal values therefore digest equally across
revisions and across releases. Neither is a digest of the other's bytes, and
neither is salted.

**Both digests are unsalted, and an unsalted digest confirms a guess.** Each is
a plain SHA-256 over a byte sequence anyone can compute, so a holder of either
projection can test a candidate offline and learn whether it is correct. This
is a property of the comparison and reproduction they exist for, not an
oversight: a salted digest would do neither.

For `value_digest` the guess is a recorded literal. For `content_digest` the
guess is a whole rendered document, which sounds harder and is not always:
a chart's templates are usually public, so the unknown part of a rendered
document may be only the value injected into it — and `bytes` publishes that
document's exact length, which narrows the search further. The manifest
projection is therefore no stronger than the values projection, and in a public
chart it can be weaker.

What both projections bound is disclosure of something an attacker cannot
enumerate; what neither does is protect a short, low-entropy or
already-suspected value. A consumer must treat either result as it would treat
the key names of a values file — safe to hold, not a secret — and an operator
who needs a guessable credential kept from a reader of this output must not
grant that reader `redacted_content`.

The path is a display projection, not a selector: a recorded key containing `.`
or `[` produces a path that cannot be parsed back unambiguously. The digest is
over the canonical JSON of the value, so two equal values digest equally across
revisions and across releases, which is what makes the projection useful for
comparison without disclosure.

Advertisement follows the same rule, one step earlier. The composition's
`helm_release_reads` selects `off`, `metadata` or `redacted_content`; the
default is `off`. Under `off` no release operation is advertised at all, under
`metadata` only the two revision reads are, and the two content projections are
advertised only under `redacted_content`. An unadvertised operation is not
described, and the binding repeats the check at dispatch so that a cached
description grants nothing.

## Decoding the stored payload

The Kubernetes API base64-encodes the bytes Helm stored, and those bytes are
themselves Helm's base64 of the gzipped JSON release, so the decode is two
base64 passes followed by a gzip pass that runs **only** when the `1f 8b 08`
magic is present. Helm's own decoder treats a missing magic as an uncompressed
body written before compression existed, and so does this one; it is not a
malformed payload.

The decoded body is bounded at 1 MiB and refuses with `capacity` beyond it,
before the JSON is parsed, so a compressed payload cannot expand without bound.
A single recorded container with more than 65,536 entries refuses the same way.
The object is validated as a release record — type, owner, name, labels — before
its payload is decompressed, so a foreign object under that name is never
expanded. `metadata.namespace` and `metadata.resourceVersion` are read the same
way: absent is an omission, and present in a shape this binding has not
established is not. A numeric `metadata.namespace` does not fall back to the
admitted selection, because that would report the caller's own input under a
field this contract says is observed, and a numeric `metadata.resourceVersion`
does not become a null `source_revision`, because that would report an absence
nothing established. Both refuse. The decoded body must itself be a JSON object: a payload decoding to
`null`, an array or a scalar refuses, because projecting it would report an
empty, complete recorded-value set over a body nothing observed, which is the
same unsound completeness claim this contract refuses one layer up. Where the
body carries its own `name`, `namespace` or `version` — each `omitempty` in the
pinned source, so absence is not disagreement — they are cross-checked against
the object's labels, since the provenance is built from those labels and a body
naming a different release must not be served under them.

## Completeness and cursors

The two collection reads carry the provider's own continuation: a returned
cursor is bound to the operation, namespace, release, deployed selection, limit
and connection partition, and is unreadable under another. `complete` is true
exactly when the provider issued no continuation.

"Issued no continuation" means the key is absent or empty, and nothing else. A
continuation the provider sent in a shape this binding has not established, and
one whose issued cursor would exceed the bound the output schema declares for
`next_cursor`, are both continuations: the page reports `complete:false` with a
null cursor. That is truthful on both counts — the selection was not observed
in full, and this binding cannot hand back a way to continue it. Reading either
as "no continuation" would claim the whole selection was observed while the
provider said otherwise. The collection's own `resourceVersion` has no such
vocabulary — a page cannot say "the revision is unreadable" — so a present but
unreadable value refuses instead of being attributed as no revision observed. A page is an observation of a
mutable collection, and `complete` does not mean the history is the whole
history Helm ever wrote — a storage driver prunes by max history, and this
binding observes what is stored now.

The two projections read one object and have no provider continuation to carry.
They return `next_cursor:null` always, and report a projection larger than the
requested page as `complete:false`. No cursor is manufactured for a collection
the provider cannot continue.

A recorded node is also omitted, and the page likewise reported `complete:false`,
when its path falls outside the bounds this operation publishes for
`items[].path` — an empty recorded key against `minLength 1`, or a concatenation
past `maxLength 1024`. Such a node is dropped with its subtree rather than
emitted, because a result that fails the operation's own published schema is not
a bounded refusal: the local runtime terminates the adapter child on that
validation failure. Dropping the whole read instead would lose a release that is
otherwise entirely readable, so the loss is reported through the mechanism that
already exists for it. `complete:false` provides no absence evidence, so a caller
must not read a missing path as an unset value.

## Refusals

| Case | Result | Provider requests |
|---|---|---|
| namespace outside the configured scope | `forbidden` | none |
| release name Helm itself would refuse | `invalid_input` | none |
| operation not admitted by `helm_release_reads` | not advertised; `forbidden` at dispatch | none |
| credential may not read that namespace's release Secrets | `forbidden` | one |
| release with no stored revisions | success, `items:[]`, `complete:true` | one |
| revision with no stored object | provider `not_found` | one |
| labelled object that is not a release Secret | `upstream_protocol` | one |

A scope refusal and an RBAC denial both surface as `forbidden`, and they are
distinguished by observation rather than by code: the scope refusal issues no
provider request at all. Neither is ever reported as an empty history. This
binding performs no SelfSubjectAccessReview pre-check; the exact target tuples
these operations would check are
`(list,"","v1","secrets",<namespace>,"","")` for the collection reads and
`(get,"","v1","secrets",<namespace>,"sh.helm.release.v1.<release>.v<revision>","")`
for the projections, and stating them is not a claim that they are checked.

## Evidence and obligations

The recorded usage this serves is in
`docs/evidence/recent-adapter-usage-20260909/actions.csv`: `history` 19 sites,
`get values` 3, `status` 1 and `get manifest` 1, which is 24 of that family's 25
release-state sites. The twenty-fifth is `helm list` (1 site), a listing of every
release in a scope rather than of one release's revisions; it is not implemented
here and needs its own selection and admission. Chart rendering, linting,
packaging and registry access are a local tool family this binding does not
implement, and release mutation is excluded outright.

Unresolved, and recorded as such in
[the model](../../../spec/ess/domains/helm.yaml): the resource identity of a
manifest document, because this binding has no YAML reader and a line-based
extraction would be a guess; whether the separator split matches a YAML reader's
document boundaries; the relation between a revision and the cluster resources
it manages; whether an absent recorded path means unset or defaulted, since
chart defaults live in the chart this binding does not read; and whether a
release name identifies one release beyond its namespace.
