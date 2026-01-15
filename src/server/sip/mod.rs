//! Pure Rust SIP Stack for VoIP CRM
//!
//! This module provides direct SIP trunk integration without external services.
//! Features:
//! - SIP registration with any trunk provider
//! - Outbound and inbound call handling
//! - RTP audio streaming for AI integration
//! - G.711 codec support (PCMU/PCMA)

mod config;
mod codec;
mod rtp;
mod audio_mixer;
mod audio_converter;
mod user_agent;
mod call;

pub use config::SipConfig;
pub use user_agent::{SipUserAgent, AgentState};

// Public API re-exports for external use
#[allow(unused_imports)]
pub use call::{SipCall, CallState, CallDirection};
#[allow(unused_imports)]
pub use codec::G711Codec;
#[allow(unused_imports)]
pub use rtp::RtpSession;
#[allow(unused_imports)]
pub use audio_mixer::{AudioMixer, MixMode};
#[allow(unused_imports)]
pub use audio_converter::AudioConverter;

use thiserror::Error;

/// SIP-related errors
#[derive(Error, Debug)]
pub enum SipError {
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Call failed: {0}")]
    CallFailed(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Codec error: {0}")]
    Codec(String),

    #[error("RTP error: {0}")]
    Rtp(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Not registered")]
    NotRegistered,

    #[error("Call not found: {0}")]
    CallNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
