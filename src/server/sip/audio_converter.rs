//! Audio Format Converter
//!
//! Converts raw PCM audio data to WAV file format for call recordings.
//! Supports both mono and stereo audio with configurable sample rates.

use super::SipError;
use std::io::Cursor;

/// Audio format converter for call recordings
pub struct AudioConverter;

impl AudioConverter {
    /// Convert PCM samples to WAV format
    ///
    /// # Arguments
    /// * `pcm_samples` - Raw PCM audio samples (16-bit signed integers)
    /// * `sample_rate` - Audio sample rate in Hz (e.g., 8000, 16000, 44100)
    /// * `channels` - Number of audio channels (1 for mono, 2 for stereo)
    ///
    /// # Returns
    /// * `Result<Vec<u8>, SipError>` - WAV file data as bytes
    ///
    /// # Examples
    /// ```
    /// # use voip_crm::server::sip::audio_converter::AudioConverter;
    /// // Convert mono audio at 8kHz
    /// let samples = vec![0i16, 100, 200, 300];
    /// let wav_data = AudioConverter::pcm_to_wav(&samples, 8000, 1).unwrap();
    /// assert!(!wav_data.is_empty());
    /// ```
    ///
    /// # Note
    /// For stereo audio, samples should be interleaved: [L, R, L, R, ...]
    pub fn pcm_to_wav(
        pcm_samples: &[i16],
        sample_rate: u32,
        channels: u16,
    ) -> Result<Vec<u8>, SipError> {
        // Validate parameters
        if channels == 0 || channels > 2 {
            return Err(SipError::Codec(format!(
                "Invalid number of channels: {}. Must be 1 (mono) or 2 (stereo)",
                channels
            )));
        }

        if sample_rate == 0 {
            return Err(SipError::Codec(
                "Invalid sample rate: must be greater than 0".to_string(),
            ));
        }

        if pcm_samples.is_empty() {
            return Err(SipError::Codec(
                "Cannot convert empty audio samples".to_string(),
            ));
        }

        // Create WAV file in memory
        let mut cursor = Cursor::new(Vec::new());

        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        // Create WAV writer
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| SipError::Codec(format!("Failed to create WAV writer: {}", e)))?;

        // Write all PCM samples
        for &sample in pcm_samples {
            writer
                .write_sample(sample)
                .map_err(|e| SipError::Codec(format!("Failed to write sample: {}", e)))?;
        }

        // Finalize WAV file
        writer
            .finalize()
            .map_err(|e| SipError::Codec(format!("Failed to finalize WAV: {}", e)))?;

        // Return the WAV data
        Ok(cursor.into_inner())
    }

    /// Read PCM samples from a WAV file
    ///
    /// # Arguments
    /// * `wav_data` - WAV file data as bytes
    ///
    /// # Returns
    /// * `Result<(Vec<i16>, u32, u16), SipError>` - Tuple of (PCM samples, sample rate, channels)
    ///
    /// # Examples
    /// ```
    /// # use voip_crm::server::sip::audio_converter::AudioConverter;
    /// # let samples = vec![0i16, 100, 200];
    /// # let wav_data = AudioConverter::pcm_to_wav(&samples, 8000, 1).unwrap();
    /// let (pcm_samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();
    /// assert_eq!(sample_rate, 8000);
    /// assert_eq!(channels, 1);
    /// ```
    pub fn wav_to_pcm(wav_data: &[u8]) -> Result<(Vec<i16>, u32, u16), SipError> {
        let cursor = Cursor::new(wav_data);

        let mut reader = hound::WavReader::new(cursor)
            .map_err(|e| SipError::Codec(format!("Failed to read WAV file: {}", e)))?;

        let spec = reader.spec();

        // Validate format
        if spec.sample_format != hound::SampleFormat::Int {
            return Err(SipError::Codec(
                "Only PCM integer format is supported".to_string(),
            ));
        }

        if spec.bits_per_sample != 16 {
            return Err(SipError::Codec(format!(
                "Only 16-bit samples are supported, got {}",
                spec.bits_per_sample
            )));
        }

        // Read all samples
        let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
        let samples = samples
            .map_err(|e| SipError::Codec(format!("Failed to read WAV samples: {}", e)))?;

        Ok((samples, spec.sample_rate, spec.channels))
    }

    /// Load WAV file from filesystem and extract PCM samples
    ///
    /// # Arguments
    /// * `file_path` - Path to the WAV file
    ///
    /// # Returns
    /// * `Result<(Vec<i16>, u32, u16), SipError>` - Tuple of (PCM samples, sample rate, channels)
    pub async fn load_wav_file(file_path: &str) -> Result<(Vec<i16>, u32, u16), SipError> {
        // Read file asynchronously
        let wav_data = tokio::fs::read(file_path)
            .await
            .map_err(|e| SipError::Codec(format!("Failed to read WAV file '{}': {}", file_path, e)))?;

        Self::wav_to_pcm(&wav_data)
    }

    /// Get the expected WAV file size for given parameters
    ///
    /// # Arguments
    /// * `sample_count` - Number of PCM samples
    /// * `channels` - Number of audio channels
    ///
    /// # Returns
    /// * `usize` - Expected WAV file size in bytes
    ///
    /// # Note
    /// WAV header is 44 bytes, plus 2 bytes per sample per channel
    pub fn expected_wav_size(sample_count: usize, channels: u16) -> usize {
        44 + (sample_count * 2) // 44-byte header + 2 bytes per 16-bit sample
    }

    /// Calculate duration from sample count and sample rate
    ///
    /// # Arguments
    /// * `sample_count` - Number of PCM samples (total, not per channel)
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `channels` - Number of audio channels
    ///
    /// # Returns
    /// * `f64` - Duration in seconds
    ///
    /// # Examples
    /// ```
    /// # use voip_crm::server::sip::audio_converter::AudioConverter;
    /// // 8000 mono samples at 8kHz = 1 second
    /// let duration = AudioConverter::calculate_duration(8000, 8000, 1);
    /// assert_eq!(duration, 1.0);
    ///
    /// // 16000 stereo samples (8000 per channel) at 8kHz = 1 second
    /// let duration = AudioConverter::calculate_duration(16000, 8000, 2);
    /// assert_eq!(duration, 1.0);
    /// ```
    pub fn calculate_duration(sample_count: usize, sample_rate: u32, channels: u16) -> f64 {
        if sample_rate == 0 || channels == 0 {
            return 0.0;
        }

        // For stereo, samples are interleaved, so divide by channels to get frame count
        let frames = sample_count as f64 / channels as f64;
        frames / sample_rate as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_to_wav_mono_basic() {
        let samples = vec![0i16, 100, 200, 300, 400, 500];
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 1);

        assert!(result.is_ok());
        let wav_data = result.unwrap();

        // WAV file should have header (44 bytes) + data
        assert!(wav_data.len() >= 44);

        // Check RIFF header
        assert_eq!(&wav_data[0..4], b"RIFF");
        assert_eq!(&wav_data[8..12], b"WAVE");
    }

    #[test]
    fn test_pcm_to_wav_stereo_basic() {
        // Stereo: interleaved [L, R, L, R, ...]
        let samples = vec![100i16, 50, 200, 100, 300, 150];
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 2);

        assert!(result.is_ok());
        let wav_data = result.unwrap();

        // Should have proper WAV structure
        assert!(wav_data.len() >= 44);
        assert_eq!(&wav_data[0..4], b"RIFF");
        assert_eq!(&wav_data[8..12], b"WAVE");
    }

    #[test]
    fn test_pcm_to_wav_different_sample_rates() {
        let samples = vec![0i16, 100, 200];

        // Test 8kHz (standard telephone)
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 1);
        assert!(result.is_ok());

        // Test 16kHz (wideband)
        let result = AudioConverter::pcm_to_wav(&samples, 16000, 1);
        assert!(result.is_ok());

        // Test 44.1kHz (CD quality)
        let result = AudioConverter::pcm_to_wav(&samples, 44100, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pcm_to_wav_empty_samples() {
        let samples: Vec<i16> = vec![];
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 1);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_pcm_to_wav_invalid_channels() {
        let samples = vec![0i16, 100, 200];

        // Zero channels
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid number of channels"));

        // Too many channels
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid number of channels"));
    }

    #[test]
    fn test_pcm_to_wav_invalid_sample_rate() {
        let samples = vec![0i16, 100, 200];
        let result = AudioConverter::pcm_to_wav(&samples, 0, 1);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid sample rate"));
    }

    #[test]
    fn test_pcm_to_wav_large_samples() {
        // Test with a larger audio buffer (1 second at 8kHz)
        let samples: Vec<i16> = (0..8000).map(|i| (i % 1000) as i16).collect();
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 1);

        assert!(result.is_ok());
        let wav_data = result.unwrap();

        // Should be approximately 44 + (8000 * 2) bytes
        let expected_size = AudioConverter::expected_wav_size(8000, 1);
        assert_eq!(wav_data.len(), expected_size);
    }

    #[test]
    fn test_pcm_to_wav_extreme_values() {
        // Test with extreme sample values (full 16-bit range)
        let samples = vec![i16::MIN, i16::MAX, 0, -1000, 1000];
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 1);

        assert!(result.is_ok());
    }

    #[test]
    fn test_pcm_to_wav_silence() {
        // Test with silence (all zeros)
        let samples = vec![0i16; 1000];
        let result = AudioConverter::pcm_to_wav(&samples, 8000, 1);

        assert!(result.is_ok());
        let wav_data = result.unwrap();
        assert!(wav_data.len() > 44);
    }

    #[test]
    fn test_expected_wav_size() {
        // Mono: 44 bytes header + (samples * 2)
        assert_eq!(AudioConverter::expected_wav_size(100, 1), 44 + 200);

        // Stereo: same calculation (samples already includes both channels)
        assert_eq!(AudioConverter::expected_wav_size(200, 2), 44 + 400);

        // Empty
        assert_eq!(AudioConverter::expected_wav_size(0, 1), 44);
    }

    #[test]
    fn test_calculate_duration_mono() {
        // 8000 samples at 8kHz = 1 second
        let duration = AudioConverter::calculate_duration(8000, 8000, 1);
        assert_eq!(duration, 1.0);

        // 16000 samples at 8kHz = 2 seconds
        let duration = AudioConverter::calculate_duration(16000, 8000, 1);
        assert_eq!(duration, 2.0);

        // 4000 samples at 8kHz = 0.5 seconds
        let duration = AudioConverter::calculate_duration(4000, 8000, 1);
        assert_eq!(duration, 0.5);
    }

    #[test]
    fn test_calculate_duration_stereo() {
        // 16000 stereo samples (8000 frames) at 8kHz = 1 second
        let duration = AudioConverter::calculate_duration(16000, 8000, 2);
        assert_eq!(duration, 1.0);

        // 32000 stereo samples (16000 frames) at 8kHz = 2 seconds
        let duration = AudioConverter::calculate_duration(32000, 8000, 2);
        assert_eq!(duration, 2.0);
    }

    #[test]
    fn test_calculate_duration_different_sample_rates() {
        // 16000 samples at 16kHz = 1 second
        let duration = AudioConverter::calculate_duration(16000, 16000, 1);
        assert_eq!(duration, 1.0);

        // 44100 samples at 44.1kHz = 1 second
        let duration = AudioConverter::calculate_duration(44100, 44100, 1);
        assert_eq!(duration, 1.0);
    }

    #[test]
    fn test_calculate_duration_edge_cases() {
        // Zero sample rate
        let duration = AudioConverter::calculate_duration(8000, 0, 1);
        assert_eq!(duration, 0.0);

        // Zero channels
        let duration = AudioConverter::calculate_duration(8000, 8000, 0);
        assert_eq!(duration, 0.0);

        // Zero samples
        let duration = AudioConverter::calculate_duration(0, 8000, 1);
        assert_eq!(duration, 0.0);
    }

    #[test]
    fn test_wav_format_compatibility() {
        // Ensure WAV output is compatible with AudioMixer
        let samples = vec![100i16, 200, 300, 400];
        let wav_data = AudioConverter::pcm_to_wav(&samples, 8000, 1).unwrap();

        // Parse WAV header to verify format
        assert_eq!(&wav_data[0..4], b"RIFF");
        assert_eq!(&wav_data[8..12], b"WAVE");
        assert_eq!(&wav_data[12..16], b"fmt ");

        // Verify it's PCM format (format code = 1)
        let format_code = u16::from_le_bytes([wav_data[20], wav_data[21]]);
        assert_eq!(format_code, 1); // PCM

        // Verify channels
        let num_channels = u16::from_le_bytes([wav_data[22], wav_data[23]]);
        assert_eq!(num_channels, 1);

        // Verify sample rate
        let sample_rate = u32::from_le_bytes([wav_data[24], wav_data[25], wav_data[26], wav_data[27]]);
        assert_eq!(sample_rate, 8000);

        // Verify bits per sample
        let bits_per_sample = u16::from_le_bytes([wav_data[34], wav_data[35]]);
        assert_eq!(bits_per_sample, 16);
    }

    #[test]
    fn test_wav_to_pcm_basic() {
        // Create a WAV file and read it back
        let original_samples = vec![0i16, 100, 200, 300, 400, 500];
        let wav_data = AudioConverter::pcm_to_wav(&original_samples, 8000, 1).unwrap();

        // Read it back
        let (samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();

        assert_eq!(samples, original_samples);
        assert_eq!(sample_rate, 8000);
        assert_eq!(channels, 1);
    }

    #[test]
    fn test_wav_to_pcm_stereo() {
        // Create a stereo WAV file and read it back
        let original_samples = vec![100i16, 50, 200, 100, 300, 150];
        let wav_data = AudioConverter::pcm_to_wav(&original_samples, 8000, 2).unwrap();

        // Read it back
        let (samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();

        assert_eq!(samples, original_samples);
        assert_eq!(sample_rate, 8000);
        assert_eq!(channels, 2);
    }

    #[test]
    fn test_wav_to_pcm_different_sample_rates() {
        // Test 16kHz
        let original_samples = vec![0i16, 100, 200, 300];
        let wav_data = AudioConverter::pcm_to_wav(&original_samples, 16000, 1).unwrap();
        let (samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();
        assert_eq!(samples, original_samples);
        assert_eq!(sample_rate, 16000);
        assert_eq!(channels, 1);

        // Test 44.1kHz
        let wav_data = AudioConverter::pcm_to_wav(&original_samples, 44100, 1).unwrap();
        let (samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();
        assert_eq!(samples, original_samples);
        assert_eq!(sample_rate, 44100);
        assert_eq!(channels, 1);
    }

    #[test]
    fn test_wav_to_pcm_large_file() {
        // Test with a larger audio buffer (1 second at 8kHz)
        let original_samples: Vec<i16> = (0..8000).map(|i| (i % 1000) as i16).collect();
        let wav_data = AudioConverter::pcm_to_wav(&original_samples, 8000, 1).unwrap();

        let (samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();

        assert_eq!(samples, original_samples);
        assert_eq!(sample_rate, 8000);
        assert_eq!(channels, 1);
    }

    #[test]
    fn test_wav_to_pcm_invalid_data() {
        // Test with invalid WAV data
        let invalid_data = vec![0u8, 1, 2, 3, 4, 5];
        let result = AudioConverter::wav_to_pcm(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_wav_to_pcm_empty_data() {
        // Test with empty data
        let empty_data: Vec<u8> = vec![];
        let result = AudioConverter::wav_to_pcm(&empty_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_wav_roundtrip() {
        // Test complete roundtrip: PCM -> WAV -> PCM
        let original_samples = vec![i16::MIN, -1000, 0, 1000, i16::MAX];
        let wav_data = AudioConverter::pcm_to_wav(&original_samples, 8000, 1).unwrap();
        let (samples, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();

        assert_eq!(samples, original_samples);
        assert_eq!(sample_rate, 8000);
        assert_eq!(channels, 1);
    }
}
