use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// AI Agent Settings stored in database
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAgentSettings {
    pub id: i64,
    #[serde(rename = "agentId")]
    pub agent_id: i64,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "greetingMessage")]
    pub greeting_message: Option<String>,
    #[serde(rename = "voiceId")]
    pub voice_id: Option<String>,
    pub language: String,
    #[serde(rename = "maxResponseTokens")]
    pub max_response_tokens: Option<i32>,
    pub temperature: Option<f64>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Global AI configuration (not agent-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAiConfig {
    pub model: String,
    #[serde(rename = "useClaudeCode")]
    pub use_claude_code: bool,
    #[serde(rename = "fallbackToApi")]
    pub fallback_to_api: bool,
    #[serde(rename = "defaultVoice")]
    pub default_voice: String,
    #[serde(rename = "maxCallDuration")]
    pub max_call_duration: i32,
    #[serde(rename = "sttProvider")]
    pub stt_provider: String,
    #[serde(rename = "ttsProvider")]
    pub tts_provider: String,
}

impl Default for GlobalAiConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-5-20250514".to_string(),
            use_claude_code: true,
            fallback_to_api: true,
            default_voice: "alloy".to_string(),
            max_call_duration: 300,
            stt_provider: "deepgram".to_string(),
            tts_provider: "openai".to_string(),
        }
    }
}

/// Request to create/update AI settings for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertAiSettingsRequest {
    #[serde(rename = "agentId")]
    pub agent_id: i64,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "greetingMessage")]
    pub greeting_message: Option<String>,
    #[serde(rename = "voiceId")]
    pub voice_id: Option<String>,
    pub language: Option<String>,
    #[serde(rename = "maxResponseTokens")]
    pub max_response_tokens: Option<i32>,
    pub temperature: Option<f64>,
}

/// Prompt template for AI agents
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
    pub variables: Vec<String>,
}

/// AI conversation message
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// AI call session state
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCallSession {
    #[serde(rename = "callId")]
    pub call_id: i64,
    #[serde(rename = "agentId")]
    pub agent_id: i64,
    #[serde(rename = "leadId")]
    pub lead_id: Option<i64>,
    pub conversation: Vec<ConversationMessage>,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    #[serde(rename = "startedAt")]
    pub started_at: DateTime<Utc>,
}

/// Request for Claude API
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: i32,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

/// Response from Claude API
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: ClaudeUsage,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

/// TTS request
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: String,
    pub model: Option<String>,
}

/// STT response
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttResponse {
    pub text: String,
    pub confidence: Option<f64>,
    #[serde(rename = "isFinal")]
    pub is_final: bool,
}
