---
title: Invoke a definition file
sidebar_position: 4
description: Run an operation of a hand-written connector definition without configuring an instance, connecting on first use.
lede: One command lowers the definition, asks once whether to connect, and invokes; the configuration file is never written.
source: apps/connectors/src/local/definition.rs, crates/connectors-host/src/local/definitions.rs, contracts/cli/v1alpha1/semantics.md
---

# Invoke a definition file

A `connectors-definition/1` file names a vendored API document, the auth profiles that
authenticate against it and the operations it exposes. `connectors operations invoke
--definition <file> --operation <key>` runs one of those operations with no configured instance:
it lowers the file, connects the first time after one question, and invokes through the same
owner, admission, approval and audit as a configured instance. Run the commands from the
repository root after `cargo build --locked -p connectors -p connectors-build -p
connectors-catalog-provider`.

## Lay out a definitions tree

A definitions tree holds `connectors/<name>.yaml`, the shared auth files under `auth/` and the
vendored sources under `upstream/`. `connectors-build definition import` writes one from a
shipped selection; here the GitLab one, pointed at a self-hosted GitLab:

```sh
target/debug/connectors-build definition import --catalog gitlab --definitions target/try/defs
sed -i 's/default: gitlab.com}/default: gitlab.example.com}/' target/try/defs/connectors/gitlab.yaml
```

```text
target/try/defs/connectors/gitlab.yaml
target/try/defs/upstream/gitlab/openapi_v3.yaml.gz
target/try/defs/upstream/gitlab/openapi_v3.amendments.json
target/try/defs/upstream/gitlab/LICENSE
target/try/defs/upstream/gitlab/README.md
```

Every variable of the definition's `base_url` needs a default: the instance talks to the
endpoint the definition states, and trusts the platform's roots. `--definitions-root <dir>`
names the tree when the file is not in its `connectors/` directory.

## Invoke

A configuration and a state directory, here a private pair under `target/try`; the
configuration configures no instance:

```sh
mkdir -m 700 target/try/cli
target/debug/connectors --config "$PWD/target/try/cli/config.toml" \
  --state-dir "$PWD/target/try/cli/state" setup init | jq -r .disposition
```

```text
created
```

On a terminal, `operations invoke --definition target/try/defs/connectors/gitlab.yaml
--operation project.get --input-json '{"id":"group/project"}'` with those two globals lowers the
file, finds that its instance holds no connection, and asks once,
`No connection for gitlab/definition. Connect now? [y/N]`. Answered `y`, it runs the profile's
own acquisition, exactly as `connections connect` does: the token prompt, `GitLab personal
access token:` with the input hidden, or, for an OAuth profile, the consent with the client file
`--credential-file` names. The provider validates the credential, the connection is saved, and
the operation runs. Every later run with the same file asks nothing, and neither does a run after
a change limited to the operations: the connection is bound to the provider and the profile, not
to the operation list. `--schema` and `--revision` may be left out; they are the description's.

The instance is the connector's instance `definition`: `--connector gitlab --instance
definition` selects it in every other command, and results name it so.

## Reuse a stored credential

When another instance of the same connector already holds a connection, for example a configured
`[connectors.gitlab.instances.selfhosted]` at the same self-hosted GitLab, the one question lists
it instead: its number, `gitlab/selfhosted`, its API base and the account it validated as, never
the token. The question ends `[1/c/N]`. Answered `1`, the owner copies that connection's stored
credential to a new connection of the definition's instance, has the definition's provider
validate it, and saves it only when the provider answers the same account; the operation then runs
with no token prompt. `c` connects fresh as above; anything else, or no terminal, refuses as
above. A connection is offered only when it is ready, not stale or revoked, and bound to the same
API base and the same auth profile; an OAuth profile whose refresh rotates is never copied. When
the provider answers another account, nothing is saved, the configured instance's connection is
left as it was, and the command offers to connect fresh. After a copy the two connections are
independent: revoking one leaves the other.

Without a terminal, as in a script or a CI job, nothing is asked and nothing is sent; `setsid`
runs the same command that way. The refusal names the step that clears it:

```sh
setsid -w target/debug/connectors --output json --config "$PWD/target/try/cli/config.toml" \
  --state-dir "$PWD/target/try/cli/state" operations invoke \
  --definition target/try/defs/connectors/gitlab.yaml --operation project.get \
  --input-json '{"id":"group/project"}'
```

```json
{"error":{"code":"failure","data":{"code":"not_found","kind":"operational","next_action":"create_connection","stage":"admission"}},"ok":false}
```

`connections connect --connector gitlab --instance definition --profile gitlab.pat` connects it
without the question.

## What it keeps

The lowered definition is cached under the state directory, in an owner-only directory named by
the digest of every file its lowering reads and the lowerer's release; a run with
unchanged inputs reuses it. The instance's entry is held in the owner-only file
`definitions/instances.toml` beside the cache:

```sh
ls target/try/cli/state/definitions
```

```text
69e791a136bb2addfbf06c965b06e8fd3c335f5bb448cbd7e1eda97d7a2a342d
instances.toml
```

The configuration file is never written. If it already configures the connector's instance
`definition`, that instance is kept and the command is refused with `configuration_rule`
`definition_instance_configured`. A change to the file's provider authority or auth profile is
refused as `lifecycle_conflict`, as a read on that connection is; reusing the stored credential
for it is not part of this release. At most 64 connector ids are held, and no command forgets
one: revoke its connection with `connections revoke`, then remove `definitions/` from the state
directory while no owner runs. The full rules are in the
[CLI contract](../reference/contracts/cli.md).
