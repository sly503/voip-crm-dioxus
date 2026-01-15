# Automatic Retention Calculation for Call Recordings

## Overview

This document describes the automatic retention_until calculation system implemented for call recordings. The system ensures that recordings are automatically assigned an appropriate retention period based on configurable policies.

## Implementation

### Core Function: `calculate_retention_until`

**Location:** `src/server/db/recordings.rs`

**Signature:**
```rust
pub async fn calculate_retention_until(
    pool: &PgPool,
    campaign_id: Option<i64>,
    agent_id: Option<i64>,
) -> Result<DateTime<Utc>, sqlx::Error>
```

### Policy Priority

The function implements a priority-based lookup system:

1. **Campaign-specific retention policy** (Highest Priority)
   - If the call has a campaign_id and a retention policy exists for that campaign
   - Uses `get_campaign_retention_policy(pool, campaign_id)`

2. **Agent-specific retention policy** (Medium Priority)
   - If the call has an agent_id and a retention policy exists for that agent
   - Uses `get_agent_retention_policy(pool, agent_id)`

3. **Default retention policy** (Fallback)
   - A retention policy marked as `is_default = true`
   - Uses `get_default_retention_policy(pool)`

4. **Environment variable** (Ultimate Fallback)
   - `DEFAULT_RETENTION_DAYS` environment variable
   - Defaults to 90 days if not set

## Usage Example

When a recording is uploaded (either via API or from SIP call finalization), the retention_until date is automatically calculated:

```rust
// 1. Get the call to extract campaign_id and agent_id
let call = db::calls::get_by_id(&pool, call_id).await?;

// 2. Calculate retention_until based on policies
let retention_until = db::recordings::calculate_retention_until(
    &pool,
    call.campaign_id,  // Optional<i64>
    call.agent_id,     // Optional<i64>
).await?;

// 3. Store the recording with calculated retention_until
let recording = db::recordings::insert_recording(
    &pool,
    call_id,
    &file_path,
    file_size,
    duration_seconds,
    "wav",
    "default",
    retention_until,  // Automatically calculated!
    metadata,
).await?;
```

## Configuration

### Environment Variables

```bash
# Default retention period in days (used when no policy exists)
DEFAULT_RETENTION_DAYS=90
```

### Retention Policies

Retention policies are managed through the API:

**Create a default policy:**
```bash
POST /api/retention-policies
{
  "name": "Default Retention",
  "retention_days": 90,
  "applies_to": "ALL",
  "is_default": true
}
```

**Create a campaign-specific policy:**
```bash
POST /api/retention-policies
{
  "name": "Sales Campaign - Extended",
  "retention_days": 180,
  "applies_to": "CAMPAIGN",
  "campaign_id": 123,
  "is_default": false
}
```

**Create an agent-specific policy:**
```bash
POST /api/retention-policies
{
  "name": "Senior Agent - Short",
  "retention_days": 30,
  "applies_to": "AGENT",
  "agent_id": 456,
  "is_default": false
}
```

## Examples

### Scenario 1: Campaign-Specific Policy
```
Call Details:
- campaign_id: 5
- agent_id: 10

Policies in Database:
- Campaign 5: 180 days
- Agent 10: 30 days
- Default: 90 days

Result: 180 days (campaign policy takes priority)
```

### Scenario 2: Agent-Specific Policy
```
Call Details:
- campaign_id: None
- agent_id: 10

Policies in Database:
- Agent 10: 30 days
- Default: 90 days

Result: 30 days (agent policy, no campaign policy exists)
```

### Scenario 3: Default Policy
```
Call Details:
- campaign_id: 99 (no policy exists)
- agent_id: None

Policies in Database:
- Default: 90 days

Result: 90 days (default policy)
```

### Scenario 4: Environment Fallback
```
Call Details:
- campaign_id: None
- agent_id: None

Policies in Database:
- (none)

Environment:
- DEFAULT_RETENTION_DAYS=120

Result: 120 days (environment variable)
```

### Scenario 5: Ultimate Fallback
```
Call Details:
- campaign_id: None
- agent_id: None

Policies in Database:
- (none)

Environment:
- DEFAULT_RETENTION_DAYS not set

Result: 90 days (hardcoded default)
```

## Logging

The function logs which policy is being used for transparency:

```
DEBUG Using campaign retention policy 'Sales Extended' (180 days) for campaign 5
DEBUG Using agent retention policy 'Senior Agent Short' (30 days) for agent 10
DEBUG Using default retention policy 'Standard Retention' (90 days)
DEBUG No retention policy found, using environment default (90 days)
```

## Integration Points

### Current Integration
- **Database Module**: `src/server/db/recordings.rs`
  - Function is available for use by any recording creation code

### Future Integration (Phase 3+)
- **SIP Call Handler**: When `finalize_recording()` completes
  - WAV data is stored via `storage.store_recording()`
  - Recording metadata is saved via `db::recordings::insert_recording()`
  - `calculate_retention_until()` is called to determine retention_until

- **Upload API**: `src/server/recordings_api.rs`
  - `upload_recording()` handler will use `calculate_retention_until()`
  - See commented example implementation in the handler

## Testing

### Manual Testing

1. **Create retention policies:**
   ```bash
   # Default policy
   curl -X POST http://localhost:3000/api/retention-policies \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "name": "Default 90 Days",
       "retention_days": 90,
       "applies_to": "ALL",
       "is_default": true
     }'

   # Campaign policy
   curl -X POST http://localhost:3000/api/retention-policies \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "name": "Sales Extended",
       "retention_days": 180,
       "applies_to": "CAMPAIGN",
       "campaign_id": 1,
       "is_default": false
     }'
   ```

2. **Create a test recording** (when upload endpoint is implemented):
   - The retention_until will be automatically calculated
   - Check the logs to see which policy was used
   - Verify the retention_until date in the database

### Database Verification

```sql
-- Check retention policies
SELECT id, name, retention_days, applies_to, campaign_id, agent_id, is_default
FROM recording_retention_policies
ORDER BY is_default DESC, created_at DESC;

-- Check recordings and their retention dates
SELECT
  cr.id,
  cr.call_id,
  c.campaign_id,
  c.agent_id,
  cr.uploaded_at,
  cr.retention_until,
  EXTRACT(DAY FROM (cr.retention_until - cr.uploaded_at)) as retention_days
FROM call_recordings cr
JOIN calls c ON c.id = cr.call_id
ORDER BY cr.uploaded_at DESC
LIMIT 10;
```

## Benefits

1. **Automatic Compliance**: Recordings are automatically assigned retention periods based on business rules
2. **Flexible Configuration**: Different retention periods for different campaigns, agents, or global defaults
3. **Transparent Logging**: Clear logs show which policy is being applied
4. **Graceful Fallback**: Multiple fallback levels ensure recordings always get a retention period
5. **Easy Management**: Retention policies can be changed without code changes

## Related Files

- `src/server/db/recordings.rs` - Core implementation
- `src/server/recordings_api.rs` - API handlers (upload endpoint shows usage pattern)
- `migrations/005_recording_retention_policies.sql` - Database schema
- `src/models/recording.rs` - Data models

## Future Enhancements

1. **Retention Policy Updates**: Ability to recalculate retention_until for existing recordings when policies change
2. **Bulk Updates**: API endpoint to apply new retention policies to multiple recordings
3. **Audit Trail**: Log which policy was used for each recording in metadata
4. **Policy Templates**: Pre-configured retention policy templates for common use cases
