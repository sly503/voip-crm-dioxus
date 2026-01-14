//! SIP Call Management
//!
//! Represents an active SIP call with state management.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};

use super::rtp::{RtpSession, AudioFrame};
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
        }

        // Record end time when transitioning to Ended/Failed
        if (state == CallState::Ended || state == CallState::Failed)
            && *current != CallState::Ended
            && *current != CallState::Failed
        {
            *self.ended_at.write().await = Some(Utc::now());
        }

        *current = state;

        // Notify listeners
        let _ = self.event_tx.send(CallEvent::StateChanged(state)).await;
    }

    /// Set the RTP session
    pub fn set_rtp_session(&mut self, session: Arc<RtpSession>) {
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
