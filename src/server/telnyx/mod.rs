//! Telnyx Voice API client

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelnyxError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {message}")]
    Api { message: String },
    #[error("Missing call control ID")]
    MissingCallControlId,
}

#[derive(Clone)]
pub struct TelnyxClient {
    client: Client,
    api_key: String,
    connection_id: String,
    base_url: String,
}

impl TelnyxClient {
    pub fn new(api_key: String, connection_id: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            connection_id,
            base_url: "https://api.telnyx.com/v2".to_string(),
        }
    }

    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, TelnyxError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TelnyxError::Api { message: error_text });
        }

        Ok(response.json().await?)
    }

    /// Initiate an outbound call
    pub async fn dial(
        &self,
        to: &str,
        from: &str,
        webhook_url: Option<&str>,
    ) -> Result<DialResponse, TelnyxError> {
        let request = DialRequest {
            to,
            from,
            connection_id: &self.connection_id,
            webhook_url: webhook_url.unwrap_or(""),
            webhook_url_method: "POST",
            answer_machine_detection: Some("detect"),
        };

        let response: TelnyxResponse<DialData> = self.post("/calls", &request).await?;
        Ok(DialResponse {
            call_control_id: response.data.call_control_id,
            call_leg_id: response.data.call_leg_id,
            call_session_id: response.data.call_session_id,
        })
    }

    /// Answer an incoming call
    pub async fn answer(&self, call_control_id: &str) -> Result<(), TelnyxError> {
        let request = CallControlRequest {
            client_state: None,
            command_id: None,
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/answer", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Hang up a call
    pub async fn hangup(&self, call_control_id: &str) -> Result<(), TelnyxError> {
        let request = CallControlRequest {
            client_state: None,
            command_id: None,
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/hangup", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Transfer call to another number
    pub async fn transfer(&self, call_control_id: &str, to: &str) -> Result<(), TelnyxError> {
        let request = TransferRequest { to };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/transfer", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Bridge two calls together
    pub async fn bridge(
        &self,
        call_control_id: &str,
        target_call_control_id: &str,
    ) -> Result<(), TelnyxError> {
        let request = BridgeRequest {
            call_control_id: target_call_control_id,
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/bridge", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Speak text-to-speech on the call
    pub async fn speak(
        &self,
        call_control_id: &str,
        text: &str,
        voice: Option<&str>,
    ) -> Result<(), TelnyxError> {
        let request = SpeakRequest {
            payload: text,
            voice: voice.unwrap_or("female"),
            language: "en-US",
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/speak", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Play audio file on the call
    pub async fn play_audio(&self, call_control_id: &str, audio_url: &str) -> Result<(), TelnyxError> {
        let request = PlayAudioRequest { audio_url };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/playback_start", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Start call recording
    pub async fn start_recording(
        &self,
        call_control_id: &str,
        channels: &str,
    ) -> Result<(), TelnyxError> {
        let request = RecordingRequest {
            channels,
            format: "mp3",
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/record_start", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Stop call recording
    pub async fn stop_recording(&self, call_control_id: &str) -> Result<(), TelnyxError> {
        let request = CallControlRequest {
            client_state: None,
            command_id: None,
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/record_stop", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Put call on hold (mute and play hold music)
    pub async fn hold(&self, call_control_id: &str, audio_url: Option<&str>) -> Result<(), TelnyxError> {
        // Mute the call
        let mute_request = MuteRequest { mute: true };
        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/mute", call_control_id), &mute_request)
            .await?;

        // Play hold music if URL provided
        if let Some(url) = audio_url {
            self.play_audio(call_control_id, url).await?;
        }

        Ok(())
    }

    /// Resume call from hold
    pub async fn unhold(&self, call_control_id: &str) -> Result<(), TelnyxError> {
        let request = MuteRequest { mute: false };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/mute", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Send DTMF tones
    pub async fn send_dtmf(&self, call_control_id: &str, digits: &str) -> Result<(), TelnyxError> {
        let request = DtmfRequest { digits };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/send_dtmf", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Start media streaming (for AI integration)
    pub async fn start_streaming(
        &self,
        call_control_id: &str,
        stream_url: &str,
    ) -> Result<(), TelnyxError> {
        let request = StreamingRequest {
            stream_url,
            stream_track: "both_tracks",
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/streaming_start", call_control_id), &request)
            .await?;
        Ok(())
    }

    /// Stop media streaming
    pub async fn stop_streaming(&self, call_control_id: &str) -> Result<(), TelnyxError> {
        let request = CallControlRequest {
            client_state: None,
            command_id: None,
        };

        let _: TelnyxResponse<serde_json::Value> = self
            .post(&format!("/calls/{}/actions/streaming_stop", call_control_id), &request)
            .await?;
        Ok(())
    }
}

// Request/Response types

#[derive(Serialize)]
struct DialRequest<'a> {
    to: &'a str,
    from: &'a str,
    connection_id: &'a str,
    webhook_url: &'a str,
    webhook_url_method: &'a str,
    answer_machine_detection: Option<&'a str>,
}

#[derive(Serialize)]
struct CallControlRequest {
    client_state: Option<String>,
    command_id: Option<String>,
}

#[derive(Serialize)]
struct TransferRequest<'a> {
    to: &'a str,
}

#[derive(Serialize)]
struct BridgeRequest<'a> {
    call_control_id: &'a str,
}

#[derive(Serialize)]
struct SpeakRequest<'a> {
    payload: &'a str,
    voice: &'a str,
    language: &'a str,
}

#[derive(Serialize)]
struct PlayAudioRequest<'a> {
    audio_url: &'a str,
}

#[derive(Serialize)]
struct RecordingRequest<'a> {
    channels: &'a str,
    format: &'a str,
}

#[derive(Serialize)]
struct MuteRequest {
    mute: bool,
}

#[derive(Serialize)]
struct DtmfRequest<'a> {
    digits: &'a str,
}

#[derive(Serialize)]
struct StreamingRequest<'a> {
    stream_url: &'a str,
    stream_track: &'a str,
}

#[derive(Deserialize)]
struct TelnyxResponse<T> {
    data: T,
}

#[derive(Deserialize)]
struct DialData {
    call_control_id: String,
    call_leg_id: String,
    call_session_id: String,
}

#[derive(Debug)]
pub struct DialResponse {
    pub call_control_id: String,
    pub call_leg_id: String,
    pub call_session_id: String,
}

// Webhook event types
#[derive(Debug, Deserialize)]
pub struct TelnyxWebhookEvent {
    pub data: WebhookData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookData {
    pub event_type: String,
    pub payload: WebhookPayload,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub call_control_id: Option<String>,
    pub call_leg_id: Option<String>,
    pub call_session_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub state: Option<String>,
    pub client_state: Option<String>,
    pub recording_url: Option<String>,
    pub result: Option<String>,
}

impl TelnyxWebhookEvent {
    pub fn event_type(&self) -> &str {
        &self.data.event_type
    }

    pub fn call_control_id(&self) -> Option<&str> {
        self.data.payload.call_control_id.as_deref()
    }
}
