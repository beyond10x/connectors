use crate::{
    ClientKind, DeviceAuthorization, DeviceDialect, DialectContext, Failure, GrantType, Granted,
    Refreshed, RequestEncoding, Secret, ServerDeclaration, Transport, Waiter,
    declaration::https,
    pkce::{pkce_challenge, random},
    token::{self, Answer, TokenResponse, is_token, require},
};
use serde::Deserialize;
use std::{
    fmt,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// The RFC 8628 §3.4 grant type of a device-code poll.
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
/// The RFC 8628 §3.2 polling interval when the server names none.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);
/// What RFC 8628 §3.5 `slow_down` adds to the interval, for this and every later poll.
const SLOW_DOWN: Duration = Duration::from_secs(5);

/// One client of one authorization server: a validated declaration, the client id, the secret
/// of a confidential client, and the transport every request goes through.
pub struct Client<T> {
    declaration: ServerDeclaration,
    client_id: String,
    secret: Option<Secret>,
    transport: T,
}

impl<T> fmt::Debug for Client<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("declaration", &self.declaration)
            .field("client_id", &self.client_id)
            .field("secret", &self.secret)
            .finish_non_exhaustive()
    }
}

/// One authorization-code flow in progress: the authorize URL to show, and the PKCE verifier
/// and `state` that only this process holds. Consumed by [`Client::exchange`].
pub struct Authorization {
    url: String,
    redirect_uri: String,
    verifier: Secret,
    state: Secret,
}

impl fmt::Debug for Authorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Authorization").finish_non_exhaustive()
    }
}

impl Authorization {
    /// The authorize URL the person opens. It carries `state` and the PKCE challenge, never
    /// the verifier.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The redirect URI this flow sent and will send again in the exchange.
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Whether `received` is this flow's `state`, compared in constant time.
    pub(crate) fn state_matches(&self, received: &str) -> bool {
        bool::from(received.as_bytes().ct_eq(self.state.expose().as_bytes()))
    }
}

/// An authorization code a receiver accepted for one [`Authorization`].
#[derive(Debug)]
pub struct AuthorizationCode(pub(crate) Secret);

impl AuthorizationCode {
    pub fn expose(&self) -> &str {
        self.0.expose()
    }
}

impl<T: Transport> Client<T> {
    /// A client for `declaration`. The client id is the declaration's, else
    /// `configured_client_id` (both given, they must agree); a confidential client needs
    /// `secret`, a public one must not be given one. Refused as
    /// [`Failure::InvalidDeclaration`] when the declaration does not validate
    /// ([`ServerDeclaration::validate`]) or the credentials do not fit it.
    pub fn new(
        declaration: ServerDeclaration,
        configured_client_id: Option<&str>,
        secret: Option<Secret>,
        transport: T,
    ) -> Result<Self, Failure> {
        declaration.validate()?;
        let client_id = match (declaration.client_id.as_deref(), configured_client_id) {
            (Some(declared), Some(configured)) if declared != configured => {
                return Err(Failure::InvalidDeclaration);
            }
            (Some(id), _) | (None, Some(id)) => id.to_owned(),
            (None, None) => return Err(Failure::InvalidDeclaration),
        };
        let fits = match (declaration.client_kind, &secret) {
            (ClientKind::Public, None) => true,
            (ClientKind::Confidential, Some(secret)) => !secret.expose().is_empty(),
            _ => false,
        };
        if client_id.is_empty() || !fits {
            return Err(Failure::InvalidDeclaration);
        }
        Ok(Self {
            declaration,
            client_id,
            secret,
            transport,
        })
    }

    pub fn declaration(&self) -> &ServerDeclaration {
        &self.declaration
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Starts an authorization-code flow that redirects to `redirect_uri`: draws a fresh PKCE
    /// verifier and `state` and builds the authorize URL: `response_type`, `client_id`,
    /// `redirect_uri`, `scope`, `code_challenge`, `code_challenge_method=S256` and `state`,
    /// then the declaration's `authorize_parameters` in declared order.
    pub fn begin(&self, redirect_uri: &str) -> Result<Authorization, Failure> {
        let mut url = self
            .declaration
            .authorize_url
            .as_deref()
            .and_then(https)
            .filter(|_| self.grants(GrantType::AuthorizationCode))
            .ok_or(Failure::InvalidDeclaration)?;
        url::Url::parse(redirect_uri).map_err(|_| Failure::InvalidDeclaration)?;
        let verifier = random()?;
        let state = random()?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", &self.client_id)
                .append_pair("redirect_uri", redirect_uri);
            if !self.declaration.requested_scopes.is_empty() {
                query.append_pair("scope", &self.declaration.requested_scopes.join(" "));
            }
            query
                .append_pair("code_challenge", &pkce_challenge(verifier.expose()))
                .append_pair("code_challenge_method", "S256")
                .append_pair("state", state.expose());
            for parameter in &self.declaration.authorize_parameters {
                query.append_pair(&parameter.name, &parameter.value);
            }
        }
        Ok(Authorization {
            url: url.into(),
            redirect_uri: redirect_uri.to_owned(),
            verifier,
            state,
        })
    }

    /// Exchanges `code` for tokens at the token endpoint: the RFC 6749 §4.1.3 fields with the
    /// PKCE verifier, the secret of a confidential client, and `state` when the declaration sets
    /// `state_in_exchange`.
    pub fn exchange(
        &self,
        authorization: Authorization,
        code: AuthorizationCode,
    ) -> Result<Granted, Failure> {
        let state = self
            .declaration
            .state_in_exchange
            .then(|| authorization.state.expose());
        self.exchange_code(
            code.expose(),
            authorization.verifier.expose(),
            &authorization.redirect_uri,
            state,
        )
    }

    fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
        state: Option<&str>,
    ) -> Result<Granted, Failure> {
        let mut fields = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", self.client_id.as_str()),
            ("code_verifier", verifier),
        ];
        fields.extend(self.secret_field());
        fields.extend(state.map(|state| ("state", state)));
        let body = self
            .post(
                &self.declaration.token_url,
                self.declaration.request_encoding,
                &fields,
            )
            .map_err(Answer::failure)?;
        self.granted(&body)
    }

    /// Exchanges `refresh_token` for a new access token (RFC 6749 §6), in the declaration's
    /// `refresh_encoding`. `granted_scopes` are the grant's current scopes, kept when the
    /// response omits `scope`; a response without `refresh_token` keeps the one sent.
    pub fn refresh(
        &self,
        refresh_token: &Secret,
        granted_scopes: &[String],
    ) -> Result<Refreshed, Failure> {
        let mut fields = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose()),
            ("client_id", self.client_id.as_str()),
        ];
        fields.extend(self.secret_field());
        let body = self
            .post(
                &self.declaration.token_url,
                self.declaration.refresh_encoding,
                &fields,
            )
            .map_err(Answer::failure)?;
        let response = TokenResponse::parse(&body)?;
        let scopes = response.scopes.unwrap_or_else(|| granted_scopes.to_vec());
        require(&self.declaration.required_scopes, &scopes)?;
        let (refresh_token, rotated) = match response.refresh_token {
            Some(new) => {
                let rotated = !bool::from(
                    new.expose()
                        .as_bytes()
                        .ct_eq(refresh_token.expose().as_bytes()),
                );
                (new, rotated)
            }
            None => (refresh_token.clone(), false),
        };
        Ok(Refreshed {
            access_token: response.access_token,
            refresh_token,
            rotated,
            expires_in: response.expires_in,
            scopes,
        })
    }

    /// Starts a device grant at the declaration's `device_authorization_url` (RFC 8628 §3.1).
    pub fn device_authorize(&self) -> Result<DeviceAuthorization, Failure> {
        let url = self
            .declaration
            .device_authorization_url
            .as_deref()
            .filter(|_| self.grants(GrantType::DeviceCode))
            .ok_or(Failure::InvalidDeclaration)?;
        let scope = self.declaration.requested_scopes.join(" ");
        let mut fields = vec![("client_id", self.client_id.as_str())];
        if !scope.is_empty() {
            fields.push(("scope", scope.as_str()));
        }
        fields.extend(self.secret_field());
        let body = self
            .post(url, self.declaration.request_encoding, &fields)
            .map_err(Answer::failure)?;
        #[derive(Deserialize)]
        struct Body {
            device_code: Secret,
            user_code: String,
            verification_uri: String,
            verification_uri_complete: Option<String>,
            expires_in: u64,
            interval: Option<u64>,
        }
        let body: Body = serde_json::from_slice(&body).map_err(|_| Failure::InvalidResponse)?;
        let shown = |text: &str| !text.is_empty() && !text.chars().any(char::is_control);
        if !is_token(body.device_code.expose())
            || !shown(&body.user_code)
            || https(&body.verification_uri).is_none()
            || body
                .verification_uri_complete
                .as_deref()
                .is_some_and(|uri| https(uri).is_none())
            || body.expires_in == 0
        {
            return Err(Failure::InvalidResponse);
        }
        Ok(DeviceAuthorization {
            device_code: body.device_code,
            user_code: body.user_code,
            verification_uri: body.verification_uri,
            verification_uri_complete: body.verification_uri_complete,
            expires_in: Duration::from_secs(body.expires_in),
            interval: body
                .interval
                .filter(|&seconds| seconds > 0)
                .map_or(DEFAULT_INTERVAL, Duration::from_secs),
            issued: Instant::now(),
        })
    }

    /// Polls the token endpoint for `device` until the person decides (RFC 8628 §3.4–3.5):
    /// waits one interval before each poll; `authorization_pending` polls again, `slow_down`
    /// adds 5 s to the interval for good; `expired_token`, `access_denied`, the device code's
    /// own expiry, any other failure, and the waiter's cancellation end the grant.
    pub fn poll_device(
        &self,
        device: &DeviceAuthorization,
        waiter: &dyn Waiter,
    ) -> Result<Granted, Failure> {
        let mut interval = device.interval;
        let mut waited = Duration::ZERO;
        let mut fields = vec![
            ("grant_type", DEVICE_GRANT),
            ("device_code", device.device_code.expose()),
            ("client_id", self.client_id.as_str()),
        ];
        fields.extend(self.secret_field());
        loop {
            if waited + interval > device.expires_in
                || device.issued.elapsed() + interval > device.expires_in
            {
                return Err(Failure::ExpiredToken);
            }
            waiter.wait(interval)?;
            waited += interval;
            match self.post(
                &self.declaration.token_url,
                self.declaration.request_encoding,
                &fields,
            ) {
                Ok(body) => return self.granted(&body),
                Err(Answer::Pending) => {}
                Err(Answer::SlowDown) => interval += SLOW_DOWN,
                Err(Answer::Failed(failure)) => return Err(failure),
            }
        }
    }

    /// Runs a provider's own device flow and exchanges the code and verifier it returns at the
    /// token endpoint, like any other authorization code (without `state`: a dialect has none).
    pub fn device_dialect(
        &self,
        dialect: &dyn DeviceDialect,
        waiter: &dyn Waiter,
    ) -> Result<Granted, Failure> {
        let grant = dialect.authorize(&DialectContext {
            declaration: &self.declaration,
            client_id: &self.client_id,
            transport: &self.transport,
            waiter,
        })?;
        self.exchange_code(
            grant.code.expose(),
            grant.verifier.expose(),
            &grant.redirect_uri,
            None,
        )
    }

    fn grants(&self, grant: GrantType) -> bool {
        self.declaration.grant_types.contains(&grant)
    }

    fn secret_field(&self) -> Option<(&'static str, &str)> {
        self.secret
            .as_ref()
            .map(|secret| ("client_secret", secret.expose()))
    }

    fn post(
        &self,
        url: &str,
        encoding: RequestEncoding,
        fields: &[(&str, &str)],
    ) -> Result<Zeroizing<Vec<u8>>, Answer> {
        token::post(&self.transport, url, encoding, fields)
    }

    /// A code or device exchange's tokens; omitted `scope` means the requested scopes.
    fn granted(&self, body: &[u8]) -> Result<Granted, Failure> {
        let response = TokenResponse::parse(body)?;
        let scopes = response
            .scopes
            .unwrap_or_else(|| self.declaration.requested_scopes.clone());
        require(&self.declaration.required_scopes, &scopes)?;
        Ok(Granted {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
            scopes,
        })
    }
}
