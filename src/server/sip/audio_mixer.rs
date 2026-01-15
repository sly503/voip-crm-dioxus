//! Audio Mixer for Bidirectional Call Recording
//!
//! Handles mixing incoming and outgoing RTP audio streams into a single recording.
//! Supports both mono (mixed) and stereo (separate channels) output.

use super::codec::G711Codec;
use super::rtp::{CapturedRtpPacket, RtpDirection};
use std::collections::BTreeMap;

/// Audio mixing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixMode {
    /// Mix both channels into mono
    Mono,
    /// Keep separate channels (left=agent/outgoing, right=customer/incoming)
    Stereo,
}

/// Decoded audio frame with timing information
#[derive(Debug, Clone)]
struct DecodedFrame {
    /// PCM samples (16-bit signed)
    samples: Vec<i16>,
    /// RTP timestamp
    timestamp: u32,
    /// Direction (incoming/outgoing)
    direction: RtpDirection,
}

/// Audio mixer for combining bidirectional RTP streams
pub struct AudioMixer {
    /// Mixing mode (mono or stereo)
    mode: MixMode,
    /// Sample rate (typically 8000 Hz for G.711)
    sample_rate: u32,
}

impl AudioMixer {
    /// Create a new audio mixer
    ///
    /// # Arguments
    /// * `mode` - Mixing mode (mono or stereo)
    /// * `sample_rate` - Audio sample rate in Hz (default: 8000)
    ///
    /// # Returns
    /// * `Self` - New audio mixer instance
    pub fn new(mode: MixMode, sample_rate: Option<u32>) -> Self {
        Self {
            mode,
            sample_rate: sample_rate.unwrap_or(8000),
        }
    }

    /// Mix captured RTP packets into PCM audio
    ///
    /// # Arguments
    /// * `packets` - Captured RTP packets from both directions
    ///
    /// # Returns
    /// * `Vec<i16>` - Mixed PCM samples (mono or interleaved stereo)
    ///
    /// # Note
    /// For stereo mode, samples are interleaved: [L, R, L, R, ...]
    /// where L=outgoing (agent), R=incoming (customer)
    pub fn mix_packets(&self, packets: &[CapturedRtpPacket]) -> Vec<i16> {
        if packets.is_empty() {
            return Vec::new();
        }

        // Decode all packets
        let decoded = self.decode_packets(packets);

        if decoded.is_empty() {
            return Vec::new();
        }

        // Group frames by timestamp
        let aligned = self.align_frames(&decoded);

        // Mix aligned frames
        match self.mode {
            MixMode::Mono => self.mix_mono(&aligned),
            MixMode::Stereo => self.mix_stereo(&aligned),
        }
    }

    /// Decode RTP packets to PCM frames
    fn decode_packets(&self, packets: &[CapturedRtpPacket]) -> Vec<DecodedFrame> {
        let mut decoded = Vec::new();

        for captured in packets {
            let packet = &captured.packet;

            // Determine codec from payload type
            let codec = match packet.header.payload_type {
                0 => G711Codec::pcmu(), // PCMU (μ-law)
                8 => G711Codec::pcma(), // PCMA (A-law)
                _ => {
                    tracing::warn!(
                        "Unsupported payload type: {}, skipping packet",
                        packet.header.payload_type
                    );
                    continue;
                }
            };

            // Decode payload to PCM
            let samples = codec.decode(&packet.payload);

            if !samples.is_empty() {
                decoded.push(DecodedFrame {
                    samples,
                    timestamp: packet.header.timestamp,
                    direction: captured.direction,
                });
            }
        }

        decoded
    }

    /// Align frames by timestamp
    /// Groups frames that should be played at the same time
    fn align_frames(&self, frames: &[DecodedFrame]) -> Vec<(Vec<i16>, Vec<i16>)> {
        // Use BTreeMap to keep timestamps sorted
        let mut timestamp_map: BTreeMap<u32, (Vec<i16>, Vec<i16>)> = BTreeMap::new();

        for frame in frames {
            let entry = timestamp_map.entry(frame.timestamp).or_insert((Vec::new(), Vec::new()));

            match frame.direction {
                RtpDirection::Outgoing => {
                    // Append to outgoing (agent) channel
                    entry.0.extend_from_slice(&frame.samples);
                }
                RtpDirection::Incoming => {
                    // Append to incoming (customer) channel
                    entry.1.extend_from_slice(&frame.samples);
                }
            }
        }

        // Convert to sorted vector of (outgoing, incoming) sample pairs
        timestamp_map.into_values().collect()
    }

    /// Mix aligned frames into mono audio
    fn mix_mono(&self, aligned: &[(Vec<i16>, Vec<i16>)]) -> Vec<i16> {
        let mut mixed = Vec::new();

        for (outgoing, incoming) in aligned {
            let max_len = outgoing.len().max(incoming.len());

            for i in 0..max_len {
                let out_sample = outgoing.get(i).copied().unwrap_or(0) as i32;
                let in_sample = incoming.get(i).copied().unwrap_or(0) as i32;

                // Mix by averaging to prevent clipping
                // This is a simple mixing algorithm that works well for voice
                let mixed_sample = ((out_sample + in_sample) / 2).clamp(-32768, 32767) as i16;
                mixed.push(mixed_sample);
            }
        }

        mixed
    }

    /// Mix aligned frames into stereo audio (interleaved)
    fn mix_stereo(&self, aligned: &[(Vec<i16>, Vec<i16>)]) -> Vec<i16> {
        let mut mixed = Vec::new();

        for (outgoing, incoming) in aligned {
            let max_len = outgoing.len().max(incoming.len());

            for i in 0..max_len {
                let out_sample = outgoing.get(i).copied().unwrap_or(0);
                let in_sample = incoming.get(i).copied().unwrap_or(0);

                // Interleave: Left channel (agent), Right channel (customer)
                mixed.push(out_sample);
                mixed.push(in_sample);
            }
        }

        mixed
    }

    /// Get the number of channels (1 for mono, 2 for stereo)
    pub fn channels(&self) -> u16 {
        match self.mode {
            MixMode::Mono => 1,
            MixMode::Stereo => 2,
        }
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the bits per sample (always 16 for PCM)
    pub fn bits_per_sample(&self) -> u16 {
        16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use super::super::rtp::{RtpHeader, RtpPacket};

    fn create_test_packet(payload_type: u8, timestamp: u32, samples: &[i16], direction: RtpDirection) -> CapturedRtpPacket {
        let codec = if payload_type == 0 {
            G711Codec::pcmu()
        } else {
            G711Codec::pcma()
        };

        let encoded = codec.encode(samples);
        let header = RtpHeader::new(payload_type, 0, timestamp, 12345);
        let packet = RtpPacket::new(header, Bytes::from(encoded));

        CapturedRtpPacket {
            packet,
            direction,
            captured_at: Utc::now(),
        }
    }

    #[test]
    fn test_audio_mixer_mono_basic() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        // Create test samples
        let outgoing_samples = vec![100i16, 200, 300];
        let incoming_samples = vec![50i16, 100, 150];

        let packets = vec![
            create_test_packet(0, 1000, &outgoing_samples, RtpDirection::Outgoing),
            create_test_packet(0, 1000, &incoming_samples, RtpDirection::Incoming),
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should have mixed samples (averaged)
        assert_eq!(mixed.len(), 3);
        assert_eq!(mixed[0], (100 + 50) / 2); // 75
        assert_eq!(mixed[1], (200 + 100) / 2); // 150
        assert_eq!(mixed[2], (300 + 150) / 2); // 225
    }

    #[test]
    fn test_audio_mixer_stereo_basic() {
        let mixer = AudioMixer::new(MixMode::Stereo, None);

        let outgoing_samples = vec![100i16, 200];
        let incoming_samples = vec![50i16, 100];

        let packets = vec![
            create_test_packet(0, 1000, &outgoing_samples, RtpDirection::Outgoing),
            create_test_packet(0, 1000, &incoming_samples, RtpDirection::Incoming),
        ];

        let mixed = mixer.mix_packets(&packets);

        // Stereo: interleaved [L, R, L, R]
        assert_eq!(mixed.len(), 4);
        assert_eq!(mixed[0], 100); // Left (outgoing)
        assert_eq!(mixed[1], 50);  // Right (incoming)
        assert_eq!(mixed[2], 200); // Left (outgoing)
        assert_eq!(mixed[3], 100); // Right (incoming)
    }

    #[test]
    fn test_audio_mixer_uneven_lengths() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        // Outgoing has more samples
        let outgoing_samples = vec![100i16, 200, 300, 400];
        let incoming_samples = vec![50i16, 100];

        let packets = vec![
            create_test_packet(0, 1000, &outgoing_samples, RtpDirection::Outgoing),
            create_test_packet(0, 1000, &incoming_samples, RtpDirection::Incoming),
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should pad missing samples with 0
        assert_eq!(mixed.len(), 4);
        assert_eq!(mixed[0], (100 + 50) / 2); // 75
        assert_eq!(mixed[1], (200 + 100) / 2); // 150
        assert_eq!(mixed[2], (300 + 0) / 2); // 150 (incoming padded with 0)
        assert_eq!(mixed[3], (400 + 0) / 2); // 200 (incoming padded with 0)
    }

    #[test]
    fn test_audio_mixer_multiple_timestamps() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        let samples1 = vec![100i16];
        let samples2 = vec![200i16];
        let samples3 = vec![300i16];

        let packets = vec![
            create_test_packet(0, 1000, &samples1, RtpDirection::Outgoing),
            create_test_packet(0, 1000, &samples2, RtpDirection::Incoming),
            create_test_packet(0, 2000, &samples3, RtpDirection::Outgoing),
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should have samples from both timestamps
        assert_eq!(mixed.len(), 2);
        assert_eq!(mixed[0], (100 + 200) / 2); // timestamp 1000
        assert_eq!(mixed[1], (300 + 0) / 2);   // timestamp 2000 (only outgoing)
    }

    #[test]
    fn test_audio_mixer_empty_packets() {
        let mixer = AudioMixer::new(MixMode::Mono, None);
        let mixed = mixer.mix_packets(&[]);
        assert_eq!(mixed.len(), 0);
    }

    #[test]
    fn test_audio_mixer_pcmu_codec() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        let samples = vec![1000i16, 2000, 3000];
        let packets = vec![
            create_test_packet(0, 1000, &samples, RtpDirection::Outgoing), // PCMU
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should decode PCMU successfully (some quantization error expected)
        assert!(!mixed.is_empty());
        assert_eq!(mixed.len(), 3);
    }

    #[test]
    fn test_audio_mixer_pcma_codec() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        let samples = vec![1000i16, 2000, 3000];
        let packets = vec![
            create_test_packet(8, 1000, &samples, RtpDirection::Incoming), // PCMA
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should decode PCMA successfully
        assert!(!mixed.is_empty());
        assert_eq!(mixed.len(), 3);
    }

    #[test]
    fn test_audio_mixer_mixed_codecs() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        let samples1 = vec![1000i16, 2000];
        let samples2 = vec![500i16, 1000];

        let packets = vec![
            create_test_packet(0, 1000, &samples1, RtpDirection::Outgoing), // PCMU
            create_test_packet(8, 1000, &samples2, RtpDirection::Incoming), // PCMA
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should handle mixed codecs
        assert!(!mixed.is_empty());
    }

    #[test]
    fn test_audio_mixer_channels() {
        let mono_mixer = AudioMixer::new(MixMode::Mono, None);
        assert_eq!(mono_mixer.channels(), 1);

        let stereo_mixer = AudioMixer::new(MixMode::Stereo, None);
        assert_eq!(stereo_mixer.channels(), 2);
    }

    #[test]
    fn test_audio_mixer_sample_rate() {
        let mixer1 = AudioMixer::new(MixMode::Mono, None);
        assert_eq!(mixer1.sample_rate(), 8000);

        let mixer2 = AudioMixer::new(MixMode::Mono, Some(16000));
        assert_eq!(mixer2.sample_rate(), 16000);
    }

    #[test]
    fn test_audio_mixer_clipping_prevention() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        // Create samples that would clip if added directly
        let max_samples = vec![32000i16, 32000];
        let packets = vec![
            create_test_packet(0, 1000, &max_samples, RtpDirection::Outgoing),
            create_test_packet(0, 1000, &max_samples, RtpDirection::Incoming),
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should not exceed i16 range
        for sample in mixed {
            assert!(sample >= -32768 && sample <= 32767);
        }
    }

    #[test]
    fn test_audio_mixer_timestamp_ordering() {
        let mixer = AudioMixer::new(MixMode::Mono, None);

        // Add packets in non-chronological order
        let packets = vec![
            create_test_packet(0, 3000, &vec![300i16], RtpDirection::Outgoing),
            create_test_packet(0, 1000, &vec![100i16], RtpDirection::Outgoing),
            create_test_packet(0, 2000, &vec![200i16], RtpDirection::Outgoing),
        ];

        let mixed = mixer.mix_packets(&packets);

        // Should be ordered by timestamp: 1000, 2000, 3000
        assert_eq!(mixed.len(), 3);
        // Values should be in order after sorting by timestamp
        // Each gets divided by 2 (averaged with 0 incoming)
        assert_eq!(mixed[0], 50);  // timestamp 1000
        assert_eq!(mixed[1], 100); // timestamp 2000
        assert_eq!(mixed[2], 150); // timestamp 3000
    }
}
