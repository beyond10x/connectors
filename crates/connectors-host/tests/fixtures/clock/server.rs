//! Synthetic test-only source. Its chosen clock is not evidence of actual UTC.
//! Independent encoder: no production framing/verifier helpers are called.
// Three test crates include this file and each uses a different part of it.
#![allow(dead_code)]
use ring::{
    digest,
    signature::{Ed25519KeyPair, KeyPair},
};
use std::{
    collections::BTreeMap,
    net::UdpSocket,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

/// The default source's root key (seed 7).
pub fn root_key() -> [u8; 32] {
    key(7)
}

/// The root key of the synthetic source whose constant seed is `seed`; each
/// quorum source in a test uses its own seed, so no two share a key.
pub fn key(seed: u8) -> [u8; 32] {
    Ed25519KeyPair::from_seed_unchecked(&[seed; 32])
        .unwrap()
        .public_key()
        .as_ref()
        .try_into()
        .unwrap()
}

/// One synthetic source's TOML `[[approval_clock.sources]]` entry at the wire
/// version the fixture signs by default.
pub fn source_toml(address: &str, seed: u8) -> String {
    use base64::Engine;
    format!(
        "[[approval_clock.sources]]\naddress='{address}'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n",
        base64::engine::general_purpose::STANDARD.encode(key(seed))
    )
}

/// A `roughtime-quorum/1` `[approval_clock]` table over `sources`
/// (address and seed), in that order.
pub fn quorum_toml(quorum: usize, max_rate_error_ppm: u32, sources: &[(&str, u8)]) -> String {
    let mut table = format!(
        "[approval_clock]\nformat='roughtime-quorum/1'\nquorum={quorum}\nmax_rate_error_ppm={max_rate_error_ppm}\n"
    );
    for (address, seed) in sources {
        table.push_str(&source_toml(address, *seed));
    }
    table
}

/// The answer a [`Server`] gives to its `n`th request (from 0): the reply
/// bytes, or `None` to stay silent.
pub type Answer = Box<dyn Fn(usize, &[u8]) -> Option<Vec<u8>> + Send>;

/// One synthetic UDP source on 127.0.0.1. It counts every request in
/// `requests` and answers only while `respond` holds.
pub struct Server {
    pub address: String,
    pub requests: Arc<AtomicUsize>,
    pub respond: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Server {
    /// A source answering every request with `fixture`'s signed reply.
    pub fn fixture(fixture: Fixture) -> Self {
        Self::spawn(Box::new(move |_, request| Some(fixture.reply(request))))
    }
    /// A source that receives and never answers.
    pub fn silent() -> Self {
        Self::spawn(Box::new(|_, _| None))
    }
    pub fn spawn(answer: Answer) -> Self {
        Self::shared(
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicBool::new(true)),
            answer,
        )
    }
    /// A source whose request count and answering switch are shared with
    /// other sources, so one switch silences a whole quorum.
    pub fn shared(requests: Arc<AtomicUsize>, respond: Arc<AtomicBool>, answer: Answer) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let address = socket.local_addr().unwrap().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let (stopped, counted, answering) = (stop.clone(), requests.clone(), respond.clone());
        let thread = std::thread::spawn(move || {
            let mut packet = [0; 4096];
            while !stopped.load(Ordering::SeqCst) {
                match socket.recv_from(&mut packet) {
                    Ok((size, peer)) => {
                        let n = counted.fetch_add(1, Ordering::SeqCst);
                        if !answering.load(Ordering::SeqCst) {
                            continue;
                        }
                        if let Some(reply) = answer(n, &packet[..size]) {
                            socket.send_to(&reply, peer).unwrap();
                        }
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(e) => panic!("fixture clock source: {e}"),
                }
            }
        });
        Self {
            address,
            requests,
            respond,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
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

#[derive(Clone)]
pub struct Fixture {
    /// The root key's seed ([`key`]); 7 is [`root_key`].
    pub seed: u8,
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
            seed: 7,
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
        let root = Ed25519KeyPair::from_seed_unchecked(&[self.seed; 32]).unwrap();
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
