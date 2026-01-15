# Subtask 6.5 Verification: Storage Usage Monitoring and Alerts

## Overview
Implemented comprehensive storage usage monitoring and alert system that:
- Monitors storage usage hourly
- Sends email alerts to admins when storage exceeds threshold
- Sends daily storage reports to admins
- Configurable alert threshold via environment variable

## Implementation Details

### 1. Email Service Enhancements (`src/server/email.rs`)
Added two new email methods:

#### Storage Alert Email
- **Method**: `send_storage_alert()`
- **Trigger**: When storage usage exceeds threshold (default 80%)
- **Recipients**: All admin users with verified emails
- **Frequency**: Maximum once per 24 hours (throttled to prevent spam)
- **Content**:
  - Current storage usage (GB)
  - Storage quota (GB)
  - Usage percentage
  - Color-coded progress bar (amber/orange/red based on severity)
  - Recommended actions to free up space

#### Daily Storage Report Email
- **Method**: `send_daily_storage_report()`
- **Trigger**: Daily at 8:00 AM local time
- **Recipients**: All admin users with verified emails
- **Content**:
  - Total recordings count
  - Storage used and quota
  - Usage percentage with visual progress bar
  - Today's activity (recordings added/deleted)
  - Net change calculation

Both emails include:
- HTML and plain text versions for compatibility
- Professional styling matching existing email templates
- Clear actionable information

### 2. Database Functions (`src/server/db/`)

#### User Database (`users.rs`)
- **Function**: `get_admins()`
- **Purpose**: Retrieve all admin users with verified emails
- **Returns**: `Vec<User>` containing admin users
- **Used for**: Determining email recipients for alerts and reports

#### Recording Database (`recordings.rs`)
- **Function**: `get_today_stats()`
- **Purpose**: Get today's recording statistics
- **Returns**: `(i32, i32)` - (recordings_added, recordings_deleted)
- **Used for**: Daily report email content

### 3. Storage Monitoring Scheduler (`src/server/automation.rs`)

#### New Method: `start_storage_monitoring()`
Spawns a background task that runs the storage monitoring loop.

#### Background Task: `run_storage_monitoring_loop()`
**Schedule**: Checks every hour (3600 seconds)

**Alert Logic**:
- Check storage stats via `storage.get_storage_stats()`
- If `quota_percentage >= STORAGE_ALERT_THRESHOLD`:
  - Check if alert was sent in last 24 hours
  - If not, send alert to all admins
  - Update last alert timestamp

**Daily Report Logic**:
- Check if current time is 8:00-8:30 AM local time
- Check if report was already sent today
- If not, send daily report to all admins
- Update last report date

**Features**:
- Graceful shutdown support
- Error handling with logging
- Alert throttling to prevent spam
- Timezone-aware scheduling

### 4. Configuration

#### Environment Variables (.env.example, .env)
```bash
# Storage alert threshold percentage (default: 80)
# Email alerts are sent to admins when storage usage exceeds this percentage
STORAGE_ALERT_THRESHOLD=80
```

Default value: 80% if not specified

## Email Templates

### Storage Alert Email
**Subject**: `⚠️ Storage Alert: 85.5 GB used of 100.0 GB (85.5%)`

**Features**:
- Warning icon (⚠️)
- Color-coded alert (amber for 80-90%, orange for 90-95%, red for 95%+)
- Storage statistics table
- Visual progress bar
- Recommendations section with 5 actionable items
- Responsive design

### Daily Storage Report Email
**Subject**: `📊 Daily Storage Report - 2024-01-15`

**Features**:
- Report date and time
- Statistics cards for total recordings and storage used
- Storage capacity progress bar
- Today's activity breakdown:
  - Recordings added (+)
  - Recordings deleted (-)
  - Net change
- Color-coded progress bar (green <80%, amber 80-90%, red >90%)

## Integration Points

### Server Startup
To enable storage monitoring, add to server initialization (e.g., `src/server/mod.rs`):

```rust
// After initializing storage and email service
automation_manager.start_storage_monitoring(
    Arc::new(storage),
    Arc::new(email_service),
).await;
```

## Testing

### Manual Testing

#### 1. Test Storage Alert
To test storage alerts without waiting for threshold:

1. Temporarily reduce `STORAGE_ALERT_THRESHOLD` to a low value (e.g., 1):
   ```bash
   STORAGE_ALERT_THRESHOLD=1
   ```

2. Upload recordings to exceed the threshold

3. Wait up to 1 hour for the next check, or reduce the interval in code temporarily

4. Check logs for:
   ```
   Storage usage at 5.2% (threshold: 1.0%) - sending alerts
   Sent storage alert to admin@example.com
   ```

5. Verify email received by admin users

#### 2. Test Daily Report
To test daily reports without waiting for 8 AM:

1. Temporarily modify the time check in `run_storage_monitoring_loop()`:
   ```rust
   // Change from hour() == 8 to current hour
   if local_now.hour() == <current_hour> && local_now.minute() < 30 {
   ```

2. Wait for the next hourly check

3. Check logs for:
   ```
   Sending daily storage report
   Sent daily storage report to admin@example.com
   ```

4. Verify email received by admin users

### Unit Testing

The email template functions can be tested by building HTML and checking for expected content:

```rust
#[test]
fn test_storage_alert_contains_stats() {
    let service = EmailService::from_env().unwrap();
    let html = service.build_storage_alert_html(85.5, 100.0, 85.5);

    assert!(html.contains("85.5"));
    assert!(html.contains("100.0"));
    assert!(html.contains("85.5%"));
}
```

## Production Deployment Checklist

- [ ] Configure SMTP settings in environment variables
- [ ] Set appropriate `STORAGE_ALERT_THRESHOLD` (recommended: 80)
- [ ] Ensure at least one admin user exists with verified email
- [ ] Start storage monitoring scheduler in server initialization
- [ ] Monitor logs for successful alert/report delivery
- [ ] Test email delivery by temporarily lowering threshold
- [ ] Verify emails are not being marked as spam

## Monitoring and Maintenance

### Log Messages to Watch

**Success**:
- `Started storage monitoring scheduler`
- `Sent storage alert to <email>`
- `Sent daily storage report to <email>`

**Errors**:
- `Failed to get storage stats: <error>`
- `Failed to send storage alert to <email>: <error>`
- `Failed to get admin users for storage alerts: <error>`

### Expected Behavior

1. **Normal Operation** (<80% usage):
   - Hourly check runs silently
   - Daily report sent at 8 AM
   - No alerts sent

2. **High Usage** (80-90%):
   - Alert sent once when threshold first exceeded
   - Daily alert if usage remains high
   - Amber-colored warnings in emails

3. **Critical Usage** (>90%):
   - More urgent alert styling (orange/red)
   - Daily reports show critical status
   - Recommendations remain the same

## Alert Throttling

Alerts are throttled to prevent spam:
- **Storage Alerts**: Maximum once per 24 hours
- **Daily Reports**: Exactly once per day at 8 AM

Even if storage remains above threshold, alerts will only be sent once per day.

## Email Deliverability

To ensure emails are delivered:

1. **SMTP Configuration**: Use a reputable SMTP service (Gmail, SendGrid, Mailgun)
2. **From Address**: Use a verified domain email address
3. **SPF/DKIM**: Configure DNS records for email authentication
4. **Testing**: Test with actual admin email addresses before production
5. **Monitoring**: Watch for bounce/failure logs

## Troubleshooting

### Problem: Alerts not being sent
**Check**:
1. Are there admin users in the database?
2. Do admin users have `email_verified = true`?
3. Is SMTP configured correctly?
4. Check server logs for error messages
5. Is storage actually above threshold?

### Problem: Daily report sent multiple times
**Check**:
1. Are multiple instances of the server running?
2. Check the `last_report_sent` state management logic
3. Verify time zone handling is correct

### Problem: Alert emails in spam folder
**Solution**:
1. Configure SPF/DKIM records
2. Use a verified sending domain
3. Request recipients to whitelist the sender
4. Test with different email providers

## Future Enhancements

Potential improvements for future versions:
- Configurable report schedule (not just 8 AM)
- Weekly/monthly summary reports
- Per-campaign storage breakdown in reports
- Predictive alerts (e.g., "storage will be full in X days")
- Slack/webhook integration for alerts
- Storage trends and analytics
- Configurable alert recipients (not just admins)

## Code Quality

- ✅ Follows existing code patterns from other email templates
- ✅ Comprehensive error handling and logging
- ✅ Graceful shutdown support
- ✅ Configuration via environment variables
- ✅ No hardcoded values
- ✅ Documentation and comments
- ✅ Consistent with existing automation patterns
