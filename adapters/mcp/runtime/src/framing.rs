//! Bounded newline framing. Interpretation, deadlines and authority belong to the caller.
//! Oversized lines are drained to their boundary; truncated EOF never yields a frame.

pub struct LineDecoder {
    limit: usize,
    pending: Vec<u8>,
    discarding: bool,
}
impl LineDecoder {
    /// The caller supplies an admitted positive wire-byte limit, including newline.
    pub fn new(limit: usize) -> std::io::Result<Self> {
        if limit == 0 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            limit,
            pending: Vec::new(),
            discarding: false,
        })
    }

    /// Consume at most one line, returning its payload only after the newline.
    /// The returned byte count lets the caller apply backpressure before the next
    /// line in the same input chunk. Invalid UTF-8 is preserved for the JSON parser.
    pub fn feed(&mut self, bytes: &[u8]) -> (usize, Option<std::io::Result<Vec<u8>>>) {
        let end = bytes.iter().position(|byte| *byte == b'\n');
        let payload = &bytes[..end.unwrap_or(bytes.len())];
        if !self.discarding {
            // Reserve the required newline even while the line is incomplete.
            let size = self
                .pending
                .len()
                .checked_add(payload.len())
                .and_then(|n| n.checked_add(1));
            if size.is_none_or(|n| n > self.limit) {
                self.pending.clear();
                self.discarding = true;
            } else {
                self.pending.extend_from_slice(payload);
            }
        }
        let Some(end) = end else {
            return (bytes.len(), None);
        };
        let frame = if self.discarding {
            self.discarding = false;
            Err(std::io::ErrorKind::InvalidData.into())
        } else {
            Ok(std::mem::take(&mut self.pending))
        };
        (end + 1, Some(frame))
    }

    /// Discard partial input on EOF. True records that a partial/oversized line
    /// was discarded; it must not become a protocol request or successful prefix.
    pub fn finish(&mut self) -> bool {
        let discarded = self.discarding || !self.pending.is_empty();
        self.pending.clear();
        self.discarding = false;
        discarded
    }

    #[must_use]
    pub fn retained_octets(&self) -> usize {
        self.pending.len()
    }
}
