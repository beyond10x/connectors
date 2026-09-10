//! Roughtime draft-19 test version, with ring cryptography and bounded framing.
//! Verify original signed byte strings, never a reconstructed projection.
use super::{Failure, Result};
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature,
};
use sha2::{Digest, Sha512};
#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

pub(super) const PACKET_SIZE: usize = 1024;
const VERSION: u32 = 0x8000000c;
const DELEGATION: &[u8] = b"RoughTime v1 delegation signature\0";
const RESPONSE: &[u8] = b"RoughTime v1 response signature\0";
pub(super) struct Sample {
    pub midpoint: u64,
    pub radius: u32,
}
const fn tag(t: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*t)
}
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha512::new();
    for p in parts {
        h.update(p);
    }
    let mut out = [0; 32];
    out.copy_from_slice(&h.finalize()[..32]);
    out
}
pub(super) fn request(key: &[u8; 32]) -> Result<([u8; PACKET_SIZE], [u8; 32])> {
    let mut nonce = [0; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| Failure::Unavailable)?;
    let body = encode(vec![
        (tag(b"VER\0"), VERSION.to_le_bytes().to_vec()),
        (tag(b"SRV\0"), hash(&[&[0xff], key]).to_vec()),
        (tag(b"NONC"), nonce.to_vec()),
        (tag(b"TYPE"), 0u32.to_le_bytes().to_vec()),
        (
            tag(b"ZZZZ"),
            vec![0; PACKET_SIZE - 12 - 40 - 4 - 32 - 32 - 4],
        ),
    ]);
    let mut request = [0; PACKET_SIZE];
    request[..8].copy_from_slice(b"ROUGHTIM");
    request[8..12].copy_from_slice(&(body.len() as u32).to_le_bytes());
    request[12..].copy_from_slice(&body);
    Ok((request, nonce))
}
fn encode(mut fields: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    fields.sort_unstable_by_key(|(t, _)| *t);
    let mut out = Vec::new();
    out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
    let mut offset = 0u32;
    for (_, value) in fields.iter().take(fields.len() - 1) {
        offset += value.len() as u32;
        out.extend_from_slice(&offset.to_le_bytes());
    }
    for (t, _) in &fields {
        out.extend_from_slice(&t.to_le_bytes());
    }
    for (_, value) in fields {
        out.extend(value);
    }
    out
}
fn u32le(b: &[u8]) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.try_into().map_err(|_| Failure::Unavailable)?,
    ))
}
fn u64le(b: &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.try_into().map_err(|_| Failure::Unavailable)?,
    ))
}
struct Message<'a>(Vec<(u32, &'a [u8])>);
impl<'a> Message<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < 8 || bytes.len() > PACKET_SIZE || !bytes.len().is_multiple_of(4) {
            return Err(Failure::Unavailable);
        }
        let count = u32le(&bytes[..4])? as usize;
        if !(1..=64).contains(&count) || count * 8 > bytes.len() {
            return Err(Failure::Unavailable);
        }
        let values = &bytes[count * 8..];
        let mut fields = Vec::with_capacity(count);
        let mut start = 0;
        let mut previous = None;
        for i in 0..count {
            let t = u32le(&bytes[(count + i) * 4..(count + i + 1) * 4])?;
            let end = if i + 1 == count {
                values.len()
            } else {
                u32le(&bytes[(i + 1) * 4..(i + 2) * 4])? as usize
            };
            if previous.is_some_and(|p| p >= t) || end % 4 != 0 || end < start || end > values.len()
            {
                return Err(Failure::Unavailable);
            }
            fields.push((t, &values[start..end]));
            previous = Some(t);
            start = end;
        }
        Ok(Self(fields))
    }
    fn get(&self, name: &[u8; 4]) -> Result<&'a [u8]> {
        self.0
            .iter()
            .find(|(t, _)| *t == tag(name))
            .map(|(_, b)| *b)
            .ok_or(Failure::Unavailable)
    }
}
fn frame(bytes: &[u8]) -> Result<Message<'_>> {
    if bytes.len() < 12
        || bytes.len() > PACKET_SIZE
        || &bytes[..8] != b"ROUGHTIM"
        || u32le(&bytes[8..12])? as usize != bytes.len() - 12
    {
        return Err(Failure::Unavailable);
    }
    Message::parse(&bytes[12..])
}
fn signature(key: &[u8], context: &[u8], bytes: &[u8], sig: &[u8]) -> Result<()> {
    if key.len() != 32 || sig.len() != 64 {
        return Err(Failure::Unavailable);
    }
    let message = [context, bytes].concat();
    signature::UnparsedPublicKey::new(&signature::ED25519, key)
        .verify(&message, sig)
        .map_err(|_| Failure::Unavailable)
}
pub(super) fn verify(
    bytes: &[u8],
    request: &[u8],
    nonce: &[u8; 32],
    key: &[u8; 32],
) -> Result<Sample> {
    let response = frame(bytes)?;
    if response.get(b"NONC")? != nonce || u32le(response.get(b"TYPE")?)? != 1 {
        return Err(Failure::Unavailable);
    }
    let srep_bytes = response.get(b"SREP")?;
    let srep = Message::parse(srep_bytes)?;
    if u32le(srep.get(b"VER\0")?)? != VERSION {
        return Err(Failure::Unavailable);
    }
    let versions = srep.get(b"VERS")?;
    if versions.is_empty() || versions.len() > 128 || versions.len() % 4 != 0 {
        return Err(Failure::Unavailable);
    }
    let mut previous = None;
    let mut selected = false;
    for v in versions.as_chunks::<4>().0 {
        let v = u32le(v)?;
        if previous.is_some_and(|p| p >= v) {
            return Err(Failure::Unavailable);
        }
        selected |= v == VERSION;
        previous = Some(v);
    }
    let midpoint = u64le(srep.get(b"MIDP")?)?;
    let radius = u32le(srep.get(b"RADI")?)?;
    let cert = Message::parse(response.get(b"CERT")?)?;
    let dele_bytes = cert.get(b"DELE")?;
    let dele = Message::parse(dele_bytes)?;
    let mint = u64le(dele.get(b"MINT")?)?;
    let maxt = u64le(dele.get(b"MAXT")?)?;
    if !selected || radius == 0 || midpoint < mint || midpoint > maxt {
        return Err(Failure::Unavailable);
    }
    signature(key, DELEGATION, dele_bytes, cert.get(b"SIG\0")?)?;
    signature(
        dele.get(b"PUBK")?,
        RESPONSE,
        srep_bytes,
        response.get(b"SIG\0")?,
    )?;
    let path = response.get(b"PATH")?;
    if path.len() % 32 != 0 || path.len() > 32 * 32 {
        return Err(Failure::Unavailable);
    }
    let mut index = u32le(response.get(b"INDX")?)?;
    let mut h = hash(&[&[0], request]);
    for sibling in path.as_chunks::<32>().0 {
        h = if index & 1 == 0 {
            hash(&[&[1], &h, sibling])
        } else {
            hash(&[&[1], sibling, &h])
        };
        index >>= 1;
    }
    if index != 0 || srep.get(b"ROOT")? != h {
        return Err(Failure::Unavailable);
    }
    Ok(Sample { midpoint, radius })
}
