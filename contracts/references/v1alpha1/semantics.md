# references/v1alpha1

**Status:** proposed, not implemented. No runtime serves it. The declarations it reads are specified
in `beyond10x/connectors-definitions` (`spec/domains/references.yaml`) and validated there on Jira and
GitLab; `architecture-decision-record:provider-definitions-and-runtimes` places them in the connector
definition.

## Purpose

A person pastes `DEV-1234`, `https://acme.atlassian.net/browse/DEV-1234` or
`https://gitlab.com/group/app/-/merge_requests/42` into a chat, a ticket or a prompt. A consumer wants
to know which connector item that is and to show it in one line or one card, without code of its own
per connector. This contract gives every connector that declares references the same three answers:

| Operation | Input | Answer |
|---|---|---|
| `references.extract` | text | the references found in it, each with its kind, its matched span and its named values (`key`, `site`, `iid`, …) |
| `references.resolve` | one extracted reference | the item it names, read through the connector's own read operation |
| `references.render` | a resolved item and an output name | the named output (`summary`, …) as text |

A runtime exposes the three as one interface (a Rust trait, a Go interface); a connector that declares
no references answers `extract` with nothing.

## What a connector declares

Per reference kind, in the connector definition:

| Field | Meaning |
|---|---|
| `match[]` | how the kind is recognized: `url`, an RFC 6570 URI template matched in reverse, whose variables are the reference's values; or `pattern`, a regular expression with named groups, for references that are not URLs |
| `resolve` | the operation key that reads the item, and which values bind to which of its parameters |
| `outputs` | named templates over the resolved item: literal text with `{name}` (a reference value) and `{/json/pointer}` (a field of the item) |

A `views` declaration names a field selection of the resolved item (`card`, `row`), each field a
label and a JSON pointer, for a consumer that renders it itself.

## Guarantees

- **No new authority.** `resolve` invokes a declared read operation of the connector through the
  host, so it is admitted, approved and audited as that operation is; extracting grants nothing and
  calls nothing.
- **Bounded matching.** Patterns are matched by a linear-time engine (Rust `regex`, RE2); a pattern
  that engine refuses is refused when the definition is validated. Input text is bounded by the host
  before extraction.
- **Values are checked before use.** A matched value is bound only to the parameter `resolve` names;
  it is percent-encoded as that parameter's location requires, never spliced into a path segment of
  its own choosing.
- **A URL reference matches only the connection's own endpoint.** `{site}` or `{hostname}` in a `url`
  template is compared with the configured instance; a URL for another site is not resolved through
  this connection.
- **Outputs are data.** A template selects and joins fields; it computes nothing and runs nothing. A
  missing field renders as empty, never as an error that leaks the item's shape.

## Not covered

Body rendering (ADF, HTML, markdown to text) is a separate, named renderer the runtime implements; a
view may name one for a field. Reference discovery across connectors (which connector a bare key
belongs to) is the consumer's choice of connection, not this contract's.
