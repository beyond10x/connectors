---
title: Google OAuth setup
sidebar_position: 9
description: Create the OAuth client and choose the publishing status once for the Gmail, Calendar, Drive and Slides connectors.
lede: One Desktop-app client file serves every Google connector; the publishing status decides whether the refresh token expires after seven days.
source: docs/catalog-google-oauth.md, docs/catalog-google-gmail.md
---

# Google OAuth setup

The Gmail, Calendar, Drive and Slides pages under [Connectors](../connectors/index.md) each
list their own API and scopes. The client and the publishing status are the same for all of them.

1. In the Google Cloud console, select or create a project, enable the connector's API, and
   configure the OAuth consent screen with the scopes its page lists. An instance that names
   no profile is read-only and requests only the `.readonly` scope; one that names
   `profile = "google.oauth"` requests every scope its operations need, reads and writes alike. An
   instance can admit only a company's accounts with `allowed_domains`.
2. Under *Credentials*, create an OAuth client ID of type *Desktop app* and download its JSON.
   One client file serves every Google connector. Make it readable by you alone; the CLI refuses a
   credential file anyone else can read:

   ```sh
   chmod 600 ~/Downloads/client_secret_*.json
   ```

3. Choose the publishing status. Google expires the refresh token of an app in *Testing* seven days
   after consent, so each connection then needs a [reconnect](./reconnect-a-connection.md) every
   week, and every account that connects must be listed as a test user. An app used only by its
   developer can be set to *In production* without Google's verification
   ([Google's exemption for personal use](https://support.google.com/cloud/answer/13464323)),
   which lifts the seven-day expiry.

The [Google OAuth guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-google-oauth.md)
describes the client file and the consent flow in full.
