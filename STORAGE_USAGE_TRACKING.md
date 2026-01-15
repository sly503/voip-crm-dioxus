# Storage Usage Tracking and Quota Enforcement

This document describes the implementation of storage usage tracking and quota enforcement for the call recording system.

## Overview

The storage system now includes comprehensive tracking and quota management features:

1. **Database-Integrated Tracking**: Storage usage is tracked in the `storage_usage_log` table
2. **Real-time Statistics**: File count and total size are tracked on every store/delete operation
3. **Daily Usage Logs**: Recordings added/deleted per day are tracked automatically
4. **Quota Enforcement**: Storage operations are blocked when quota is exceeded
5. **Quota Warnings**: System can detect when storage exceeds 80% capacity

## Components

### Database Layer (`src/server/db/recordings.rs`)

Functions for managing storage usage in the database:

- `get_or_create_today_usage()` - Get or create storage usage entry for current day
- `update_daily_storage_stats()` - Update total files and size for a date
- `increment_recordings_added()` - Increment count of recordings added today
- `increment_recordings_deleted()` - Increment count of recordings deleted today
- `get_usage_history(days)` - Get usage history for the past N days
- `get_total_storage_stats()` - Get aggregate statistics from database

### Storage Tracking Layer (`src/server/storage/mod.rs`)

#### StorageUsageTracker

Coordinates between storage operations and database tracking:

```rust
pub struct StorageUsageTracker {
    pool: PgPool,
}

impl StorageUsageTracker {
    pub async fn update_stats(&self, total_files: u64, total_size_bytes: u64) -> StorageResult<()>
    pub async fn record_addition(&self) -> StorageResult<()>
    pub async fn record_deletion(&self) -> StorageResult<()>
    pub async fn get_history(&self, days: i32) -> StorageResult<Vec<StorageUsage>>
}
```

#### LocalFileStorage Enhancements

Enhanced with optional usage tracking:

- `with_tracking(base_path, max_storage_gb, encryption_ctx, pool)` - Create storage with database tracking
- `get_storage_stats()` - Get comprehensive statistics including quota percentage and daily usage
- `is_quota_warning()` - Check if usage exceeds 80% of quota
- `get_usage_percentage()` - Get current usage as percentage of quota
- Automatic tracking on `store_recording()` and `delete_recording()`

## Usage Examples

### Initialize Storage with Tracking

```rust
use crate::server::storage::{StorageConfig, LocalFileStorage};
use sqlx::PgPool;

// Load configuration
let config = StorageConfig::from_env()?;

// Create storage with database tracking
let pool = /* your PgPool */;
let encryption_ctx = EncryptionContext::from_hex(&config.encryption_key, "default")?;
let storage = LocalFileStorage::with_tracking(
    &config.recordings_path,
    config.max_storage_gb,
    encryption_ctx,
    pool.clone(),
);

// Initialize
storage.init().await?;
```

### Store a Recording (with automatic tracking)

```rust
// Storage automatically tracks this operation
let file = storage.store_recording(call_id, audio_data, "wav").await?;

// Database is automatically updated with:
// - Incremented recordings_added counter for today
// - Updated total_files and total_size_bytes for today
```

### Delete a Recording (with automatic tracking)

```rust
// Storage automatically tracks this operation
storage.delete_recording(&file_path).await?;

// Database is automatically updated with:
// - Incremented recordings_deleted counter for today
// - Updated total_files and total_size_bytes for today
```

### Check Quota Status

```rust
// Check if approaching quota (>80%)
if storage.is_quota_warning().await? {
    println!("Warning: Storage is over 80% full!");
}

// Get exact usage percentage
let percentage = storage.get_usage_percentage().await?;
println!("Storage usage: {:.1}%", percentage);

// Get comprehensive statistics
let stats = storage.get_storage_stats().await?;
println!("Files: {}", stats.total_files);
println!("Size: {:.2} GB / {:.2} GB", stats.total_size_gb, stats.quota_gb);
println!("Usage: {:.1}%", stats.quota_percentage);
```

### Get Usage History

```rust
// Get last 30 days of usage
if let Some(tracker) = storage.usage_tracker() {
    let history = tracker.get_history(30).await?;

    for day in history {
        println!("{}: {} files, {} bytes, +{} added, -{} deleted",
            day.date,
            day.total_files,
            day.total_size_bytes,
            day.recordings_added,
            day.recordings_deleted
        );
    }
}
```

## Database Schema

The `storage_usage_log` table tracks daily statistics:

```sql
CREATE TABLE storage_usage_log (
    id BIGSERIAL PRIMARY KEY,
    date DATE NOT NULL UNIQUE,
    total_files BIGINT NOT NULL DEFAULT 0,
    total_size_bytes BIGINT NOT NULL DEFAULT 0,
    recordings_added INT NOT NULL DEFAULT 0,
    recordings_deleted INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Quota Enforcement

Quota is enforced in `LocalFileStorage::store_recording()`:

1. Before storing a file, `check_quota()` is called
2. If current usage + file size > max quota, operation fails
3. Error returned: `StorageError::QuotaExceeded { used, quota }`
4. No file is written and no database changes are made

## Configuration

Storage configuration is loaded from environment variables:

- `RECORDINGS_PATH` - Base directory for recordings (default: `./recordings`)
- `MAX_STORAGE_GB` - Maximum storage quota in GB (default: `100`)
- `ENCRYPTION_KEY` - 32-byte hex-encoded encryption key (required)
- `DEFAULT_RETENTION_DAYS` - Default retention period (default: `90`)

## Testing

Comprehensive tests are included in `src/server/storage/mod.rs`:

- `test_quota_warning()` - Verifies quota warning detection
- `test_usage_percentage()` - Verifies usage percentage calculation
- `test_storage_quota()` - Verifies quota enforcement on store
- `test_storage_info()` - Verifies storage statistics tracking

## Performance Considerations

1. **Async Operations**: All database operations are async and non-blocking
2. **Error Handling**: Failed tracking operations log warnings but don't fail the main operation
3. **Efficient Queries**: Database queries use indexes on date columns
4. **Upsert Pattern**: Storage stats use `ON CONFLICT DO UPDATE` for efficiency

## Future Enhancements

Potential improvements for future iterations:

1. **Background Sync**: Periodic background job to sync filesystem stats with database
2. **Retention Integration**: Automatic cleanup when quota is exceeded
3. **Storage Alerts**: Email/webhook notifications when quota thresholds are reached
4. **Multi-tenant Quotas**: Per-campaign or per-agent quota limits
5. **S3 Backend**: Alternative storage backend using AWS S3 or compatible services
