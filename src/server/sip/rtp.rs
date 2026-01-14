//! RTP (Real-time Transport Protocol) Session Handler
//!
//! Handles RTP audio streaming for SIP calls.
//! Implements RFC 3550 for RTP packet format.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use bytes::{BufMut, Bytes, BytesMut};

use super::codec::G711Codec;
use super::config::SipCodec;
use super::SipError;

/// RTP packet header (12 bytes minimum)
#[derive(Debug, Clone)]
pub struct RtpHeader {
    /// RTP version (always 2)
    pub version: u8,
    /// Padding flag
    pub padding: bool,
    /// Extension flag
    pub extension: bool,
    /// CSRC count
    pub csrc_count: u8,
    /// Marker bit
    pub marker: bool,
    /// Payload type (0 = PCMU, 8 = PCMA)
    pub payload_type: u8,
    /// Sequence number
    pub sequence: u16,
    /// Timestamp
    pub timestamp: u32,
    /// Synchronization source identifier
    pub ssrc: u32,
}

impl RtpHeader {
    pub fn new(payload_type: u8, sequence: u16, timestamp: u32, ssrc: u32) -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type,
            sequence,
            timestamp,
            ssrc,
        }
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(12);

        // First byte: V(2) P(1) X(1) CC(4)
        let first_byte = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | self.csrc_count;
        buf.put_u8(first_byte);

        // Second byte: M(1) PT(7)
        let second_byte = ((self.marker as u8) << 7) | self.payload_type;
        buf.put_u8(second_byte);

        // Sequence number (2 bytes)
        buf.put_u16(self.sequence);

        // Timestamp (4 bytes)
        buf.put_u32(self.timestamp);

        // SSRC (4 bytes)
        buf.put_u32(self.ssrc);

        buf
    }

    /// Parse header from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), SipError> {
        if data.len() < 12 {
            return Err(SipError::Rtp("RTP packet too short".to_string()));
        }

        let first_byte = data[0];
        let version = first_byte >> 6;
        if version != 2 {
            return Err(SipError::Rtp(format!("Invalid RTP version: {}", version)));
        }

        let padding = (first_byte >> 5) & 1 == 1;
        let extension = (first_byte >> 4) & 1 == 1;
        let csrc_count = first_byte & 0x0F;

        let second_byte = data[1];
        let marker = (second_byte >> 7) & 1 == 1;
        let payload_type = second_byte & 0x7F;

        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let header_len = 12 + (csrc_count as usize * 4);

        Ok((
            Self {
                version,
                padding,
                extension,
                csrc_count,
                marker,
                payload_type,
                sequence,
                timestamp,
                ssrc,
            },
            header_len,
        ))
    }
}

/// RTP packet with header and payload
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub payload: Bytes,
}

impl RtpPacket {
    pub fn new(header: RtpHeader, payload: impl Into<Bytes>) -> Self {
        Self {
            header,
            payload: payload.into(),
        }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = self.header.to_bytes();
        buf.extend_from_slice(&self.payload);
        buf.freeze()
    }

    /// Parse packet from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, SipError> {
        let (header, header_len) = RtpHeader::from_bytes(data)?;
        let payload = Bytes::copy_from_slice(&data[header_len..]);

        Ok(Self { header, payload })
    }
}

/// Audio frame received from remote party
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// PCM audio samples (16-bit signed)
    pub samples: Vec<i16>,
    /// Timestamp in samples
    pub timestamp: u32,
    /// Sequence number
    pub sequence: u16,
}

/// RTP Session for a SIP call
pub struct RtpSession {
    /// Local UDP socket for RTP
    socket: Arc<UdpSocket>,
    /// Remote RTP endpoint
    remote_addr: RwLock<Option<SocketAddr>>,
    /// SSRC for outgoing packets
    ssrc: u32,
    /// Current sequence number
    sequence: RwLock<u16>,
    /// Current timestamp
    timestamp: RwLock<u32>,
    /// Audio codec
    codec: G711Codec,
    /// Payload type
    payload_type: u8,
    /// Channel for received audio frames
    audio_tx: mpsc::Sender<AudioFrame>,
    /// Receiver for audio frames
    audio_rx: RwLock<Option<mpsc::Receiver<AudioFrame>>>,
    /// Running flag
    running: RwLock<bool>,
}

impl RtpSession {
    /// Create a new RTP session
    /// Tries the suggested port first, then tries nearby ports if that fails
    pub async fn new(suggested_port: u16, codec_type: SipCodec) -> Result<Self, SipError> {
        // Try suggested port first, then try up to 50 more ports
        let socket = Self::try_bind_port(suggested_port, 50).await?;
        let ssrc = rand::random::<u32>();
        let codec = G711Codec::new(codec_type);
        let payload_type = codec_type.payload_type();

        let (audio_tx, audio_rx) = mpsc::channel(100);

        Ok(Self {
            socket: Arc::new(socket),
            remote_addr: RwLock::new(None),
            ssrc,
            sequence: RwLock::new(rand::random::<u16>()),
            timestamp: RwLock::new(rand::random::<u32>()),
            codec,
            payload_type,
            audio_tx,
            audio_rx: RwLock::new(Some(audio_rx)),
            running: RwLock::new(false),
        })
    }

    /// Try to bind to a port, trying multiple ports if necessary
    async fn try_bind_port(start_port: u16, max_attempts: u16) -> Result<UdpSocket, SipError> {
        let mut port = start_port;
        for attempt in 0..max_attempts {
            match UdpSocket::bind(format!("0.0.0.0:{}", port)).await {
                Ok(socket) => {
                    if attempt > 0 {
                        tracing::debug!("RTP bound to port {} after {} attempts", port, attempt + 1);
                    }
                    return Ok(socket);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                    // Try next even port (RTP uses even ports)
                    port = port.wrapping_add(2);
                    if port < 1024 {
                        port = 10000; // Wrap around to safe range
                    }
                }
                Err(e) => {
                    return Err(SipError::Rtp(format!("Failed to bind RTP socket: {}", e)));
                }
            }
        }
        Err(SipError::Rtp(format!(
            "Could not find available RTP port after {} attempts starting from {}",
            max_attempts, start_port
        )))
    }

    /// Get the local port
    pub fn local_port(&self) -> u16 {
        self.socket.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    /// Set the remote RTP endpoint
    pub async fn set_remote(&self, addr: SocketAddr) {
        *self.remote_addr.write().await = Some(addr);
    }

    /// Take the audio receiver (can only be called once)
    pub async fn take_audio_receiver(&self) -> Option<mpsc::Receiver<AudioFrame>> {
        self.audio_rx.write().await.take()
    }

    /// Start receiving RTP packets
    pub async fn start(&self) -> Result<(), SipError> {
        *self.running.write().await = true;

        let socket = self.socket.clone();
        let audio_tx = self.audio_tx.clone();
        let codec_type = self.payload_type;
        let _running = Arc::new(*self.running.read().await);

        // Spawn receiver task
        tokio::spawn(async move {
            let codec = if codec_type == 0 {
                G711Codec::pcmu()
            } else {
                G711Codec::pcma()
            };

            let mut buf = [0u8; 2048];

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, _addr)) => {
                        if let Ok(packet) = RtpPacket::from_bytes(&buf[..len]) {
                            // Decode audio
                            let samples = codec.decode(&packet.payload);

                            let frame = AudioFrame {
                                samples,
                                timestamp: packet.header.timestamp,
                                sequence: packet.header.sequence,
                            };

                            if audio_tx.send(frame).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("RTP receive error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the RTP session
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// Send audio samples
    pub async fn send_audio(&self, samples: &[i16]) -> Result<(), SipError> {
        let remote = self.remote_addr.read().await;
        let remote_addr = remote.as_ref().ok_or(SipError::Rtp("No remote address set".to_string()))?;

        // Encode audio
        let payload = self.codec.encode(samples);

        // Get and increment sequence/timestamp
        let sequence = {
            let mut seq = self.sequence.write().await;
            let current = *seq;
            *seq = seq.wrapping_add(1);
            current
        };

        let timestamp = {
            let mut ts = self.timestamp.write().await;
            let current = *ts;
            *ts = ts.wrapping_add(samples.len() as u32);
            current
        };

        // Build packet
        let header = RtpHeader::new(self.payload_type, sequence, timestamp, self.ssrc);
        let packet = RtpPacket::new(header, payload);

        // Send
        self.socket.send_to(&packet.to_bytes(), remote_addr).await?;

        Ok(())
    }

    /// Send raw encoded audio (already G.711 encoded)
    pub async fn send_encoded(&self, encoded: &[u8]) -> Result<(), SipError> {
        let remote = self.remote_addr.read().await;
        let remote_addr = remote.as_ref().ok_or(SipError::Rtp("No remote address set".to_string()))?;

        let sequence = {
            let mut seq = self.sequence.write().await;
            let current = *seq;
            *seq = seq.wrapping_add(1);
            current
        };

        let timestamp = {
            let mut ts = self.timestamp.write().await;
            let current = *ts;
            *ts = ts.wrapping_add(encoded.len() as u32);
            current
        };

        let header = RtpHeader::new(self.payload_type, sequence, timestamp, self.ssrc);
        let packet = RtpPacket::new(header, Bytes::copy_from_slice(encoded));

        self.socket.send_to(&packet.to_bytes(), remote_addr).await?;

        Ok(())
    }

    /// Generate silence (160 samples = 20ms at 8kHz)
    pub fn silence_frame() -> Vec<i16> {
        vec![0i16; 160]
    }
}

/// RTP port allocator
pub struct RtpPortAllocator {
    start: u16,
    end: u16,
    current: RwLock<u16>,
}

impl RtpPortAllocator {
    pub fn new(start: u16, end: u16) -> Self {
        Self {
            start,
            end,
            current: RwLock::new(start),
        }
    }

    /// Allocate next available port (even number for RTP, odd for RTCP)
    pub async fn allocate(&self) -> u16 {
        let mut current = self.current.write().await;
        let port = *current;

        // Increment by 2 (RTP uses even ports, RTCP uses odd)
        *current = if *current + 2 >= self.end {
            self.start
        } else {
            *current + 2
        };

        // Ensure even port
        if port % 2 != 0 {
            port + 1
        } else {
            port
        }
    }
}
