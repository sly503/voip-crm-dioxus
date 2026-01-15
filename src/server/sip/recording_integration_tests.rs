//! Integration Tests for RTP Recording and Audio Mixing
//!
//! Tests the complete recording pipeline:
//! RTP Packet Capture -> Audio Mixing -> WAV Conversion
//!
//! These tests verify:
//! - End-to-end recording workflow
//! - Audio quality preservation
//! - Bidirectional mixing (mono and stereo)
//! - Format conversion accuracy
//! - Realistic call scenarios

#[cfg(test)]
mod tests {
    use super::super::{
        audio_converter::AudioConverter,
        audio_mixer::{AudioMixer, MixMode},
        codec::G711Codec,
        rtp::{CapturedRtpPacket, RtpDirection, RtpHeader, RtpPacket, RtpRecorder},
    };
    use bytes::Bytes;
    use chrono::Utc;

    // ============================================================================
    // Helper Functions
    // ============================================================================

    /// Create a test RTP packet with encoded audio
    fn create_rtp_packet(
        payload_type: u8,
        sequence: u16,
        timestamp: u32,
        samples: &[i16],
    ) -> RtpPacket {
        let codec = if payload_type == 0 {
            G711Codec::pcmu()
        } else {
            G711Codec::pcma()
        };

        let encoded = codec.encode(samples);
        let header = RtpHeader::new(payload_type, sequence, timestamp, 12345);
        RtpPacket::new(header, Bytes::from(encoded))
    }

    /// Create a captured RTP packet
    fn create_captured_packet(
        payload_type: u8,
        sequence: u16,
        timestamp: u32,
        samples: &[i16],
        direction: RtpDirection,
    ) -> CapturedRtpPacket {
        CapturedRtpPacket {
            packet: create_rtp_packet(payload_type, sequence, timestamp, samples),
            direction,
            captured_at: Utc::now(),
        }
    }

    /// Generate a test tone (sine wave)
    fn generate_tone(frequency: f64, duration_seconds: f64, sample_rate: u32) -> Vec<i16> {
        let num_samples = (duration_seconds * sample_rate as f64) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f64 / sample_rate as f64;
            let value = (2.0 * std::f64::consts::PI * frequency * t).sin();
            // Scale to 16-bit range (use 50% amplitude to prevent clipping when mixed)
            samples.push((value * 16000.0) as i16);
        }

        samples
    }

    /// Generate silence
    fn generate_silence(duration_seconds: f64, sample_rate: u32) -> Vec<i16> {
        let num_samples = (duration_seconds * sample_rate as f64) as usize;
        vec![0i16; num_samples]
    }

    /// Calculate RMS (Root Mean Square) for audio quality measurement
    fn calculate_rms(samples: &[i16]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
        (sum_squares / samples.len() as f64).sqrt()
    }

    /// Calculate SNR (Signal-to-Noise Ratio) in dB
    fn calculate_snr(signal: &[i16], noisy_signal: &[i16]) -> f64 {
        if signal.len() != noisy_signal.len() || signal.is_empty() {
            return 0.0;
        }

        let signal_power: f64 = signal.iter().map(|&s| (s as f64).powi(2)).sum();

        let noise_power: f64 = signal
            .iter()
            .zip(noisy_signal.iter())
            .map(|(&s, &n)| ((s as f64) - (n as f64)).powi(2))
            .sum();

        if noise_power == 0.0 {
            return f64::INFINITY;
        }

        10.0 * (signal_power / noise_power).log10()
    }

    // ============================================================================
    // End-to-End Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_complete_recording_pipeline_mono() {
        // Test the complete pipeline: RTP capture -> Mix -> WAV conversion

        // Step 1: Generate test audio
        let outgoing_tone = generate_tone(440.0, 0.1, 8000); // 440 Hz tone (A4 note)
        let incoming_tone = generate_tone(880.0, 0.1, 8000); // 880 Hz tone (A5 note)

        // Step 2: Create RTP packets
        let mut packets = Vec::new();

        // Split audio into chunks (160 samples = 20ms at 8kHz)
        let chunk_size = 160;
        let mut timestamp = 0u32;
        let mut sequence = 0u16;

        for chunk_idx in 0..(outgoing_tone.len() / chunk_size) {
            let start = chunk_idx * chunk_size;
            let end = (start + chunk_size).min(outgoing_tone.len());

            // Outgoing packet
            packets.push(create_captured_packet(
                0,
                sequence,
                timestamp,
                &outgoing_tone[start..end],
                RtpDirection::Outgoing,
            ));
            sequence += 1;

            // Incoming packet (same timestamp for simultaneous audio)
            let incoming_end = end.min(incoming_tone.len());
            packets.push(create_captured_packet(
                0,
                sequence,
                timestamp,
                &incoming_tone[start..incoming_end],
                RtpDirection::Incoming,
            ));
            sequence += 1;

            timestamp += chunk_size as u32;
        }

        // Step 3: Mix audio (mono mode)
        let mixer = AudioMixer::new(MixMode::Mono, Some(8000));
        let mixed_pcm = mixer.mix_packets(&packets);

        // Verify mixed audio
        assert!(!mixed_pcm.is_empty(), "Mixed audio should not be empty");
        assert!(
            mixed_pcm.len() >= outgoing_tone.len(),
            "Mixed audio should have at least {} samples, got {}",
            outgoing_tone.len(),
            mixed_pcm.len()
        );

        // Verify audio quality (should have reasonable RMS)
        let rms = calculate_rms(&mixed_pcm);
        assert!(
            rms > 1000.0,
            "RMS should be > 1000 for mixed tones, got {}",
            rms
        );

        // Step 4: Convert to WAV
        let wav_data = AudioConverter::pcm_to_wav(&mixed_pcm, 8000, 1).unwrap();

        // Verify WAV format
        assert!(wav_data.len() > 44, "WAV file should have header + data");
        assert_eq!(&wav_data[0..4], b"RIFF");
        assert_eq!(&wav_data[8..12], b"WAVE");

        // Step 5: Verify roundtrip (WAV -> PCM)
        let (decoded_pcm, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();
        assert_eq!(sample_rate, 8000);
        assert_eq!(channels, 1);
        assert_eq!(decoded_pcm.len(), mixed_pcm.len());

        // Verify audio integrity (samples should match)
        for (i, (&original, &decoded)) in mixed_pcm.iter().zip(decoded_pcm.iter()).enumerate() {
            assert_eq!(
                original, decoded,
                "Sample {} mismatch: {} != {}",
                i, original, decoded
            );
        }
    }

    #[tokio::test]
    async fn test_complete_recording_pipeline_stereo() {
        // Test stereo recording pipeline

        // Generate different tones for agent and customer
        let agent_tone = generate_tone(300.0, 0.1, 8000); // 300 Hz
        let customer_tone = generate_tone(600.0, 0.1, 8000); // 600 Hz

        // Create RTP packets
        let mut packets = Vec::new();
        let chunk_size = 160;
        let mut timestamp = 0u32;
        let mut sequence = 0u16;

        for chunk_idx in 0..(agent_tone.len() / chunk_size) {
            let start = chunk_idx * chunk_size;
            let end = (start + chunk_size).min(agent_tone.len());

            packets.push(create_captured_packet(
                0,
                sequence,
                timestamp,
                &agent_tone[start..end],
                RtpDirection::Outgoing,
            ));
            sequence += 1;

            let customer_end = end.min(customer_tone.len());
            packets.push(create_captured_packet(
                0,
                sequence,
                timestamp,
                &customer_tone[start..customer_end],
                RtpDirection::Incoming,
            ));
            sequence += 1;

            timestamp += chunk_size as u32;
        }

        // Mix in stereo mode
        let mixer = AudioMixer::new(MixMode::Stereo, Some(8000));
        let mixed_pcm = mixer.mix_packets(&packets);

        // Verify stereo interleaving (should have double the samples)
        assert!(
            mixed_pcm.len() >= agent_tone.len() * 2,
            "Stereo should have 2x samples"
        );

        // Extract left and right channels
        let mut left_channel = Vec::new();
        let mut right_channel = Vec::new();
        for i in (0..mixed_pcm.len()).step_by(2) {
            left_channel.push(mixed_pcm[i]);
            if i + 1 < mixed_pcm.len() {
                right_channel.push(mixed_pcm[i + 1]);
            }
        }

        // Verify channels are different (agent vs customer)
        let left_rms = calculate_rms(&left_channel);
        let right_rms = calculate_rms(&right_channel);
        assert!(left_rms > 1000.0, "Left channel should have signal");
        assert!(right_rms > 1000.0, "Right channel should have signal");

        // Convert to WAV (stereo)
        let wav_data = AudioConverter::pcm_to_wav(&mixed_pcm, 8000, 2).unwrap();

        // Verify WAV format
        assert_eq!(&wav_data[0..4], b"RIFF");
        let num_channels = u16::from_le_bytes([wav_data[22], wav_data[23]]);
        assert_eq!(num_channels, 2, "Should be stereo");

        // Roundtrip test
        let (decoded_pcm, sample_rate, channels) = AudioConverter::wav_to_pcm(&wav_data).unwrap();
        assert_eq!(sample_rate, 8000);
        assert_eq!(channels, 2);
        assert_eq!(decoded_pcm, mixed_pcm);
    }

    #[tokio::test]
    async fn test_rtp_recorder_integration() {
        // Test RtpRecorder with realistic capture scenario

        let recorder = RtpRecorder::new(Some(1000));
        recorder.start().await;

        // Simulate a 1-second call (50 packets per direction at 20ms intervals)
        let chunk_size = 160; // 20ms at 8kHz
        let mut sequence = 0u16;
        let mut timestamp = 0u32;

        for i in 0..50 {
            // Generate audio samples
            let samples = generate_tone(400.0 + (i as f64 * 10.0), 0.02, 8000);

            // Outgoing packet
            let outgoing = create_rtp_packet(0, sequence, timestamp, &samples[..chunk_size]);
            recorder.capture(outgoing, RtpDirection::Outgoing).await;
            sequence += 1;

            // Incoming packet
            let incoming = create_rtp_packet(0, sequence, timestamp, &samples[..chunk_size]);
            recorder.capture(incoming, RtpDirection::Incoming).await;
            sequence += 1;

            timestamp += chunk_size as u32;
        }

        // Verify packet count
        assert_eq!(recorder.packet_count().await, 100); // 50 * 2 directions

        // Get packets and mix
        let packets = recorder.get_packets().await;
        let mixer = AudioMixer::new(MixMode::Stereo, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Verify mixed audio
        assert!(!mixed.is_empty());

        // Expected: 50 * 160 samples per channel * 2 channels = 16,000 samples
        let expected_samples = 50 * 160 * 2;
        assert_eq!(
            mixed.len(),
            expected_samples,
            "Expected {} stereo samples",
            expected_samples
        );
    }

    #[tokio::test]
    async fn test_audio_quality_preservation() {
        // Test that audio quality is preserved through the pipeline

        // Generate high-quality test signal
        let original_signal = generate_tone(1000.0, 0.2, 8000); // 200ms of 1kHz tone

        // Create packets
        let mut packets = Vec::new();
        let chunk_size = 160;
        let mut timestamp = 0u32;
        let mut sequence = 0u16;

        for chunk_idx in 0..(original_signal.len() / chunk_size) {
            let start = chunk_idx * chunk_size;
            let end = (start + chunk_size).min(original_signal.len());

            packets.push(create_captured_packet(
                0,
                sequence,
                timestamp,
                &original_signal[start..end],
                RtpDirection::Outgoing,
            ));
            sequence += 1;
            timestamp += chunk_size as u32;
        }

        // Mix (mono - just one channel)
        let mixer = AudioMixer::new(MixMode::Mono, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Convert to WAV and back
        let wav_data = AudioConverter::pcm_to_wav(&mixed, 8000, 1).unwrap();
        let (decoded, _, _) = AudioConverter::wav_to_pcm(&wav_data).unwrap();

        // Calculate SNR (should be very high since we're just encoding/decoding)
        let snr = calculate_snr(&mixed, &decoded);
        assert!(
            snr > 40.0 || snr.is_infinite(),
            "SNR should be > 40 dB, got {}",
            snr
        );

        // RMS should be similar
        let original_rms = calculate_rms(&mixed);
        let decoded_rms = calculate_rms(&decoded);
        let rms_ratio = decoded_rms / original_rms;
        assert!(
            (rms_ratio - 1.0).abs() < 0.01,
            "RMS should be preserved, got ratio {}",
            rms_ratio
        );
    }

    #[tokio::test]
    async fn test_codec_mixing_pcmu_pcma() {
        // Test mixing packets with different codecs (PCMU and PCMA)

        let samples1 = generate_tone(400.0, 0.05, 8000);
        let samples2 = generate_tone(800.0, 0.05, 8000);

        let mut packets = Vec::new();
        let chunk_size = 160;

        for i in 0..(samples1.len() / chunk_size) {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(samples1.len());
            let timestamp = (i * chunk_size) as u32;

            // PCMU for outgoing
            packets.push(create_captured_packet(
                0,
                i as u16,
                timestamp,
                &samples1[start..end],
                RtpDirection::Outgoing,
            ));

            // PCMA for incoming
            packets.push(create_captured_packet(
                8,
                (i + 1) as u16,
                timestamp,
                &samples2[start..end],
                RtpDirection::Incoming,
            ));
        }

        // Mix
        let mixer = AudioMixer::new(MixMode::Mono, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Verify mixed audio
        assert!(!mixed.is_empty());
        let rms = calculate_rms(&mixed);
        assert!(rms > 1000.0, "Should have signal after mixing codecs");
    }

    #[tokio::test]
    async fn test_packet_loss_tolerance() {
        // Test that the pipeline handles missing packets gracefully

        let tone = generate_tone(500.0, 0.1, 8000);
        let mut packets = Vec::new();
        let chunk_size = 160;

        for i in 0..(tone.len() / chunk_size) {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(tone.len());
            let timestamp = (i * chunk_size) as u32;

            // Simulate 20% packet loss (skip every 5th packet)
            if i % 5 == 0 {
                continue; // Drop packet
            }

            packets.push(create_captured_packet(
                0,
                i as u16,
                timestamp,
                &tone[start..end],
                RtpDirection::Outgoing,
            ));
        }

        // Mix
        let mixer = AudioMixer::new(MixMode::Mono, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Should still have audio despite packet loss
        assert!(!mixed.is_empty());
        let rms = calculate_rms(&mixed);
        assert!(rms > 500.0, "Should still have signal despite packet loss");

        // Expected samples: ~80% of original (due to 20% loss)
        let expected_min = (tone.len() as f64 * 0.7) as usize;
        assert!(
            mixed.len() >= expected_min,
            "Should have at least 70% of samples despite packet loss"
        );
    }

    #[tokio::test]
    async fn test_out_of_order_packets() {
        // Test that the mixer handles out-of-order packets correctly

        let tone = generate_tone(600.0, 0.06, 8000); // 60ms
        let chunk_size = 160;
        let mut packets = Vec::new();

        // Create packets
        for i in 0..(tone.len() / chunk_size) {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(tone.len());
            let timestamp = (i * chunk_size) as u32;

            packets.push(create_captured_packet(
                0,
                i as u16,
                timestamp,
                &tone[start..end],
                RtpDirection::Outgoing,
            ));
        }

        // Shuffle packets to simulate out-of-order arrival
        if packets.len() >= 3 {
            packets.swap(0, 2); // Swap first and third
            packets.swap(1, 3); // Swap second and fourth
        }

        // Mix - should reorder by timestamp
        let mixer = AudioMixer::new(MixMode::Mono, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Should still produce correct audio
        assert!(!mixed.is_empty());
        assert_eq!(mixed.len(), tone.len());
    }

    #[tokio::test]
    async fn test_silence_detection() {
        // Test recording of silence (important for pause detection)

        let silence = generate_silence(0.1, 8000);
        let mut packets = Vec::new();
        let chunk_size = 160;

        for i in 0..(silence.len() / chunk_size) {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(silence.len());
            let timestamp = (i * chunk_size) as u32;

            packets.push(create_captured_packet(
                0,
                i as u16,
                timestamp,
                &silence[start..end],
                RtpDirection::Outgoing,
            ));
        }

        // Mix
        let mixer = AudioMixer::new(MixMode::Mono, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Verify silence
        assert!(!mixed.is_empty());
        let rms = calculate_rms(&mixed);
        assert!(
            rms < 1.0,
            "Silence should have very low RMS, got {}",
            rms
        );

        // All samples should be zero or very close
        for &sample in &mixed {
            assert!(
                sample.abs() < 10,
                "Silence samples should be near zero, got {}",
                sample
            );
        }
    }

    #[tokio::test]
    async fn test_long_duration_recording() {
        // Test recording for a longer duration (1 second)

        let duration = 1.0; // 1 second
        let sample_rate = 8000;
        let chunk_size = 160;
        let num_chunks = ((duration * sample_rate as f64) / chunk_size as f64) as usize;

        let mut packets = Vec::new();
        let mut timestamp = 0u32;

        for i in 0..num_chunks {
            // Generate varying tone (frequency sweep)
            let frequency = 300.0 + (i as f64 * 5.0);
            let samples = generate_tone(frequency, 0.02, sample_rate);

            packets.push(create_captured_packet(
                0,
                i as u16,
                timestamp,
                &samples[..chunk_size],
                RtpDirection::Outgoing,
            ));

            timestamp += chunk_size as u32;
        }

        // Mix
        let mixer = AudioMixer::new(MixMode::Mono, Some(sample_rate));
        let mixed = mixer.mix_packets(&packets);

        // Verify duration
        let actual_duration = AudioConverter::calculate_duration(mixed.len(), sample_rate, 1);
        assert!(
            (actual_duration - duration).abs() < 0.05,
            "Duration should be ~{} seconds, got {}",
            duration,
            actual_duration
        );

        // Convert to WAV
        let wav_data = AudioConverter::pcm_to_wav(&mixed, sample_rate, 1).unwrap();
        let expected_size = AudioConverter::expected_wav_size(mixed.len(), 1);
        assert_eq!(wav_data.len(), expected_size);
    }

    #[tokio::test]
    async fn test_bidirectional_timing_alignment() {
        // Test that bidirectional audio is properly time-aligned

        // Create agent and customer speaking at different times
        let silence_100ms = generate_silence(0.1, 8000);
        let agent_tone = generate_tone(400.0, 0.1, 8000);
        let customer_tone = generate_tone(800.0, 0.1, 8000);

        let mut packets = Vec::new();
        let chunk_size = 160;

        // Agent speaks first (0-100ms)
        for i in 0..(agent_tone.len() / chunk_size) {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(agent_tone.len());
            let timestamp = (i * chunk_size) as u32;

            packets.push(create_captured_packet(
                0,
                i as u16,
                timestamp,
                &agent_tone[start..end],
                RtpDirection::Outgoing,
            ));
        }

        // Customer speaks second (100-200ms)
        let num_agent_chunks = agent_tone.len() / chunk_size;
        for i in 0..(customer_tone.len() / chunk_size) {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(customer_tone.len());
            let timestamp = ((num_agent_chunks + i) * chunk_size) as u32;

            packets.push(create_captured_packet(
                0,
                (num_agent_chunks + i) as u16,
                timestamp,
                &customer_tone[start..end],
                RtpDirection::Incoming,
            ));
        }

        // Mix in stereo to see separate channels
        let mixer = AudioMixer::new(MixMode::Stereo, Some(8000));
        let mixed = mixer.mix_packets(&packets);

        // Extract channels
        let mut left = Vec::new();
        let mut right = Vec::new();
        for i in (0..mixed.len()).step_by(2) {
            left.push(mixed[i]);
            if i + 1 < mixed.len() {
                right.push(mixed[i + 1]);
            }
        }

        // Verify timing: agent in first half, customer in second half
        let half = left.len() / 2;

        let left_first_half_rms = calculate_rms(&left[..half]);
        let left_second_half_rms = calculate_rms(&left[half..]);
        let right_first_half_rms = calculate_rms(&right[..half]);
        let right_second_half_rms = calculate_rms(&right[half..]);

        // Agent (left) should be stronger in first half
        assert!(
            left_first_half_rms > left_second_half_rms * 2.0,
            "Agent should speak in first half"
        );

        // Customer (right) should be stronger in second half
        assert!(
            right_second_half_rms > right_first_half_rms * 2.0,
            "Customer should speak in second half"
        );
    }

    #[tokio::test]
    async fn test_recorder_buffer_overflow() {
        // Test that recorder handles buffer overflow correctly

        let recorder = RtpRecorder::new(Some(10)); // Small buffer
        recorder.start().await;

        // Try to capture more than buffer size
        for i in 0..20 {
            let samples = vec![i as i16; 160];
            let packet = create_rtp_packet(0, i as u16, i * 160, &samples);
            recorder.capture(packet, RtpDirection::Outgoing).await;
        }

        // Should cap at buffer size
        assert_eq!(recorder.packet_count().await, 10);

        // Latest packets should be retained
        let packets = recorder.get_packets().await;
        assert_eq!(packets.len(), 10);

        // First packet should be sequence 10 (oldest dropped)
        assert_eq!(packets[0].packet.header.sequence, 10);
    }

    #[tokio::test]
    async fn test_wav_file_metadata() {
        // Test that WAV files have correct metadata

        let samples = generate_tone(440.0, 0.5, 8000);
        let wav_data = AudioConverter::pcm_to_wav(&samples, 8000, 1).unwrap();

        // Verify WAV header metadata
        assert_eq!(&wav_data[0..4], b"RIFF");
        assert_eq!(&wav_data[8..12], b"WAVE");
        assert_eq!(&wav_data[12..16], b"fmt ");

        // Format code (PCM = 1)
        let format_code = u16::from_le_bytes([wav_data[20], wav_data[21]]);
        assert_eq!(format_code, 1);

        // Channels
        let channels = u16::from_le_bytes([wav_data[22], wav_data[23]]);
        assert_eq!(channels, 1);

        // Sample rate
        let sample_rate =
            u32::from_le_bytes([wav_data[24], wav_data[25], wav_data[26], wav_data[27]]);
        assert_eq!(sample_rate, 8000);

        // Bits per sample
        let bits_per_sample = u16::from_le_bytes([wav_data[34], wav_data[35]]);
        assert_eq!(bits_per_sample, 16);

        // Data chunk
        assert_eq!(&wav_data[36..40], b"data");
    }

    #[test]
    fn test_audio_duration_calculation() {
        // Test duration calculation accuracy

        // 1 second at 8kHz mono
        let duration = AudioConverter::calculate_duration(8000, 8000, 1);
        assert_eq!(duration, 1.0);

        // 1 second at 8kHz stereo (16000 interleaved samples)
        let duration = AudioConverter::calculate_duration(16000, 8000, 2);
        assert_eq!(duration, 1.0);

        // 30 seconds at 8kHz mono
        let duration = AudioConverter::calculate_duration(240000, 8000, 1);
        assert_eq!(duration, 30.0);

        // 1 minute at 8kHz mono
        let duration = AudioConverter::calculate_duration(480000, 8000, 1);
        assert_eq!(duration, 60.0);
    }

    #[test]
    fn test_wav_file_size_estimation() {
        // Test WAV file size estimation

        // Mono: 44 byte header + 2 bytes per sample
        assert_eq!(AudioConverter::expected_wav_size(8000, 1), 44 + 16000);

        // Stereo: 44 byte header + 2 bytes per sample (interleaved)
        assert_eq!(AudioConverter::expected_wav_size(16000, 2), 44 + 32000);

        // Large file (1 hour at 8kHz mono)
        let one_hour_samples = 8000 * 60 * 60;
        let expected_size = 44 + (one_hour_samples * 2);
        assert_eq!(
            AudioConverter::expected_wav_size(one_hour_samples, 1),
            expected_size
        );
    }
}
