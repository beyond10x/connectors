# CQL admission for body-bearing page search

This is the selected structural grammar and rewriting contract for
[confluence-cql](semantics.md#7-body-bearing-confluence-cql-search).
It is proposed adapter semantics, not a parser implementation or a claim that ESS
validates CQL. Native field, function, date and text-search meaning stays with the
configured Confluence service. A local parser establishes the expression boundary
needed to conjoin trusted scope; it does not evaluate the query or infer a grant.

The official [keyword](https://developer.atlassian.com/cloud/confluence/cql-keywords/)
and [operator](https://developer.atlassian.com/cloud/confluence/cql-operators/)
references describe Boolean clauses, grouping, ordering, comparisons, membership
and text predicates. [Fields](https://developer.atlassian.com/cloud/confluence/cql-fields/)
and [functions](https://developer.atlassian.com/cloud/confluence/cql-functions/)
retain their provider meaning and supported combinations. Exact source bytes are
retained in the [provider manifest](../../../../../docs/evidence/datasource-semantics-20260908/provider-source-hashes.json).
The grammar/bounds below are receiver selections based on those constructs; the
references do not supply a verified Rust parser or prove a deployed binding.

## Structural grammar

Keywords are ASCII case-insensitive whole tokens outside quoted strings. Consume
the entire input, allowing only whitespace after the optional final ordering:

```ebnf
query      = expression, [ order_by ];
expression = conjunction, { "OR", conjunction };
conjunction = negation, { "AND", negation };
negation   = { "NOT" }, ( "(", expression, ")" | predicate );
predicate  = field, comparison, operand
           | field, [ "NOT" ], "IN", "(", operand, { ",", operand }, ")";
comparison = "=" | "!=" | ">" | ">=" | "<" | "<=" | "~" | "!~";
operand    = scalar | function;
function   = word, "(", [ operand, { ",", operand } ], ")";
field      = word, { "[", scalar, "]", [ word ] };
scalar     = word | quoted;
order_by   = "ORDER", "BY", sort_key, { ",", sort_key };
sort_key   = field, [ "ASC" | "DESC" ];
```

`word` is a nonempty UTF-8 token excluding whitespace and the delimiters
`()[],= !<>~"'\\;`. Reserved Boolean/ordering keywords cannot stand for an
unquoted word value. Dotted native names remain one word; a word after `]` in a
field must start with `.`. This admits native property paths without interpreting
their values as caller-controlled scope. A quoted token uses matching single or
double quotes; a backslash consumes the next Unicode scalar inside that token.
Retain the original bytes of every escape and value. Unterminated quotes/escapes,
unbalanced delimiters, stray trailing tokens and comment forms outside quoted
tokens (`#`, `//`, `/*`, `*/`, `--`) are refused. Apart from HT/LF/CR whitespace,
ASCII control characters are refused. Semicolons are never statement separators.

This grammar distinguishes syntax from provider validation. A structurally valid
unknown field/function or unsupported field/operator combination may be rejected
by the admitted metadata request as safe `invalid_input`; it never causes a body
request or raw provider-error disclosure. The adapter does not replace native
queries with a fixed field/function whitelist. Syntax outside this selected
grammar requires a reviewed binding extension before advertisement.

Limits: 8 KiB initial CQL, 2,048 syntax nodes, nesting depth 64 (including function
calls and NOT nesting), at most 16 sort keys, and at most 16 KiB final rewritten
CQL. The complete encoded provider request target is at most 64 KiB. Exceeding a
bound is `invalid_input` before dispatch. These work bounds are receiver policy;
they do not bound the provider's internal index work.

## Trusted conjunction and continuation

The receiver retains exact byte spans for the parsed predicate and ordering.
It constructs `type = page AND space IN (<keys>) AND (<original predicate>)`
and appends the original ORDER BY span when present. The positive fixed clauses
come first. Whitespace and syntax inside the original predicate/order are not
reserialized, and quoted text containing `ORDER BY` is never split into an order.
The provider receives the result as one percent-encoded `cql` parameter.

Keys come only from the admitted ID/key configuration mapping, ordered by the
canonical numeric space IDs. Quote each entire key with double quotes, escaping
backslash as `\\` and double quote as `\"`; reject control characters and a key
whose exact literal meaning cannot be established by the selected binding.
No caller space assertion contributes to this list. An empty allowed set refuses
the read before dispatch. Stale/reassigned key mappings cannot grant content:
the subsequent v2 body request filters by the admitted stable space IDs.

Continuation carries the same original predicate, constructed scope, ordering,
configuration/auth context and limits. A provider link supplies only its validated
cursor coordinate under records §4.3. Relative date functions continue to have
provider-native evaluation; the contract does not turn a live CQL search into a
frozen result snapshot.

## Required parser vectors

| Input or change | Required decision |
|---|---|
| `label = ops OR title ~ "error" ORDER BY created DESC, title` | Wrap the complete OR expression; append both original sort keys outside the fixed conjunction |
| `NOT (label = private OR title ~ "ORDER BY")` | Preserve NOT/grouping and treat quoted text as a value, not a suffix |
| `created >= now("-4w") AND creator = currentUser()` | Preserve native functions and their provider interpretation |
| `text ~ "\\\"quoted phrase\\\""` | Keep the accepted text-search escape bytes unchanged |
| `space = forbidden OR type = blogpost` | Remains inside the conjunction; it cannot bypass fixed allowed-space/page predicates |
| Extra `)`, a second ORDER BY, a comment or trailing statement | Refuse syntax before provider dispatch |
| A configured key contains a quote/backslash or begins with a digit | Use one exact quoted literal or refuse an unverified binding; never concatenate raw key text |
| Provider next link changes CQL, scope, expansion, origin, path or limit | Refuse continuation; retain no replacement authority |

These are declared expectations for future parser/provider fixtures. This document
and the ESS selection shape do not execute or independently verify them.
