use super::types::{ProofBinding, canonical, digest, identifier};
use super::*;
use crate::local::mutations::{Clock, ClockInterval};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_sdk::Secret;
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::{self, KeyPair},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

const TYPE: &str = "b10x.connectors-approval.v1+jws";
const MAX_SAFE: i64 = 9_007_199_254_740_991;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: String,
    kid: String,
    typ: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Claims {
    pub iss: String,
    pub aud: String,
    pub reference: String,
    pub subject: Subject,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
}
pub(super) struct Decoded<'a> {
    header: Header,
    claims: Claims,
    signing_input: &'a [u8],
    signature: Vec<u8>,
    proof_sha256: [u8; 32],
}
pub(super) fn decode(evidence: &Evidence) -> Result<Decoded<'_>> {
    let compact = std::str::from_utf8(evidence.bytes()).map_err(|_| Failure::Refused)?;
    let parts: Vec<_> = compact.split('.').collect();
    if parts.len() != 3 {
        return Err(Failure::Refused);
    }
    let header: Header = strict(&decode64(parts[0], 512)?)?;
    let claims: Claims = strict(&decode64(parts[1], 12 * 1024)?)?;
    let signature = decode64(parts[2], 64)?;
    if header.alg != "Ed25519"
        || header.typ != TYPE
        || signature.len() != 64
        || claims.reference != evidence.reference()
    {
        return Err(Failure::Refused);
    }
    identifier(&header.kid)?;
    identifier(&claims.iss)?;
    identifier(&claims.aud)?;
    digest(&claims.reference)?;
    claims.subject.canonical_bytes()?;
    Ok(Decoded {
        header,
        claims,
        signature,
        signing_input: &evidence.bytes()[..parts[0].len() + 1 + parts[1].len()],
        proof_sha256: Sha256::digest(evidence.bytes()).into(),
    })
}
fn strict<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T> {
    let value: T = connectors_core::read_json(bytes).map_err(|_| Failure::Refused)?;
    // This also rejects omitted nullable members and noncanonical number, escape,
    // whitespace and field-order spellings after the duplicate/unknown checks.
    if canonical(&value)? != bytes {
        return Err(Failure::Refused);
    }
    Ok(value)
}
fn decode64(value: &str, limit: usize) -> Result<Vec<u8>> {
    if value.len() > limit.div_ceil(3) * 4 {
        return Err(Failure::Refused);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| Failure::Refused)?;
    if bytes.len() > limit || URL_SAFE_NO_PAD.encode(&bytes) != value {
        return Err(Failure::Refused);
    }
    Ok(bytes)
}
pub(super) fn now(clock: &impl Clock) -> Result<ClockInterval> {
    let t = clock.now().map_err(|_| Failure::Unavailable)?;
    if t.lower_unix_ms < 0
        || t.upper_unix_ms > MAX_SAFE
        || !(0..=4000).contains(
            &t.upper_unix_ms
                .checked_sub(t.lower_unix_ms)
                .ok_or(Failure::Unavailable)?,
        )
    {
        return Err(Failure::Unavailable);
    }
    Ok(t)
}
fn public_key(key: &ConfiguredApprovalKey, kid: &str, t: ClockInterval) -> Result<Vec<u8>> {
    identifier(&key.issuer)?;
    identifier(&key.audience)?;
    identifier(&key.kid)?;
    if key.revoked
        || kid != key.kid
        || key.not_before_unix_ms < 0
        || key.not_after_unix_ms > MAX_SAFE
        || key.not_before_unix_ms > t.lower_unix_ms
        || t.upper_unix_ms >= key.not_after_unix_ms
    {
        return Err(Failure::Refused);
    }
    let bytes = decode64(&key.public_key, 32)?;
    if bytes.len() != 32 {
        return Err(Failure::Refused);
    }
    Ok(bytes)
}
impl Decoded<'_> {
    pub(super) fn kid(&self) -> &str {
        &self.header.kid
    }
    pub(super) fn verify(
        &self,
        subject: &Subject,
        key: &ConfiguredApprovalKey,
        t: ClockInterval,
    ) -> Result<ProofBinding> {
        subject.canonical_bytes()?;
        let c = &self.claims;
        let public = public_key(key, &self.header.kid, t)?;
        if c.iss != key.issuer
            || c.aud != key.audience
            || c.subject != *subject
            || ![c.iat, c.nbf, c.exp]
                .iter()
                .all(|v| (0..=MAX_SAFE).contains(v))
            || c.iat.checked_sub(5) != Some(c.nbf)
            || c.iat.checked_add(295) != Some(c.exp)
        {
            return Err(Failure::Refused);
        }
        let nbf = c
            .nbf
            .checked_mul(1000)
            .filter(|v| *v <= MAX_SAFE)
            .ok_or(Failure::Refused)?;
        let exp = c
            .exp
            .checked_mul(1000)
            .filter(|v| *v <= MAX_SAFE)
            .ok_or(Failure::Refused)?;
        if nbf > t.lower_unix_ms || t.upper_unix_ms >= exp {
            return Err(Failure::Refused);
        }
        signature::UnparsedPublicKey::new(&signature::ED25519, public)
            .verify(self.signing_input, &self.signature)
            .map_err(|_| Failure::Refused)?;
        Ok(ProofBinding {
            subject: subject.clone(),
            issuer: c.iss.clone(),
            reference: c.reference.clone(),
            proof_sha256: self.proof_sha256,
        })
    }
}
pub fn verify(
    evidence: &Evidence,
    expected: &Subject,
    policy: &impl ReceiverPolicy,
    clock: &impl Clock,
) -> Result<VerifiedApproval> {
    let decoded = decode(evidence)?;
    let guard = policy.admit(expected, decoded.kid())?;
    let binding = decoded.verify(expected, guard.key(), now(clock)?)?;
    Ok(VerifiedApproval { binding })
}

/// In-memory signing capability. Persistent seed custody, key publication and
/// rotation remain the consumer's responsibility; never use provider credentials.
pub struct Signer {
    seed: Secret,
    kid: String,
}
impl Signer {
    pub fn from_seed(seed: Secret, kid: String) -> Result<Self> {
        identifier(&kid)?;
        if seed.0.len() != 32 {
            return Err(Failure::Refused);
        }
        Ok(Self { seed, kid })
    }
    pub fn issue(
        &self,
        subject: &Subject,
        policy: &impl IssuancePolicy,
        clock: &impl Clock,
    ) -> Result<Evidence> {
        subject.canonical_bytes()?;
        let guard = policy.authorize(subject, &self.kid)?;
        let key = guard.key();
        let t = now(clock)?;
        let public = public_key(key, &self.kid, t)?;
        let pair = signature::Ed25519KeyPair::from_seed_unchecked(&self.seed.0)
            .map_err(|_| Failure::Unavailable)?;
        if pair.public_key().as_ref() != public {
            return Err(Failure::Refused);
        }
        let iat = (t.lower_unix_ms + (t.upper_unix_ms - t.lower_unix_ms) / 2) / 1000;
        let nbf = iat
            .checked_sub(5)
            .filter(|v| *v >= 0)
            .ok_or(Failure::Refused)?;
        let exp = iat
            .checked_add(295)
            .filter(|v| v.checked_mul(1000).is_some_and(|v| v <= MAX_SAFE))
            .ok_or(Failure::Refused)?;
        let mut random = [0u8; 32];
        SystemRandom::new()
            .fill(&mut random)
            .map_err(|_| Failure::Unavailable)?;
        let reference = hex::encode(random);
        let claims = Claims {
            iss: key.issuer.clone(),
            aud: key.audience.clone(),
            reference: reference.clone(),
            subject: subject.clone(),
            iat,
            nbf,
            exp,
        };
        let header = Header {
            alg: "Ed25519".into(),
            kid: self.kid.clone(),
            typ: TYPE.into(),
        };
        let header = canonical(&header)?;
        let claims = canonical(&claims)?;
        if header.len() > 512 || claims.len() > 12 * 1024 {
            return Err(Failure::Refused);
        }
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(header),
            URL_SAFE_NO_PAD.encode(claims)
        );
        let sig = pair.sign(input.as_bytes());
        Evidence::from_protected(
            reference,
            Secret(format!("{input}.{}", URL_SAFE_NO_PAD.encode(sig.as_ref())).into_bytes()),
        )
    }
}
