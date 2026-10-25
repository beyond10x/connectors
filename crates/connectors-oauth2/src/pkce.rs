use crate::{Failure, Secret};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// RFC 7636 §4.2 S256: `BASE64URL(SHA256(ASCII(verifier)))`, unpadded.
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// 32 bytes from the system CSPRNG, unpadded base64url: 43 characters, each RFC 7636
/// unreserved. Used for the PKCE verifier (RFC 7636 §4.1) and for `state`.
pub(crate) fn random() -> Result<Secret, Failure> {
    let mut bytes = Zeroizing::new([0_u8; 32]);
    getrandom::fill(&mut bytes[..]).map_err(|_| Failure::Unavailable)?;
    Ok(Secret::new(URL_SAFE_NO_PAD.encode(&bytes[..])))
}
