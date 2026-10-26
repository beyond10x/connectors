# Claude subscription sign-in, authored here

`connector.json` declares the Claude subscription sign-in as two `oauth2_grant` profiles,
`claude.subscription` (loopback redirect on any port) and `claude.subscription-pasted` (the
person pastes the `code#state` the provider's callback page shows). Each fact of the
declaration cites the published Claude Code 2.1.310 binary in `oauth2.sources`. The profiles
ship no client id: an instance configures `oauth2_client_id`, and without one it is refused
as invalid configuration. The connector is for the operator's own Claude plan; the guide,
[`website/docs/connectors/claude-subscription.md`](../../../../website/docs/connectors/claude-subscription.md),
quotes the provider's terms.

`claude-oauth-profile.yaml` is an OpenAPI 3.1 document this repository writes itself. It
describes the one read the identity probe makes, `GET /api/oauth/profile`, and nothing else.
`operations.json` selects that read as `account.profile`, the only operation: the catalog
provider refuses an empty selection, and operations against the inference API are out of scope.

The catalog pipeline compiles the document into
[`../../generated/bundles/claude-subscription.bundle.json`](../../generated/bundles/claude-subscription.bundle.json),
rebuilt with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider claude-subscription \
  --source adapters/catalog/providers/claude-subscription/claude-oauth-profile.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile claude.subscription --replace
```

`adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle a fresh run over this file
would not reproduce byte for byte.
