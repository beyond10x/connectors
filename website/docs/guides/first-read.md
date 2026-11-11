---
title: A first read
sidebar_position: 7
description: How a connector's first read runs, how long a connection stays validated, and what the catalog provider does with a read that is rate limited.
lede: A read names an instance and an operation; validation lasts 60 seconds, and a rate-limited read is sent once more only when the delay fits.
source: docs/local-catalog-provider.md, adapters/catalog/src/local.rs, adapters/catalog/src/lib.rs, contracts/cli/v1alpha1/semantics.md
---

# A first read

Every provider page under [Connectors](../connectors/index.md) has a "A first read" section with an
example input. What is the same for every provider is here.

## Examples

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in the
repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left out
of the examples. A command that needs an account at the provider is shown without output.

## Read

```sh
connectors invoke <connector>/<instance> <operation> --input-json '<input>'
```

A read takes the instance's one connection and its current schema and revision when none is named;
`invoke --describe` prints the operation's input schema, schema and revision without calling the
provider. Every list returns one page per call unless the page says otherwise.

## Validation

Validation lasts 60 seconds after connect. After that a read is refused with
`next_action: revalidate_connection` until `connections revalidate` renews it; renewing needs no
credential. Take the connection and its revision from `connections list`, as in
[Reconnect a connection](./reconnect-a-connection.md#find-the-revision):

```sh
connectors connections revalidate --connector <connector> --instance <instance> \
  --connection "$connection" --expected-revision "$revision"
```

A provider that refuses the stored credential makes the connection stale instead, which
revalidation cannot help: see [Reconnect a connection](./reconnect-a-connection.md).

## Rate limits

A read answered `429` is sent once more when the delay the provider names fits before the call's
deadline; otherwise it is `rate_limited` with `retry_after_seconds`. On Google, a quota `403` is
`rate_limited` at once, with no delay and no second request. A write is never sent again.

What the catalog provider does exactly, from `adapters/catalog/src/local.rs` (`retry_wait`):

- The one repeat is of a read answered `429` that names a readable `Retry-After`. The delay, a
  second request as long as the first one took and a margin of 0.5 seconds must all end before the
  deadline. Otherwise the answer is returned at once, naming the delay when there is one.
- A Google quota `403` is `rate_limited` too, but it names no delay, so it is returned at once and
  is not sent again.
- A write answered `429` is an unconfirmed outcome, and a quota `403` on a write is a refusal;
  neither is repeated.
