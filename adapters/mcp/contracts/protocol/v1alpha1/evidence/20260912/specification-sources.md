# MCP specification pin — 2026-09-12

[specification-source-hashes.json](specification-source-hashes.json) retains the exact
URL, uncompressed SHA-256 and byte length of every archived file of the two MCP
specification revisions this repository authors against. Each `vendor/*.gz` archive was
created with `gzip -n`; its decompressed bytes were compared byte-for-byte with the
fetched source before the digest was recorded.

| Revision | Role | Upstream tag | Commit | Archived files |
|---|---|---|---|---|
| `2026-07-28` | primary | `2026-07-28` | `5f5440bb26a62e2cf3440b92da5a667efa03b267` | 31 |
| `2025-11-25` | interoperability | `2025-11-25` | `38c84e9f93ad191d9eb26d92b945d17bd0efcaf3` | 23 |

## Which authority is pinned

The pinned authority is the upstream specification repository
`modelcontextprotocol/modelcontextprotocol`, at the release tag that carries each
revision's own date. It is not the sibling `../mcp` repository, and no `../mcp` commit is
pinned by this record. The two are different authorities: upstream publishes the
protocol, `../mcp` implements a consumer of it. Its `crates/b10x-mcp-types/src/lib.rs`
lines 15 and 17 declare `CURRENT_PROTOCOL_VERSION = "2026-07-28"` and
`LEGACY_PROTOCOL_VERSION = "2025-11-25"`; that is a consumer's selection, not the
specification.

The revisions are named by the operator's recorded plan, not selected here.
`initiative:complete-local-connectors` line 33 reads: "Pin that exact published MCP
revision here and implement outbound and inbound tools/resources/prompts over stdio and
Streamable HTTP, with version-specific 2026-07-28 and explicit 2025-11-25
interoperability, caller isolation and authenticated local verification of the
cloud-capable server profile." Line 32 names the sibling repository: "Expand ../mcp with
resources, prompts, servers, progress/cancellation, consumer-owned stdio and explicit
OAuth one-use exchange/publication seams." That milestone's publication is not a
prerequisite of this pin; nothing is published, fetched into a build, or depended on
here.

## Reproducing this record

Two checks, because the record makes two different claims. The first is about content —
the archived bytes are the bytes the digests describe. The second is about provenance —
each entry's `url`, `commit`, `path` and `archive` agree with the revision it claims. A
content check alone passes an entry repointed at the other revision's file, and this
record says the archives rather than the network are the retained authority from here on,
so nothing else would ever catch that.

```sh
cd adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912

# 1. content
diff \
  <(jq -r '.[] | [.sha256, .bytes, .archive] | @tsv' specification-source-hashes.json | LC_ALL=C sort) \
  <(for archive in vendor/*.gz; do
      printf '%s\t%s\t%s\n' \
        "$(gzip -dc "$archive" | sha256sum | cut -d' ' -f1)" \
        "$(gzip -dc "$archive" | wc -c)" \
        "$archive"
    done | LC_ALL=C sort) \
  && echo "verified $(jq length specification-source-hashes.json) archived specification sources"

# 2. provenance
jq -e -r '
  "https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/" as $prefix
  | [ .[] | . as $e
      | select(($e.url != ($prefix + $e.commit + "/" + $e.path))
            or (($e.path | startswith("docs/specification/" + $e.revision + "/"))
             or ($e.path | startswith("schema/" + $e.revision + "/")) | not)
            or ($e.archive != ("vendor/" + $e.file + ".gz")))
      | $e.file ] as $broken
  | ([ .[] | {revision, commit} ] | unique | length) as $pairs
  | if ($broken | length) == 0 and $pairs == 2
    then "provenance holds for \(length) entries: url = prefix + commit + path, path under its own revision, archive named after its file, one commit per revision"
    else error("provenance disagrees for \($broken | join(", ")) with \($pairs) revision/commit pair(s)")
    end
' specification-source-hashes.json
```

On 2026-09-12 these printed `verified 54 archived specification sources` and `provenance
holds for 54 entries: …`, both exit 0. The content check compares both directions: a
missing archive, an unrecorded archive, an altered digest and an altered byte length each
fail the diff with a non-zero exit. The provenance check fails, exit 5, naming the file,
when an entry's `path`, `commit` and `url` are repointed at the other revision — a
mutation the content check passes unchanged, because the bytes are untouched. The same
four rules are asserted in Rust by
`crates/connectors-build/tests/mcp_specification_pin_adversary.rs::manifest_provenance_fields_agree_with_the_revision_each_entry_claims`,
so a reader following this record runs the check the suite runs.

## What is archived, and what is deliberately not

Archived, per revision: every normative `.mdx` document under
`docs/specification/<revision>/`, and the normative `schema/<revision>/schema.ts`.

Not archived, with the reason:

- `docs/specification/<revision>/schema.mdx` — a generated typedoc rendering of
  `schema.ts` (726,080 and 442,839 bytes); the normative source it is generated from is
  archived instead.
- `schema/<revision>/schema.mdx` — the typedoc template that rendering is produced from
  (1,771 bytes at `2026-07-28`, 2,316 bytes at `2025-11-25`): frontmatter, `##` headings
  and `{/* @category … */}` markers only, with no prose and no occurrence of MUST, SHOULD
  or MAY. Both were fetched and read at their pinned commits before being excluded.
- `schema/<revision>/schema.json` — generated from `schema.ts`.
- `schema/<revision>/examples/**` — non-normative example payloads.
- `docs/specification/<revision>/server/*.png` — illustrations, not normative text.

The second entry is also a warning for whoever extends this manifest. The archive name is
derived by flattening the upstream path *below* its revision directory, so
`docs/specification/<revision>/schema.mdx` and `schema/<revision>/schema.mdx` both flatten
to `mcp-<revision>-schema.mdx` and cannot both be archived under the current naming rule.
Adding either one means extending the rule — carrying the `docs/` or `schema/` prefix into
the name — not overwriting the other.

Each of these remains retrievable at the same pinned commit through the URL prefix
recorded in the manifest. Adding one is a later append to this directory, not a refetch
of what is already pinned.

## Sections later contract statements will cite

Line numbers are lines of the archived uncompressed file.

| Later contract concern | Archived source | Section |
|---|---|---|
| Nouns, message shapes and error codes | `mcp-2026-07-28-schema.ts` (30, 450), `mcp-2025-11-25-schema.ts` (14) | `LATEST_PROTOCOL_VERSION`, `UNSUPPORTED_PROTOCOL_VERSION`; read the version-string trap below before citing either constant |
| Transport profiles | `mcp-2026-07-28-basic-transports-index.mdx` (27, 38, 52, 81), `mcp-2026-07-28-basic-transports-stdio.mdx`, `mcp-2026-07-28-basic-transports-streamable-http.mdx`, `mcp-2025-11-25-basic-transports.mdx` (22, 54) | Messages, Request Metadata, Cancellation, Backward Compatibility |
| Version negotiation and the 2025-11-25 seam | `mcp-2026-07-28-basic-versioning.mdx` (41, 80, 126), `mcp-2026-07-28-server-discover.mdx` (11, 33, 62), `mcp-2025-11-25-basic-lifecycle.mdx` (38, 167, 248, 265) | Protocol Version Negotiation, Extension Negotiation, Backward Compatibility with Initialization-Based Versions; Lifecycle Phases, **Version Negotiation** (167, the legacy `initialize` handshake this seam is against), Timeouts, Error Handling |
| Outbound auth lifecycle | `mcp-2026-07-28-basic-authorization-index.mdx` (47, 136, 254), `mcp-2026-07-28-basic-authorization-authorization-server-discovery.mdx`, `mcp-2026-07-28-basic-authorization-client-registration.mdx`, `mcp-2025-11-25-basic-authorization.mdx` (41, 74, 198, 352) | Roles, Authorization Flow Steps, Access Token Usage |
| Invocation results and errors | `mcp-2026-07-28-server-tools.mdx` (45, 76, 280, 738), `mcp-2025-11-25-server-tools.mdx` (38, 55, 449) | Capabilities, Protocol Messages, Data Types, Error Handling |
| Capability projection | `mcp-2026-07-28-server-resources.mdx` (39, 83), `mcp-2026-07-28-server-prompts.mdx` (39, 66), `mcp-2026-07-28-server-discover.mdx` (33) | Capabilities, Protocol Messages, Response |
| Mutation replay and cancellation | `mcp-2026-07-28-basic-patterns-mrtr.mdx` (25), `mcp-2026-07-28-basic-patterns-cancellation.mdx`, `mcp-2026-07-28-server-tools.mdx` (683), `mcp-2025-11-25-basic-utilities-tasks.mdx` (35, 121, 388) | Multi Round-Trip Requests, Stateful Tools, Behavior Requirements |
| Streaming, progress and subscriptions | `mcp-2026-07-28-basic-patterns-subscriptions.mdx` (12, 52, 116), `mcp-2026-07-28-basic-patterns-progress.mdx`, `mcp-2026-07-28-basic-transports-streamable-http.mdx` (107, 166) | Opening a Stream, Acknowledgment, Cancellation, Receiving Messages, Message Flow |
| Cloud-placement isolation and disclosure | `mcp-2026-07-28-basic-transports-streamable-http.mdx` (54, 650), `mcp-2026-07-28-basic-authorization-security-considerations.mdx`, `mcp-2025-11-25-basic-security_best_practices.mdx` | Security & Endpoint, Backward Compatibility |
| Composition provenance and statelessness | `mcp-2026-07-28-basic-index.mdx` (25, 182, 221, 320) | Messages, Statelessness, Auth, General fields |
| Human and machine discovery surface | `mcp-2026-07-28-server-discover.mdx` (62), `mcp-2026-07-28-server-utilities-pagination.mdx`, `mcp-2026-07-28-client-elicitation.mdx` | When to Call |

## The version-string trap this pin makes checkable

`2025-11-25` identifies the revision by its upstream directory and release tag, and the
revision does declare that string in its own prose: 18 of the 22 archived `.mdx`
documents of that revision carry an `<Info>**Protocol Revision**: 2025-11-25</Info>`
banner in their opening lines — line 7 of most, line 5 of
`mcp-2025-11-25-server-index.mdx` — and two more name it in running text,
`mcp-2025-11-25-basic-utilities-tasks.mdx` line 11 ("Tasks were introduced in version
2025-11-25 of the MCP specification") and `mcp-2025-11-25-client-elicitation.mdx` line
329. The four without the banner are `architecture-index`, `basic-security_best_practices`,
`changelog` and `index`. The banner is the revision's self-identification, not a
documentation path, and an earlier draft of this record claimed the opposite; that claim
was wrong and is withdrawn here rather than left for the eleven stories that read this
pin. The archived bytes are unchanged — only this description of them is corrected.

What is true, and is the trap worth carrying, is that the *machine-readable* places a
reader would look first disagree with the banner. `mcp-2025-11-25-schema.ts` line 14
declares `LATEST_PROTOCOL_VERSION = "DRAFT-2025-v3"`; the initialization examples in
`mcp-2025-11-25-basic-lifecycle.mdx` lines 61 and 107 carry
`"protocolVersion": "2024-11-05"`; and `mcp-2025-11-25-basic-transports.mdx` line 271
gives `MCP-Protocol-Version: 2025-06-18` as its header example. So a contract statement
that reads the version string out of that revision's schema constant or its examples gets
a different answer from the one its own banner gives. The primary revision carries no
such banner in any of its 30 archived `.mdx` documents; it states versions through
`mcp-2026-07-28-basic-versioning.mdx` instead.

The negotiable wire version string `2025-11-25` is stated by the primary revision:
`mcp-2026-07-28-basic-versioning.mdx` lines 41–68 show
`"supported": ["2026-07-28", "2025-11-25"]` in the `UnsupportedProtocolVersionError`
example, and lines 126–150 specify how a client detects an initialization-based server.
A later contract statement about the interoperability version string cites that, not the
`LATEST_PROTOCOL_VERSION` constant of the older schema. No inference is drawn here about
why upstream left that constant at a draft value.

## Limits

These are protocol declarations. They are evidence about the protocol, not evidence that
any MCP server, including a future Connectors one, implements any of it; a capability
listed in these documents is not implemented support. No MCP connection was made, no
executable was added and no ESS model, entity, relation, transport selection or
capability profile is declared by this record. Transport and capability profile selection
is a separate authoring obligation that reads this pin.

These are retained research bytes, not an adopted upstream build dependency, completed
source/license packaging or a vendored library. The bytes were fetched over the network on
2026-09-12 from the two tag commits above; from here on the archives, not the network, are
the retained authority, and no normal build refetches them.
