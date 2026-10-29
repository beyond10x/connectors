---
title: Getting started
sidebar_position: 2
description: Build Connectors from source, start one connector as a standalone service and watch it describe itself and refuse what it must.
---

# Getting started

Build the workspace, start the Kubernetes connector as a standalone service and ask it over HTTP
what it offers. No Kubernetes cluster and no real credential are needed: the service answers its
descriptor without contacting its provider, and the invocation that would need the provider is
refused by name. Every command below was run in a checkout of the repository, starting at its
root, and the output under it is what that run printed. The `connectors` command itself runs
configured instances on your machine and does not call a standalone service;
[Set up the local CLI](./guides/set-up-the-local-cli.md) starts there.

## Build

You need Linux x86_64, Rust 1.91 or later and `jq`. From a checkout of
[the repository](https://github.com/beyond10x/connectors):

```sh
cargo build --locked -p connectors -p connectors-kubernetes
target/debug/connectors --version
```

```text
connectors 0.60.0
```

Ordinary builds read checked-in generated Rust and need no ESS. Cargo downloads dependencies on
the first build. Package distribution is not configured.

## Or install a release binary

The walkthrough below builds from source because it also needs the Kubernetes service. For the
`connectors` command alone, each release page carries `connectors-<version>-<target>.tar.gz` for
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and
`aarch64-apple-darwin`, with a `SHA256SUMS` file; releases cut before the binaries workflow
existed
([`release-binaries.yml`](https://github.com/beyond10x/connectors/blob/main/.github/workflows/release-binaries.yml))
carry source archives only, and there is no Windows binary. Each archive holds a directory with
two binaries, `connectors` and the catalog provider `connectors-catalog-provider` (archives of
0.50.0 and earlier hold `connectors` alone), and `README.md` and `CHANGELOG.md`; the Kubernetes
service and the other connectors are built from source, as their guides show. A catalog instance
that pins nothing is served by the `connectors-catalog-provider` next to `connectors`, so keep the
two binaries in one directory. The Linux binaries are built on Ubuntu 24.04 and need its glibc or
a newer one.

On Linux, with the version and target you want, and `bin` a directory on your `PATH`:

```sh
version=X.Y.Z
target=x86_64-unknown-linux-gnu
bin="$HOME/bin"
base="https://github.com/beyond10x/connectors/releases/download/v$version"
curl -fsSLO "$base/connectors-$version-$target.tar.gz"
curl -fsSLO "$base/SHA256SUMS"
grep " connectors-$version-$target.tar.gz\$" SHA256SUMS | sha256sum -c -
tar -xzf "connectors-$version-$target.tar.gz"
mkdir -p "$bin"
install -m 0755 "connectors-$version-$target/connectors" "$bin/connectors"
install -m 0755 "connectors-$version-$target/connectors-catalog-provider" "$bin/connectors-catalog-provider"
"$bin/connectors" --version
"$bin/connectors-catalog-provider" --version
```

With `version=0.56.0` it printed:

```text
connectors-0.56.0-x86_64-unknown-linux-gnu.tar.gz: OK
connectors 0.56.0
connectors-catalog-provider 0.56.0
```

On macOS, use `target=aarch64-apple-darwin` (Apple silicon) or `x86_64-apple-darwin` (Intel),
and check the archive with `shasum -a 256 -c -` in place of `sha256sum -c -`; the other commands
are the same. The macOS binaries are not signed yet. A binary downloaded with `curl` runs as is;
one downloaded through a browser carries Gatekeeper's quarantine attribute and is refused until
you remove it, from the extracted directory, with
`xattr -d com.apple.quarantine connectors connectors-catalog-provider`. An unsigned binary also
cannot read login keychain items that an earlier build of `connectors` wrote, so credentials are
stored again after an upgrade.

On Windows, build `connectors` from the release tag's source. The CLI and its local host build and
run on macOS and Windows, each under its own CI job; the guarantees each platform holds only in a
weaker form are listed under [macOS](./reference/contracts/cli.md#macos-platform-guarantees) and
[Windows](./reference/contracts/cli.md#windows-platform-guarantees) platform guarantees.

## Start a service

Work in a directory under the git-ignored `target/`, with a service credential that callers
present to the service and a provider credential that the service would present to Kubernetes.
Credential files must be regular files owned by you with mode `0600`.

```sh
mkdir -p target/try && cd target/try
head -c 32 /dev/urandom | base64 > service.secret
printf 'not-a-real-token\n' > kubernetes.secret
chmod 600 service.secret kubernetes.secret
```

Save this as `kubernetes.yaml`. It permits two resource kinds in one namespace and points at a
Kubernetes API address where nothing listens:

```yaml title="kubernetes.yaml"
service:
  instance: kubernetes-local
  listen: 127.0.0.1:17102
  service_credential: {kind: file, path: service.secret}
http:
  base_url: https://127.0.0.1:6443/
  credential: {kind: file, path: kubernetes.secret}
  credential_header: authorization
  bearer: true
adapter:  # the internal implementation's own settings
  namespaces: [engineering]
  resource_kinds: [pods, services]
  discover_hosts: false
```

Start the service in the background:

```sh
../debug/connectors-kubernetes --config kubernetes.yaml > kubernetes.log 2>&1 &
```

## Ask it what it offers

A caller presents the service credential as a bearer token. `GET /v1/describe` answers the
descriptor:

```sh
token="authorization: Bearer $(cat service.secret)"
curl -s -H "$token" http://127.0.0.1:17102/v1/describe \
  | jq '{instance, revision, operations: [.operations[] | {id, contract, profile}]}'
```

```json
{
  "instance": "kubernetes-local",
  "revision": "3efdc565dd468f2d87f49faadd36ab0430f8560641ed79dbfb831be0066ea8b9",
  "operations": [
    {
      "id": "resources.list",
      "contract": "datasource.records/v1alpha1",
      "profile": "kubernetes-list"
    },
    {
      "id": "resources.get",
      "contract": "datasource.records/v1alpha1",
      "profile": "kubernetes-get"
    },
    {
      "id": "namespaces.list",
      "contract": "datasource.records/v1alpha1",
      "profile": "kubernetes-configured-namespaces"
    },
    {
      "id": "deployments.history",
      "contract": "datasource.records/v1alpha1",
      "profile": "kubernetes-rollout-history"
    },
    {
      "id": "endpoints.discover",
      "contract": "endpoint_discovery/v1alpha1",
      "profile": "kubernetes-endpointslice"
    }
  ]
}
```

The full descriptor also carries each operation's input and output schema. The revision changes
when the descriptor does; an invocation names the revision it was built against. The service
listens on loopback HTTP here; anywhere else it belongs behind TLS.

## Invoke, and read the refusals

`POST /v1/invoke` takes one request: the wire version, a request id the answer repeats, the
operation, the descriptor revision and the input. Ask for five pods. The service admits the
request and dispatches it, and Kubernetes is not there:

```sh
revision=$(curl -s -H "$token" http://127.0.0.1:17102/v1/describe | jq -r .revision)
jq -n --arg revision "$revision" '{version: "v1alpha1", request_id: "try-1",
  operation: "resources.list", revision: $revision,
  input: {namespace: "engineering", kind: "pods", limit: 5}}' > pods.json
curl -s -w '\nHTTP %{http_code}\n' -H "$token" -H 'content-type: application/json' \
  --data @pods.json http://127.0.0.1:17102/v1/invoke
```

```text
{"version":"v1alpha1","request_id":"try-1","status":"error","error":{"code":"unavailable","message":"dependency unavailable"}}
HTTP 503
```

A kind outside the operation's schema is refused before anything is dispatched:

```sh
jq -n --arg revision "$revision" '{version: "v1alpha1", request_id: "try-2",
  operation: "resources.list", revision: $revision,
  input: {namespace: "engineering", kind: "secrets", limit: 5}}' > secrets.json
curl -s -w '\nHTTP %{http_code}\n' -H "$token" -H 'content-type: application/json' \
  --data @secrets.json http://127.0.0.1:17102/v1/invoke
```

```text
{"version":"v1alpha1","request_id":"try-2","status":"error","error":{"code":"invalid_input","message":"value does not match its declared schema"}}
HTTP 400
```

A request built against another revision is refused as stale, never re-resolved:

```sh
jq -n '{version: "v1alpha1", request_id: "try-3",
  operation: "resources.list", revision: "an-older-revision",
  input: {namespace: "engineering", kind: "pods", limit: 5}}' > stale.json
curl -s -w '\nHTTP %{http_code}\n' -H "$token" -H 'content-type: application/json' \
  --data @stale.json http://127.0.0.1:17102/v1/invoke
```

```text
{"version":"v1alpha1","request_id":"try-3","status":"error","error":{"code":"stale_description","message":"refresh the descriptor before resubmitting"}}
HTTP 409
```

A caller without the service credential is refused before it learns anything:

```sh
head -c 32 /dev/urandom | base64 > other.secret && chmod 600 other.secret
curl -s -w '\nHTTP %{http_code}\n' -H "authorization: Bearer $(cat other.secret)" \
  http://127.0.0.1:17102/v1/describe
```

```text
{"code":"unauthorized","message":"service authentication failed"}
HTTP 401
```

The [service contract](./reference/contracts/service.md) states every code and status. Stop the
service with your shell's job control when you are done.

## Next

- From a Rust program, put one federation host in front of several services:
  [Federate services](./guides/federate-adapter-services.md), for internal connector processes.
- Use the local CLI with saved connections: [Set up the local CLI](./guides/set-up-the-local-cli.md).
- See what each connector can do and which ones run: [connector reference](./reference/adapters/index.md) and the [status page](./status.md).
- Follow an illustrated request from a laptop to GitLab: [Follow a request](/docs/examples/follow-a-request).
