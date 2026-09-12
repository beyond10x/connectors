# Helm release reads through the local Kubernetes CLI

`story:kubernetes-helm-release-reads` covers the 25 release-read command sites in
`docs/evidence/recent-adapter-usage-20260909/actions.csv` that land on execution
semantics this repository already has: reads of Helm's release storage objects
through the Kubernetes API. No local `helm` binary and no OCI registry is
involved, so none of this touches `decision-blocker:helm-execution-family`, which
holds the other 106.

## What ran

| | |
|---|---|
| command | `cargo run --locked -p connectors-build -- gate --msrv` |
| branch | `wave/helm-reads-20260912` |
| result | **33 gate steps, 71 test targets, every step exit 0** |
| log | [gate.log](gate.log), ending `GATE-EXIT:0` |

## Pinned upstream sources

Sixteen Helm sources with URL, uncompressed SHA-256 and byte length, across
**both** lines — v4.3.0 at `bec5b06` and v3.22.0 at `144ca65` — because a cluster's
Secrets may come from either and every field this binding reads agrees across
them. An adversary re-derived all sixteen and checked roughly fifty cited lines
against both archives.

## What the evidence is, and what it is not

Fixture cluster. **No real cluster has answered these operations.** The website
entry says `Read-only observation; dedicated sandbox acceptance open` for exactly
that reason.

## Two findings worth keeping

**A digest must name the bytes it covers.** The manifest digest went through three
statements in one commit — over the JSON encoding, over the text, over the trimmed
text — and only the third round made `sha256sum` over a stored document reproduce
it. The pinned source settles which bytes: Helm writes every document, the last
included, as `"---\n# Source: %s\n%s\n"`, so a rule that strips the newline at
end-of-input breaks the final document of every real manifest.

**An unsalted digest confirms a guess.** The recorded-value digest bounds
disclosure of a value nobody can enumerate and does not protect one an attacker
can. The contract now says so, for both digests, and names the operator decision
that follows: do not grant `redacted_content` to a reader who must not confirm a
guessable credential.

## A test that fails only when the machine is busy

The first gate run was red on one deadline test in `connectors-host`, a crate this
wave does not touch. It passed three of three alone and the branch gates green on
a quiet machine. Recorded rather than re-run silently: a suite that is red only
under concurrent load gets called flaky rather than fixed.
