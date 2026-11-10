---
title: Pinned instances
sidebar_position: 10
description: How an instance that names its own executable is pinned, and what the instance listing shows for any instance.
lede: A pinned instance names its executable, that executable's SHA-256 and the configuration revision the executable prints.
source: docs/local-kubernetes-cli.md, docs/local-postgres-cli.md, docs/local-tavily.md
---

# Pinned instances

A connector with a declaration shipped beside `connectors-catalog-provider` needs no pin: the
instance is derived. One with none is pinned. Its provider page gives its native file and the
values specific to it (`alias`, `instance_id`, `profiles`, `operations`, `path`, `args`).

## Pin an executable

1. Write the owner-only native file the page shows.
2. Print the executable's bootstrap with `--print-local-bootstrap` and hash the executable with
   `sha256sum`.
3. Put both in the instance: `configuration_revision` from the bootstrap and `sha256` of the
   executable.

Copy `configuration_revision` from the bootstrap verbatim; it is not the SHA-256 of any file. An
operation left out of `operations` is refused. After rebuilding the executable, update `sha256`
and stop the running process before the next command.

## The listing

`connectors instances list` shows every instance, derived or pinned, by connector, instance and
`instance_id`:

```sh
connectors instances list | jq -c '.[] | arrays | .[]'
```

```json
{"connector":"gitlab","instance":"work","instance_id":"gitlab.work","restart":"never","startup":"on-demand"}
```
