//! Claude API client for AI-powered call handling
//!
//! This module provides integration with the Anthropic Claude API
//! for generating AI agent responses during calls.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Claude API client
#[derive(Clone)]
pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: Arc<RwLock<String>>,
}

#[derive(Debug, Serialize)]
struct ClaudeApiRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeApiResponse {
    id: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: i32,
    output_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct ClaudeError {
    error: ClaudeErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ClaudeErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

/// Response from the Claude API
#[derive(Debug, Clone)]
pub struct ClaudeResponse {
    pub text: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub stop_reason: Option<String>,
}

impl ClaudeClient {
    /// Create a new Claude client
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: Arc::new(RwLock::new("claude-sonnet-4-5-20250514".to_string())),
        }
    }

    /// Set the model to use
    pub async fn set_model(&self, model: String) {
        *self.model.write().await = model;
    }

    /// Send a message to Claude and get a response
    pub async fn send_message(
        &self,
        system_prompt: Option<&str>,
        messages: Vec<Message>,
        max_tokens: i32,
        temperature: Option<f64>,
    ) -> Result<ClaudeResponse, ClaudeApiError> {
        let model = self.model.read().await.clone();

        let request = ClaudeApiRequest {
            model,
            max_tokens,
            messages,
            system: system_prompt.map(|s| s.to_string()),
            temperature,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ClaudeApiError::NetworkError(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ClaudeApiError::NetworkError(e.to_string()))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<ClaudeError>(&body) {
                return Err(ClaudeApiError::ApiError {
                    status: status.as_u16(),
                    message: error.error.message,
                    error_type: error.error.error_type,
                });
            }
            return Err(ClaudeApiError::ApiError {
                status: status.as_u16(),
                message: body,
                error_type: "unknown".to_string(),
            });
        }

        let api_response: ClaudeApiResponse = serde_json::from_str(&body)
            .map_err(|e| ClaudeApiError::ParseError(e.to_string()))?;

        // Extract text from content blocks
        let text = api_response
            .content
            .iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(ClaudeResponse {
            text,
            input_tokens: api_response.usage.input_tokens,
            output_tokens: api_response.usage.output_tokens,
            stop_reason: api_response.stop_reason,
        })
    }

    /// Generate a response for a call conversation
    pub async fn generate_call_response(
        &self,
        system_prompt: &str,
        conversation_history: Vec<Message>,
        user_speech: &str,
        max_tokens: i32,
    ) -> Result<String, ClaudeApiError> {
        let mut messages = conversation_history;
        messages.push(Message {
            role: "user".to_string(),
            content: user_speech.to_string(),
        });

        let response = self
            .send_message(Some(system_prompt), messages, max_tokens, Some(0.7))
            .await?;

        Ok(response.text)
    }

    /// Generate a greeting for an AI agent call
    pub async fn generate_greeting(
        &self,
        system_prompt: &str,
        lead_name: Option<&str>,
        campaign_context: Option<&str>,
    ) -> Result<String, ClaudeApiError> {
        let greeting_prompt = format!(
            "Generate a natural, friendly greeting to start a phone call.{}{}",
            lead_name
                .map(|n| format!(" The person's name is {}.", n))
                .unwrap_or_default(),
            campaign_context
                .map(|c| format!(" Context: {}", c))
                .unwrap_or_default()
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: greeting_prompt,
        }];

        let response = self
            .send_message(Some(system_prompt), messages, 100, Some(0.8))
            .await?;

        Ok(response.text)
    }
}

/// Errors that can occur when calling the Claude API
#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum ClaudeApiError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("API error ({status}): {message}")]
    ApiError {
        status: u16,
        message: String,
        error_type: String,
    },

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }
}
