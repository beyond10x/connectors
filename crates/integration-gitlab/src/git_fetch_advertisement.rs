//! Bounded legacy Git advertisement filtering for one admitted snapshot.

use async_trait::async_trait;
use service::{EgressByteStream, EgressTransportError};

const MAX_ADVERTISEMENT_BUFFER_BYTES: usize = 256 * 1024;

pub(super) struct ExactAdvertisementStream {
    inner: Box<dyn EgressByteStream>,
    expected_ref: Vec<u8>,
    expected_commit: Vec<u8>,
    buffered: Vec<u8>,
    upstream_done: bool,
    saw_service: bool,
    saw_prelude_flush: bool,
    saw_first_ref: bool,
    saw_target: bool,
}

impl ExactAdvertisementStream {
    pub(super) fn new(
        inner: Box<dyn EgressByteStream>,
        reference: String,
        expected_commit: String,
    ) -> Self {
        Self {
            inner,
            expected_ref: format!("refs/heads/{reference}").into_bytes(),
            expected_commit: expected_commit.into_bytes(),
            buffered: Vec::new(),
            upstream_done: false,
            saw_service: false,
            saw_prelude_flush: false,
            saw_first_ref: false,
            saw_target: false,
        }
    }

    fn parse_available(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        let mut emitted = Vec::new();
        while let Some(header) = self.buffered.get(..4) {
            let header = std::str::from_utf8(header).map_err(|_| EgressTransportError::Refused)?;
            let length =
                usize::from_str_radix(header, 16).map_err(|_| EgressTransportError::Refused)?;
            if length == 0 {
                self.buffered.drain(..4);
                if self.saw_service && !self.saw_prelude_flush {
                    self.saw_prelude_flush = true;
                    emitted.extend_from_slice(b"0000");
                } else if self.saw_target {
                    emitted.extend_from_slice(b"0000");
                }
                continue;
            }
            if !(4..=65_520).contains(&length) {
                return Err(EgressTransportError::Refused);
            }
            if self.buffered.len() < length {
                break;
            }
            let packet = self.buffered.drain(..length).collect::<Vec<_>>();
            let payload = &packet[4..];
            if payload == b"# service=git-upload-pack\n" {
                if self.saw_service || self.saw_first_ref {
                    return Err(EgressTransportError::Refused);
                }
                self.saw_service = true;
                emitted.extend_from_slice(&packet);
                continue;
            }
            let Some((object, reference, capabilities)) = advertised_ref(payload) else {
                return Err(EgressTransportError::Refused);
            };
            if !self.saw_service || !self.saw_prelude_flush {
                return Err(EgressTransportError::Refused);
            }
            if !self.saw_first_ref {
                self.saw_first_ref = true;
                // The first legacy row owns capabilities. HEAD must be the exact default tip.
                if reference != b"HEAD" || object != self.expected_commit {
                    return Err(EgressTransportError::Refused);
                }
            }
            let admitted = object == self.expected_commit
                && (reference == b"HEAD" || reference == self.expected_ref);
            if admitted {
                if let Some(capabilities) = capabilities {
                    for capability in capabilities.split(|byte| *byte == b' ') {
                        if let Some(target) = capability.strip_prefix(b"symref=HEAD:") {
                            if target != self.expected_ref {
                                return Err(EgressTransportError::Refused);
                            }
                        }
                    }
                }
                if reference == self.expected_ref {
                    self.saw_target = true;
                }
                emitted.extend_from_slice(&packet);
            }
        }
        Ok((!emitted.is_empty()).then_some(emitted))
    }
}

#[async_trait]
impl EgressByteStream for ExactAdvertisementStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        loop {
            if let Some(emitted) = self.parse_available()? {
                return Ok(Some(emitted));
            }
            if self.upstream_done {
                if self.buffered.is_empty()
                    && self.saw_service
                    && self.saw_prelude_flush
                    && self.saw_target
                {
                    return Ok(None);
                }
                return Err(EgressTransportError::Refused);
            }
            match self.inner.next_chunk().await? {
                Some(chunk)
                    if self
                        .buffered
                        .len()
                        .checked_add(chunk.len())
                        .is_some_and(|length| length <= MAX_ADVERTISEMENT_BUFFER_BYTES) =>
                {
                    self.buffered.extend_from_slice(&chunk);
                }
                Some(_) => return Err(EgressTransportError::ResponseTooLarge),
                None => self.upstream_done = true,
            }
        }
    }
}

type AdvertisedRef<'a> = (&'a [u8], &'a [u8], Option<&'a [u8]>);

fn advertised_ref(payload: &[u8]) -> Option<AdvertisedRef<'_>> {
    let payload = payload.strip_suffix(b"\n").unwrap_or(payload);
    let (row, capabilities) = payload
        .iter()
        .position(|byte| *byte == 0)
        .map_or((payload, None), |position| {
            (&payload[..position], Some(&payload[position + 1..]))
        });
    let separator = row.iter().position(|byte| *byte == b' ')?;
    let (object, reference_with_separator) = row.split_at(separator);
    let reference = &reference_with_separator[1..];
    (object.len() == 40
        && object
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        && !reference.is_empty())
    .then_some((object, reference, capabilities))
}
