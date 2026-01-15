//! Server-side code for VoIP CRM
//!
//! This module contains all backend functionality:
//! - Database access (PostgreSQL via sqlx)
//! - Telnyx API integration (cloud VoIP)
//! - SIP trunk integration (direct VoIP)
//! - Authentication (JWT)
//! - API routes
#![allow(dead_code)]

pub mod db;
pub mod telnyx;
pub mod sip;
pub mod auth;
pub mod claude;
pub mod automation;
pub mod ai_call_handler;
pub mod email;
pub mod storage;
pub mod recordings_api;

use axum::{
    routing::{delete, get, post, put},
    Router,
    extract::State,
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};
use axum::http::Method;
use tower_http::trace::TraceLayer;

use crate::models::*;
use serde::{Deserialize, Serialize};

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub telnyx: telnyx::TelnyxClient,
    pub claude: claude::ClaudeClient,
    pub automation: Arc<automation::AutomationManager>,
    pub ai_handler: Arc<ai_call_handler::AiCallHandler>,
    pub email: email::EmailService,
    pub jwt_secret: String,
    pub caller_id: String,
    pub webhook_url: String,
    pub sip_username: String,
    pub sip_password: String,
    /// Optional SIP User Agent for direct SIP trunk calls
    pub sip_agent: Option<Arc<tokio::sync::RwLock<sip::SipUserAgent>>>,
}

/// Create the Axum router with all API routes
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/api/health", get(health_check))

        // Auth routes
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/verify-email", post(auth::verify_email))
        .route("/api/auth/resend-verification", post(auth::resend_verification))
        .route("/api/auth/invite", post(auth::invite_user))
        .route("/api/auth/invitation-details", post(auth::get_invitation_details))
        .route("/api/auth/register-invitation", post(auth::register_invitation))

        // Lead routes
        .route("/api/leads", get(get_leads).post(create_lead))
        .route("/api/leads/my", get(get_my_leads))
        .route("/api/leads/{id}", get(get_lead).put(update_lead).delete(delete_lead))
        .route("/api/leads/{id}/notes", post(add_lead_note))
        .route("/api/leads/{id}/status", put(update_lead_status))
        .route("/api/leads/{id}/assign", put(assign_lead))

        // Agent routes
        .route("/api/agents", get(get_agents).post(create_agent))
        .route("/api/agents/{id}", get(get_agent).put(update_agent))
        .route("/api/agents/{id}/status", put(update_agent_status))

        // Campaign routes
        .route("/api/campaigns", get(get_campaigns).post(create_campaign))
        .route("/api/campaigns/{id}", get(get_campaign).put(update_campaign))
        .route("/api/campaigns/{id}/start", post(start_campaign))
        .route("/api/campaigns/{id}/pause", post(pause_campaign))
        .route("/api/campaigns/{id}/stop", post(stop_campaign))

        // Call routes (Telnyx integration)
        .route("/api/calls/dial", post(dial_call))
        .route("/api/calls/direct", post(direct_dial))
        .route("/api/calls/{id}/hangup", post(hangup_call))
        .route("/api/calls/{id}/transfer", post(transfer_call))
        .route("/api/calls/{id}/hold", post(hold_call))
        .route("/api/calls/{id}/unhold", post(unhold_call))
        .route("/api/calls/{id}", get(get_call))

        // Telnyx webhooks
        .route("/api/webhooks/telnyx", post(handle_telnyx_webhook))

        // Statistics
        .route("/api/stats/realtime", get(get_realtime_stats))
        .route("/api/statistics/realtime", get(get_realtime_stats))
        .route("/api/stats/agent/{id}", get(get_agent_stats))

        // WebRTC config
        .route("/api/config/webrtc", get(get_webrtc_config))

        // SIP trunk routes
        .route("/api/sip/status", get(get_sip_status))
        .route("/api/sip/dial", post(sip_dial))
        .route("/api/sip/hangup", post(sip_hangup))

        // AI Settings routes
        .route("/api/ai/settings", get(get_all_ai_settings))
        .route("/api/ai/settings/{agent_id}", get(get_ai_settings).put(upsert_ai_settings).delete(delete_ai_settings))
        .route("/api/ai/config", get(get_global_ai_config).put(update_global_ai_config))
        .route("/api/ai/templates", get(get_prompt_templates).post(create_prompt_template))
        .route("/api/ai/templates/{id}", get(get_prompt_template).put(update_prompt_template).delete(delete_prompt_template))

        // Campaign automation routes
        .route("/api/campaigns/{id}/automation/start", post(start_campaign_automation))
        .route("/api/campaigns/{id}/automation/stop", post(stop_campaign_automation))
        .route("/api/campaigns/{id}/automation/status", get(get_automation_status))

        // Recording routes
        .route("/api/recordings", get(recordings_api::search_recordings).post(recordings_api::upload_recording))
        .route("/api/recordings/{id}", get(recordings_api::get_recording_details).delete(recordings_api::delete_recording_handler))
        .route("/api/recordings/{id}/compliance-hold", put(recordings_api::update_compliance_hold))
        .route("/api/recordings/{id}/download", get(recordings_api::download_recording))
        .route("/api/recordings/{id}/stream", get(recordings_api::stream_recording))
        .route("/api/recordings/storage/stats", get(recordings_api::get_storage_stats))

        // Retention policy routes
        .route("/api/retention-policies", get(recordings_api::get_retention_policies).post(recordings_api::create_retention_policy))
        .route("/api/retention-policies/{id}", get(recordings_api::get_retention_policy).put(recordings_api::update_retention_policy).delete(recordings_api::delete_retention_policy_handler))

        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

// Health check
async fn health_check() -> &'static str {
    "OK"
}

// WebRTC config
#[derive(serde::Serialize)]
struct WebRTCConfig {
    sip_username: String,
    sip_password: String,
    caller_id: String,
}

async fn get_webrtc_config(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Json<WebRTCConfig> {
    Json(WebRTCConfig {
        sip_username: state.sip_username.clone(),
        sip_password: state.sip_password.clone(),
        caller_id: state.caller_id.clone(),
    })
}

// ============== SIP Trunk Routes ==============

#[derive(Debug, Serialize)]
struct SipStatusResponse {
    status: String,
    registered: bool,
    trunk_host: Option<String>,
    caller_id: Option<String>,
}

async fn get_sip_status(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Json<SipStatusResponse> {
    if let Some(ref sip_agent) = state.sip_agent {
        let agent = sip_agent.read().await;
        let agent_state = agent.state().await;
        let config = agent.config();

        let status = match agent_state {
            sip::AgentState::Disconnected => "disconnected",
            sip::AgentState::Connecting => "connecting",
            sip::AgentState::Registering => "registering",
            sip::AgentState::Registered => "registered",
            sip::AgentState::Failed => "failed",
        };

        Json(SipStatusResponse {
            status: status.to_string(),
            registered: agent_state == sip::AgentState::Registered,
            trunk_host: Some(config.trunk_host.clone()),
            caller_id: Some(config.caller_id.clone()),
        })
    } else {
        Json(SipStatusResponse {
            status: "not_configured".to_string(),
            registered: false,
            trunk_host: None,
            caller_id: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct SipDialRequest {
    phone_number: String,
}

#[derive(Debug, Serialize)]
struct SipDialResponse {
    success: bool,
    call_id: Option<String>,
    error: Option<String>,
}

async fn sip_dial(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<SipDialRequest>,
) -> Json<SipDialResponse> {
    if let Some(ref sip_agent) = state.sip_agent {
        let agent = sip_agent.read().await;

        // Check if registered
        if !agent.is_registered().await {
            return Json(SipDialResponse {
                success: false,
                call_id: None,
                error: Some("SIP not registered".to_string()),
            });
        }

        // Format phone number (ensure E.164 format)
        let phone = if req.phone_number.starts_with('+') {
            req.phone_number.clone()
        } else if req.phone_number.len() == 10 {
            format!("+1{}", req.phone_number)
        } else {
            req.phone_number.clone()
        };

        match agent.dial(&phone).await {
            Ok(call_id) => {
                tracing::info!("SIP call initiated: {} -> {}", call_id, phone);
                Json(SipDialResponse {
                    success: true,
                    call_id: Some(call_id),
                    error: None,
                })
            }
            Err(e) => {
                tracing::error!("SIP dial error: {:?}", e);
                Json(SipDialResponse {
                    success: false,
                    call_id: None,
                    error: Some(e.to_string()),
                })
            }
        }
    } else {
        Json(SipDialResponse {
            success: false,
            call_id: None,
            error: Some("SIP trunk not configured".to_string()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SipHangupRequest {
    call_id: String,
}

#[derive(Debug, Serialize)]
struct SipHangupResponse {
    success: bool,
    error: Option<String>,
}

async fn sip_hangup(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<SipHangupRequest>,
) -> Json<SipHangupResponse> {
    if let Some(ref sip_agent) = state.sip_agent {
        let agent = sip_agent.read().await;

        match agent.hangup(&req.call_id).await {
            Ok(()) => {
                tracing::info!("SIP call hung up: {}", req.call_id);
                Json(SipHangupResponse {
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                tracing::error!("SIP hangup error: {:?}", e);
                Json(SipHangupResponse {
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    } else {
        Json(SipHangupResponse {
            success: false,
            error: Some("SIP trunk not configured".to_string()),
        })
    }
}

// ============== Lead Routes ==============

async fn get_leads(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<Vec<Lead>>, StatusCode> {
    db::leads::get_all(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_my_leads(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<Vec<Lead>>, StatusCode> {
    // Get the agent for this user
    let agent = db::agents::get_by_user(&state.db, claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match agent {
        Some(a) => {
            // Return leads assigned to this agent
            db::leads::get_by_agent(&state.db, a.id)
                .await
                .map(Json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        None => {
            // User has no agent - return empty list
            Ok(Json(vec![]))
        }
    }
}

async fn get_lead(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Lead>, StatusCode> {
    db::leads::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_lead(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<Lead>, StatusCode> {
    db::leads::create(&state.db, req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_lead(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<Lead>, StatusCode> {
    db::leads::update(&state.db, id, req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_lead(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    db::leads::delete(&state.db, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn add_lead_note(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<AddNoteRequest>,
) -> Result<Json<Lead>, StatusCode> {
    db::leads::add_note(&state.db, id, &req.content)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_lead_status(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<Lead>, StatusCode> {
    db::leads::update_status(&state.db, id, req.status)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug, Deserialize)]
struct AssignLeadRequest {
    #[serde(rename = "agentId")]
    agent_id: i64,
}

async fn assign_lead(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<AssignLeadRequest>,
) -> Result<Json<Lead>, StatusCode> {
    db::leads::assign(&state.db, id, req.agent_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ============== Agent Routes ==============

async fn get_agents(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<Vec<Agent>>, StatusCode> {
    db::agents::get_all(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Agent>, StatusCode> {
    db::agents::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_agent(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Agent>, StatusCode> {
    db::agents::create(&state.db, req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_agent(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Agent>, StatusCode> {
    db::agents::update(&state.db, id, req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_agent_status(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdateAgentStatusRequest>,
) -> Result<Json<Agent>, StatusCode> {
    db::agents::update_status(&state.db, id, req.status)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ============== Campaign Routes ==============

async fn get_campaigns(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<Vec<Campaign>>, StatusCode> {
    db::campaigns::get_all(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_campaign(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Campaign>, StatusCode> {
    db::campaigns::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_campaign(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<CreateCampaignRequest>,
) -> Result<Json<Campaign>, StatusCode> {
    db::campaigns::create(&state.db, req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_campaign(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<CreateCampaignRequest>,
) -> Result<Json<Campaign>, StatusCode> {
    db::campaigns::update(&state.db, id, req)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn start_campaign(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Campaign>, StatusCode> {
    db::campaigns::update_status(&state.db, id, CampaignStatus::Active)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn pause_campaign(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Campaign>, StatusCode> {
    db::campaigns::update_status(&state.db, id, CampaignStatus::Paused)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn stop_campaign(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Campaign>, StatusCode> {
    db::campaigns::update_status(&state.db, id, CampaignStatus::Completed)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ============== Call Routes ==============

async fn dial_call(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<DialRequest>,
) -> Result<Json<DialResponse>, StatusCode> {
    // Get lead phone number
    let lead = db::leads::get_by_id(&state.db, req.lead_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Initiate call via Telnyx
    let dial_result = state.telnyx.dial(
        &lead.phone,
        &state.caller_id,
        Some(&state.webhook_url),
    )
        .await
        .map_err(|e| {
            tracing::error!("Telnyx dial error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create call record
    let call = db::calls::create(
        &state.db,
        req.lead_id,
        req.agent_id,
        &dial_result.call_control_id,
        &state.caller_id,
        &lead.phone,
    )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update agent status to OnCall
    let _ = db::agents::update_status(&state.db, req.agent_id, AgentStatus::OnCall).await;

    Ok(Json(DialResponse {
        call_id: call.id,
        call_control_id: dial_result.call_control_id,
        status: "initiated".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct DirectDialRequest {
    #[serde(rename = "phoneNumber")]
    phone_number: String,
    #[serde(rename = "agentId")]
    agent_id: Option<i64>,
}

/// Direct dial a phone number without a lead
async fn direct_dial(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(req): Json<DirectDialRequest>,
) -> Result<Json<DialResponse>, StatusCode> {
    // Initiate call via Telnyx
    let dial_result = state.telnyx.dial(
        &req.phone_number,
        &state.caller_id,
        Some(&state.webhook_url),
    )
        .await
        .map_err(|e| {
            tracing::error!("Telnyx dial error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create call record without lead
    let call = db::calls::create_direct(
        &state.db,
        req.agent_id,
        &dial_result.call_control_id,
        &state.caller_id,
        &req.phone_number,
    )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update agent status to OnCall if agent provided
    if let Some(agent_id) = req.agent_id {
        let _ = db::agents::update_status(&state.db, agent_id, AgentStatus::OnCall).await;
    }

    Ok(Json(DialResponse {
        call_id: call.id,
        call_control_id: dial_result.call_control_id,
        status: "initiated".to_string(),
    }))
}

async fn hangup_call(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let call = db::calls::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(call_control_id) = &call.call_control_id {
        state.telnyx.hangup(call_control_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    db::calls::set_ended(&state.db, id, Some("hangup"))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update agent status back to Ready
    if let Some(agent_id) = call.agent_id {
        let _ = db::agents::update_status(&state.db, agent_id, AgentStatus::AfterCall).await;
    }

    Ok(StatusCode::OK)
}

async fn transfer_call(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<TransferRequest>,
) -> Result<StatusCode, StatusCode> {
    let call = db::calls::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let (Some(call_control_id), Some(target)) = (&call.call_control_id, &req.target_number) {
        state.telnyx.transfer(call_control_id, target)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

async fn hold_call(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let call = db::calls::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(call_control_id) = &call.call_control_id {
        state.telnyx.hold(call_control_id, None)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

async fn unhold_call(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let call = db::calls::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(call_control_id) = &call.call_control_id {
        state.telnyx.unhold(call_control_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

async fn get_call(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Call>, StatusCode> {
    db::calls::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// ============== Webhook Handler ==============

async fn handle_telnyx_webhook(
    State(state): State<Arc<AppState>>,
    Json(event): Json<telnyx::TelnyxWebhookEvent>,
) -> StatusCode {
    tracing::info!("Received Telnyx webhook: {}", event.event_type());

    let call_control_id = match event.call_control_id() {
        Some(id) => id.to_string(),
        None => return StatusCode::OK,
    };

    // Find call by control ID
    let call = match db::calls::get_by_control_id(&state.db, &call_control_id).await {
        Ok(Some(c)) => c,
        _ => return StatusCode::OK,
    };

    // Handle different event types
    match event.event_type() {
        "call.initiated" => {
            let _ = db::calls::update_status(&state.db, call.id, CallStatus::Initiated).await;
        }
        "call.ringing" => {
            let _ = db::calls::update_status(&state.db, call.id, CallStatus::Ringing).await;
        }
        "call.answered" => {
            let _ = db::calls::set_answered(&state.db, call.id).await;

            // Check if this is an AI agent call
            if let Some(agent_id) = call.agent_id {
                if state.ai_handler.is_ai_agent(agent_id).await {
                    // Start AI session
                    if let Err(e) = state.ai_handler.start_session(
                        call.id,
                        &call_control_id,
                        agent_id,
                        call.lead_id,
                        call.campaign_id,
                    ).await {
                        tracing::error!("Failed to start AI session: {}", e);
                        // Fall back to default greeting
                        let _ = state.telnyx.speak(
                            &call_control_id,
                            "Hello, please hold while we connect you.",
                            Some("female")
                        ).await;
                    }
                } else {
                    // Non-AI call - play standard greeting
                    let _ = state.telnyx.speak(
                        &call_control_id,
                        "Hello, this is a call from the VoIP CRM system. Please hold while we connect you.",
                        Some("female")
                    ).await;
                }
            } else {
                // No agent assigned - play default greeting
                let _ = state.telnyx.speak(
                    &call_control_id,
                    "Hello, please hold while we connect you to an agent.",
                    Some("female")
                ).await;
            }
        }
        "call.bridged" => {
            let _ = db::calls::update_status(&state.db, call.id, CallStatus::Bridged).await;
        }
        "call.hangup" => {
            // End AI session if active
            let _ = state.ai_handler.end_session(&call_control_id).await;

            let _ = db::calls::set_ended(&state.db, call.id, Some("hangup")).await;
            if let Some(agent_id) = call.agent_id {
                let _ = db::agents::update_status(&state.db, agent_id, AgentStatus::AfterCall).await;
            }
        }
        "call.machine.detection.ended" => {
            // Check if answering machine and hang up if so
            if let Some(result) = &event.data.payload.result {
                if result == "machine" {
                    let _ = state.telnyx.hangup(&call_control_id).await;
                    let _ = db::calls::set_ended(&state.db, call.id, Some("voicemail")).await;
                }
            }
        }
        _ => {}
    }

    StatusCode::OK
}

// ============== Stats Routes ==============

async fn get_realtime_stats(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let stats = db::stats::get_realtime(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(stats))
}

async fn get_agent_stats(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<AgentStats>, StatusCode> {
    db::stats::get_agent_stats(&state.db, id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ============== AI Settings Routes ==============

async fn get_all_ai_settings(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<Vec<AiAgentSettings>>, StatusCode> {
    db::ai::get_all_settings(&state.db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_ai_settings(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(agent_id): axum::extract::Path<i64>,
) -> Result<Json<Option<AiAgentSettings>>, StatusCode> {
    db::ai::get_settings(&state.db, agent_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn upsert_ai_settings(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(agent_id): axum::extract::Path<i64>,
    Json(req): Json<UpsertAiSettingsRequest>,
) -> Result<Json<AiAgentSettings>, StatusCode> {
    db::ai::upsert_settings(
        &state.db,
        agent_id,
        &req.system_prompt,
        req.greeting_message.as_deref(),
        req.voice_id.as_deref(),
        req.language.as_deref().unwrap_or("en-US"),
        req.max_response_tokens,
        req.temperature,
    )
    .await
    .map(Json)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_ai_settings(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(agent_id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    db::ai::delete_settings(&state.db, agent_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_global_ai_config(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<GlobalAiConfig>, StatusCode> {
    db::ai::get_global_config(&state.db)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get global AI config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn update_global_ai_config(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(config): Json<GlobalAiConfig>,
) -> Result<Json<GlobalAiConfig>, StatusCode> {
    db::ai::update_global_config(&state.db, &config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update global AI config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(config))
}

async fn get_prompt_templates(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
) -> Result<Json<Vec<PromptTemplate>>, StatusCode> {
    db::ai::get_all_templates(&state.db)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get prompt templates: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn create_prompt_template(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    Json(template): Json<PromptTemplate>,
) -> Result<Json<PromptTemplate>, StatusCode> {
    db::ai::create_template(&state.db, &template)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to create prompt template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn get_prompt_template(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<PromptTemplate>, StatusCode> {
    db::ai::get_template(&state.db, &id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get prompt template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn update_prompt_template(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(template): Json<PromptTemplate>,
) -> Result<Json<PromptTemplate>, StatusCode> {
    db::ai::update_template(&state.db, &id, &template)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update prompt template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn delete_prompt_template(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, StatusCode> {
    db::ai::delete_template(&state.db, &id)
        .await
        .map(|deleted| if deleted { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND })
        .map_err(|e| {
            tracing::error!("Failed to delete prompt template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ============== Campaign Automation Routes ==============

#[derive(serde::Serialize)]
struct AutomationStatus {
    is_running: bool,
    calls_in_progress: i32,
    leads_processed: i32,
    last_dial_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn start_campaign_automation(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(campaign_id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Start the campaign automation
    state.automation.start_campaign(campaign_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to start automation for campaign {}: {}", campaign_id, e);
            match e {
                automation::AutomationError::CampaignNotFound(_) => StatusCode::NOT_FOUND,
                automation::AutomationError::AlreadyRunning(_) => StatusCode::CONFLICT,
                automation::AutomationError::InvalidState(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    tracing::info!("Started automation for campaign {}", campaign_id);
    Ok(StatusCode::OK)
}

async fn stop_campaign_automation(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(campaign_id): axum::extract::Path<i64>,
) -> Result<StatusCode, StatusCode> {
    state.automation.stop_campaign(campaign_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to stop automation for campaign {}: {}", campaign_id, e);
            StatusCode::NOT_FOUND
        })?;

    tracing::info!("Stopped automation for campaign {}", campaign_id);
    Ok(StatusCode::OK)
}

async fn get_automation_status(
    State(state): State<Arc<AppState>>,
    claims: auth::Claims,
    axum::extract::Path(campaign_id): axum::extract::Path<i64>,
) -> Json<AutomationStatus> {
    if let Some(status) = state.automation.get_status(campaign_id).await {
        Json(AutomationStatus {
            is_running: status.is_running,
            calls_in_progress: status.calls_in_progress,
            leads_processed: status.leads_processed,
            last_dial_at: status.last_dial_at,
        })
    } else {
        Json(AutomationStatus {
            is_running: false,
            calls_in_progress: 0,
            leads_processed: 0,
            last_dial_at: None,
        })
    }
}

/// Initialize and start the server
pub async fn run_server(database_url: &str, port: u16) -> anyhow::Result<()> {
    // Initialize database
    let pool = db::init_pool(database_url).await?;

    // Run migrations (non-fatal if already applied)
    if let Err(e) = db::run_migrations(&pool).await {
        tracing::warn!("Migration warning (may be already applied): {}", e);
    }

    // Get config from environment
    let telnyx_api_key = std::env::var("TELNYX_API_KEY").unwrap_or_default();
    let telnyx_connection_id = std::env::var("TELNYX_CONNECTION_ID").unwrap_or_default();
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());
    let caller_id = std::env::var("TELNYX_CALLER_ID").unwrap_or_default();
    let webhook_url = std::env::var("WEBHOOK_URL").unwrap_or_default();
    let sip_username = std::env::var("TELNYX_SIP_USERNAME").unwrap_or_default();
    let sip_password = std::env::var("TELNYX_SIP_PASSWORD").unwrap_or_default();
    let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

    let telnyx = telnyx::TelnyxClient::new(telnyx_api_key, telnyx_connection_id);
    let claude = claude::ClaudeClient::new(anthropic_api_key);
    let automation_manager = automation::AutomationManager::new(
        pool.clone(),
        telnyx.clone(),
        caller_id.clone(),
        webhook_url.clone(),
    );
    let ai_handler = ai_call_handler::AiCallHandler::new(
        pool.clone(),
        claude.clone(),
        telnyx.clone(),
    );

    // Initialize email service
    let email = email::EmailService::from_env()
        .unwrap_or_else(|e| {
            tracing::warn!("Email service not configured: {}. Email features will be disabled.", e);
            // Return a dummy service that will fail gracefully
            email::EmailService::new(
                "localhost",
                587,
                "noreply",
                "password",
                "noreply@localhost",
                "VoIP CRM",
                "http://localhost:3000",
            ).expect("Failed to create fallback email service")
        });

    // Optionally initialize SIP User Agent for direct trunk calls
    let sip_agent = if let Some(sip_config) = sip::SipConfig::from_env() {
        tracing::info!("SIP trunk configured: {}:{}", sip_config.trunk_host, sip_config.trunk_port);
        let (agent, _event_rx) = sip::SipUserAgent::new(sip_config);
        // Register with SIP trunk in background
        let agent = Arc::new(tokio::sync::RwLock::new(agent));
        let agent_clone = agent.clone();
        tokio::spawn(async move {
            if let Err(e) = agent_clone.read().await.register().await {
                tracing::error!("SIP registration failed: {}", e);
            }
        });
        Some(agent)
    } else {
        tracing::info!("SIP trunk not configured, using Telnyx only");
        None
    };

    let state = AppState {
        db: pool,
        telnyx,
        claude,
        automation: Arc::new(automation_manager),
        ai_handler: Arc::new(ai_handler),
        email,
        jwt_secret,
        caller_id,
        webhook_url,
        sip_username,
        sip_password,
        sip_agent,
    };

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Server running on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}
