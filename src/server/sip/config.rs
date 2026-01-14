//! SIP Configuration
//!
//! Configuration for connecting to SIP trunk providers.

use serde::{Deserialize, Serialize};

/// SIP transport protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SipTransport {
    #[default]
    Udp,
    Tcp,
    Tls,
}

impl SipTransport {
    pub fn default_port(&self) -> u16 {
        match self {
            SipTransport::Udp | SipTransport::Tcp => 5060,
            SipTransport::Tls => 5061,
        }
    }
}

impl std::fmt::Display for SipTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SipTransport::Udp => write!(f, "UDP"),
            SipTransport::Tcp => write!(f, "TCP"),
            SipTransport::Tls => write!(f, "TLS"),
        }
    }
}

/// Audio codec for SIP calls
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SipCodec {
    /// G.711 μ-law (US standard) - Payload type 0
    #[default]
    Pcmu,
    /// G.711 A-law (EU standard) - Payload type 8
    Pcma,
}

impl SipCodec {
    /// RTP payload type number
    pub fn payload_type(&self) -> u8 {
        match self {
            SipCodec::Pcmu => 0,
            SipCodec::Pcma => 8,
        }
    }

    /// Sample rate in Hz
    pub fn sample_rate(&self) -> u32 {
        8000 // G.711 always uses 8kHz
    }

    /// Samples per RTP packet (20ms of audio)
    pub fn samples_per_packet(&self) -> usize {
        160 // 20ms at 8kHz
    }

    /// Codec name for SDP
    pub fn sdp_name(&self) -> &'static str {
        match self {
            SipCodec::Pcmu => "PCMU",
            SipCodec::Pcma => "PCMA",
        }
    }
}


/// SIP trunk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SipConfig {
    /// SIP trunk hostname (e.g., "sip.twilio.com")
    pub trunk_host: String,

    /// SIP trunk port (default: 5060 for UDP/TCP, 5061 for TLS)
    pub trunk_port: u16,

    /// SIP username for authentication
    pub username: String,

    /// SIP password for authentication
    pub password: String,

    /// Caller ID / DID number (e.g., "+15551234567")
    pub caller_id: String,

    /// SIP domain (usually same as trunk_host)
    pub domain: String,

    /// Transport protocol
    pub transport: SipTransport,

    /// Preferred audio codec
    pub codec: SipCodec,

    /// Local IP for RTP (auto-detected if None)
    pub local_ip: Option<String>,

    /// Local RTP port range start
    pub rtp_port_start: u16,

    /// Local RTP port range end
    pub rtp_port_end: u16,

    /// Registration expiry in seconds
    pub register_expires: u32,

    /// Enable STUN for NAT traversal
    pub stun_server: Option<String>,

    /// User agent string
    pub user_agent: String,
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            trunk_host: String::new(),
            trunk_port: 5060,
            username: String::new(),
            password: String::new(),
            caller_id: String::new(),
            domain: String::new(),
            transport: SipTransport::Udp,
            codec: SipCodec::Pcmu,
            local_ip: None,
            rtp_port_start: 20000,
            rtp_port_end: 30000,
            register_expires: 3600,
            stun_server: None,
            user_agent: "VoIP-CRM/1.0 (Rust)".to_string(),
        }
    }
}

impl SipConfig {
    /// Create config from environment variables
    pub fn from_env() -> Option<Self> {
        let trunk_host = std::env::var("SIP_TRUNK_HOST").ok()?;
        let username = std::env::var("SIP_USERNAME").ok()?;
        let password = std::env::var("SIP_PASSWORD").ok()?;
        let caller_id = std::env::var("SIP_CALLER_ID").ok()?;

        let trunk_port = std::env::var("SIP_TRUNK_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5060);

        let transport = match std::env::var("SIP_TRANSPORT")
            .unwrap_or_default()
            .to_uppercase()
            .as_str()
        {
            "TCP" => SipTransport::Tcp,
            "TLS" => SipTransport::Tls,
            _ => SipTransport::Udp,
        };

        let domain = std::env::var("SIP_DOMAIN").unwrap_or_else(|_| trunk_host.clone());

        let codec = match std::env::var("SIP_CODEC")
            .unwrap_or_default()
            .to_uppercase()
            .as_str()
        {
            "PCMA" | "ALAW" => SipCodec::Pcma,
            _ => SipCodec::Pcmu,
        };

        Some(Self {
            trunk_host,
            trunk_port,
            username,
            password,
            caller_id,
            domain,
            transport,
            codec,
            local_ip: std::env::var("SIP_LOCAL_IP").ok(),
            rtp_port_start: std::env::var("SIP_RTP_PORT_START")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(20000),
            rtp_port_end: std::env::var("SIP_RTP_PORT_END")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(30000),
            register_expires: std::env::var("SIP_REGISTER_EXPIRES")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3600),
            stun_server: std::env::var("SIP_STUN_SERVER").ok(),
            user_agent: "VoIP-CRM/1.0 (Rust)".to_string(),
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.trunk_host.is_empty() {
            return Err("SIP trunk host is required".to_string());
        }
        if self.username.is_empty() {
            return Err("SIP username is required".to_string());
        }
        if self.password.is_empty() {
            return Err("SIP password is required".to_string());
        }
        if self.caller_id.is_empty() {
            return Err("SIP caller ID is required".to_string());
        }
        if self.rtp_port_start >= self.rtp_port_end {
            return Err("RTP port range is invalid".to_string());
        }
        Ok(())
    }

    /// Get the SIP URI for registration
    pub fn registrar_uri(&self) -> String {
        format!("sip:{}", self.domain)
    }

    /// Get the From URI for outgoing requests
    pub fn caller_uri(&self) -> String {
        format!("sip:{}@{}", self.username, self.domain)
    }

    /// Get the Contact URI
    pub fn contact_uri(&self, local_ip: &str, local_port: u16) -> String {
        format!("sip:{}@{}:{}", self.username, local_ip, local_port)
    }
}
