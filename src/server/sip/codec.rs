//! G.711 Audio Codec Implementation
//!
//! Pure Rust implementation of G.711 μ-law (PCMU) and A-law (PCMA) codecs.
//! These are the standard telephone codecs used worldwide.

use super::config::SipCodec;

/// G.711 codec for encoding/decoding telephone audio
pub struct G711Codec {
    codec_type: SipCodec,
}

impl G711Codec {
    pub fn new(codec_type: SipCodec) -> Self {
        Self { codec_type }
    }

    /// Create μ-law encoder (US standard)
    pub fn pcmu() -> Self {
        Self::new(SipCodec::Pcmu)
    }

    /// Create A-law encoder (EU standard)
    pub fn pcma() -> Self {
        Self::new(SipCodec::Pcma)
    }

    /// Encode 16-bit PCM samples to G.711
    pub fn encode(&self, pcm: &[i16]) -> Vec<u8> {
        match self.codec_type {
            SipCodec::Pcmu => pcm.iter().map(|&s| linear_to_ulaw(s)).collect(),
            SipCodec::Pcma => pcm.iter().map(|&s| linear_to_alaw(s)).collect(),
        }
    }

    /// Decode G.711 to 16-bit PCM samples
    pub fn decode(&self, encoded: &[u8]) -> Vec<i16> {
        match self.codec_type {
            SipCodec::Pcmu => encoded.iter().map(|&b| ulaw_to_linear(b)).collect(),
            SipCodec::Pcma => encoded.iter().map(|&b| alaw_to_linear(b)).collect(),
        }
    }

    /// Get the RTP payload type
    pub fn payload_type(&self) -> u8 {
        self.codec_type.payload_type()
    }
}

// μ-law encoding table segments
const ULAW_BIAS: i32 = 0x84;
const ULAW_CLIP: i32 = 32635;

/// Convert 16-bit linear PCM to μ-law
fn linear_to_ulaw(sample: i16) -> u8 {
    // Get the sign
    let sign = if sample < 0 { 0x80 } else { 0x00 };

    // Get absolute value and apply bias
    let mut sample = if sample < 0 {
        (-sample as i32).min(ULAW_CLIP)
    } else {
        (sample as i32).min(ULAW_CLIP)
    };

    sample += ULAW_BIAS;

    // Find the segment
    let exponent = match sample {
        s if s >= 0x4000 => 7,
        s if s >= 0x2000 => 6,
        s if s >= 0x1000 => 5,
        s if s >= 0x0800 => 4,
        s if s >= 0x0400 => 3,
        s if s >= 0x0200 => 2,
        s if s >= 0x0100 => 1,
        _ => 0,
    };

    let mantissa = (sample >> (exponent + 3)) & 0x0F;

    // Combine sign, exponent, and mantissa, then complement
    !(sign | (exponent << 4) | mantissa as u8)
}

/// Convert μ-law to 16-bit linear PCM
fn ulaw_to_linear(ulaw: u8) -> i16 {
    // Complement the byte
    let ulaw = !ulaw;

    let sign = ulaw & 0x80;
    let exponent = ((ulaw >> 4) & 0x07) as i32;
    let mantissa = (ulaw & 0x0F) as i32;

    // Reconstruct the linear value
    let mut sample = ((mantissa << 3) + ULAW_BIAS) << exponent;
    sample -= ULAW_BIAS;

    if sign != 0 {
        -sample as i16
    } else {
        sample as i16
    }
}

// A-law encoding constants
const ALAW_CLIP: i32 = 32767;

/// Convert 16-bit linear PCM to A-law
fn linear_to_alaw(sample: i16) -> u8 {
    let sign = if sample < 0 { 0x00 } else { 0x80 };

    let mut sample = if sample < 0 {
        (-sample as i32).min(ALAW_CLIP)
    } else {
        (sample as i32).min(ALAW_CLIP)
    };

    let (exponent, mantissa) = if sample >= 256 {
        let exp = match sample {
            s if s >= 0x4000 => 7,
            s if s >= 0x2000 => 6,
            s if s >= 0x1000 => 5,
            s if s >= 0x0800 => 4,
            s if s >= 0x0400 => 3,
            s if s >= 0x0200 => 2,
            s if s >= 0x0100 => 1,
            _ => 0,
        };
        sample >>= exp + 3;
        (exp, (sample & 0x0F) as u8)
    } else {
        sample >>= 4;
        (0, (sample & 0x0F) as u8)
    };

    // Combine and XOR with 0x55
    (sign | (exponent << 4) | mantissa) ^ 0x55
}

/// Convert A-law to 16-bit linear PCM
fn alaw_to_linear(alaw: u8) -> i16 {
    let alaw = alaw ^ 0x55;

    let sign = alaw & 0x80;
    let exponent = ((alaw >> 4) & 0x07) as i32;
    let mantissa = (alaw & 0x0F) as i32;

    let mut sample = if exponent > 0 {
        ((mantissa << 4) + 0x108) << (exponent - 1)
    } else {
        (mantissa << 4) + 0x08
    };

    if sign == 0 {
        sample = -sample;
    }

    sample as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulaw_roundtrip() {
        // Test various sample values (excluding extremes where G.711 has higher quantization error)
        let samples: Vec<i16> = vec![0, 100, 1000, 10000, -100, -1000, -10000];

        for &original in &samples {
            let encoded = linear_to_ulaw(original);
            let decoded = ulaw_to_linear(encoded);

            // G.711 is lossy, but error should be small for most values
            let error = (original as i32 - decoded as i32).abs();
            assert!(error < 500, "Error too large for {}: got {}, error {}", original, decoded, error);
        }
    }

    #[test]
    fn test_alaw_roundtrip() {
        let samples: Vec<i16> = vec![0, 100, 1000, 10000, -100, -1000, -10000];

        for &original in &samples {
            let encoded = linear_to_alaw(original);
            let decoded = alaw_to_linear(encoded);

            let error = (original as i32 - decoded as i32).abs();
            assert!(error < 500, "Error too large for {}: got {}, error {}", original, decoded, error);
        }
    }

    #[test]
    fn test_encode_decode_buffer() {
        let codec = G711Codec::pcmu();

        // Simulate 20ms of audio at 8kHz (160 samples)
        let pcm: Vec<i16> = (0..160).map(|i| ((i as f32 * 0.1).sin() * 10000.0) as i16).collect();

        let encoded = codec.encode(&pcm);
        assert_eq!(encoded.len(), 160); // 1 byte per sample

        let decoded = codec.decode(&encoded);
        assert_eq!(decoded.len(), 160);
    }
}
