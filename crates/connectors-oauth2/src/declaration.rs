//! The declaration types, mirroring `connectors.oauth2` in `ess/domains/oauth2.yaml` field for
//! field and variant for variant; `crates/connectors-build/tests/oauth2_declaration_mirrors_ess.rs`
//! holds them to the compiled specification.
use crate::Failure;
use serde::{Deserialize, Serialize};

/// A closed enum whose serde names are the ESS variant names, with `ALL` listing every variant
/// in declaration order, so the mirror test reads the variants from the type itself.
macro_rules! closed_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident => $wire:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $($(#[$variant_meta])* #[serde(rename = $wire)] $variant),+
        }

        impl $name {
            /// Every variant, in the specification's declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The variant's name in the specification and on the wire.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }
    };
}

closed_enum! {
    /// `connectors.oauth2.GrantType`: the grants a server admits for this client.
    GrantType {
        /// RFC 6749 §4.1 with PKCE S256 (RFC 7636).
        AuthorizationCode => "authorization_code",
        /// RFC 8628.
        DeviceCode => "device_code",
    }
}

closed_enum! {
    /// `connectors.oauth2.ClientKind`: whether the client authenticates with a secret.
    ClientKind {
        /// Sends `client_id` and no secret.
        Public => "public",
        /// Adds the secret the caller supplies.
        Confidential => "confidential",
    }
}

closed_enum! {
    /// `connectors.oauth2.RequestEncoding`: how a token-endpoint request body is written.
    RequestEncoding {
        /// `application/x-www-form-urlencoded` (RFC 6749 §4.1.3).
        Form => "form",
        /// `application/json`, one flat object of strings.
        Json => "json",
    }
}

closed_enum! {
    /// `connectors.oauth2.RedirectKind`: how the authorization code comes back.
    RedirectKind {
        /// A loopback listener on `127.0.0.1` at a port the system chooses.
        LoopbackAnyPort => "loopback_any_port",
        /// A loopback listener on `127.0.0.1` at the port `redirect_uri` declares.
        LoopbackFixedPort => "loopback_fixed_port",
        /// The person pastes `code#state` as one line.
        PastedCode => "pasted_code",
    }
}

closed_enum! {
    /// `connectors.oauth2.Rotation`: whether a refresh may return a new refresh token.
    Rotation {
        /// The refresh token is never replaced.
        NonRotating => "non_rotating",
        /// A refresh may return a new refresh token that replaces the old one.
        Rotating => "rotating",
    }
}

/// `connectors.oauth2.Parameter`: one extra authorize-URL parameter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub value: String,
}

/// `connectors.oauth2.ServerDeclaration`: everything public a profile declares about one
/// authorization server. It holds no secret: a confidential client's secret is entered
/// material the caller supplies to [`crate::Client::new`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerDeclaration {
    /// The authorization endpoint; required for the `authorization_code` grant.
    pub authorize_url: Option<String>,
    /// The token endpoint, for the code, device and refresh exchanges.
    pub token_url: String,
    /// The RFC 8628 device authorization endpoint; required for the `device_code` grant.
    pub device_authorization_url: Option<String>,
    /// Absent when the instance's configuration supplies a registered client id.
    pub client_id: Option<String>,
    pub client_kind: ClientKind,
    /// The encoding of the code and device exchanges.
    pub request_encoding: RequestEncoding,
    /// The encoding of the refresh exchange.
    pub refresh_encoding: RequestEncoding,
    pub redirect_kind: RedirectKind,
    /// For `loopback_any_port`, a template whose host and path are kept; for
    /// `loopback_fixed_port`, the listener's address; for `pasted_code`, the URI the server
    /// redirects to.
    pub redirect_uri: Option<String>,
    /// Extra authorize-URL parameters, appended in this order after the core ones.
    pub authorize_parameters: Vec<Parameter>,
    pub requested_scopes: Vec<String>,
    /// Scopes a grant must hold; one missing is [`Failure::InsufficientScope`].
    pub required_scopes: Vec<String>,
    pub rotation: Rotation,
    /// How long before expiry the owner refreshes; the crate only checks it is not negative.
    pub refresh_lead_seconds: i64,
    pub grant_types: Vec<GrantType>,
    /// Where each value of this declaration was read from.
    pub sources: Vec<String>,
    /// Whether the code exchange also carries the flow's `state`; a non-standard requirement
    /// of some servers. `false` sends the RFC 6749 §4.1.3 fields only.
    pub state_in_exchange: bool,
}

impl ServerDeclaration {
    /// Checks what the type cannot: HTTPS endpoints, the endpoint each declared grant needs,
    /// no extra parameter that sets a core authorize parameter, a redirect each kind can use,
    /// and a non-negative refresh lead.
    pub fn validate(&self) -> Result<(), Failure> {
        let refused = Err(Failure::InvalidDeclaration);
        if !is_https(&self.token_url) || self.refresh_lead_seconds < 0 {
            return refused;
        }
        let grants = |grant| self.grant_types.contains(&grant);
        match &self.authorize_url {
            Some(url) => {
                let Some(url) = https(url) else {
                    return refused;
                };
                if url.query_pairs().any(|(name, _)| is_core(&name)) {
                    return refused;
                }
            }
            None if grants(GrantType::AuthorizationCode) => return refused,
            None => {}
        }
        match &self.device_authorization_url {
            Some(url) if !is_https(url) => return refused,
            None if grants(GrantType::DeviceCode) => return refused,
            _ => {}
        }
        if self
            .authorize_parameters
            .iter()
            .any(|parameter| parameter.name.is_empty() || is_core(&parameter.name))
        {
            return refused;
        }
        match self.redirect_kind {
            RedirectKind::LoopbackAnyPort => match &self.redirect_uri {
                Some(template) if loopback(template).is_none() => return refused,
                _ => {}
            },
            RedirectKind::LoopbackFixedPort => {
                if self
                    .redirect_uri
                    .as_deref()
                    .and_then(loopback)
                    .and_then(|url| url.port())
                    .is_none()
                {
                    return refused;
                }
            }
            RedirectKind::PastedCode => {
                if !self
                    .redirect_uri
                    .as_deref()
                    .is_some_and(|uri| self.redirect_kind.admits(uri))
                {
                    return refused;
                }
            }
        }
        Ok(())
    }
}

impl RedirectKind {
    /// Whether an authorization of this kind may redirect to `uri`: a pasted
    /// code's redirect is a remote page and HTTPS like every endpoint; only a
    /// loopback receiver's own address is plain `http`.
    pub(crate) fn admits(self, uri: &str) -> bool {
        match self {
            Self::PastedCode => is_https(uri),
            Self::LoopbackAnyPort | Self::LoopbackFixedPort => loopback(uri).is_some(),
        }
    }
}

/// The authorize parameters the core sets; a declaration may set none of them, in any case.
pub(crate) const CORE_PARAMETERS: [&str; 7] = [
    "response_type",
    "client_id",
    "redirect_uri",
    "scope",
    "code_challenge",
    "code_challenge_method",
    "state",
];

fn is_core(name: &str) -> bool {
    CORE_PARAMETERS
        .iter()
        .any(|core| core.eq_ignore_ascii_case(name))
}

/// `url` parsed, when it is an `https` URL with a host and no fragment.
pub(crate) fn https(url: &str) -> Option<url::Url> {
    url::Url::parse(url).ok().filter(|url| {
        url.scheme() == "https" && url.host_str().is_some() && url.fragment().is_none()
    })
}

pub(crate) fn is_https(url: &str) -> bool {
    https(url).is_some()
}

/// `uri` parsed, when it is a plain `http` URI on `127.0.0.1` or `localhost`, without
/// credentials, query or fragment: the only redirect a loopback receiver can answer.
pub(crate) fn loopback(uri: &str) -> Option<url::Url> {
    url::Url::parse(uri).ok().filter(|url| {
        url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Client, Request, Response, Transport, TransportError};

    struct NoTransport;

    impl Transport for NoTransport {
        fn post(&self, _request: &Request<'_>) -> Result<Response, TransportError> {
            Err(TransportError)
        }
    }

    fn declared(kind: &str, redirect_uri: Option<&str>) -> ServerDeclaration {
        serde_json::from_value(serde_json::json!({
            "authorize_url": "https://localhost/fixture/authorize",
            "token_url": "https://localhost/fixture/token",
            "device_authorization_url": null,
            "client_id": "fixture-unit-client",
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": kind,
            "redirect_uri": redirect_uri,
            "authorize_parameters": [],
            "requested_scopes": ["fixture.read"],
            "required_scopes": [],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["authorization_code"],
            "sources": [],
            "state_in_exchange": false,
        }))
        .unwrap()
    }

    /// A pasted-code redirect is a remote page the server sends the browser
    /// to with the code, so it is HTTPS like every other endpoint; the
    /// loopback receiver's plain `http://127.0.0.1` stays the loopback kinds'
    /// own.
    #[test]
    fn a_pasted_code_redirect_must_be_https() {
        let https = "https://fixture.invalid/callback";
        assert_eq!(declared("pasted_code", Some(https)).validate(), Ok(()));
        for redirect in [
            Some("http://fixture.invalid/callback"),
            Some("http://127.0.0.1:4711/callback"),
            Some("fixture:callback"),
            Some("https://fixture.invalid/callback#fragment"),
            None,
        ] {
            assert_eq!(
                declared("pasted_code", redirect).validate(),
                Err(Failure::InvalidDeclaration),
                "{redirect:?}"
            );
        }
        let loopback = "http://127.0.0.1:4711/callback";
        assert_eq!(
            declared("loopback_fixed_port", Some(loopback)).validate(),
            Ok(())
        );
        assert_eq!(
            declared(
                "loopback_fixed_port",
                Some("https://127.0.0.1:4711/callback")
            )
            .validate(),
            Err(Failure::InvalidDeclaration)
        );
    }

    /// The redirect an authorization is begun with is one the declared kind
    /// can use: HTTPS for a pasted code, plain loopback HTTP for a receiver.
    #[test]
    fn begin_takes_only_a_redirect_its_kind_can_use() {
        let https = "https://fixture.invalid/callback";
        let loopback = "http://127.0.0.1:4711/callback";
        let pasted = Client::new(
            declared("pasted_code", Some(https)),
            None,
            None,
            NoTransport,
        )
        .unwrap();
        assert!(pasted.begin(https).is_ok());
        for redirect in ["http://fixture.invalid/callback", loopback] {
            assert_eq!(
                pasted.begin(redirect).err(),
                Some(Failure::InvalidDeclaration),
                "{redirect}"
            );
        }
        let received =
            Client::new(declared("loopback_any_port", None), None, None, NoTransport).unwrap();
        assert!(received.begin(loopback).is_ok());
        for redirect in [https, "http://fixture.invalid/callback"] {
            assert_eq!(
                received.begin(redirect).err(),
                Some(Failure::InvalidDeclaration),
                "{redirect}"
            );
        }
    }
}
