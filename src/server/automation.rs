//! Campaign automation system
//!
//! This module handles automated dialing for campaigns, including:
//! - Background task management for active campaigns
//! - Lead selection and pacing
//! - Retry logic with configurable delays
//! - Time window enforcement
//! - Retention policy enforcement for call recordings

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use chrono::{Local, NaiveTime};
use sqlx::PgPool;

use crate::models::{Campaign, CampaignStatus, Lead, AgentStatus};
use super::db;
use super::telnyx::TelnyxClient;
use super::storage::RecordingStorage;

/// Campaign automation state
#[derive(Debug, Clone)]
pub struct CampaignState {
    pub campaign_id: i64,
    pub is_running: bool,
    pub calls_in_progress: i32,
    pub leads_processed: i32,
    pub last_dial_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

/// Campaign automation manager
pub struct AutomationManager {
    db: PgPool,
    telnyx: TelnyxClient,
    caller_id: String,
    webhook_url: String,
    campaigns: Arc<RwLock<HashMap<i64, CampaignState>>>,
    shutdown: Arc<RwLock<bool>>,
}

impl AutomationManager {
    /// Create a new automation manager
    pub fn new(db: PgPool, telnyx: TelnyxClient, caller_id: String, webhook_url: String) -> Self {
        Self {
            db,
            telnyx,
            caller_id,
            webhook_url,
            campaigns: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Start automation for a campaign
    pub async fn start_campaign(&self, campaign_id: i64) -> Result<(), AutomationError> {
        // Get campaign from database
        let campaign = db::campaigns::get_by_id(&self.db, campaign_id)
            .await
            .map_err(|e| AutomationError::DatabaseError(e.to_string()))?
            .ok_or(AutomationError::CampaignNotFound(campaign_id))?;

        if campaign.status != CampaignStatus::Active {
            return Err(AutomationError::InvalidState(
                "Campaign must be active to start automation".to_string(),
            ));
        }

        // Check if already running
        {
            let campaigns = self.campaigns.read().await;
            if let Some(state) = campaigns.get(&campaign_id) {
                if state.is_running {
                    return Err(AutomationError::AlreadyRunning(campaign_id));
                }
            }
        }

        // Initialize campaign state
        let state = CampaignState {
            campaign_id,
            is_running: true,
            calls_in_progress: 0,
            leads_processed: 0,
            last_dial_at: None,
            error_message: None,
        };

        {
            let mut campaigns = self.campaigns.write().await;
            campaigns.insert(campaign_id, state);
        }

        // Start the dialing loop in a background task
        let db = self.db.clone();
        let telnyx = self.telnyx.clone();
        let caller_id = self.caller_id.clone();
        let webhook_url = self.webhook_url.clone();
        let campaigns = self.campaigns.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            Self::run_campaign_loop(
                campaign_id,
                campaign,
                db,
                telnyx,
                caller_id,
                webhook_url,
                campaigns,
                shutdown,
            )
            .await;
        });

        tracing::info!("Started automation for campaign {}", campaign_id);
        Ok(())
    }

    /// Stop automation for a campaign
    pub async fn stop_campaign(&self, campaign_id: i64) -> Result<(), AutomationError> {
        let mut campaigns = self.campaigns.write().await;
        if let Some(state) = campaigns.get_mut(&campaign_id) {
            state.is_running = false;
            tracing::info!("Stopped automation for campaign {}", campaign_id);
            Ok(())
        } else {
            Err(AutomationError::CampaignNotFound(campaign_id))
        }
    }

    /// Get the status of a campaign
    pub async fn get_status(&self, campaign_id: i64) -> Option<CampaignState> {
        let campaigns = self.campaigns.read().await;
        campaigns.get(&campaign_id).cloned()
    }

    /// Shutdown all campaigns
    pub async fn shutdown(&self) {
        *self.shutdown.write().await = true;
        let mut campaigns = self.campaigns.write().await;
        for state in campaigns.values_mut() {
            state.is_running = false;
        }
    }

    /// Start the retention policy scheduler
    /// This runs a daily background task to delete expired recordings
    pub async fn start_retention_scheduler<S: RecordingStorage + Send + Sync + 'static>(
        &self,
        storage: Arc<S>,
    ) {
        let db = self.db.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            Self::run_retention_cleanup_loop(db, storage, shutdown).await;
        });

        tracing::info!("Started retention policy scheduler");
    }

    /// Retention cleanup background task
    /// Runs daily to delete recordings past their retention_until date
    async fn run_retention_cleanup_loop<S: RecordingStorage + Send + Sync>(
        db: PgPool,
        storage: Arc<S>,
        shutdown: Arc<RwLock<bool>>,
    ) {
        // Run daily at 2 AM (local time)
        let mut interval = interval(Duration::from_secs(3600)); // Check every hour

        loop {
            interval.tick().await;

            // Check if shutdown requested
            if *shutdown.read().await {
                tracing::info!("Retention scheduler shutting down");
                break;
            }

            // Check if it's the right time to run (2 AM local time)
            let now = Local::now();
            if now.hour() != 2 || now.minute() >= 30 {
                continue; // Only run between 2:00 AM and 2:30 AM
            }

            tracing::info!("Starting retention policy cleanup...");

            match Self::cleanup_expired_recordings(&db, &storage).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!("Deleted {} expired recordings", count);
                    }
                }
                Err(e) => {
                    tracing::error!("Retention cleanup failed: {}", e);
                }
            }
        }
    }

    /// Clean up expired recordings
    /// Returns the number of recordings deleted
    async fn cleanup_expired_recordings<S: RecordingStorage>(
        db: &PgPool,
        storage: &Arc<S>,
    ) -> Result<usize, AutomationError> {
        // Query for expired recordings (past retention_until and not on compliance hold)
        let expired_recordings = sqlx::query!(
            r#"
            SELECT id, file_path
            FROM call_recordings
            WHERE retention_until < NOW()
              AND compliance_hold = false
            ORDER BY retention_until ASC
            LIMIT 1000
            "#
        )
        .fetch_all(db)
        .await
        .map_err(|e| AutomationError::DatabaseError(e.to_string()))?;

        let mut deleted_count = 0;

        for recording in expired_recordings {
            let recording_id = recording.id;
            let file_path = recording.file_path;

            // Delete from storage first
            match storage.delete_recording(&file_path).await {
                Ok(_) => {
                    // Delete from database
                    match db::recordings::delete_recording(db, recording_id).await {
                        Ok(_) => {
                            // Track deletion
                            if let Err(e) = db::recordings::increment_recordings_deleted(db).await {
                                tracing::warn!("Failed to track recording deletion: {}", e);
                            }
                            deleted_count += 1;
                            tracing::debug!("Deleted expired recording {}: {}", recording_id, file_path);
                        }
                        Err(e) => {
                            tracing::error!("Failed to delete recording {} from database: {}", recording_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to delete recording file {}: {}", file_path, e);
                    // Continue with next recording even if one fails
                }
            }
        }

        Ok(deleted_count)
    }

    /// Main campaign dialing loop
    #[allow(clippy::too_many_arguments)]
    async fn run_campaign_loop(
        campaign_id: i64,
        campaign: Campaign,
        db: PgPool,
        telnyx: TelnyxClient,
        caller_id: String,
        webhook_url: String,
        campaigns: Arc<RwLock<HashMap<i64, CampaignState>>>,
        shutdown: Arc<RwLock<bool>>,
    ) {
        let mut ticker = interval(Duration::from_secs(5)); // Check every 5 seconds

        loop {
            ticker.tick().await;

            // Check if shutdown requested
            if *shutdown.read().await {
                break;
            }

            // Check if campaign is still running
            {
                let campaigns_read = campaigns.read().await;
                if let Some(state) = campaigns_read.get(&campaign_id) {
                    if !state.is_running {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Check time window
            if !Self::is_within_time_window(&campaign) {
                tracing::debug!("Campaign {} outside time window, waiting...", campaign_id);
                continue;
            }

            // Get available agents for this campaign
            let ready_agents = match db::agents::get_ready_for_campaign(&db, campaign_id).await {
                Ok(agents) => agents,
                Err(e) => {
                    tracing::error!("Failed to get ready agents: {}", e);
                    continue;
                }
            };

            if ready_agents.is_empty() {
                tracing::debug!("No ready agents for campaign {}", campaign_id);
                continue;
            }

            // Get next lead to dial
            let lead = match Self::get_next_lead(&db, campaign_id, campaign.max_attempts.unwrap_or(3)).await {
                Some(lead) => lead,
                None => {
                    tracing::debug!("No more leads to dial for campaign {}", campaign_id);
                    // Mark campaign as completed if no more leads
                    let _ = db::campaigns::update_status(&db, campaign_id, CampaignStatus::Completed).await;
                    break;
                }
            };

            // Select an agent (round-robin or least busy)
            let agent = &ready_agents[0]; // Simple selection for now

            // Dial the lead
            match Self::dial_lead(&db, &telnyx, &caller_id, &webhook_url, &lead, agent.id, campaign_id).await {
                Ok(call_id) => {
                    tracing::info!("Dialed lead {} (call {})", lead.id, call_id);

                    // Update campaign state
                    let mut campaigns_write = campaigns.write().await;
                    if let Some(state) = campaigns_write.get_mut(&campaign_id) {
                        state.calls_in_progress += 1;
                        state.leads_processed += 1;
                        state.last_dial_at = Some(chrono::Utc::now());
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to dial lead {}: {}", lead.id, e);

                    // Update error state
                    let mut campaigns_write = campaigns.write().await;
                    if let Some(state) = campaigns_write.get_mut(&campaign_id) {
                        state.error_message = Some(e.to_string());
                    }
                }
            }

            // Respect pacing - wait between calls based on dialer mode
            let pace_delay = match campaign.dialer_mode {
                crate::models::DialerMode::Preview => Duration::from_secs(10),
                crate::models::DialerMode::Progressive => Duration::from_secs(5),
                crate::models::DialerMode::Predictive => Duration::from_secs(2),
            };
            tokio::time::sleep(pace_delay).await;
        }

        // Mark campaign as not running
        let mut campaigns_write = campaigns.write().await;
        if let Some(state) = campaigns_write.get_mut(&campaign_id) {
            state.is_running = false;
        }

        tracing::info!("Campaign {} automation loop ended", campaign_id);
    }

    /// Check if current time is within campaign time window
    fn is_within_time_window(campaign: &Campaign) -> bool {
        let now = Local::now().time();

        let start = campaign.start_time.unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        let end = campaign.end_time.unwrap_or(NaiveTime::from_hms_opt(21, 0, 0).unwrap());

        now >= start && now <= end
    }

    /// Get the next lead to dial
    async fn get_next_lead(db: &PgPool, campaign_id: i64, max_attempts: i32) -> Option<Lead> {
        // Get leads that:
        // 1. Belong to this campaign
        // 2. Have status New or Contacted
        // 3. Haven't exceeded max attempts
        // 4. Haven't been called recently (retry delay)
        sqlx::query_as::<_, Lead>(
            r"
            SELECT id, first_name, last_name, phone, email, company,
                   status, notes, campaign_id, assigned_agent_id,
                   call_attempts, last_call_at, created_at, updated_at
            FROM leads
            WHERE campaign_id = $1
              AND status IN ('New', 'Contacted')
              AND call_attempts < $2
              AND (last_call_at IS NULL OR last_call_at < NOW() - INTERVAL '30 minutes')
            ORDER BY call_attempts ASC, created_at ASC
            LIMIT 1
            "
        )
        .bind(campaign_id)
        .bind(max_attempts)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
    }

    /// Dial a lead
    async fn dial_lead(
        db: &PgPool,
        telnyx: &TelnyxClient,
        caller_id: &str,
        webhook_url: &str,
        lead: &Lead,
        agent_id: i64,
        campaign_id: i64,
    ) -> Result<i64, AutomationError> {
        // Create call record
        let call = db::calls::create_for_automation(
            db,
            Some(lead.id),
            Some(agent_id),
            Some(campaign_id),
            caller_id,
            &lead.phone,
        )
        .await
        .map_err(|e| AutomationError::DatabaseError(e.to_string()))?;

        // Update agent status
        let _ = db::agents::update_status(db, agent_id, AgentStatus::OnCall).await;

        // Update lead call attempts
        let _ = sqlx::query(
            "UPDATE leads SET call_attempts = call_attempts + 1, last_call_at = NOW() WHERE id = $1"
        )
        .bind(lead.id)
        .execute(db)
        .await;

        // Dial via Telnyx
        match telnyx.dial(&lead.phone, caller_id, Some(webhook_url)).await {
            Ok(response) => {
                // Update call with control ID
                let _ = db::calls::set_control_id(db, call.id, &response.call_control_id).await;
                Ok(call.id)
            }
            Err(e) => {
                // Mark call as failed
                let _ = db::calls::update_status(db, call.id, crate::models::CallStatus::Failed).await;
                let _ = db::agents::update_status(db, agent_id, AgentStatus::Ready).await;
                Err(AutomationError::DialError(e.to_string()))
            }
        }
    }
}

/// Automation errors
#[derive(Debug, thiserror::Error)]
pub enum AutomationError {
    #[error("Campaign not found: {0}")]
    CampaignNotFound(i64),

    #[error("Campaign {0} is already running")]
    AlreadyRunning(i64),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Dial error: {0}")]
    DialError(String),
}
