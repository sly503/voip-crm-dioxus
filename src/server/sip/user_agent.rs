//! SIP User Agent
//!
//! Main component for SIP trunk registration and call management.
//! Uses rsipstack for SIP signaling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use ftth_rsipstack::{
    dialog::{
        authenticate::Credential,
        dialog::DialogState,
        dialog_layer::DialogLayer,
        invitation::InviteOption,
        registration::Registration,
    },
    transaction::endpoint::EndpointInnerRef,
    transport::{udp::UdpConnection, TransportLayer},
    EndpointBuilder,
};
use tokio::sync::mpsc::unbounded_channel;

use super::config::{SipCodec, SipConfig};
use super::call::{CallEvent, CallState, SipCall};
use super::rtp::{AudioFrame, RtpPortAllocator, RtpSession};
use super::SipError;

/// SIP User Agent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Not connected
    Disconnected,
    /// Connecting to SIP trunk
    Connecting,
    /// Registering with SIP trunk
    Registering,
    /// Registered and ready for calls
    Registered,
    /// Registration failed
    Failed,
}

/// SIP User Agent for VoIP operations
pub struct SipUserAgent {
    /// Configuration
    config: SipConfig,
    /// Current state
    state: Arc<RwLock<AgentState>>,
    /// Active calls by call ID
    calls: RwLock<HashMap<String, Arc<RwLock<SipCall>>>>,
    /// RTP port allocator
    rtp_ports: RtpPortAllocator,
    /// Local IP address
    local_ip: RwLock<Option<String>>,
    /// Local SIP port
    local_port: RwLock<Option<u16>>,
    /// Event channel for agent-level events
    event_tx: mpsc::Sender<AgentEvent>,
    /// Shutdown flag
    shutdown: RwLock<bool>,
    /// Cancellation token for registration loop
    cancel_token: CancellationToken,
    /// Endpoint reference for making calls
    endpoint_inner: RwLock<Option<EndpointInnerRef>>,
    /// Credentials for authentication
    credential: RwLock<Option<Credential>>,
}

/// Agent-level events
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent state changed
    StateChanged(AgentState),
    /// Incoming call
    IncomingCall {
        call_id: String,
        from: String,
        to: String,
    },
    /// Call state changed
    CallStateChanged {
        call_id: String,
        state: CallState,
    },
    /// Error occurred
    Error(String),
}

impl SipUserAgent {
    /// Create a new SIP User Agent
    pub fn new(config: SipConfig) -> (Self, mpsc::Receiver<AgentEvent>) {
        let (event_tx, event_rx) = mpsc::channel(100);

        let agent = Self {
            rtp_ports: RtpPortAllocator::new(config.rtp_port_start, config.rtp_port_end),
            config,
            state: Arc::new(RwLock::new(AgentState::Disconnected)),
            calls: RwLock::new(HashMap::new()),
            local_ip: RwLock::new(None),
            local_port: RwLock::new(None),
            event_tx,
            shutdown: RwLock::new(false),
            cancel_token: CancellationToken::new(),
            endpoint_inner: RwLock::new(None),
            credential: RwLock::new(None),
        };

        (agent, event_rx)
    }

    /// Create from environment variables
    pub fn from_env() -> Option<(Self, mpsc::Receiver<AgentEvent>)> {
        let config = SipConfig::from_env()?;
        Some(Self::new(config))
    }

    /// Get current state
    pub async fn state(&self) -> AgentState {
        *self.state.read().await
    }

    /// Check if registered
    pub async fn is_registered(&self) -> bool {
        *self.state.read().await == AgentState::Registered
    }

    /// Get the configuration
    pub fn config(&self) -> &SipConfig {
        &self.config
    }

    /// Set state and notify
    async fn set_state(&self, state: AgentState) {
        *self.state.write().await = state;
        let _ = self.event_tx.send(AgentEvent::StateChanged(state)).await;
    }

    /// Detect local IP address
    async fn detect_local_ip(&self) -> Result<String, SipError> {
        // If configured, use that
        if let Some(ip) = &self.config.local_ip {
            return Ok(ip.clone());
        }

        // Try to detect by connecting to the trunk
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        socket
            .connect(format!("{}:{}", self.config.trunk_host, self.config.trunk_port))
            .await?;

        let local_addr = socket.local_addr()?;
        let ip = local_addr.ip().to_string();

        *self.local_ip.write().await = Some(ip.clone());
        Ok(ip)
    }

    /// Get the first non-loopback IPv4 address
    fn get_local_ipv4() -> Result<std::net::IpAddr, SipError> {
        for iface in get_if_addrs::get_if_addrs().map_err(|e| SipError::Transport(e.to_string()))? {
            if !iface.is_loopback() {
                if let get_if_addrs::IfAddr::V4(ref addr) = iface.addr {
                    return Ok(std::net::IpAddr::V4(addr.ip));
                }
            }
        }
        Err(SipError::Transport("No IPv4 interface found".to_string()))
    }

    /// Register with SIP trunk using rsipstack
    pub async fn register(&self) -> Result<(), SipError> {
        // Validate config
        self.config
            .validate()
            .map_err(SipError::RegistrationFailed)?;

        self.set_state(AgentState::Connecting).await;

        // Detect local IP
        let local_ip = Self::get_local_ipv4()?;
        tracing::info!("SIP User Agent local IP: {}", local_ip);
        *self.local_ip.write().await = Some(local_ip.to_string());

        // Resolve DNS for the SIP trunk
        let trunk_addr = format!("{}:{}", self.config.trunk_host, self.config.trunk_port);
        let resolved_addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host(&trunk_addr)
            .await
            .map_err(|e| SipError::Transport(format!("DNS resolution failed: {}", e)))?
            .collect();

        let server_ip = resolved_addrs.first()
            .ok_or_else(|| SipError::Transport("No addresses found for SIP trunk".to_string()))?;

        tracing::info!("SIP trunk {} resolved to {}", self.config.trunk_host, server_ip);

        self.set_state(AgentState::Registering).await;

        // Create transport layer with outbound proxy set to resolved IP
        let token = self.cancel_token.clone();
        let mut transport_layer = TransportLayer::new(token.clone());

        // Set outbound destination to the resolved IP address
        // This allows the domain name to be used in SIP headers while routing to the IP
        transport_layer.outbound = Some(ftth_rsipstack::transport::SipAddr::from(*server_ip));

        // Create UDP connection on available port
        let local_port = 15060 + (rand::random::<u16>() % 1000); // Use port range 15060-16060
        let local_addr: std::net::SocketAddr = format!("{}:{}", local_ip, local_port)
            .parse()
            .map_err(|e: std::net::AddrParseError| SipError::Transport(e.to_string()))?;

        let connection = match UdpConnection::create_connection(
            local_addr,
            None,
            Some(token.child_token()),
        )
        .await
        {
            Ok(conn) => conn,
            Err(e) => {
                self.set_state(AgentState::Failed).await;
                return Err(SipError::Transport(format!("I/O error: {}", e)));
            }
        };

        transport_layer.add_transport(connection.into());

        // Create endpoint
        let endpoint = EndpointBuilder::new()
            .with_cancel_token(token.clone())
            .with_transport_layer(transport_layer)
            .build();

        // Create credentials
        let credential = Credential {
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            realm: Some(self.config.domain.clone()),
        };

        // Create SIP URI for the server using domain name (required by most SIP servers)
        let server_uri = format!("sip:{}:{}", self.config.trunk_host, self.config.trunk_port);
        let sip_server = ftth_rsipstack::rsip::Uri::try_from(server_uri.clone())
            .map_err(|e| SipError::RegistrationFailed(format!("Invalid SIP URI: {:?}", e)))?;

        // Store endpoint reference and credentials for call making
        *self.endpoint_inner.write().await = Some(endpoint.inner.clone());
        *self.credential.write().await = Some(credential.clone());
        *self.local_port.write().await = Some(local_port);

        // Create registration handler
        let mut registration = Registration::new(endpoint.inner.clone(), Some(credential));

        // Spawn endpoint server
        let endpoint_handle = tokio::spawn({
            let endpoint = endpoint;
            async move {
                endpoint.serve().await;
            }
        });

        // Attempt registration
        tracing::info!("Attempting SIP REGISTER to {}", server_uri);

        match tokio::time::timeout(
            Duration::from_secs(10),
            registration.register(sip_server.clone(), Some(3600)),
        )
        .await
        {
            Ok(Ok(response)) => {
                if response.status_code == ftth_rsipstack::rsip::StatusCode::OK {
                    tracing::info!(
                        "SIP registration successful! Expires in {} seconds",
                        registration.expires()
                    );
                    self.set_state(AgentState::Registered).await;

                    // Start registration refresh loop in background
                    let state_ref = self.state.clone();
                    let event_tx = self.event_tx.clone();
                    let expires = registration.expires();
                    let token_clone = token.clone();

                    tokio::spawn(async move {
                        // Keep the endpoint running
                        tokio::select! {
                            _ = endpoint_handle => {
                                tracing::info!("SIP endpoint finished");
                            }
                            _ = token_clone.cancelled() => {
                                tracing::info!("SIP registration cancelled");
                            }
                            _ = async {
                                // Re-register loop
                                loop {
                                    // Wait 75% of expiration time before re-registering
                                    let refresh_time = (expires as u64 * 3) / 4;
                                    tokio::time::sleep(Duration::from_secs(refresh_time.max(30))).await;

                                    match registration.register(sip_server.clone(), Some(3600)).await {
                                        Ok(resp) if resp.status_code == ftth_rsipstack::rsip::StatusCode::OK => {
                                            tracing::debug!("SIP re-registration successful");
                                        }
                                        Ok(resp) => {
                                            tracing::warn!("SIP re-registration failed: {:?}", resp.status_code);
                                            *state_ref.write().await = AgentState::Failed;
                                            let _ = event_tx.send(AgentEvent::Error(
                                                format!("Re-registration failed: {:?}", resp.status_code)
                                            )).await;
                                            break;
                                        }
                                        Err(e) => {
                                            tracing::error!("SIP re-registration error: {:?}", e);
                                            *state_ref.write().await = AgentState::Failed;
                                            let _ = event_tx.send(AgentEvent::Error(
                                                format!("Re-registration error: {:?}", e)
                                            )).await;
                                            break;
                                        }
                                    }
                                }
                            } => {}
                        }
                    });

                    Ok(())
                } else {
                    let error_msg = format!("Registration failed: {:?}", response.status_code);
                    tracing::error!("{}", error_msg);
                    self.set_state(AgentState::Failed).await;
                    Err(SipError::RegistrationFailed(error_msg))
                }
            }
            Ok(Err(e)) => {
                let error_msg = format!("Registration error: {:?}", e);
                tracing::error!("{}", error_msg);
                self.set_state(AgentState::Failed).await;
                Err(SipError::RegistrationFailed(error_msg))
            }
            Err(_) => {
                let error_msg = "Registration timed out after 10 seconds";
                tracing::error!("{}", error_msg);
                self.set_state(AgentState::Failed).await;
                Err(SipError::Timeout(error_msg.to_string()))
            }
        }
    }

    /// Unregister from SIP trunk
    pub async fn unregister(&self) -> Result<(), SipError> {
        // Cancel registration loop
        self.cancel_token.cancel();
        self.set_state(AgentState::Disconnected).await;
        Ok(())
    }

    /// Make an outbound call
    pub async fn dial(&self, to: &str) -> Result<String, SipError> {
        if !self.is_registered().await {
            return Err(SipError::NotRegistered);
        }

        // Get endpoint reference
        let endpoint_inner = self.endpoint_inner.read().await.clone()
            .ok_or_else(|| SipError::InvalidState("Endpoint not initialized".to_string()))?;

        let credential = self.credential.read().await.clone();
        let local_ip = self.local_ip.read().await.clone()
            .ok_or_else(|| SipError::InvalidState("Local IP not set".to_string()))?;
        let local_port = self.local_port.read().await
            .ok_or_else(|| SipError::InvalidState("Local port not set".to_string()))?;

        let call_id = Uuid::new_v4().to_string();

        // Create event channel for this call
        let (call_event_tx, mut call_event_rx) = mpsc::channel(50);

        // Allocate RTP port
        let rtp_port = self.rtp_ports.allocate().await;
        let rtp_session = RtpSession::new(rtp_port, self.config.codec).await?;
        let rtp_local_port = rtp_session.local_port();

        // Create SDP offer
        let sdp_offer = self.create_sdp_offer(&local_ip, rtp_local_port);

        // Build SIP URIs
        let caller_uri = format!("sip:{}@{}", self.config.username, self.config.domain);
        let callee_uri = format!("sip:{}@{}", to.trim_start_matches('+'), self.config.trunk_host);
        let contact_uri = format!("sip:{}@{}:{}", self.config.username, local_ip, local_port);

        tracing::info!("Dialing: {} -> {}", caller_uri, callee_uri);

        // Create INVITE option
        let invite_option = InviteOption {
            caller: caller_uri.as_str().try_into()
                .map_err(|e| SipError::CallFailed(format!("Invalid caller URI: {:?}", e)))?,
            callee: callee_uri.as_str().try_into()
                .map_err(|e| SipError::CallFailed(format!("Invalid callee URI: {:?}", e)))?,
            content_type: Some("application/sdp".to_string()),
            destination: None,
            offer: Some(sdp_offer.into_bytes()),
            contact: contact_uri.as_str().try_into()
                .map_err(|e| SipError::CallFailed(format!("Invalid contact URI: {:?}", e)))?,
            credential,
            headers: None,
        };

        // Create dialog layer and send INVITE
        let dialog_layer = DialogLayer::new(endpoint_inner);
        let (state_tx, mut state_rx) = unbounded_channel();

        // Create call object
        let sip_call_id = format!("{}@{}", Uuid::new_v4(), self.config.domain);
        let mut call = SipCall::new_outbound(
            call_id.clone(),
            sip_call_id.clone(),
            self.config.caller_id.clone(),
            to.to_string(),
            call_event_tx,
        );
        call.set_rtp_session(Arc::new(rtp_session));

        // Enable recording for all calls
        call.enable_recording();

        // Store call
        let call = Arc::new(RwLock::new(call));
        self.calls.write().await.insert(call_id.clone(), call.clone());

        // Forward call events to agent events
        let event_tx = self.event_tx.clone();
        let cid = call_id.clone();

        tokio::spawn(async move {
            while let Some(event) = call_event_rx.recv().await {
                if let CallEvent::StateChanged(state) = event {
                    let _ = event_tx
                        .send(AgentEvent::CallStateChanged {
                            call_id: cid.clone(),
                            state,
                        })
                        .await;
                }
            }
        });

        // Set initial state
        {
            let call_ref = call.read().await;
            call_ref.set_state(CallState::Trying).await;
        }

        // Spawn state monitoring task - this handles real-time state updates
        let call_for_states = call.clone();
        let call_id_for_states = call_id.clone();

        tokio::spawn(async move {
            while let Some(dialog_state) = state_rx.recv().await {
                let call_ref = call_for_states.read().await;

                match dialog_state {
                    DialogState::Calling(_) => {
                        tracing::debug!("Call {} - Calling", call_id_for_states);
                    }
                    DialogState::Trying(_) => {
                        tracing::info!("Call {} - Trying (100)", call_id_for_states);
                    }
                    DialogState::Early(_, ref resp) => {
                        let status = resp.status_code.code();
                        tracing::info!("Call {} - Early response: {} (Ringing)", call_id_for_states, status);
                        call_ref.set_state(CallState::Ringing).await;
                    }
                    DialogState::Confirmed(_, _) => {
                        tracing::info!("Call {} - Confirmed (200 OK)", call_id_for_states);
                        call_ref.set_state(CallState::Active).await;
                    }
                    DialogState::Terminated(_, ref reason) => {
                        tracing::info!("Call {} - Terminated: {:?}", call_id_for_states, reason);
                        call_ref.set_state(CallState::Failed).await;
                    }
                    _ => {
                        tracing::debug!("Call {} - Other state update", call_id_for_states);
                    }
                }
            }
            tracing::debug!("Call {} - State channel closed", call_id_for_states);
        });

        // Spawn INVITE task
        let call_clone = call.clone();
        let call_id_clone = call_id.clone();

        tokio::spawn(async move {
            // Send INVITE - this blocks until we get a final response
            match dialog_layer.do_invite(invite_option, state_tx).await {
                Ok((client_dialog, response)) => {
                    let call_ref = call_clone.read().await;

                    // Check the FINAL response (not provisional - those come via state channel)
                    if let Some(resp) = response {
                        let status = resp.status_code.code();
                        tracing::info!("Call {} final response: {}", call_id_clone, status);

                        if status >= 200 && status < 300 {
                            // Call answered (200 OK)
                            tracing::info!("Call {} connected!", call_id_clone);
                            // State already set to Active via state channel

                            // Keep dialog alive - wait for BYE or hangup
                            // The dialog will handle BYE automatically
                            // We just need to keep the dialog reference alive
                            loop {
                                let state = call_ref.state().await;
                                if state == CallState::Ended || state == CallState::Failed {
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }

                            // Hangup if still active
                            if call_ref.state().await == CallState::Active {
                                tracing::info!("Call {} hanging up", call_id_clone);
                                let _ = client_dialog.bye().await;
                                call_ref.set_state(CallState::Ended).await;
                            }
                        } else {
                            // Call failed - 4xx, 5xx, 6xx response
                            let reason = match status {
                                400 => "Bad Request",
                                401 => "Unauthorized",
                                403 => "Forbidden",
                                404 => "Not Found",
                                408 => "Request Timeout",
                                480 => "Temporarily Unavailable",
                                486 => "Busy Here",
                                487 => "Request Cancelled",
                                488 => "Not Acceptable",
                                500 => "Server Error",
                                503 => "Service Unavailable",
                                600 => "Busy Everywhere",
                                603 => "Decline",
                                604 => "Does Not Exist",
                                _ => "Unknown Error",
                            };
                            tracing::warn!("Call {} failed: {} - {}", call_id_clone, status, reason);
                            call_ref.set_state(CallState::Failed).await;
                        }
                    } else {
                        // No final response received (shouldn't happen normally)
                        tracing::warn!("Call {} - No final response received", call_id_clone);
                        call_ref.set_state(CallState::Failed).await;
                    }
                }
                Err(e) => {
                    tracing::error!("Call {} INVITE error: {:?}", call_id_clone, e);
                    let call_ref = call_clone.read().await;
                    call_ref.set_state(CallState::Failed).await;
                }
            }
        });

        tracing::info!(
            "SIP call initiated: {} -> {} (call_id: {})",
            self.config.caller_id,
            to,
            call_id
        );

        Ok(call_id)
    }

    /// Create SDP offer for outbound call
    fn create_sdp_offer(&self, local_ip: &str, rtp_port: u16) -> String {
        let session_id = rand::random::<u32>();
        let codec = &self.config.codec;

        format!(
            "v=0\r\n\
             o=- {} 1 IN IP4 {}\r\n\
             s=VoIP CRM Call\r\n\
             c=IN IP4 {}\r\n\
             t=0 0\r\n\
             m=audio {} RTP/AVP {}\r\n\
             a=rtpmap:{} {}/8000\r\n\
             a=ptime:20\r\n\
             a=sendrecv\r\n",
            session_id,
            local_ip,
            local_ip,
            rtp_port,
            codec.payload_type(),
            codec.payload_type(),
            codec.sdp_name()
        )
    }

    /// Answer an incoming call
    pub async fn answer(&self, call_id: &str) -> Result<(), SipError> {
        let calls = self.calls.read().await;
        let call = calls
            .get(call_id)
            .ok_or_else(|| SipError::CallNotFound(call_id.to_string()))?;

        let call = call.read().await;
        let state = call.state().await;

        if state != CallState::Ringing {
            return Err(SipError::InvalidState(format!(
                "Cannot answer call in state: {}",
                state
            )));
        }

        call.set_state(CallState::Active).await;

        // Start RTP
        if let Some(rtp) = call.rtp_session() {
            rtp.start().await?;
        }

        Ok(())
    }

    /// Hang up a call
    pub async fn hangup(&self, call_id: &str) -> Result<(), SipError> {
        let call = {
            let calls = self.calls.read().await;
            calls
                .get(call_id)
                .ok_or_else(|| SipError::CallNotFound(call_id.to_string()))?
                .clone()
        };

        let call = call.read().await;
        let state = call.state().await;

        if state == CallState::Ended || state == CallState::Failed {
            return Ok(()); // Already ended
        }

        call.set_state(CallState::Terminating).await;

        // Stop RTP
        if let Some(rtp) = call.rtp_session() {
            rtp.stop().await;
        }

        call.set_state(CallState::Ended).await;

        // Remove from active calls
        self.calls.write().await.remove(call_id);

        tracing::info!("SIP call ended: {}", call_id);

        Ok(())
    }

    /// Get a call by ID
    pub async fn get_call(&self, call_id: &str) -> Option<Arc<RwLock<SipCall>>> {
        self.calls.read().await.get(call_id).cloned()
    }

    /// Get all active calls
    pub async fn active_calls(&self) -> Vec<String> {
        self.calls.read().await.keys().cloned().collect()
    }

    /// Send audio to a call
    pub async fn send_audio(&self, call_id: &str, samples: &[i16]) -> Result<(), SipError> {
        let calls = self.calls.read().await;
        let call = calls
            .get(call_id)
            .ok_or_else(|| SipError::CallNotFound(call_id.to_string()))?;

        let call = call.read().await;
        call.send_audio(samples).await
    }

    /// Get audio receiver for a call
    pub async fn take_audio_receiver(&self, call_id: &str) -> Option<mpsc::Receiver<AudioFrame>> {
        let calls = self.calls.read().await;
        if let Some(call) = calls.get(call_id) {
            let call = call.read().await;
            call.take_audio_receiver().await
        } else {
            None
        }
    }

    /// Shutdown the user agent
    pub async fn shutdown(&self) {
        *self.shutdown.write().await = true;

        // Hang up all calls
        let call_ids: Vec<String> = self.calls.read().await.keys().cloned().collect();
        for call_id in call_ids {
            let _ = self.hangup(&call_id).await;
        }

        // Unregister
        let _ = self.unregister().await;
    }

    /// Simulate call state progression (for testing without full SIP)
    pub async fn simulate_call_connected(&self, call_id: &str) -> Result<(), SipError> {
        let calls = self.calls.read().await;
        let call = calls
            .get(call_id)
            .ok_or_else(|| SipError::CallNotFound(call_id.to_string()))?;

        let call = call.read().await;

        // Progress through states
        call.set_state(CallState::Ringing).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        call.set_state(CallState::Active).await;

        // Start RTP
        if let Some(rtp) = call.rtp_session() {
            rtp.start().await?;
        }

        Ok(())
    }
}

impl Drop for SipUserAgent {
    fn drop(&mut self) {
        tracing::info!("SIP User Agent shutting down");
        self.cancel_token.cancel();
    }
}

/// Builder for SipUserAgent
pub struct SipUserAgentBuilder {
    config: SipConfig,
}

impl SipUserAgentBuilder {
    pub fn new() -> Self {
        Self {
            config: SipConfig::default(),
        }
    }

    pub fn trunk(mut self, host: &str, port: u16) -> Self {
        self.config.trunk_host = host.to_string();
        self.config.trunk_port = port;
        self.config.domain = host.to_string();
        self
    }

    pub fn credentials(mut self, username: &str, password: &str) -> Self {
        self.config.username = username.to_string();
        self.config.password = password.to_string();
        self
    }

    pub fn caller_id(mut self, caller_id: &str) -> Self {
        self.config.caller_id = caller_id.to_string();
        self
    }

    pub fn codec(mut self, codec: SipCodec) -> Self {
        self.config.codec = codec;
        self
    }

    pub fn build(self) -> (SipUserAgent, mpsc::Receiver<AgentEvent>) {
        SipUserAgent::new(self.config)
    }
}

impl Default for SipUserAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
