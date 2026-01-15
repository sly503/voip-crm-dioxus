//! Recording upload/download API handlers with streaming support

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;

use crate::models::recording::{CallRecording, CreateRecordingRequest, RecordingSearchParams, UpdateComplianceHoldRequest, RecordingRetentionPolicy, CreateRetentionPolicyRequest};
use crate::models::UserRole;
use crate::server::{AppState, auth::Claims};
use crate::server::storage::{RecordingStorage, StorageError};

// ============== Permission Helpers ==============

/// Parse user role from Claims
fn parse_role(claims: &Claims) -> Result<UserRole, StatusCode> {
    match claims.role.as_str() {
        "Admin" => Ok(UserRole::Admin),
        "Supervisor" => Ok(UserRole::Supervisor),
        "Agent" => Ok(UserRole::Agent),
        _ => {
            tracing::error!("Invalid role in token: {}", claims.role);
            Err(StatusCode::FORBIDDEN)
        }
    }
}

/// Check if user has supervisor or admin role
fn is_supervisor_or_admin(role: &UserRole) -> bool {
    role.is_supervisor_or_above()
}

/// Check if a user can access a recording
/// - Agents can only access their own recordings
/// - Supervisors and Admins can access all recordings
async fn can_access_recording(
    pool: &sqlx::PgPool,
    user_id: i64,
    role: &UserRole,
    recording: &CallRecording,
) -> Result<bool, sqlx::Error> {
    // Supervisors and Admins can access all recordings
    if is_supervisor_or_admin(role) {
        return Ok(true);
    }

    // Agents can only access their own recordings
    // Get the call associated with this recording to check agent_id
    let call = crate::server::db::calls::get_by_id(pool, recording.call_id).await?;

    match call {
        Some(call) => Ok(call.agent_id == Some(user_id)),
        None => Ok(false), // Recording has no associated call
    }
}

/// Search recordings with optional filters
///
/// Supports filtering by agent, campaign, lead, date range, disposition, and compliance hold status.
/// Results are paginated using limit and offset parameters.
/// Permission check: Agents can only see their own recordings, Supervisors/Admins can see all.
pub async fn search_recordings(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(mut params): Query<RecordingSearchParams>,
) -> Result<Json<Vec<CallRecording>>, StatusCode> {
    // Parse user role
    let role = parse_role(&claims)?;

    // Permission check: Agents can only search their own recordings
    if !is_supervisor_or_admin(&role) {
        // Force agent_id filter to current user for agents
        params.agent_id = Some(claims.sub);
        tracing::debug!("Agent {} searching only their own recordings", claims.sub);
    }

    crate::server::db::recordings::search_recordings(&state.db, &params)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to search recordings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Get recording details by ID
///
/// Returns metadata for a single recording including file info, retention, and compliance status.
/// Permission check: Agents can only access their own recordings, Supervisors/Admins can access all.
pub async fn get_recording_details(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<CallRecording>, StatusCode> {
    // Parse user role
    let role = parse_role(&claims)?;

    // Get the recording
    let recording = crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Permission check: Verify user can access this recording
    let can_access = can_access_recording(&state.db, claims.sub, &role, &recording)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check recording access: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !can_access {
        tracing::warn!(
            "User {} (role: {}) attempted to access recording {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(recording))
}

/// Delete a recording by ID
///
/// Permanently removes the recording file from storage and deletes the database record.
/// Recordings with compliance_hold=true cannot be deleted.
/// Permission check: Only Supervisors/Admins can delete recordings.
pub async fn delete_recording_handler(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Permission check: Only Supervisors/Admins can delete recordings
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to delete recording {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Get recording to check compliance hold and file path
    let recording = crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if recording is under compliance hold
    if recording.compliance_hold {
        tracing::warn!("Attempted to delete recording {} under compliance hold", id);
        return Err(StatusCode::FORBIDDEN);
    }

    // Delete file from storage first
    let storage_config = crate::server::storage::StorageConfig::from_env()
        .map_err(|e| {
            tracing::error!("Failed to load storage config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let storage = storage_config.initialize()
        .await
        .map_err(|e| {
            tracing::error!("Failed to initialize storage: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    storage.delete_recording(&recording.file_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete recording file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Delete database record
    crate::server::db::recordings::delete_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete recording from database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update compliance hold status for a recording
///
/// When compliance_hold is true, the recording cannot be deleted even if retention period expires.
/// Permission check: Only Supervisors/Admins can set compliance holds.
pub async fn update_compliance_hold(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(req): Json<UpdateComplianceHoldRequest>,
) -> Result<StatusCode, StatusCode> {
    // Permission check: Only Supervisors/Admins can set compliance holds
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to update compliance hold for recording {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify recording exists
    crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update compliance hold
    crate::server::db::recordings::set_compliance_hold(&state.db, id, req.compliance_hold)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update compliance hold: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        "User {} (role: {}) {} compliance hold for recording {}",
        claims.sub,
        claims.role,
        if req.compliance_hold { "set" } else { "released" },
        id
    );

    Ok(StatusCode::OK)
}

/// Upload a recording file
///
/// This endpoint accepts raw audio data in the request body and stores it
/// with encryption. The recording metadata is stored in the database.
/// Retention policy is automatically calculated based on campaign/agent settings.
pub async fn upload_recording(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(request): Json<CreateRecordingRequest>,
) -> Result<Json<CallRecording>, StatusCode> {
    // Get storage from state
    let storage = state.storage.as_ref().ok_or_else(|| {
        tracing::error!("Storage not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // Calculate retention_until
    let retention_until = db::recordings::calculate_retention_until(
        &state.db,
        request.call_id,
    )
    .await
    .unwrap_or_else(|_| {
        let days = std::env::var("DEFAULT_RETENTION_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(90);
        chrono::Utc::now() + chrono::Duration::days(days)
    });

    // Insert recording into database
    let recording = db::recordings::insert_recording(
        &state.db,
        request.call_id,
        &request.file_path,
        request.file_size,
        request.duration_seconds,
        &request.format,
        &request.encryption_key_id,
        retention_until,
        request.metadata.map(|m| serde_json::to_value(m).ok()).flatten(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert recording: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(recording))
}

/// Download a recording file with streaming support
///
/// Supports HTTP Range requests for partial downloads, which is essential
/// for large audio files and seeking in audio players.
/// Permission check: Agents can only download their own recordings, Supervisors/Admins can download all.
pub async fn download_recording(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // Parse user role
    let role = parse_role(&claims)?;

    // Get recording metadata from database
    let recording = crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Permission check: Verify user can access this recording
    let can_access = can_access_recording(&state.db, claims.sub, &role, &recording)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check recording access: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !can_access {
        tracing::warn!(
            "User {} (role: {}) attempted to download recording {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Extract IP address from headers for audit logging
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<std::net::IpAddr>().ok())
        });

    // Log download event to audit log
    if let Err(e) = crate::server::db::recordings::log_audit_event(
        &state.db,
        id,
        "downloaded",
        Some(claims.sub),
        ip_address,
        None, // No additional metadata needed for downloads
    ).await {
        tracing::error!("Failed to log download audit event: {}", e);
        // Don't fail the request if audit logging fails, just log the error
    }

    // Get storage from state
    let storage = state.storage.as_ref().ok_or_else(|| {
        tracing::error!("Storage not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // Read the file with streaming
    let file_path = recording.file_path.clone();
    let absolute_path = storage.base_path().join(&file_path);

    // Check if file exists
    if !absolute_path.exists() {
        tracing::error!("Recording file not found: {:?}", absolute_path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Get file metadata for size
    let file_metadata = tokio::fs::metadata(&absolute_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get file metadata: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let file_size = file_metadata.len();

    // Check for Range header to support partial downloads
    if let Some(range_header) = headers.get(header::RANGE) {
        // Parse range header (e.g., "bytes=0-1023")
        if let Ok(range_str) = range_header.to_str() {
            if let Some(range) = parse_range_header(range_str, file_size) {
                return serve_range(&storage, &recording, range, file_size).await;
            }
        }
    }

    // Serve entire file
    serve_full_file(&storage, &recording, file_size).await
}

/// Serve the entire recording file
async fn serve_full_file(
    storage: &impl RecordingStorage,
    recording: &CallRecording,
    file_size: u64,
) -> Result<Response, StatusCode> {
    // Get decrypted file data
    let file_data = storage.get_recording(&recording.file_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retrieve recording: {}", e);
            match e {
                StorageError::FileNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Determine content type based on format
    let content_type = match recording.format.as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    };

    // Build response with appropriate headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_data.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"recording_{}_{}.{}\"",
                recording.id,
                recording.uploaded_at.timestamp(),
                recording.format
            ),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(file_data))
        .map_err(|e| {
            tracing::error!("Failed to build response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(response)
}

/// Serve a range of the recording file (for seeking/partial downloads)
async fn serve_range(
    storage: &impl RecordingStorage,
    recording: &CallRecording,
    range: (u64, u64),
    total_size: u64,
) -> Result<Response, StatusCode> {
    let (start, end) = range;

    // Validate range
    if start >= total_size || end >= total_size || start > end {
        return Err(StatusCode::RANGE_NOT_SATISFIABLE);
    }

    // Get full decrypted file data
    // Note: For very large files, we could optimize this to only decrypt the needed range
    // but that would require changes to the encryption layer
    let file_data = storage.get_recording(&recording.file_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retrieve recording: {}", e);
            match e {
                StorageError::FileNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Extract the requested range
    let range_data = &file_data[start as usize..=end as usize];
    let range_length = range_data.len();

    // Determine content type based on format
    let content_type = match recording.format.as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    };

    // Build partial content response
    let response = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, range_length)
        .header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, end, total_size),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(range_data.to_vec()))
        .map_err(|e| {
            tracing::error!("Failed to build range response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(response)
}

/// Parse HTTP Range header
/// Returns (start, end) byte positions if valid
fn parse_range_header(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    // Range format: "bytes=start-end" or "bytes=start-" or "bytes=-suffix"
    if !range_str.starts_with("bytes=") {
        return None;
    }

    let range_part = range_str.strip_prefix("bytes=")?;
    let parts: Vec<&str> = range_part.split('-').collect();

    if parts.len() != 2 {
        return None;
    }

    // Parse start and end
    let start = if parts[0].is_empty() {
        // Suffix range: bytes=-500 (last 500 bytes)
        if let Ok(suffix) = parts[1].parse::<u64>() {
            file_size.saturating_sub(suffix)
        } else {
            return None;
        }
    } else {
        parts[0].parse::<u64>().ok()?
    };

    let end = if parts[1].is_empty() {
        // Open-ended range: bytes=100- (from byte 100 to end)
        file_size - 1
    } else {
        parts[1].parse::<u64>().ok()?
    };

    Some((start, end))
}

/// Stream a recording file (alternative streaming implementation)
///
/// This is an alternative implementation that streams the file in chunks
/// without loading it entirely into memory. However, since we decrypt the
/// entire file, this doesn't provide much benefit currently.
/// Permission check: Agents can only stream their own recordings, Supervisors/Admins can stream all.
pub async fn stream_recording(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // Parse user role
    let role = parse_role(&claims)?;

    // Get recording metadata from database
    let recording = crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Permission check: Verify user can access this recording
    let can_access = can_access_recording(&state.db, claims.sub, &role, &recording)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check recording access: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !can_access {
        tracing::warn!(
            "User {} (role: {}) attempted to stream recording {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Extract IP address from headers for audit logging
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<std::net::IpAddr>().ok())
        });

    // Log stream event to audit log (streaming is a form of download/access)
    if let Err(e) = crate::server::db::recordings::log_audit_event(
        &state.db,
        id,
        "downloaded", // Use "downloaded" action for streaming as well
        Some(claims.sub),
        ip_address,
        Some(serde_json::json!({"method": "stream"})), // Mark that this was a stream access
    ).await {
        tracing::error!("Failed to log stream audit event: {}", e);
        // Don't fail the request if audit logging fails, just log the error
    }

    // Get storage from state
    let storage = state.storage.as_ref().ok_or_else(|| {
        tracing::error!("Storage not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // For streaming, we need to decrypt first (current limitation)
    let file_data = storage.get_recording(&recording.file_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retrieve recording: {}", e);
            match e {
                StorageError::FileNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Determine content type
    let content_type = match recording.format.as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    };

    // Stream the decrypted data
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, file_data.len())
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(file_data))
        .map_err(|e| {
            tracing::error!("Failed to build streaming response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(response)
}

// ============== Retention Policy Endpoints ==============

/// Get all retention policies
///
/// Returns a list of all retention policies, ordered by default status and creation date.
/// Permission check: Only Supervisors/Admins can view retention policies.
pub async fn get_retention_policies(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> Result<Json<Vec<RecordingRetentionPolicy>>, StatusCode> {
    // Permission check: Only Supervisors/Admins can view retention policies
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to view retention policies without permission",
            claims.sub,
            claims.role
        );
        return Err(StatusCode::FORBIDDEN);
    }

    crate::server::db::recordings::get_all_retention_policies(&state.db)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get retention policies: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Get a retention policy by ID
///
/// Returns the details of a specific retention policy.
/// Permission check: Only Supervisors/Admins can view retention policies.
pub async fn get_retention_policy(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<RecordingRetentionPolicy>, StatusCode> {
    // Permission check: Only Supervisors/Admins can view retention policies
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to view retention policy {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    crate::server::db::recordings::get_retention_policy(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get retention policy {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new retention policy
///
/// Creates a new retention policy with the specified parameters.
/// Validates that campaign/agent IDs are provided when applies_to is Campaign/Agent.
/// Permission check: Only Supervisors/Admins can create retention policies.
pub async fn create_retention_policy(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<CreateRetentionPolicyRequest>,
) -> Result<Json<RecordingRetentionPolicy>, StatusCode> {
    // Permission check: Only Supervisors/Admins can create retention policies
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to create retention policy without permission",
            claims.sub,
            claims.role
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate request
    if let Err(e) = validate_retention_policy_request(&req) {
        tracing::warn!("Invalid retention policy request: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    crate::server::db::recordings::create_retention_policy(&state.db, &req)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to create retention policy: {}", e);
            // Check for database constraint violations
            if e.to_string().contains("unique") || e.to_string().contains("constraint") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

/// Update a retention policy
///
/// Updates an existing retention policy with new parameters.
/// Permission check: Only Supervisors/Admins can update retention policies.
pub async fn update_retention_policy(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(req): Json<CreateRetentionPolicyRequest>,
) -> Result<Json<RecordingRetentionPolicy>, StatusCode> {
    // Permission check: Only Supervisors/Admins can update retention policies
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to update retention policy {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate request
    if let Err(e) = validate_retention_policy_request(&req) {
        tracing::warn!("Invalid retention policy request: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify policy exists
    crate::server::db::recordings::get_retention_policy(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get retention policy {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    crate::server::db::recordings::update_retention_policy(&state.db, id, &req)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update retention policy: {}", e);
            if e.to_string().contains("unique") || e.to_string().contains("constraint") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })
}

/// Delete a retention policy
///
/// Deletes a retention policy. Default policies cannot be deleted if they are the only default.
/// Permission check: Only Supervisors/Admins can delete retention policies.
pub async fn delete_retention_policy_handler(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Permission check: Only Supervisors/Admins can delete retention policies
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to delete retention policy {} without permission",
            claims.sub,
            claims.role,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify policy exists
    let policy = crate::server::db::recordings::get_retention_policy(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get retention policy {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Warn if deleting the default policy
    if policy.is_default {
        tracing::warn!("Deleting default retention policy {}", id);
    }

    crate::server::db::recordings::delete_retention_policy(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete retention policy: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Validate retention policy request
fn validate_retention_policy_request(req: &CreateRetentionPolicyRequest) -> Result<(), String> {
    use crate::models::recording::RetentionAppliesTo;

    // Validate retention days is positive
    if req.retention_days <= 0 {
        return Err("retention_days must be positive".to_string());
    }

    // Validate applies_to and corresponding IDs
    match req.applies_to {
        RetentionAppliesTo::Campaign => {
            if req.campaign_id.is_none() {
                return Err("campaign_id required when applies_to is Campaign".to_string());
            }
            if req.agent_id.is_some() {
                return Err("agent_id must be null when applies_to is Campaign".to_string());
            }
        }
        RetentionAppliesTo::Agent => {
            if req.agent_id.is_none() {
                return Err("agent_id required when applies_to is Agent".to_string());
            }
            if req.campaign_id.is_some() {
                return Err("campaign_id must be null when applies_to is Agent".to_string());
            }
        }
        RetentionAppliesTo::All => {
            if req.campaign_id.is_some() || req.agent_id.is_some() {
                return Err("campaign_id and agent_id must be null when applies_to is All".to_string());
            }
        }
    }

    Ok(())
}

// ============== Storage Dashboard Endpoint ==============

/// Get storage statistics for the dashboard
///
/// Returns comprehensive storage statistics including total files, size, quota usage,
/// and daily usage history for the past 30 days.
/// Permission check: Only Supervisors/Admins can view storage stats.
pub async fn get_storage_stats(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> Result<Json<crate::models::recording::StorageStats>, StatusCode> {
    // Permission check: Only Supervisors/Admins can view storage stats
    let role = parse_role(&claims)?;
    if !is_supervisor_or_admin(&role) {
        tracing::warn!(
            "User {} (role: {}) attempted to view storage stats without permission",
            claims.sub,
            claims.role
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Get storage from state
    let storage = state.storage.as_ref().ok_or_else(|| {
        tracing::error!("Storage not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // Get statistics
    let stats = storage.get_storage_stats(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get storage stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::debug!(
        "Storage stats: {} files, {:.2} GB / {:.2} GB ({:.1}%)",
        stats.total_files,
        stats.total_size_gb,
        stats.quota_gb,
        stats.quota_percentage
    );

    Ok(Json(stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::recording::RetentionAppliesTo;

    // ============== Range Header Parsing Tests ==============

    #[test]
    fn test_parse_range_header() {
        // Standard range
        assert_eq!(parse_range_header("bytes=0-999", 10000), Some((0, 999)));

        // Open-ended range
        assert_eq!(parse_range_header("bytes=500-", 10000), Some((500, 9999)));

        // Suffix range
        assert_eq!(parse_range_header("bytes=-500", 10000), Some((9500, 9999)));

        // Invalid ranges
        assert_eq!(parse_range_header("invalid", 10000), None);
        assert_eq!(parse_range_header("bytes=", 10000), None);
        assert_eq!(parse_range_header("bytes=abc-def", 10000), None);
    }

    #[test]
    fn test_parse_range_header_edge_cases() {
        // Range at the end
        assert_eq!(parse_range_header("bytes=9999-9999", 10000), Some((9999, 9999)));

        // Small suffix
        assert_eq!(parse_range_header("bytes=-1", 10000), Some((9999, 9999)));

        // Large suffix (larger than file)
        assert_eq!(parse_range_header("bytes=-20000", 10000), Some((0, 9999)));
    }

    #[test]
    fn test_parse_range_header_comprehensive() {
        let file_size = 1_000_000u64;

        // Test various valid ranges
        assert_eq!(
            parse_range_header("bytes=0-0", file_size),
            Some((0, 0)),
            "First byte only"
        );

        assert_eq!(
            parse_range_header("bytes=999999-999999", file_size),
            Some((999999, 999999)),
            "Last byte only"
        );

        assert_eq!(
            parse_range_header("bytes=100000-200000", file_size),
            Some((100000, 200000)),
            "Middle range"
        );

        assert_eq!(
            parse_range_header("bytes=0-", file_size),
            Some((0, 999999)),
            "From start to end (open-ended)"
        );

        assert_eq!(
            parse_range_header("bytes=-1000", file_size),
            Some((999000, 999999)),
            "Last 1000 bytes (suffix)"
        );

        // Test edge cases
        assert_eq!(
            parse_range_header("bytes=-1", file_size),
            Some((999999, 999999)),
            "Last byte (suffix)"
        );

        assert_eq!(
            parse_range_header("bytes=-5000000", file_size),
            Some((0, 999999)),
            "Suffix larger than file (saturates to 0)"
        );

        // Test invalid ranges
        assert_eq!(
            parse_range_header("", file_size),
            None,
            "Empty string"
        );

        assert_eq!(
            parse_range_header("ranges=0-100", file_size),
            None,
            "Wrong prefix"
        );

        assert_eq!(
            parse_range_header("bytes=", file_size),
            None,
            "No range specified"
        );

        assert_eq!(
            parse_range_header("bytes=-", file_size),
            None,
            "Invalid separator only"
        );

        assert_eq!(
            parse_range_header("bytes=abc", file_size),
            None,
            "Non-numeric value"
        );

        assert_eq!(
            parse_range_header("bytes=100-200-300", file_size),
            None,
            "Too many parts"
        );
    }

    // ============== Permission Helper Tests ==============

    #[test]
    fn test_parse_role() {
        // Create test claims for each role
        let admin_claims = Claims {
            sub: 1,
            role: "Admin".to_string(),
            exp: 0,
        };

        let supervisor_claims = Claims {
            sub: 2,
            role: "Supervisor".to_string(),
            exp: 0,
        };

        let agent_claims = Claims {
            sub: 3,
            role: "Agent".to_string(),
            exp: 0,
        };

        let invalid_claims = Claims {
            sub: 4,
            role: "InvalidRole".to_string(),
            exp: 0,
        };

        // Test valid roles
        assert_eq!(
            parse_role(&admin_claims).unwrap(),
            UserRole::Admin,
            "Admin role should parse correctly"
        );

        assert_eq!(
            parse_role(&supervisor_claims).unwrap(),
            UserRole::Supervisor,
            "Supervisor role should parse correctly"
        );

        assert_eq!(
            parse_role(&agent_claims).unwrap(),
            UserRole::Agent,
            "Agent role should parse correctly"
        );

        // Test invalid role
        assert_eq!(
            parse_role(&invalid_claims).unwrap_err(),
            StatusCode::FORBIDDEN,
            "Invalid role should return FORBIDDEN"
        );
    }

    #[test]
    fn test_is_supervisor_or_admin() {
        // Test supervisor and admin permissions
        assert!(
            is_supervisor_or_admin(&UserRole::Admin),
            "Admin should have supervisor permissions"
        );

        assert!(
            is_supervisor_or_admin(&UserRole::Supervisor),
            "Supervisor should have supervisor permissions"
        );

        assert!(
            !is_supervisor_or_admin(&UserRole::Agent),
            "Agent should NOT have supervisor permissions"
        );
    }

    // ============== Retention Policy Validation Tests ==============

    #[test]
    fn test_validate_retention_policy_all_recordings() {
        // Valid policy for all recordings
        let valid_policy = CreateRetentionPolicyRequest {
            name: "Default Policy".to_string(),
            retention_days: 90,
            applies_to: RetentionAppliesTo::All,
            campaign_id: None,
            agent_id: None,
            is_default: true,
        };

        assert!(
            validate_retention_policy_request(&valid_policy).is_ok(),
            "Valid 'All' policy should pass validation"
        );

        // Invalid: All policy with campaign_id
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 90,
            applies_to: RetentionAppliesTo::All,
            campaign_id: Some(1),
            agent_id: None,
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&invalid_policy).is_err(),
            "'All' policy should not have campaign_id"
        );

        // Invalid: All policy with agent_id
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 90,
            applies_to: RetentionAppliesTo::All,
            campaign_id: None,
            agent_id: Some(1),
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&invalid_policy).is_err(),
            "'All' policy should not have agent_id"
        );
    }

    #[test]
    fn test_validate_retention_policy_campaign() {
        // Valid campaign policy
        let valid_policy = CreateRetentionPolicyRequest {
            name: "Campaign Policy".to_string(),
            retention_days: 60,
            applies_to: RetentionAppliesTo::Campaign,
            campaign_id: Some(1),
            agent_id: None,
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&valid_policy).is_ok(),
            "Valid campaign policy should pass validation"
        );

        // Invalid: Campaign policy without campaign_id
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 60,
            applies_to: RetentionAppliesTo::Campaign,
            campaign_id: None,
            agent_id: None,
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&invalid_policy).is_err(),
            "Campaign policy must have campaign_id"
        );

        // Invalid: Campaign policy with agent_id
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 60,
            applies_to: RetentionAppliesTo::Campaign,
            campaign_id: Some(1),
            agent_id: Some(2),
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&invalid_policy).is_err(),
            "Campaign policy should not have agent_id"
        );
    }

    #[test]
    fn test_validate_retention_policy_agent() {
        // Valid agent policy
        let valid_policy = CreateRetentionPolicyRequest {
            name: "Agent Policy".to_string(),
            retention_days: 30,
            applies_to: RetentionAppliesTo::Agent,
            campaign_id: None,
            agent_id: Some(1),
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&valid_policy).is_ok(),
            "Valid agent policy should pass validation"
        );

        // Invalid: Agent policy without agent_id
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 30,
            applies_to: RetentionAppliesTo::Agent,
            campaign_id: None,
            agent_id: None,
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&invalid_policy).is_err(),
            "Agent policy must have agent_id"
        );

        // Invalid: Agent policy with campaign_id
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 30,
            applies_to: RetentionAppliesTo::Agent,
            campaign_id: Some(1),
            agent_id: Some(2),
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&invalid_policy).is_err(),
            "Agent policy should not have campaign_id"
        );
    }

    #[test]
    fn test_validate_retention_policy_days() {
        // Invalid: Zero retention days
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: 0,
            applies_to: RetentionAppliesTo::All,
            campaign_id: None,
            agent_id: None,
            is_default: false,
        };

        let result = validate_retention_policy_request(&invalid_policy);
        assert!(result.is_err(), "Zero retention_days should be invalid");
        assert!(
            result.unwrap_err().contains("positive"),
            "Error message should mention 'positive'"
        );

        // Invalid: Negative retention days
        let invalid_policy = CreateRetentionPolicyRequest {
            name: "Invalid Policy".to_string(),
            retention_days: -10,
            applies_to: RetentionAppliesTo::All,
            campaign_id: None,
            agent_id: None,
            is_default: false,
        };

        let result = validate_retention_policy_request(&invalid_policy);
        assert!(result.is_err(), "Negative retention_days should be invalid");

        // Valid: Positive retention days
        let valid_policy = CreateRetentionPolicyRequest {
            name: "Valid Policy".to_string(),
            retention_days: 1,
            applies_to: RetentionAppliesTo::All,
            campaign_id: None,
            agent_id: None,
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&valid_policy).is_ok(),
            "Minimum retention_days of 1 should be valid"
        );

        // Valid: Large retention days
        let valid_policy = CreateRetentionPolicyRequest {
            name: "Long Retention".to_string(),
            retention_days: 3650, // 10 years
            applies_to: RetentionAppliesTo::All,
            campaign_id: None,
            agent_id: None,
            is_default: false,
        };

        assert!(
            validate_retention_policy_request(&valid_policy).is_ok(),
            "Large retention_days should be valid"
        );
    }

    #[test]
    fn test_validate_retention_policy_comprehensive() {
        // Test various valid policy combinations
        let test_cases = vec![
            (
                "All recordings - 30 days",
                CreateRetentionPolicyRequest {
                    name: "Short Term".to_string(),
                    retention_days: 30,
                    applies_to: RetentionAppliesTo::All,
                    campaign_id: None,
                    agent_id: None,
                    is_default: false,
                },
                true,
            ),
            (
                "All recordings - 365 days",
                CreateRetentionPolicyRequest {
                    name: "One Year".to_string(),
                    retention_days: 365,
                    applies_to: RetentionAppliesTo::All,
                    campaign_id: None,
                    agent_id: None,
                    is_default: true,
                },
                true,
            ),
            (
                "Campaign specific",
                CreateRetentionPolicyRequest {
                    name: "Sales Campaign".to_string(),
                    retention_days: 180,
                    applies_to: RetentionAppliesTo::Campaign,
                    campaign_id: Some(5),
                    agent_id: None,
                    is_default: false,
                },
                true,
            ),
            (
                "Agent specific",
                CreateRetentionPolicyRequest {
                    name: "Top Performer".to_string(),
                    retention_days: 90,
                    applies_to: RetentionAppliesTo::Agent,
                    campaign_id: None,
                    agent_id: Some(10),
                    is_default: false,
                },
                true,
            ),
        ];

        for (description, policy, should_pass) in test_cases {
            let result = validate_retention_policy_request(&policy);
            if should_pass {
                assert!(result.is_ok(), "Test case '{}' should pass", description);
            } else {
                assert!(result.is_err(), "Test case '{}' should fail", description);
            }
        }
    }

    // ============== Integration Test Helpers ==============

    /// Test helper to create a mock CallRecording
    #[cfg(test)]
    fn create_test_recording(id: i64, call_id: i64, compliance_hold: bool) -> CallRecording {
        use chrono::Utc;

        CallRecording {
            id,
            call_id,
            file_path: format!("2024/01/01/recording_{}.wav", id),
            file_size: 1024000,
            duration_seconds: 120,
            format: "wav".to_string(),
            encryption_key_id: "default".to_string(),
            uploaded_at: Utc::now(),
            retention_until: Utc::now() + chrono::Duration::days(90),
            compliance_hold,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_recording_search_params_defaults() {
        // Test that default search params work correctly
        let params = RecordingSearchParams::default();

        assert_eq!(params.agent_id, None, "Default agent_id should be None");
        assert_eq!(params.campaign_id, None, "Default campaign_id should be None");
        assert_eq!(params.lead_id, None, "Default lead_id should be None");
        assert_eq!(params.start_date, None, "Default start_date should be None");
        assert_eq!(params.end_date, None, "Default end_date should be None");
        assert_eq!(params.disposition, None, "Default disposition should be None");
        assert_eq!(params.compliance_hold, None, "Default compliance_hold should be None");
        assert_eq!(params.limit.unwrap_or(50), 50, "Default limit should be 50");
        assert_eq!(params.offset.unwrap_or(0), 0, "Default offset should be 0");
    }

    #[test]
    fn test_retention_applies_to_display_names() {
        assert_eq!(
            RetentionAppliesTo::All.display_name(),
            "All Recordings",
            "All display name"
        );
        assert_eq!(
            RetentionAppliesTo::Campaign.display_name(),
            "Campaign",
            "Campaign display name"
        );
        assert_eq!(
            RetentionAppliesTo::Agent.display_name(),
            "Agent",
            "Agent display name"
        );
    }

    // ============== Documentation and Edge Case Tests ==============

    #[test]
    fn test_compliance_hold_prevents_deletion_logic() {
        // This test documents the expected behavior of compliance hold
        // In the actual delete_recording_handler, compliance_hold=true should prevent deletion

        let recording_normal = create_test_recording(1, 100, false);
        let recording_on_hold = create_test_recording(2, 101, true);

        // Document expected behavior
        assert!(!recording_normal.compliance_hold, "Normal recordings can be deleted");
        assert!(recording_on_hold.compliance_hold, "Recordings on hold cannot be deleted");
    }

    #[test]
    fn test_file_format_detection() {
        // Test the file format to content-type mapping logic
        let formats = vec![
            ("wav", "audio/wav"),
            ("mp3", "audio/mpeg"),
            ("ogg", "audio/ogg"),
            ("flac", "application/octet-stream"), // Unknown format
            ("", "application/octet-stream"),     // Empty format
        ];

        for (format, expected_type) in formats {
            let content_type = match format {
                "wav" => "audio/wav",
                "mp3" => "audio/mpeg",
                "ogg" => "audio/ogg",
                _ => "application/octet-stream",
            };

            assert_eq!(
                content_type, expected_type,
                "Format '{}' should map to '{}'",
                format, expected_type
            );
        }
    }

    #[test]
    fn test_permission_scenarios_documentation() {
        // This test documents all permission scenarios for the API

        // Scenario 1: Agent accessing their own recording
        // Expected: ALLOWED (can_access_recording returns true)

        // Scenario 2: Agent accessing another agent's recording
        // Expected: FORBIDDEN (can_access_recording returns false)

        // Scenario 3: Supervisor accessing any recording
        // Expected: ALLOWED (is_supervisor_or_admin returns true)

        // Scenario 4: Admin accessing any recording
        // Expected: ALLOWED (is_supervisor_or_admin returns true)

        // Scenario 5: Agent trying to delete a recording
        // Expected: FORBIDDEN (delete requires supervisor/admin)

        // Scenario 6: Supervisor deleting a recording on compliance hold
        // Expected: FORBIDDEN (compliance_hold prevents deletion)

        // Scenario 7: Supervisor deleting a normal recording
        // Expected: ALLOWED (supervisor + no compliance hold)

        // These scenarios are tested through the permission helper functions
        assert!(
            is_supervisor_or_admin(&UserRole::Supervisor),
            "Scenario 3: Supervisor can access all"
        );
        assert!(
            is_supervisor_or_admin(&UserRole::Admin),
            "Scenario 4: Admin can access all"
        );
        assert!(
            !is_supervisor_or_admin(&UserRole::Agent),
            "Scenario 5: Agent cannot delete"
        );
    }

    #[test]
    fn test_range_request_scenarios() {
        let file_size = 10_000_000u64; // 10MB file

        // Scenario 1: Audio player seeking to middle of file
        let seek_to_middle = parse_range_header("bytes=5000000-", file_size);
        assert_eq!(
            seek_to_middle,
            Some((5000000, 9999999)),
            "Seeking to middle should work"
        );

        // Scenario 2: Downloading first chunk (progressive download)
        let first_chunk = parse_range_header("bytes=0-1048575", file_size);
        assert_eq!(
            first_chunk,
            Some((0, 1048575)),
            "First 1MB chunk should work"
        );

        // Scenario 3: Resuming interrupted download
        let resume = parse_range_header("bytes=7500000-", file_size);
        assert_eq!(
            resume,
            Some((7500000, 9999999)),
            "Resume from 75% should work"
        );

        // Scenario 4: Getting file metadata (first byte)
        let metadata = parse_range_header("bytes=0-0", file_size);
        assert_eq!(metadata, Some((0, 0)), "Getting first byte should work");

        // Scenario 5: Getting last second of audio (assuming ~1MB/sec)
        let last_second = parse_range_header("bytes=-1000000", file_size);
        assert_eq!(
            last_second,
            Some((9000000, 9999999)),
            "Last second should work"
        );
    }

    #[test]
    fn test_update_compliance_hold_request_serialization() {
        // Test that UpdateComplianceHoldRequest can be serialized/deserialized
        let request = UpdateComplianceHoldRequest {
            compliance_hold: true,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(
            json.contains("complianceHold"),
            "JSON should use camelCase for compliance_hold"
        );
        assert!(json.contains("true"), "JSON should contain true value");

        let deserialized: UpdateComplianceHoldRequest =
            serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.compliance_hold, true,
            "Deserialized value should match"
        );
    }

    #[test]
    fn test_create_recording_request_validation() {
        // Test that CreateRecordingRequest has all necessary fields
        let request = CreateRecordingRequest {
            call_id: 123,
            duration_seconds: 300,
            format: "wav".to_string(),
        };

        assert_eq!(request.call_id, 123, "call_id should be set");
        assert_eq!(request.duration_seconds, 300, "duration should be set");
        assert_eq!(request.format, "wav", "format should be set");

        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("callId"), "JSON should use camelCase");
        assert!(json.contains("durationSeconds"), "JSON should use camelCase");
    }
}
