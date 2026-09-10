//! Synthetic test-only source. Its chosen clock is not evidence of actual UTC.
//! Independent encoder: no production framing/verifier helpers are called.
use ring::{
    digest,
    signature::{Ed25519KeyPair, KeyPair},
};
use std::collections::BTreeMap;

pub fn root_key() -> [u8; 32] {
    Ed25519KeyPair::from_seed_unchecked(&[7; 32])
        .unwrap()
        .public_key()
        .as_ref()
        .try_into()
        .unwrap()
}
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut ctx = digest::Context::new(&digest::SHA512);
    for part in parts {
        ctx.update(part);
    }
    ctx.finish().as_ref()[..32].try_into().unwrap()
}
pub fn message(fields: impl IntoIterator<Item = ([u8; 4], Vec<u8>)>) -> Vec<u8> {
    let fields: BTreeMap<u32, Vec<u8>> = fields
        .into_iter()
        .map(|(t, b)| (u32::from_le_bytes(t), b))
        .collect();
    let mut header = vec![(fields.len() as u32).to_le_bytes().to_vec()];
    let mut offset = 0u32;
    for b in fields.values().take(fields.len() - 1) {
        offset += b.len() as u32;
        header.push(offset.to_le_bytes().to_vec());
    }
    header.extend(fields.keys().map(|t| t.to_le_bytes().to_vec()));
    header.extend(fields.into_values());
    header.concat()
}
pub fn frame(body: &[u8]) -> Vec<u8> {
    [
        b"ROUGHTIM".as_slice(),
        &(body.len() as u32).to_le_bytes(),
        body,
    ]
    .concat()
}
pub fn fields(body: &[u8]) -> BTreeMap<[u8; 4], Vec<u8>> {
    let n = u32::from_le_bytes(body[..4].try_into().unwrap()) as usize;
    let data = &body[n * 8..];
    let mut offsets = vec![0];
    offsets.extend(
        body[4..n * 4]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b) as usize),
    );
    offsets.push(data.len());
    body[n * 4..n * 8]
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, data[offsets[i]..offsets[i + 1]].to_vec()))
        .collect()
}
fn signed(pair: &Ed25519KeyPair, context: &[u8], bytes: &[u8]) -> Vec<u8> {
    pair.sign(&[context, bytes].concat()).as_ref().to_vec()
}

pub struct Fixture {
    pub midpoint: u64,
    pub radius: u32,
    pub mint: u64,
    pub maxt: u64,
    pub version: u32,
    pub versions: Vec<u32>,
    pub msg_type: u32,
    pub extra: bool,
    pub index: u32,
    pub sibling: Option<[u8; 32]>,
    pub nonce_override: Option<[u8; 32]>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            midpoint: 1_789_076_475,
            radius: 1,
            mint: 1_789_000_000,
            maxt: 1_790_000_000,
            version: 0x8000000c,
            versions: vec![0x8000000c],
            msg_type: 1,
            extra: false,
            index: 0,
            sibling: None,
            nonce_override: None,
        }
    }
}
impl Fixture {
    pub fn reply(&self, request: &[u8]) -> Vec<u8> {
        let root = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        let delegated = Ed25519KeyPair::from_seed_unchecked(&[9; 32]).unwrap();
        let mut delegation = vec![
            (*b"PUBK", delegated.public_key().as_ref().to_vec()),
            (*b"MINT", self.mint.to_le_bytes().to_vec()),
            (*b"MAXT", self.maxt.to_le_bytes().to_vec()),
        ];
        if self.extra {
            delegation.push((*b"FOO\0", vec![0; 4]));
        }
        let delegation = message(delegation);
        let mut certificate = vec![
            (*b"DELE", delegation.clone()),
            (
                *b"SIG\0",
                signed(&root, b"RoughTime v1 delegation signature\0", &delegation),
            ),
        ];
        if self.extra {
            certificate.push((*b"FOO\0", vec![0; 4]));
        }
        let certificate = message(certificate);
        let mut h = hash(&[&[0], request]);
        if let Some(sibling) = self.sibling {
            h = if self.index & 1 == 0 {
                hash(&[&[1], &h, &sibling])
            } else {
                hash(&[&[1], &sibling, &h])
            };
        }
        let mut response = vec![
            (*b"VER\0", self.version.to_le_bytes().to_vec()),
            (*b"RADI", self.radius.to_le_bytes().to_vec()),
            (*b"MIDP", self.midpoint.to_le_bytes().to_vec()),
            (
                *b"VERS",
                self.versions.iter().flat_map(|v| v.to_le_bytes()).collect(),
            ),
            (*b"ROOT", h.to_vec()),
        ];
        if self.extra {
            response.push((*b"FOO\0", vec![0; 4]));
        }
        let response = message(response);
        let nonce = self
            .nonce_override
            .map(|v| v.to_vec())
            .unwrap_or_else(|| fields(&request[12..])[b"NONC"].clone());
        let mut packet = vec![
            (
                *b"SIG\0",
                signed(&delegated, b"RoughTime v1 response signature\0", &response),
            ),
            (*b"NONC", nonce),
            (*b"TYPE", self.msg_type.to_le_bytes().to_vec()),
            (
                *b"PATH",
                self.sibling.map(|v| v.to_vec()).unwrap_or_default(),
            ),
            (*b"SREP", response),
            (*b"CERT", certificate),
            (*b"INDX", self.index.to_le_bytes().to_vec()),
        ];
        if self.extra {
            packet.push((*b"FOO\0", vec![0; 4]));
        }
        frame(&message(packet))
    }
}
