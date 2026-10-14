# Install a release binary

```sh
version=X.Y.Z
target=x86_64-unknown-linux-gnu
tar -xzf "connectors-$version-$target.tar.gz"
for binary in connectors connectors-catalog-provider; do
  install -m 0755 "connectors-$version-$target/$binary" ~/.local/bin/
done
install -m 0755 "connectors-${version}-${target}/connectors-catalog-provider" ~/.local/bin/
connectors --version && connectors-catalog-provider --version
```

```console
$ connectors-catalog-provider --version
```
