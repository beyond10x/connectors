# Connect a Google API with OAuth

A catalog configuration whose profile uses `"scheme": "oauth2_refresh"` (see the
[catalog provider guide](local-catalog-provider.md)) stores
`{"client_id","client_secret","refresh_token"}` for a connection. Google issues
that refresh token only after a person consents in a browser. `connections
connect` and `connections repair` do that for you when you hand them the OAuth
client file Google's console downloads. The stored material is an
OAuth-acquired non-rotating static entry
(`architecture-decision-record:oauth-material-as-static-entry`): Google does not
rotate refresh tokens for installed apps, so the entry never changes after
consent, and the provider process turns it into short-lived access tokens that
are never written down.

## Create the Google OAuth client

1. In the Google Cloud console, select or create a project and enable each API
   the configuration reads.
2. Configure the OAuth consent screen. Add the scopes the configuration lists
   in `requested_scopes`. While the app's publishing status is *Testing*, add
   every Google account that will connect as a test user.
3. Under *Credentials*, create an *OAuth client ID* of application type
   *Desktop app*, and download its JSON. The file has one `installed` object
   with `client_id`, `client_secret`, `auth_uri`, `token_uri` and
   `redirect_uris`.
4. Make the download readable by you alone. The CLI refuses a credential file
   that anyone else can read, or that is a link:

   ```sh
   chmod 600 ~/Downloads/client_secret_*.json
   ```

5. Write the configuration's endpoints exactly as the file states them:
   `authorize_url` equal to its `auth_uri` (currently
   `https://accounts.google.com/o/oauth2/auth`) and `token_url` equal to its
   `token_uri` (`https://oauth2.googleapis.com/token`), byte for byte. A file
   whose endpoints differ is refused with `protected_entry_unavailable` before
   any browser step.

## Connect with OAuth

```sh
connectors connections connect --adapter drive --profile google.drive \
  --credential-file ~/Downloads/client_secret_1234.json
```

The CLI starts the owner's capture, which gives it the profile's
`authorize_url`, `token_url` and scopes, and then:

1. listens on `127.0.0.1` on a free port for the redirect;
2. writes the consent address to your terminal, which you open in a browser on
   the same machine. Without a terminal it writes one line to standard error,
   `connectors: consent-url <address>`, so a program running the CLI can find
   the address and hand it to a person;
3. after you consent, reads the redirect, checks its `state`, and exchanges the
   code together with its PKCE S256 verifier at `token_url`. This exchange uses
   the platform trust roots only: the configuration's `token_ca_file` applies
   to the provider's own refresh and validation requests, not to it;
4. submits `{client_id, client_secret, refresh_token}` as the connection's
   entry. The owner validates it with a refresh of its own, checks the identity
   and `minimum_scopes`, and stores it in custody.

The scopes requested are `requested_scopes` plus `openid`, with
`access_type=offline` and `prompt=consent`, so Google returns a refresh token on
every consent. The client file itself is read and not kept.

Consent has 240 seconds. If it takes longer, the command ends with `timeout` and
stores nothing; run it again. Ctrl-C ends the command, again storing nothing.
Only a connection that completes a request counts: one that closes, or sends no
complete request within 2 seconds, as a browser's speculative connections do,
is dropped and changes nothing. The first request for the redirect path `/`
decides. A declined consent or a redirect with the wrong `state` ends the
command without storing anything. A request after the first is answered 400
and does not change the outcome; a request for another path is answered 404.
The browser has to run on the machine the CLI runs on, because Google
redirects to that machine's loopback address; over SSH, forward the port the
consent address names.

A file that is not Google's installed-client JSON, such as an entry already
holding `{client_id, client_secret, refresh_token}`, is submitted unchanged, as
for every other profile.

## Repair

When Google stops honouring the refresh token (it was revoked, the account's
password changed, or it expired), an invoke fails with `service_failure`,
`service_code: unauthorized` and `next_action: repair_connection`. Repair the
connection with the same client file:

```sh
connectors connections repair --adapter drive --connection <connection> \
  --expected-revision <revision> --credential-file ~/Downloads/client_secret_1234.json
```

Repair runs the same consent and replaces the entry. Consent with the same
Google account: another account's identity is refused as `identity_mismatch`
and the connection keeps its previous entry.

## Testing-mode expiry

Google expires the refresh tokens of an app whose publishing status is
*Testing* seven days after consent. Every connection made for such an app needs
a repair once a week; publishing the app lifts the limit, subject to Google's
verification for sensitive scopes. Each connection is its own consent, so a
configuration per Google API means one consent, and one weekly repair, per
configuration.

## Verification

The consent flow's refusals (`state`, error redirect, second request, timeout,
interrupt, mismatched endpoints and profile fields) are unit tests of
`crates/connectors-host/src/local/oauth.rs`. The CLI journeys
`oauth_connect_journey` and `oauth_repair_journey` in
`adapters/catalog/tests/local_runtime/cli_journey.rs` run the production CLI
against a fixture consent and token host. A debug build of the CLI follows the
consent address itself when `CONNECTORS_TEST_OAUTH_FOLLOW` names the fixture's
trust root; a release build compiles that hook out. A live Google read is not
part of this guide's evidence.
