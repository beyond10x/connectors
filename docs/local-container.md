# Running `connectors` in a container

A container has no session bus and no keyring, so it uses the read-only
[mounted-secret custody binding](local-mounted-secret-custody.md): the platform delivers the
credential as a file in a read-only volume, and the owner configuration selects that volume
explicitly. This page shows the setup, with a Kubernetes CronJob as the example.

## What the configuration stage needs

The run needs an absolute configuration path and state path, an owner-only configuration
file, and `owner_uid` equal to the user the container runs as. No other variable.

- Pass both `--config <absolute path>` and `--state-dir <absolute path>`. Then no
  environment variable is read: `HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `PATH` may all
  be absent (`env -i`). Executables are absolute and digest-pinned, so `PATH` is never
  consulted.
- Alternatively set `HOME` to a directory that holds `.config/connectors/config.toml`, or
  set `XDG_CONFIG_HOME` and `XDG_STATE_HOME`.

When the configuration stage refuses, `invalid_configuration` names its rule in
`configuration_rule`, never a path:

| `configuration_rule` | Meaning |
|---|---|
| `config_path_unresolved` | No `--config`, no `XDG_CONFIG_HOME` and no `HOME`, or a path that is not absolute. |
| `config_missing` | No file at the resolved path. |
| `config_not_private` | The file, or a directory above it, is not owner-only (owned by root or the run user and not group- or other-writable; the file itself mode `0600` with one link). |
| `owner_uid_mismatch` | `owner_uid` is not the run user. |

## The image

- Bake the configuration into the image, owned by the run user, mode `0600`, in a `0700`
  directory whose ancestors are root-owned and not group- or other-writable. A ConfigMap
  cannot hold it: its files are root-owned, symbolically linked and group-readable.
- Give the run user a state directory in the image's writable layer, for example
  `/srv/connectors/state`, owned by that user. An `emptyDir` or a persistent volume root
  with `fsGroup` is group-writable and is refused.
- Keep `readOnlyRootFilesystem: false` for that layer.

```toml
# /srv/connectors/config.toml, owned 10001:10001, mode 0600
format = "connectors-local/4"
owner_uid = 10001

[custody]
binding = "mounted_secret"
directory = "/run/connectors/custody"

[adapters.tavily]
# instance_id, adapter_id, configuration_revision, protocol, private_protocol,
# executable and permissions as for any adapter entry.
```

The custody directory must have no symbolic link in its path: mount it under `/run`, not
`/var/run`, which is a link on most images.

## The CronJob

UIDs are illustrative.

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: connectors-job
spec:
  schedule: "0 * * * *"
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: Never
          securityContext:
            runAsUser: 10001
            runAsGroup: 10001
            runAsNonRoot: true
            fsGroup: 10001
          containers:
            - name: job
              image: <image with /srv/connectors owned 10001:10001, mode 0700>
              command: ["/bin/sh", "-c"]
              args:
                - >-
                  connectors --config /srv/connectors/config.toml --state-dir /srv/connectors/state setup init &&
                  connectors --config /srv/connectors/config.toml --state-dir /srv/connectors/state connections connect --adapter tavily --profile tavily.api-key &&
                  connectors --config /srv/connectors/config.toml --state-dir /srv/connectors/state connections list --adapter tavily
              securityContext:
                readOnlyRootFilesystem: false
                allowPrivilegeEscalation: false
              volumeMounts:
                - name: custody
                  mountPath: /run/connectors/custody
                  readOnly: true
          volumes:
            - name: custody
              secret:
                secretName: connectors-custody
                defaultMode: 0400
                items:
                  - key: tavily-key
                    path: tavily/tavily.api-key
```

- **Ownership.** Secret files are root-owned. A non-root container reads them only through
  group read: `fsGroup` makes the pod's group the files' group and adds group read, so
  `defaultMode: 0400` presents as `0440 root:10001`, which the binding admits. Without
  `fsGroup` the file is unreadable by user 10001 and `setup check` reports the custody
  prerequisite failed.
- **Layout.** The kubelet mounts a Secret volume read-only in the projected layout
  (`..data` naming a timestamped directory). A CronJob pod reads the volume as it was when
  the pod started; the next pod sees an update.
- **State.** With ephemeral state each run does `setup init` (which keeps the baked
  configuration and creates the state), `connections connect` (one provider validation),
  then the work. The connection reference is in the connect result's JSON.
- **Rotation.** Update the Secret. A run that reuses its state refuses a read under the old
  version with `next_action = revalidate_connection`; `connections revalidate` pins the new
  bytes.
- **Revocation.** `connections revoke` stops Connectors using the item. Delete or rotate the
  Secret as well.

`setup check` in this pod reports `custody_backend = "mounted_secret"`.
