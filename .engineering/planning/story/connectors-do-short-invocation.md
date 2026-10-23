---
format: aep.planning-md/3
id: story:connectors-do-short-invocation
kind: story
status: draft
title: connectors invoke <connector>[/<instance>] <operation> is the one invocation, local or remote
relations:
- decomposes: epic:fluxplane-plugin-parity
revision: 14
---
## Problem

Posting one Slack message today takes six commands. Each one repeats `--connector`, `--instance`,
`--connection`, `--operation`, `--schema` and `--revision`
(`website/docs/connectors/slack.md:116-135`). An invocation names three things the CLI could look up
itself.

## Request

Operator, 2026-10-22, verbatim: "i would prefer it were: connectors do $connector/$operation and if
there were multiple $connector=slack then --instance because mandatory, otherwiuse i t can be
omitter".

## Outcome

One command invokes every operation, local or remote:

```sh
connectors invoke slack/work-1 chat.postMessage <<'JSON'
{"channel": "C0123456789", "text": "hello"}
JSON
```

- The first argument is `<connector>[/<instance>]` in the README's vocabulary (`gmail/private`,
  `gitlab/work`): `slack` is the connector, `work-1` the instance. `/<instance>` may be left out when
  the connector has exactly one instance; with two or more it is required, and leaving it out is
  refused by name, listing the instances.
- No output, help text or argument names the implementation that serves an instance. `catalog`
  is an internal adapter name (`architecture-decision-record:connector-vocabulary`), so it never
  appears where a connector name is expected.
- The connection, `--schema` and `--revision` are resolved from the instance's current admitted
  descriptor, so the caller does not pass them.
- Input is read from stdin when stdin is not a terminal; `--input-json` and `--input-file` stay
  accepted. A terminal stdin with no input flag is refused, not waited on.
- `--describe` prints the operation's description, its input schema and whether it is a read or a
  write, and invokes nothing.

## Endpoint

An endpoint is a property of the instance, given once. The instance table already carries it
(`README.md`, `[connectors.gitlab.instances.work]` with `endpoint = "https://gitlab.example.com"`).
Connecting an instance whose connector needs an endpoint asks for it, or takes it once as
`connections connect gitlab/work --endpoint https://gitlab.example.com`, and stores it in the
instance. No later command takes `--endpoint`; `invoke`, `describe` and every other command read it
from the instance. A remote service is an instance with a remote endpoint, reached as
`connectors invoke gitlab/ref-1 <operation>`.

## Removed, not deprecated

Operator, 2026-10-22: "i dont want to keep anything old". The release that ships this story removes:

- the service form `connectors invoke --endpoint <url> --token-file <file> --operation <op> --input
  <json>` (`apps/connectors/src/compat.rs:11`, `legacy_route` in `apps/connectors/src/main.rs`) and
  its siblings `describe` and `serve` under the same route, once their remote case is an instance;
- `operations invoke`, replaced by `invoke`.

It is a breaking change, named in its CHANGELOG entry with the replacement for each removed form.

## Open questions (operator to settle before this story is proposed)

- A write still needs its approval (`approvals prepare`, `clock-check`, `issue`). Should `invoke`
  run that flow for a write after one confirmation, or refuse a write without `--approval-file`?
- With two connections on one instance: refuse and list them, or pick the single `ready` one?

## Acceptance (draft)

- `invoke slack chat.postMessage` with one `slack` instance and input on stdin invokes it.
- `invoke slack chat.postMessage` with two `slack` instances is refused by a named code, and the
  refusal lists both instances.
- `connections connect gitlab/ref-1 --endpoint <url>` stores `endpoint` in the instance; a later
  `invoke gitlab/ref-1 <operation>` with no `--endpoint` reaches that url.
- `invoke`, `describe` and `operations` refuse an `--endpoint` flag.
- `invoke slack/<instance> <operation> --describe` prints the description, the input schema and
  read or write, and makes no provider call.
- `invoke` with a terminal stdin and no input flag is refused without waiting.
- `invoke` against a stale revision, or one the instance no longer admits, is refused, not
  re-resolved silently.
- `instances list`, `invoke` output and every refusal name the connector and instance only; none
  contains `catalog` as a connector name.
- `connectors invoke --endpoint …` and `operations invoke` are refused as unknown, and the
  CHANGELOG names their replacements.

## Command spelling

Operator, 2026-10-22, verbatim, superseding the `do` and `cop` spellings: "okay, make it this:
connectors invoke $provider(/$instance) $op | for example: \"connectors invoke slack/work-1
chat.postMessage << {...}\"".

Operator, 2026-10-22, verbatim, on keeping `invoke --endpoint` as a separate route: "whats the
difference between invoke slack and invoke homer + --endpoint ? its the same for gitlab ...
\"connectors invoke gitlab/work-1 --endpoint=gitlab/ref-1\" - only difference is one additional
argument. Also I am totally triggered by this additional \"catalog\" thing - please again, look at
\"[the dex CLI]\" - why is the CLI so much easier".

Reference: the `dex` CLI (read 2026-10-22, `internal/cli/*.go`) is `dex <tool> <noun> <verb>
<args>`, for example `dex slack send <channel|@user> <message>` and `dex gl mr ls`. Each tool has one
configuration and a `--url` override (`homer.go:2453`, `loki.go:695`, `prometheus.go:890`); no
command names an implementation.

`connectors do` and a `cop` binary name are not built. The release archives keep their contents.
