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

use crate::models::recording::{CallRecording, CreateRecordingRequest, RecordingSearchParams, UpdateComplianceHoldRequest};
use crate::server::{AppState, auth::Claims};
use crate::server::storage::{RecordingStorage, StorageError};

/// Search recordings with optional filters
///
/// Supports filtering by agent, campaign, lead, date range, disposition, and compliance hold status.
/// Results are paginated using limit and offset parameters.
pub async fn search_recordings(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(params): Query<RecordingSearchParams>,
) -> Result<Json<Vec<CallRecording>>, StatusCode> {
    // TODO: Permission check will be added in subtask 4.6
    // Agents should only see their own recordings, Supervisors/Admins can see all

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
pub async fn get_recording_details(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Json<CallRecording>, StatusCode> {
    // TODO: Permission check will be added in subtask 4.6
    // Agents can only access their own recordings, Supervisors/Admins can access all

    crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Delete a recording by ID
///
/// Permanently removes the recording file from storage and deletes the database record.
/// Recordings with compliance_hold=true cannot be deleted.
pub async fn delete_recording_handler(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Permission check will be added in subtask 4.6
    // Only Supervisors/Admins should be able to delete recordings

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
pub async fn update_compliance_hold(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(req): Json<UpdateComplianceHoldRequest>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Permission check will be added in subtask 4.6
    // Only Supervisors/Admins should be able to set compliance holds

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

    Ok(StatusCode::OK)
}

/// Upload a recording file
///
/// This endpoint accepts raw audio data in the request body and stores it
/// with encryption. The recording metadata is stored in the database.
pub async fn upload_recording(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<CreateRecordingRequest>,
) -> Result<Json<CallRecording>, StatusCode> {
    // Note: In a real implementation, the audio data would come from the SIP stack
    // For now, this is a placeholder that would be called by the recording system

    // TODO: This will be integrated with the SIP recording system in phase 3
    // The actual file data would come from the RTP packet capture and audio mixing

    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Download a recording file with streaming support
///
/// Supports HTTP Range requests for partial downloads, which is essential
/// for large audio files and seeking in audio players.
pub async fn download_recording(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    // Get recording metadata from database
    let recording = crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // TODO: Check permissions - agents can only access their own recordings
    // Supervisors/Admins can access all recordings
    // This will be implemented in subtask 4.6

    // Get storage instance from state
    // For now, we'll need to create a storage instance on demand
    // In a production system, this would be part of AppState
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
pub async fn stream_recording(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(id): Path<i64>,
) -> Result<Response, StatusCode> {
    // Get recording metadata from database
    let recording = crate::server::db::recordings::get_recording(&state.db, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recording {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get storage instance
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
