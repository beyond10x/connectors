//! B10x Session Authority v1 issuance and serving-endpoint redemption.

use std::collections::BTreeSet;
use std::sync::Mutex;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub const AUTHORITY_TYPE: &str = "dl-session+jwt";
pub const DPOP_TYPE: &str = "dpop+jwt";
/// Authorization scheme used only on the brokered WebSocket upgrade.
pub const AUTHORIZATION_SCHEME: &str = "DLSession";
/// Header carrying the proof-of-possession compact JWS.
pub const DPOP_HEADER: &str = "dpop";
pub const MAX_AUTHORITY_LIFETIME_SECONDS: u64 = 60;
pub const CLOCK_SKEW_SECONDS: u64 = 5;

/// Compact signed material that never reveals itself through `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct SensitiveCompact(String);

impl SensitiveCompact {
    fn new(value: String) -> Self {
        Self(value)
    }

    /// Borrow the compact value only at an explicit wire boundary.
    pub fn as_wire_value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SensitiveCompact {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// Proof-of-possession confirmation claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confirmation {
    pub jkt: String,
}

/// Closed B10x Session Authority v1 claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionAuthorityClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub act: String,
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
    pub jti: String,
    pub cnf: Confirmation,
    pub dl_org: String,
    pub dl_deployment: String,
    pub dl_connection: String,
    pub dl_grant: String,
    pub dl_resource: String,
    pub dl_operation: String,
    pub dl_channel_kind: String,
    pub dl_protocol: String,
    pub dl_endpoint: String,
    pub dl_session_lease_exp: u64,
}

/// Authority returned to the connecting endpoint after brokerage.
///
/// This is deliberately distinct from [`RedeemedAuthority`]: the voice endpoint may inspect and
/// present an issued authority, but only the serving endpoint can turn it into redemption proof.
#[derive(Clone, PartialEq, Eq)]
pub struct IssuedAuthority {
    compact: SensitiveCompact,
    claims: SessionAuthorityClaims,
}

impl IssuedAuthority {
    /// Public, non-secret routing claims signed into the authority.
    pub fn claims(&self) -> &SessionAuthorityClaims {
        &self.claims
    }

    /// Redacting compact value for an authorization header or DPoP hash.
    pub fn compact(&self) -> &SensitiveCompact {
        &self.compact
    }

    /// Build the value presented by the connecting endpoint during the exact WebSocket upgrade.
    pub fn presentation(
        &self,
        method: impl Into<String>,
        uri: impl Into<String>,
        dpop: SensitiveCompact,
    ) -> RedemptionRequest {
        RedemptionRequest {
            method: method.into(),
            uri: uri.into(),
            authority: self.compact.clone(),
            dpop,
        }
    }
}

impl std::fmt::Debug for IssuedAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedAuthority")
            .field("compact", &"<redacted>")
            .field("claims", &self.claims)
            .finish()
    }
}

/// Proof that the serving endpoint verified and atomically redeemed the authority. The private
/// field prevents a byte-plane adapter from being constructed from unverified claims alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedeemedAuthority(SessionAuthorityClaims);

impl RedeemedAuthority {
    pub fn claims(&self) -> &SessionAuthorityClaims {
        &self.0
    }
}

/// Issuer inputs after Connector Grant admission and endpoint selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRequest {
    pub audience: String,
    pub subject: String,
    pub actor: String,
    pub organization: String,
    pub deployment: String,
    pub connection: String,
    pub grant: String,
    pub resource: String,
    pub operation: String,
    pub channel_kind: String,
    pub protocol: String,
    pub endpoint: String,
    pub proof_thumbprint: String,
    pub authority_id: String,
    pub issued_at: u64,
    pub not_before: u64,
    pub expires_at: u64,
    pub lease_expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthorityError {
    #[error("authority lifetime must be positive and at most 60 seconds")]
    InvalidLifetime,
    #[error("session lease must outlive authority establishment")]
    InvalidLease,
    #[error("session lease expired")]
    LeaseExpired,
    #[error("WebSocket upgrade URI must be an exact absolute wss URI without query or fragment")]
    InvalidEndpoint,
    #[error("compact JWS is malformed")]
    Malformed,
    #[error("signed object has an unsupported type, algorithm, or key")]
    UnsupportedSignature,
    #[error("signature verification failed")]
    InvalidSignature,
    #[error("authority is not active")]
    NotActive,
    #[error("authority expired")]
    Expired,
    #[error("authority binding `{0}` does not match the admitted endpoint")]
    BindingMismatch(&'static str),
    #[error("DPoP proof does not bind the exact method and URI")]
    DpopTargetMismatch,
    #[error("DPoP proof key does not match cnf.jkt")]
    DpopKeyMismatch,
    #[error("DPoP proof is not fresh")]
    DpopNotFresh,
    #[error("DPoP ath does not bind the presented authority")]
    DpopAuthorityMismatch,
    #[error("authority was already redeemed")]
    Replayed,
    #[error("issuer key or deployment is revoked")]
    Revoked,
    #[error("replay store failed: {0}")]
    ReplayStore(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityHeader {
    typ: String,
    alg: String,
    kid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicJwk {
    kty: String,
    crv: String,
    x: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DpopHeader {
    typ: String,
    alg: String,
    jwk: PublicJwk,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DpopClaims {
    htu: String,
    htm: String,
    iat: u64,
    jti: String,
    ath: String,
}

/// Issuer with one deployment signing identity.
pub struct AuthorityIssuer {
    issuer: String,
    key_id: String,
    signing_key: SigningKey,
}

impl AuthorityIssuer {
    pub fn new(
        issuer: impl Into<String>,
        key_id: impl Into<String>,
        signing_key: SigningKey,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            key_id: key_id.into(),
            signing_key,
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn issue(&self, request: IssueRequest) -> Result<IssuedAuthority, AuthorityError> {
        validate_endpoint(&request.endpoint)?;
        let lifetime = request.expires_at.saturating_sub(request.issued_at);
        if lifetime == 0 || lifetime > MAX_AUTHORITY_LIFETIME_SECONDS {
            return Err(AuthorityError::InvalidLifetime);
        }
        if request.not_before < request.issued_at || request.not_before > request.expires_at {
            return Err(AuthorityError::InvalidLifetime);
        }
        if request.lease_expires_at < request.expires_at {
            return Err(AuthorityError::InvalidLease);
        }
        let header = AuthorityHeader {
            typ: AUTHORITY_TYPE.to_owned(),
            alg: "EdDSA".to_owned(),
            kid: self.key_id.clone(),
        };
        let claims = SessionAuthorityClaims {
            iss: self.issuer.clone(),
            aud: request.audience,
            sub: request.subject,
            act: request.actor,
            iat: request.issued_at,
            nbf: request.not_before,
            exp: request.expires_at,
            jti: request.authority_id,
            cnf: Confirmation {
                jkt: request.proof_thumbprint,
            },
            dl_org: request.organization,
            dl_deployment: request.deployment,
            dl_connection: request.connection,
            dl_grant: request.grant,
            dl_resource: request.resource,
            dl_operation: request.operation,
            dl_channel_kind: request.channel_kind,
            dl_protocol: request.protocol,
            dl_endpoint: request.endpoint,
            dl_session_lease_exp: request.lease_expires_at,
        };
        let compact = sign_compact(&header, &claims, &self.signing_key)?;
        Ok(IssuedAuthority {
            compact: SensitiveCompact::new(compact),
            claims,
        })
    }
}

/// Ephemeral client proof key. It is generated before brokerage and never sent as a private key.
pub struct ProofKey(SigningKey);

impl ProofKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(SigningKey::from_bytes(bytes))
    }

    pub fn thumbprint(&self) -> String {
        jwk_thumbprint(&public_jwk(&self.0.verifying_key()))
    }

    pub fn proof(
        &self,
        method: &str,
        uri: &str,
        authority: &IssuedAuthority,
        issued_at: u64,
        proof_id: impl Into<String>,
    ) -> Result<SensitiveCompact, AuthorityError> {
        validate_endpoint(uri)?;
        if method != "GET" {
            return Err(AuthorityError::DpopTargetMismatch);
        }
        let header = DpopHeader {
            typ: DPOP_TYPE.to_owned(),
            alg: "EdDSA".to_owned(),
            jwk: public_jwk(&self.0.verifying_key()),
        };
        let claims = DpopClaims {
            htu: uri.to_owned(),
            htm: method.to_owned(),
            iat: issued_at,
            jti: proof_id.into(),
            ath: hash_b64(authority.compact().as_wire_value().as_bytes()),
        };
        sign_compact(&header, &claims, &self.0).map(SensitiveCompact::new)
    }
}

/// Exact admitted facts the serving endpoint expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedAuthority {
    pub issuer: String,
    pub audience: String,
    pub subject: String,
    pub actor: String,
    pub organization: String,
    pub deployment: String,
    pub connection: String,
    pub grant: String,
    pub resource: String,
    pub operation: String,
    pub channel_kind: String,
    pub protocol: String,
    pub endpoint: String,
}

/// Atomic replay storage. The serving endpoint calls it only after every signature and binding
/// check, and before accepting any session bytes.
pub trait ReplayStore: Send + Sync {
    fn redeem_once(
        &self,
        issuer: &str,
        authority_id: &str,
        expires_at: u64,
    ) -> Result<bool, String>;
}

/// Revocation view for deployment/session signing identities.
pub trait RevocationView: Send + Sync {
    fn is_revoked(&self, issuer: &str, key_id: &str, deployment: &str) -> bool;
}

/// Deterministic in-memory replay store for local and conformance use.
#[derive(Default)]
pub struct InMemoryReplayStore(Mutex<BTreeSet<(String, String)>>);

impl ReplayStore for InMemoryReplayStore {
    fn redeem_once(
        &self,
        issuer: &str,
        authority_id: &str,
        _expires_at: u64,
    ) -> Result<bool, String> {
        Ok(self
            .0
            .lock()
            .map_err(|_| "replay store lock poisoned".to_owned())?
            .insert((issuer.to_owned(), authority_id.to_owned())))
    }
}

/// Empty revocation view for local conformance tests.
pub struct NoRevocations;

impl RevocationView for NoRevocations {
    fn is_revoked(&self, _issuer: &str, _key_id: &str, _deployment: &str) -> bool {
        false
    }
}

/// Sensitive network inputs to one WebSocket upgrade. `Debug` redacts both compact objects.
pub struct RedemptionRequest {
    pub method: String,
    pub uri: String,
    pub authority: SensitiveCompact,
    pub dpop: SensitiveCompact,
}

impl std::fmt::Debug for RedemptionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedemptionRequest")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("authority", &"<redacted>")
            .field("dpop", &"<redacted>")
            .finish()
    }
}

/// Serving-endpoint verifier. The connecting endpoint presents; only this side redeems.
pub struct AuthorityRedeemer<'a> {
    trusted_issuer: String,
    trusted_key_id: String,
    trusted_key: VerifyingKey,
    replay: &'a dyn ReplayStore,
    revocations: &'a dyn RevocationView,
}

impl<'a> AuthorityRedeemer<'a> {
    pub fn new(
        trusted_issuer: impl Into<String>,
        trusted_key_id: impl Into<String>,
        trusted_key: VerifyingKey,
        replay: &'a dyn ReplayStore,
        revocations: &'a dyn RevocationView,
    ) -> Self {
        Self {
            trusted_issuer: trusted_issuer.into(),
            trusted_key_id: trusted_key_id.into(),
            trusted_key,
            replay,
            revocations,
        }
    }

    pub fn redeem(
        &self,
        request: &RedemptionRequest,
        expected: &ExpectedAuthority,
        now: u64,
    ) -> Result<RedeemedAuthority, AuthorityError> {
        validate_endpoint(&request.uri)?;
        if request.method != "GET" || request.uri != expected.endpoint {
            return Err(AuthorityError::DpopTargetMismatch);
        }

        let (header, claims): (AuthorityHeader, SessionAuthorityClaims) =
            verify_compact(request.authority.as_wire_value(), &self.trusted_key)?;
        if header.typ != AUTHORITY_TYPE
            || header.alg != "EdDSA"
            || header.kid != self.trusted_key_id
            || claims.iss != self.trusted_issuer
        {
            return Err(AuthorityError::UnsupportedSignature);
        }
        if self
            .revocations
            .is_revoked(&claims.iss, &header.kid, &claims.dl_deployment)
        {
            return Err(AuthorityError::Revoked);
        }
        let Some(lifetime) = claims.exp.checked_sub(claims.iat) else {
            return Err(AuthorityError::InvalidLifetime);
        };
        if lifetime == 0
            || lifetime > MAX_AUTHORITY_LIFETIME_SECONDS
            || claims.nbf < claims.iat
            || claims.nbf > claims.exp
        {
            return Err(AuthorityError::InvalidLifetime);
        }
        if claims.dl_session_lease_exp < claims.exp {
            return Err(AuthorityError::InvalidLease);
        }
        if now > claims.dl_session_lease_exp {
            return Err(AuthorityError::LeaseExpired);
        }
        if now.saturating_add(CLOCK_SKEW_SECONDS) < claims.nbf {
            return Err(AuthorityError::NotActive);
        }
        if now > claims.exp.saturating_add(CLOCK_SKEW_SECONDS) {
            return Err(AuthorityError::Expired);
        }

        check_binding("issuer", &claims.iss, &expected.issuer)?;
        check_binding("audience", &claims.aud, &expected.audience)?;
        check_binding("subject", &claims.sub, &expected.subject)?;
        check_binding("actor", &claims.act, &expected.actor)?;
        check_binding("organization", &claims.dl_org, &expected.organization)?;
        check_binding("deployment", &claims.dl_deployment, &expected.deployment)?;
        check_binding("connection", &claims.dl_connection, &expected.connection)?;
        check_binding("grant", &claims.dl_grant, &expected.grant)?;
        check_binding("resource", &claims.dl_resource, &expected.resource)?;
        check_binding("operation", &claims.dl_operation, &expected.operation)?;
        check_binding(
            "channel_kind",
            &claims.dl_channel_kind,
            &expected.channel_kind,
        )?;
        check_binding("protocol", &claims.dl_protocol, &expected.protocol)?;
        check_binding("endpoint", &claims.dl_endpoint, &expected.endpoint)?;

        let (dpop_header, dpop): (DpopHeader, DpopClaims) =
            decode_compact(request.dpop.as_wire_value())?;
        if dpop_header.typ != DPOP_TYPE
            || dpop_header.alg != "EdDSA"
            || dpop_header.jwk.kty != "OKP"
            || dpop_header.jwk.crv != "Ed25519"
        {
            return Err(AuthorityError::UnsupportedSignature);
        }
        let proof_key = verifying_key(&dpop_header.jwk)?;
        verify_signature(request.dpop.as_wire_value(), &proof_key)?;
        if jwk_thumbprint(&dpop_header.jwk) != claims.cnf.jkt {
            return Err(AuthorityError::DpopKeyMismatch);
        }
        if dpop.htm != request.method || dpop.htu != request.uri {
            return Err(AuthorityError::DpopTargetMismatch);
        }
        if now.abs_diff(dpop.iat) > CLOCK_SKEW_SECONDS {
            return Err(AuthorityError::DpopNotFresh);
        }
        if dpop.ath != hash_b64(request.authority.as_wire_value().as_bytes()) {
            return Err(AuthorityError::DpopAuthorityMismatch);
        }

        let first = self
            .replay
            .redeem_once(&claims.iss, &claims.jti, claims.exp)
            .map_err(AuthorityError::ReplayStore)?;
        if !first {
            return Err(AuthorityError::Replayed);
        }
        Ok(RedeemedAuthority(claims))
    }
}

fn check_binding(name: &'static str, actual: &str, expected: &str) -> Result<(), AuthorityError> {
    if actual == expected {
        Ok(())
    } else {
        Err(AuthorityError::BindingMismatch(name))
    }
}

fn validate_endpoint(uri: &str) -> Result<(), AuthorityError> {
    let Some(rest) = uri.strip_prefix("wss://") else {
        return Err(AuthorityError::InvalidEndpoint);
    };
    let Some((authority, path)) = rest.split_once('/') else {
        return Err(AuthorityError::InvalidEndpoint);
    };
    if authority.is_empty()
        || path.is_empty()
        || uri.contains(['?', '#'])
        || !uri.is_ascii()
        || authority.contains('@')
        || path.split('/').any(|segment| matches!(segment, "." | ".."))
    {
        return Err(AuthorityError::InvalidEndpoint);
    }
    Ok(())
}

fn public_jwk(key: &VerifyingKey) -> PublicJwk {
    PublicJwk {
        kty: "OKP".to_owned(),
        crv: "Ed25519".to_owned(),
        x: URL_SAFE_NO_PAD.encode(key.as_bytes()),
    }
}

fn verifying_key(jwk: &PublicJwk) -> Result<VerifyingKey, AuthorityError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(&jwk.x)
        .map_err(|_| AuthorityError::Malformed)?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| AuthorityError::Malformed)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| AuthorityError::Malformed)
}

fn jwk_thumbprint(jwk: &PublicJwk) -> String {
    hash_b64(
        format!(
            "{{\"crv\":\"{}\",\"kty\":\"{}\",\"x\":\"{}\"}}",
            jwk.crv, jwk.kty, jwk.x
        )
        .as_bytes(),
    )
}

fn hash_b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

fn sign_compact<H: Serialize, P: Serialize>(
    header: &H,
    payload: &P,
    key: &SigningKey,
) -> Result<String, AuthorityError> {
    let header =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(header).map_err(|_| AuthorityError::Malformed)?);
    let payload =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload).map_err(|_| AuthorityError::Malformed)?);
    let signing_input = format!("{header}.{payload}");
    let signature = key.sign(signing_input.as_bytes());
    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

fn decode_compact<H: DeserializeOwned, P: DeserializeOwned>(
    compact: &str,
) -> Result<(H, P), AuthorityError> {
    let parts = compact.split('.').collect::<Vec<_>>();
    let [header, payload, _signature] = parts.as_slice() else {
        return Err(AuthorityError::Malformed);
    };
    let header = URL_SAFE_NO_PAD
        .decode(header)
        .map_err(|_| AuthorityError::Malformed)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AuthorityError::Malformed)?;
    Ok((
        serde_json::from_slice(&header).map_err(|_| AuthorityError::Malformed)?,
        serde_json::from_slice(&payload).map_err(|_| AuthorityError::Malformed)?,
    ))
}

fn verify_signature(compact: &str, key: &VerifyingKey) -> Result<(), AuthorityError> {
    let parts = compact.split('.').collect::<Vec<_>>();
    let [header, payload, signature] = parts.as_slice() else {
        return Err(AuthorityError::Malformed);
    };
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| AuthorityError::Malformed)?;
    let signature =
        Signature::try_from(signature.as_slice()).map_err(|_| AuthorityError::Malformed)?;
    key.verify(format!("{header}.{payload}").as_bytes(), &signature)
        .map_err(|_| AuthorityError::InvalidSignature)
}

fn verify_compact<H: DeserializeOwned, P: DeserializeOwned>(
    compact: &str,
    key: &VerifyingKey,
) -> Result<(H, P), AuthorityError> {
    verify_signature(compact, key)?;
    decode_compact(compact)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AllRevoked;

    impl RevocationView for AllRevoked {
        fn is_revoked(&self, _issuer: &str, _key_id: &str, _deployment: &str) -> bool {
            true
        }
    }

    fn fixture() -> (
        AuthorityIssuer,
        ProofKey,
        ExpectedAuthority,
        IssuedAuthority,
        SensitiveCompact,
    ) {
        let issuer = AuthorityIssuer::new(
            "https://connectors.example",
            "key-1",
            SigningKey::from_bytes(&[7; 32]),
        );
        let proof_key = ProofKey::from_bytes(&[9; 32]);
        let endpoint = "wss://voice.example/rtvbp".to_owned();
        let expected = ExpectedAuthority {
            issuer: "https://connectors.example".to_owned(),
            audience: "application-deployment-1".to_owned(),
            subject: "principal-1".to_owned(),
            actor: "voice-service-1".to_owned(),
            organization: "org-1".to_owned(),
            deployment: "application-deployment-1".to_owned(),
            connection: "connection-1".to_owned(),
            grant: "grant-1".to_owned(),
            resource: "voice-endpoint-1".to_owned(),
            operation: "carrier-call-establish".to_owned(),
            channel_kind: "voice".to_owned(),
            protocol: "b10x.voice.v1".to_owned(),
            endpoint: endpoint.clone(),
        };
        let authority = issuer
            .issue(IssueRequest {
                audience: expected.audience.clone(),
                subject: expected.subject.clone(),
                actor: expected.actor.clone(),
                organization: expected.organization.clone(),
                deployment: expected.deployment.clone(),
                connection: expected.connection.clone(),
                grant: expected.grant.clone(),
                resource: expected.resource.clone(),
                operation: expected.operation.clone(),
                channel_kind: expected.channel_kind.clone(),
                protocol: expected.protocol.clone(),
                endpoint: endpoint.clone(),
                proof_thumbprint: proof_key.thumbprint(),
                authority_id: "authority-1".to_owned(),
                issued_at: 1_000,
                not_before: 1_000,
                expires_at: 1_060,
                lease_expires_at: 2_000,
            })
            .expect("authority issued");
        let proof = proof_key
            .proof("GET", &endpoint, &authority, 1_001, "proof-1")
            .expect("proof issued");
        (issuer, proof_key, expected, authority, proof)
    }

    #[test]
    fn serving_endpoint_redeems_once() {
        let (issuer, _, expected, authority, proof) = fixture();
        let replay = InMemoryReplayStore::default();
        let redeemer = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &NoRevocations,
        );
        let request = authority.presentation("GET", expected.endpoint.clone(), proof);
        redeemer
            .redeem(&request, &expected, 1_001)
            .expect("first redemption succeeds");
        assert_eq!(
            redeemer.redeem(&request, &expected, 1_001),
            Err(AuthorityError::Replayed)
        );
    }

    #[test]
    fn proof_is_bound_to_exact_upgrade_uri() {
        let (issuer, _, expected, authority, proof) = fixture();
        let replay = InMemoryReplayStore::default();
        let redeemer = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &NoRevocations,
        );
        let request = authority.presentation("GET", "wss://voice.example/other", proof);
        assert_eq!(
            redeemer.redeem(&request, &expected, 1_001),
            Err(AuthorityError::DpopTargetMismatch)
        );
    }

    #[test]
    fn audience_expiry_and_revocation_fail_before_redemption() {
        let (issuer, _, mut expected, authority, proof) = fixture();
        let replay = InMemoryReplayStore::default();
        let request = authority.presentation("GET", expected.endpoint.clone(), proof);
        let redeemer = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &NoRevocations,
        );
        expected.audience = "another-deployment".to_owned();
        assert_eq!(
            redeemer.redeem(&request, &expected, 1_001),
            Err(AuthorityError::BindingMismatch("audience"))
        );

        let (issuer, _, expected, authority, proof) = fixture();
        let replay = InMemoryReplayStore::default();
        let request = authority.presentation("GET", expected.endpoint.clone(), proof);
        let redeemer = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &NoRevocations,
        );
        assert_eq!(
            redeemer.redeem(&request, &expected, 1_066),
            Err(AuthorityError::Expired)
        );

        let (issuer, _, expected, authority, proof) = fixture();
        let replay = InMemoryReplayStore::default();
        let request = authority.presentation("GET", expected.endpoint.clone(), proof);
        let redeemer = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &AllRevoked,
        );
        assert_eq!(
            redeemer.redeem(&request, &expected, 1_001),
            Err(AuthorityError::Revoked)
        );
    }

    #[test]
    fn session_lease_cannot_be_extended_by_authority_clock_skew() {
        let (issuer, proof_key, expected, _, _) = fixture();
        let authority = issuer
            .issue(IssueRequest {
                audience: expected.audience.clone(),
                subject: expected.subject.clone(),
                actor: expected.actor.clone(),
                organization: expected.organization.clone(),
                deployment: expected.deployment.clone(),
                connection: expected.connection.clone(),
                grant: expected.grant.clone(),
                resource: expected.resource.clone(),
                operation: expected.operation.clone(),
                channel_kind: expected.channel_kind.clone(),
                protocol: expected.protocol.clone(),
                endpoint: expected.endpoint.clone(),
                proof_thumbprint: proof_key.thumbprint(),
                authority_id: "lease-bound-authority".to_owned(),
                issued_at: 1_000,
                not_before: 1_000,
                expires_at: 1_060,
                lease_expires_at: 1_060,
            })
            .expect("authority issued");
        let proof = proof_key
            .proof("GET", &expected.endpoint, &authority, 1_061, "proof-lease")
            .expect("proof issued");
        let request = authority.presentation("GET", expected.endpoint.clone(), proof);
        let replay = InMemoryReplayStore::default();
        let redeemer = AuthorityRedeemer::new(
            expected.issuer.clone(),
            "key-1",
            issuer.verifying_key(),
            &replay,
            &NoRevocations,
        );
        assert_eq!(
            redeemer.redeem(&request, &expected, 1_061),
            Err(AuthorityError::LeaseExpired)
        );
    }

    #[test]
    fn debug_never_prints_authority_or_proof() {
        let (_, _, expected, authority, proof) = fixture();
        let authority_raw = authority.compact().as_wire_value().to_owned();
        let proof_raw = proof.as_wire_value().to_owned();
        let issued_printed = format!("{authority:?}");
        let request = authority.presentation("GET", expected.endpoint, proof);
        let printed = format!("{request:?}");
        assert!(!issued_printed.contains(&authority_raw));
        assert!(!printed.contains(&authority_raw));
        assert!(!printed.contains(&proof_raw));
        assert_eq!(printed.matches("<redacted>").count(), 2);
    }
}
