//! Protocol-neutral local-audio vocabulary.
//!
//! Nothing here names a synthesizer product, a voice, or an executable. The variants classify
//! **device stacks**, which is a fact about the machine an operation is placed on, not a fact about
//! any vendor. The closed `audio_v1` driver is the only code that knows how a stack is driven.

use serde::{Deserialize, Serialize};

/// The local audio sink family a deployment admitted, or that a driver probe resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSink {
    /// PipeWire.
    PipeWire,
    /// PulseAudio.
    PulseAudio,
    /// ALSA.
    Alsa,
}

impl AudioSink {
    /// The sink candidates, in the exact order a probe considers them.
    ///
    /// The order is a deployment fact, not a preference a caller can express: a probe takes the
    /// first stack present and never retries a failed one through another.
    #[must_use]
    pub const fn candidates() -> [Self; 3] {
        [Self::PipeWire, Self::PulseAudio, Self::Alsa]
    }

    /// The stable token this sink is recorded as.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PipeWire => "pipe-wire",
            Self::PulseAudio => "pulse-audio",
            Self::Alsa => "alsa",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_candidate_is_distinct_and_ordered() {
        let candidates = AudioSink::candidates();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0], AudioSink::PipeWire);
        for pair in candidates.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
    }

    #[test]
    fn the_recorded_token_round_trips_through_serde() {
        for sink in AudioSink::candidates() {
            let encoded = serde_json::to_string(&sink).expect("sink encodes");
            assert_eq!(encoded, format!("\"{}\"", sink.as_str()));
        }
    }
}
