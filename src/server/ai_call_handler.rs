//! AI Call Handler
//!
//! This module manages AI-powered call sessions, including:
//! - Starting AI conversations when calls are answered
//! - Generating AI responses using Claude
//! - Speaking responses via Telnyx TTS
//! - Managing conversation history

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::claude::{ClaudeClient, Message};
use super::telnyx::TelnyxClient;
use super::db;
use crate::models::AiAgentSettings;

/// Active AI call session
#[derive(Debug, Clone)]
pub struct AiCallSession {
    pub call_id: i64,
    pub call_control_id: String,
    pub agent_id: i64,
    pub lead_id: Option<i64>,
    pub campaign_id: Option<i64>,
    pub system_prompt: String,
    pub conversation: Vec<Message>,
    pub started_at: DateTime<Utc>,
    pub voice: String,
    pub max_tokens: i32,
    pub temperature: f64,
}

/// AI Call Handler manages all AI-powered call sessions
pub struct AiCallHandler {
    db: PgPool,
    claude: ClaudeClient,
    telnyx: TelnyxClient,
    sessions: Arc<RwLock<HashMap<String, AiCallSession>>>,
}

impl AiCallHandler {
    /// Create a new AI call handler
    pub fn new(db: PgPool, claude: ClaudeClient, telnyx: TelnyxClient) -> Self {
        Self {
            db,
            claude,
            telnyx,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start an AI session for a call
    pub async fn start_session(
        &self,
        call_id: i64,
        call_control_id: &str,
        agent_id: i64,
        lead_id: Option<i64>,
        campaign_id: Option<i64>,
    ) -> Result<(), AiCallError> {
        // Get AI settings for this agent
        let settings = db::ai::get_settings(&self.db, agent_id)
            .await
            .map_err(|e| AiCallError::DatabaseError(e.to_string()))?
            .ok_or(AiCallError::NoAiSettings(agent_id))?;

        // Get lead info for personalization
        let lead_name = if let Some(lid) = lead_id {
            db::leads::get_by_id(&self.db, lid)
                .await
                .ok()
                .flatten()
                .and_then(|l| {
                    match (&l.first_name, &l.last_name) {
                        (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
                        (Some(first), None) => Some(first.clone()),
                        (None, Some(last)) => Some(last.clone()),
                        (None, None) => None,
                    }
                })
        } else {
            None
        };

        // Build system prompt with context
        let system_prompt = self.build_system_prompt(&settings, lead_name.as_deref());

        // Get voice (clone before consuming)
        let voice = settings.voice_id.clone().unwrap_or_else(|| "female".to_string());

        // Create session
        let session = AiCallSession {
            call_id,
            call_control_id: call_control_id.to_string(),
            agent_id,
            lead_id,
            campaign_id,
            system_prompt: system_prompt.clone(),
            conversation: Vec::new(),
            started_at: Utc::now(),
            voice: voice.clone(),
            max_tokens: settings.max_response_tokens.unwrap_or(150),
            temperature: settings.temperature.unwrap_or(0.7),
        };

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(call_control_id.to_string(), session);
        }

        // Generate and speak greeting
        let greeting = if let Some(custom_greeting) = &settings.greeting_message {
            custom_greeting.clone()
        } else {
            self.claude
                .generate_greeting(&system_prompt, lead_name.as_deref(), None)
                .await
                .map_err(|e| AiCallError::ClaudeError(e.to_string()))?
        };

        // Speak the greeting
        self.telnyx
            .speak(call_control_id, &greeting, Some(&voice))
            .await
            .map_err(|e| AiCallError::TelnyxError(e.to_string()))?;

        // Add greeting to conversation history
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(call_control_id) {
                session.conversation.push(Message {
                    role: "assistant".to_string(),
                    content: greeting,
                });
            }
        }

        tracing::info!("Started AI session for call {} (agent {})", call_id, agent_id);
        Ok(())
    }

    /// Process user speech and generate AI response
    pub async fn process_speech(
        &self,
        call_control_id: &str,
        speech_text: &str,
    ) -> Result<String, AiCallError> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions.get(call_control_id).cloned()
                .ok_or_else(|| AiCallError::SessionNotFound(call_control_id.to_string()))?
        };

        // Generate AI response
        let response = self.claude
            .generate_call_response(
                &session.system_prompt,
                session.conversation.clone(),
                speech_text,
                session.max_tokens,
            )
            .await
            .map_err(|e| AiCallError::ClaudeError(e.to_string()))?;

        // Update conversation history
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(call_control_id) {
                session.conversation.push(Message {
                    role: "user".to_string(),
                    content: speech_text.to_string(),
                });
                session.conversation.push(Message {
                    role: "assistant".to_string(),
                    content: response.clone(),
                });
            }
        }

        // Speak the response
        self.telnyx
            .speak(call_control_id, &response, Some(&session.voice))
            .await
            .map_err(|e| AiCallError::TelnyxError(e.to_string()))?;

        tracing::debug!("AI response for call {}: {}", call_control_id, response);
        Ok(response)
    }

    /// End an AI session
    pub async fn end_session(&self, call_control_id: &str) -> Option<AiCallSession> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.remove(call_control_id);

        if let Some(ref s) = session {
            tracing::info!("Ended AI session for call {} (duration: {}s)",
                s.call_id,
                (Utc::now() - s.started_at).num_seconds()
            );
        }

        session
    }

    /// Check if a call has an active AI session
    pub async fn has_session(&self, call_control_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(call_control_id)
    }

    /// Get session info
    pub async fn get_session(&self, call_control_id: &str) -> Option<AiCallSession> {
        let sessions = self.sessions.read().await;
        sessions.get(call_control_id).cloned()
    }

    /// Build system prompt with context
    fn build_system_prompt(&self, settings: &AiAgentSettings, lead_name: Option<&str>) -> String {
        let mut prompt = settings.system_prompt.clone();

        // Add lead context if available
        if let Some(name) = lead_name {
            prompt.push_str(&format!("\n\nYou are currently speaking with {}.", name));
        }

        // Add conversation guidelines
        prompt.push_str("\n\nGuidelines for this phone call:
- Keep responses concise and natural for voice
- Use conversational language, not formal writing
- Pause naturally between thoughts
- Ask clarifying questions when needed
- Be helpful, friendly, and professional
- If the caller wants to speak to a human, offer to transfer them");

        prompt
    }

    /// Check if an agent has AI enabled
    pub async fn is_ai_agent(&self, agent_id: i64) -> bool {
        matches!(db::ai::get_settings(&self.db, agent_id).await, Ok(Some(_)))
    }
}

/// AI Call Handler errors
#[derive(Debug, thiserror::Error)]
pub enum AiCallError {
    #[error("No AI settings for agent {0}")]
    NoAiSettings(i64),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Claude API error: {0}")]
    ClaudeError(String),

    #[error("Telnyx error: {0}")]
    TelnyxError(String),
}
