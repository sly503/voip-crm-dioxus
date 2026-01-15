//! SIP Call Management
//!
//! Represents an active SIP call with state management.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};

use super::rtp::{RtpSession, AudioFrame, RtpRecorder};
use super::audio_mixer::{AudioMixer, MixMode};
use super::audio_converter::AudioConverter;
use super::SipError;

/// Call direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection {
    /// Outbound call (we initiated)
    Outbound,
    /// Inbound call (we received)
    Inbound,
}

/// Call state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallState {
    /// Initial state, call is being set up
    Trying,
    /// Call is ringing at remote party
    Ringing,
    /// Call is connected and active
    Active,
    /// Call is on hold
    Held,
    /// Call is being terminated
    Terminating,
    /// Call has ended
    Ended,
    /// Call failed
    Failed,
}

impl std::fmt::Display for CallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallState::Trying => write!(f, "Trying"),
            CallState::Ringing => write!(f, "Ringing"),
            CallState::Active => write!(f, "Active"),
            CallState::Held => write!(f, "Held"),
            CallState::Terminating => write!(f, "Terminating"),
            CallState::Ended => write!(f, "Ended"),
            CallState::Failed => write!(f, "Failed"),
        }
    }
}

/// Call event for state updates
#[derive(Debug, Clone)]
pub enum CallEvent {
    /// Call state changed
    StateChanged(CallState),
    /// DTMF digit received
    DtmfReceived(char),
    /// Remote party info updated
    RemoteInfo { display_name: Option<String>, uri: String },
    /// Call duration update (in seconds)
    Duration(u64),
    /// Error occurred
    Error(String),
}

/// Represents an active SIP call
pub struct SipCall {
    /// Unique call ID
    pub call_id: String,
    /// SIP Call-ID header
    pub sip_call_id: String,
    /// Call direction
    pub direction: CallDirection,
    /// Remote party number/URI
    pub remote_party: String,
    /// Local party number/URI
    pub local_party: String,
    /// Current call state
    state: RwLock<CallState>,
    /// RTP session for audio
    rtp_session: Option<Arc<RtpSession>>,
    /// Call start time
    pub started_at: DateTime<Utc>,
    /// Call connect time (when answered)
    connected_at: RwLock<Option<DateTime<Utc>>>,
    /// Call end time
    ended_at: RwLock<Option<DateTime<Utc>>>,
    /// Event sender
    event_tx: mpsc::Sender<CallEvent>,
    /// Dialog state (for rsipstack integration)
    #[allow(dead_code)]
    dialog_id: Option<String>,
    /// RTP recorder for call recording (if enabled)
    recorder: Option<Arc<RtpRecorder>>,
    /// Recording enabled flag
    recording_enabled: bool,
}

impl SipCall {
    /// Create a new outbound call
    pub fn new_outbound(
        call_id: String,
        sip_call_id: String,
        local_party: String,
        remote_party: String,
        event_tx: mpsc::Sender<CallEvent>,
    ) -> Self {
        Self {
            call_id,
            sip_call_id,
            direction: CallDirection::Outbound,
            remote_party,
            local_party,
            state: RwLock::new(CallState::Trying),
            rtp_session: None,
            started_at: Utc::now(),
            connected_at: RwLock::new(None),
            ended_at: RwLock::new(None),
            event_tx,
            dialog_id: None,
            recorder: None,
            recording_enabled: false,
        }
    }

    /// Create a new inbound call
    pub fn new_inbound(
        call_id: String,
        sip_call_id: String,
        local_party: String,
        remote_party: String,
        event_tx: mpsc::Sender<CallEvent>,
    ) -> Self {
        Self {
            call_id,
            sip_call_id,
            direction: CallDirection::Inbound,
            remote_party,
            local_party,
            state: RwLock::new(CallState::Ringing),
            rtp_session: None,
            started_at: Utc::now(),
            connected_at: RwLock::new(None),
            ended_at: RwLock::new(None),
            event_tx,
            dialog_id: None,
            recorder: None,
            recording_enabled: false,
        }
    }

    /// Get call ID
    pub fn id(&self) -> &str {
        &self.call_id
    }

    /// Get current state
    pub async fn state(&self) -> CallState {
        *self.state.read().await
    }

    /// Set call state
    pub async fn set_state(&self, state: CallState) {
        let mut current = self.state.write().await;

        // Record connect time when transitioning to Active
        if state == CallState::Active && *current != CallState::Active {
            *self.connected_at.write().await = Some(Utc::now());

            // Start recording when call becomes active
            if self.recording_enabled {
                if let Err(e) = self.start_recording().await {
                    tracing::error!("Failed to start recording: {}", e);
                }
            }
        }

        // Record end time when transitioning to Ended/Failed
        if (state == CallState::Ended || state == CallState::Failed)
            && *current != CallState::Ended
            && *current != CallState::Failed
        {
            *self.ended_at.write().await = Some(Utc::now());

            // Stop recording when call ends
            if self.recording_enabled {
                if let Err(e) = self.stop_recording().await {
                    tracing::error!("Failed to stop recording: {}", e);
                }
            }
        }

        *current = state;

        // Notify listeners
        let _ = self.event_tx.send(CallEvent::StateChanged(state)).await;
    }

    /// Set the RTP session
    /// If recording is enabled, the recorder will be extracted from the RTP session
    pub fn set_rtp_session(&mut self, session: Arc<RtpSession>) {
        // If recording is enabled, get the recorder from the RTP session
        if self.recording_enabled {
            if let Some(recorder) = session.recorder() {
                self.recorder = Some(recorder.clone());
                tracing::debug!("Recorder attached to call {}", self.call_id);
            } else {
                tracing::warn!("Recording enabled but RTP session has no recorder for call {}", self.call_id);
            }
        }

        self.rtp_session = Some(session);
    }

    /// Get the RTP session
    pub fn rtp_session(&self) -> Option<&Arc<RtpSession>> {
        self.rtp_session.as_ref()
    }

    /// Check if call is active (can send/receive audio)
    pub async fn is_active(&self) -> bool {
        matches!(self.state().await, CallState::Active | CallState::Held)
    }

    /// Get call duration in seconds (since connect, or since start if not connected)
    pub async fn duration(&self) -> u64 {
        let connect_time = self.connected_at.read().await;
        let end_time = self.ended_at.read().await;

        let start = connect_time.unwrap_or(self.started_at);
        let end = end_time.unwrap_or_else(Utc::now);

        (end - start).num_seconds().max(0) as u64
    }

    /// Send audio to the remote party
    pub async fn send_audio(&self, samples: &[i16]) -> Result<(), SipError> {
        if let Some(rtp) = &self.rtp_session {
            rtp.send_audio(samples).await
        } else {
            Err(SipError::InvalidState("No RTP session".to_string()))
        }
    }

    /// Get the audio receiver for this call
    pub async fn take_audio_receiver(&self) -> Option<mpsc::Receiver<AudioFrame>> {
        if let Some(rtp) = &self.rtp_session {
            rtp.take_audio_receiver().await
        } else {
            None
        }
    }

    /// Send DTMF digit
    pub async fn send_dtmf(&self, digit: char) -> Result<(), SipError> {
        // DTMF can be sent in-band (RFC 2833) or via SIP INFO
        // For now, we'll use in-band DTMF

        if !self.is_active().await {
            return Err(SipError::InvalidState("Call not active".to_string()));
        }

        // Generate DTMF tone samples (simplified - proper implementation would use RFC 2833)
        let samples = generate_dtmf_samples(digit);
        self.send_audio(&samples).await
    }

    /// Handle incoming DTMF
    pub async fn on_dtmf_received(&self, digit: char) {
        let _ = self.event_tx.send(CallEvent::DtmfReceived(digit)).await;
    }

    /// Enable recording for this call
    /// Must be called before RTP session is created to enable recording
    pub fn enable_recording(&mut self) {
        self.recording_enabled = true;
        tracing::info!("Recording enabled for call {}", self.call_id);
    }

    /// Check if recording is enabled
    pub fn is_recording_enabled(&self) -> bool {
        self.recording_enabled
    }

    /// Start recording RTP packets
    async fn start_recording(&self) -> Result<(), SipError> {
        if let Some(ref recorder) = self.recorder {
            recorder.start().await;
            tracing::info!("Recording started for call {}", self.call_id);
            Ok(())
        } else {
            Err(SipError::InvalidState("No recorder available".to_string()))
        }
    }

    /// Stop recording RTP packets
    async fn stop_recording(&self) -> Result<(), SipError> {
        if let Some(ref recorder) = self.recorder {
            recorder.stop().await;
            tracing::info!("Recording stopped for call {}", self.call_id);
            Ok(())
        } else {
            Err(SipError::InvalidState("No recorder available".to_string()))
        }
    }

    /// Finalize the recording and return the WAV data
    /// This should be called after the call ends to process the recorded audio
    ///
    /// # Returns
    /// * `Ok(Some(Vec<u8>))` - WAV file data if recording was successful
    /// * `Ok(None)` - No recording available (recording not enabled or no packets captured)
    /// * `Err(SipError)` - Error processing the recording
    pub async fn finalize_recording(&self) -> Result<Option<Vec<u8>>, SipError> {
        // Check if recording is enabled
        if !self.recording_enabled {
            return Ok(None);
        }

        // Get the recorder
        let recorder = match &self.recorder {
            Some(rec) => rec,
            None => return Ok(None),
        };

        // Drain captured packets
        let packets = recorder.drain_packets().await;
        if packets.is_empty() {
            tracing::warn!("No RTP packets captured for call {}", self.call_id);
            return Ok(None);
        }

        tracing::info!("Processing {} RTP packets for call {}", packets.len(), self.call_id);

        // Mix audio packets (stereo mode: agent on left, customer on right)
        let mixer = AudioMixer::new(MixMode::Stereo, Some(8000));
        let pcm_samples = mixer.mix_packets(&packets);

        if pcm_samples.is_empty() {
            tracing::warn!("No audio samples after mixing for call {}", self.call_id);
            return Ok(None);
        }

        tracing::info!("Mixed {} PCM samples for call {}", pcm_samples.len(), self.call_id);

        // Convert to WAV format (stereo, 8kHz)
        let wav_data = AudioConverter::pcm_to_wav(&pcm_samples, 8000, 2)?;

        tracing::info!(
            "Recording finalized for call {}: {} bytes WAV",
            self.call_id,
            wav_data.len()
        );

        Ok(Some(wav_data))
    }

    /// Set the RTP recorder reference
    /// This should be called after enabling recording on the RTP session
    pub(crate) fn set_recorder(&mut self, recorder: Arc<RtpRecorder>) {
        self.recorder = Some(recorder);
    }
}

/// Generate DTMF tone samples
fn generate_dtmf_samples(digit: char) -> Vec<i16> {
    let (low_freq, high_freq) = match digit {
        '1' => (697.0, 1209.0),
        '2' => (697.0, 1336.0),
        '3' => (697.0, 1477.0),
        '4' => (770.0, 1209.0),
        '5' => (770.0, 1336.0),
        '6' => (770.0, 1477.0),
        '7' => (852.0, 1209.0),
        '8' => (852.0, 1336.0),
        '9' => (852.0, 1477.0),
        '*' => (941.0, 1209.0),
        '0' => (941.0, 1336.0),
        '#' => (941.0, 1477.0),
        'A' => (697.0, 1633.0),
        'B' => (770.0, 1633.0),
        'C' => (852.0, 1633.0),
        'D' => (941.0, 1633.0),
        _ => return vec![0i16; 160], // Silence for unknown
    };

    // Generate 160 samples (20ms at 8kHz)
    let sample_rate = 8000.0;
    let duration_samples = 160;
    let amplitude = 8000.0;

    (0..duration_samples)
        .map(|i| {
            let t = i as f64 / sample_rate;
            let low = (2.0 * std::f64::consts::PI * low_freq * t).sin();
            let high = (2.0 * std::f64::consts::PI * high_freq * t).sin();
            ((low + high) * amplitude / 2.0) as i16
        })
        .collect()
}

/// Call statistics
#[derive(Debug, Clone)]
pub struct CallStats {
    pub call_id: String,
    pub direction: CallDirection,
    pub state: CallState,
    pub remote_party: String,
    pub duration_seconds: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recording_enabled_flag() {
        let (tx, _rx) = mpsc::channel(10);
        let mut call = SipCall::new_outbound(
            "test-call-1".to_string(),
            "sip-call-1".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Initially recording should be disabled
        assert!(!call.is_recording_enabled());

        // Enable recording
        call.enable_recording();
        assert!(call.is_recording_enabled());
    }

    #[tokio::test]
    async fn test_recording_lifecycle_without_recorder() {
        let (tx, _rx) = mpsc::channel(10);
        let mut call = SipCall::new_outbound(
            "test-call-2".to_string(),
            "sip-call-2".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Enable recording
        call.enable_recording();

        // Try to finalize recording without a recorder (should return None)
        let result = call.finalize_recording().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_recording_lifecycle_with_recorder() {
        let (tx, _rx) = mpsc::channel(10);
        let mut call = SipCall::new_outbound(
            "test-call-3".to_string(),
            "sip-call-3".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Enable recording
        call.enable_recording();

        // Create and attach a recorder
        let recorder = Arc::new(RtpRecorder::new(None));
        call.set_recorder(recorder.clone());

        // Start recording manually
        let start_result = call.start_recording().await;
        assert!(start_result.is_ok());
        assert!(recorder.is_enabled().await);

        // Stop recording manually
        let stop_result = call.stop_recording().await;
        assert!(stop_result.is_ok());
        assert!(!recorder.is_enabled().await);
    }

    #[tokio::test]
    async fn test_recording_starts_on_active_state() {
        let (tx, mut rx) = mpsc::channel(10);
        let mut call = SipCall::new_outbound(
            "test-call-4".to_string(),
            "sip-call-4".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Enable recording and attach recorder
        call.enable_recording();
        let recorder = Arc::new(RtpRecorder::new(None));
        call.set_recorder(recorder.clone());

        // Initially not recording
        assert!(!recorder.is_enabled().await);

        // Transition to Active state - should start recording
        call.set_state(CallState::Active).await;
        assert!(recorder.is_enabled().await);

        // Verify state change event was sent
        let event = rx.recv().await;
        assert!(matches!(event, Some(CallEvent::StateChanged(CallState::Active))));
    }

    #[tokio::test]
    async fn test_recording_stops_on_ended_state() {
        let (tx, mut rx) = mpsc::channel(10);
        let mut call = SipCall::new_outbound(
            "test-call-5".to_string(),
            "sip-call-5".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Enable recording and attach recorder
        call.enable_recording();
        let recorder = Arc::new(RtpRecorder::new(None));
        call.set_recorder(recorder.clone());

        // Start recording
        call.set_state(CallState::Active).await;
        assert!(recorder.is_enabled().await);

        // Clear the Active event
        rx.recv().await;

        // Transition to Ended state - should stop recording
        call.set_state(CallState::Ended).await;
        assert!(!recorder.is_enabled().await);

        // Verify state change event was sent
        let event = rx.recv().await;
        assert!(matches!(event, Some(CallEvent::StateChanged(CallState::Ended))));
    }

    #[tokio::test]
    async fn test_finalize_recording_without_enabled() {
        let (tx, _rx) = mpsc::channel(10);
        let call = SipCall::new_outbound(
            "test-call-6".to_string(),
            "sip-call-6".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Recording not enabled - should return None
        let result = call.finalize_recording().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_finalize_recording_with_empty_packets() {
        let (tx, _rx) = mpsc::channel(10);
        let mut call = SipCall::new_outbound(
            "test-call-7".to_string(),
            "sip-call-7".to_string(),
            "local@test.com".to_string(),
            "remote@test.com".to_string(),
            tx,
        );

        // Enable recording and attach recorder
        call.enable_recording();
        let recorder = Arc::new(RtpRecorder::new(None));
        call.set_recorder(recorder.clone());

        // Finalize without any packets - should return None
        let result = call.finalize_recording().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
