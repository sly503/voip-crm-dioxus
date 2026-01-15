# VoIP CRM - Full Stack Rust Application

A modern, full-featured VoIP CRM system built entirely in Rust, featuring AI-powered calls, real-time analytics, and comprehensive call recording capabilities.

## Features

- **VoIP Integration**: Direct SIP trunk connectivity with Telnyx or any standard SIP provider
- **AI-Powered Calls**: Claude AI integration for intelligent call assistance
- **Call Recording**: Automatic recording with encryption, compliance holds, and retention policies
- **Lead Management**: Track and manage leads with detailed call history
- **Campaign Management**: Organize outreach efforts with campaign tracking
- **User Management**: Role-based access control (Admin, Supervisor, Agent)
- **Real-time Analytics**: Supervisor dashboard with live call monitoring
- **Email Notifications**: SMTP integration for user invitations and alerts
- **Audio Playback**: In-browser playback with speed controls and seeking

## Architecture

- **Frontend**: Dioxus (Rust UI framework) - compiles to WebAssembly
- **Backend**: Axum web framework with async Rust
- **Database**: PostgreSQL with SQLx
- **Authentication**: JWT-based with bcrypt password hashing
- **SIP Stack**: Pure Rust implementation (ftth-rsipstack)
- **Audio Processing**: Real-time RTP capture, mixing, and WAV encoding
- **Encryption**: AES-256-GCM for call recordings at rest

## System Requirements

- **Rust**: 1.70 or later
- **PostgreSQL**: 14 or later
- **Node.js**: 18+ (for Dioxus CLI)
- **Operating System**: Linux, macOS, or Windows
- **Storage**: Minimum 100GB recommended for call recordings
- **Memory**: 2GB RAM minimum, 4GB+ recommended

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/yourorg/voip-crm.git
cd voip-crm
```

### 2. Install Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Dioxus CLI
cargo install dioxus-cli

# Install wasm32 target for web builds
rustup target add wasm32-unknown-unknown
```

### 3. Set Up Database

```bash
# Start PostgreSQL (using Docker Compose)
docker-compose up -d postgres

# Or install PostgreSQL locally
# Fedora: sudo dnf install postgresql postgresql-server
# Ubuntu: sudo apt install postgresql postgresql-contrib

# Create database and run migrations
createdb voipcrm
psql voipcrm < migrations/001_initial_schema.sql
psql voipcrm < migrations/002_ai_enhancements.sql
psql voipcrm < migrations/003_email_verification.sql
psql voipcrm < migrations/004_call_recordings.sql
psql voipcrm < migrations/005_recording_retention_policies.sql
psql voipcrm < migrations/006_storage_usage_log.sql
psql voipcrm < migrations/007_add_recording_fields_to_campaigns.sql
psql voipcrm < migrations/008_recording_audit_log.sql
```

### 4. Configure Environment

Copy the example environment file and customize it:

```bash
cp .env.example .env
```

Edit `.env` with your configuration (see [Environment Variables](#environment-variables) below).

### 5. Run the Application

**Backend Server:**
```bash
cargo run --release
```

**Frontend (Development):**
```bash
dx serve --platform web --port 8080
```

**Production Build:**
```bash
dx build --release --platform web
```

The application will be available at:
- Backend API: http://localhost:3000
- Frontend: http://localhost:8080 (dev) or served by backend in production

## Environment Variables

### Core Configuration

```bash
# Database
DATABASE_URL=postgres://voipcrm:voipcrm123@localhost:5432/voipcrm

# Server
PORT=3000
JWT_SECRET=your-secure-jwt-secret-change-in-production

# Application
APP_URL=http://localhost:3000
REGISTRATION_ENABLED=true
```

### VoIP Configuration

```bash
# Telnyx API (Option 1)
TELNYX_API_KEY=your-telnyx-api-key
TELNYX_CONNECTION_ID=your-telnyx-connection-id
TELNYX_CALLER_ID=+15551234567
WEBHOOK_URL=https://your-domain.com/api/webhooks/telnyx

# Direct SIP Trunk (Option 2 - More cost-effective)
SIP_TRUNK_HOST=sip.yourprovider.com
SIP_TRUNK_PORT=5060
SIP_USERNAME=your-sip-username
SIP_PASSWORD=your-sip-password
SIP_CALLER_ID=+15551234567
SIP_TRANSPORT=UDP
SIP_CODEC=PCMU
```

### AI Integration

```bash
# Anthropic Claude API
ANTHROPIC_API_KEY=your-anthropic-api-key
```

### Email Configuration

```bash
# SMTP Settings
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your-smtp-username
SMTP_PASSWORD=your-smtp-password
SMTP_FROM_EMAIL=noreply@voipcrm.local
SMTP_FROM_NAME=VoIP CRM
```

### Call Recording Configuration

The call recording feature requires specific configuration for secure storage, encryption, and retention policies.

#### Environment Variables

```bash
# Storage Location
RECORDINGS_PATH=./recordings
# Default: ./recordings
# Recommendation: Use absolute path to dedicated storage volume in production
# Example: /mnt/recordings or /var/lib/voipcrm/recordings

# Storage Quota (in gigabytes)
MAX_STORAGE_GB=100
# Default: 100GB
# Recommendation: Set based on expected call volume
# Calculation: ~1MB per minute of recording
#   1000 calls/day × 5 min avg × 1MB/min × 30 days ≈ 150GB/month

# Default Retention Period (in days)
DEFAULT_RETENTION_DAYS=90
# Default: 90 days
# Recommendation: Check compliance requirements (e.g., GDPR, industry regulations)
# Note: This is a fallback; retention policies can be configured per campaign/agent

# Encryption Key (32 bytes hex-encoded, 64 characters)
ENCRYPTION_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
# ⚠️  CRITICAL: Generate a secure random key using:
#     openssl rand -hex 32
# NEVER use the example value in production!
# NEVER commit your production encryption key to version control!
# Store in environment or secrets manager (e.g., AWS Secrets Manager, HashiCorp Vault)

# Storage Alert Threshold (percentage)
STORAGE_ALERT_THRESHOLD=80
# Default: 80%
# Alerts are sent to admins when storage usage exceeds this percentage
# Daily reports are sent at 8:00 AM local time
```

## Call Recording Setup

The call recording feature automatically captures, encrypts, and manages all outbound calls. Follow these steps to set it up properly.

### 1. Generate Encryption Key

**⚠️ CRITICAL SECURITY STEP**

Generate a secure 32-byte encryption key using OpenSSL:

```bash
openssl rand -hex 32
```

Add the generated key to your `.env` file:

```bash
ENCRYPTION_KEY=<paste-your-generated-key-here>
```

**Security Best Practices:**
- **NEVER** use the example key from `.env.example` in production
- **NEVER** commit your production encryption key to version control
- **STORE SECURELY**: Use environment variables, secrets manager, or encrypted configuration
- **ROTATE KEYS**: Plan for periodic key rotation (see Key Rotation section below)
- **BACKUP KEYS**: Store encryption keys securely separate from backups

### 2. Configure Storage Location

Choose a storage location with sufficient space:

```bash
# Create recordings directory
sudo mkdir -p /mnt/recordings
sudo chown youruser:yourgroup /mnt/recordings
sudo chmod 700 /mnt/recordings

# Update .env
RECORDINGS_PATH=/mnt/recordings
```

**Storage Recommendations:**

| Call Volume | Avg Duration | Storage/Month | Recommended Quota |
|-------------|--------------|---------------|-------------------|
| 100 calls/day | 3 minutes | ~9GB | 50GB |
| 500 calls/day | 5 minutes | ~75GB | 150GB |
| 1000 calls/day | 5 minutes | ~150GB | 300GB |
| 5000 calls/day | 5 minutes | ~750GB | 1TB |

**Formula**: `Calls/day × Avg Duration (min) × 1MB/min × 30 days`

**Directory Structure** (automatically created):
```
recordings/
├── 2026/
│   ├── 01/
│   │   ├── 15/
│   │   │   ├── recording_123.wav.enc
│   │   │   ├── recording_124.wav.enc
│   │   │   └── ...
│   │   ├── 16/
│   │   └── ...
│   └── 02/
└── ...
```

### 3. Set Up Retention Policies

Retention policies determine how long recordings are kept before automatic deletion.

**Priority Order:**
1. Campaign-specific policy (highest priority)
2. Agent-specific policy
3. Default policy (configured in database)
4. Environment variable `DEFAULT_RETENTION_DAYS`

**Create Default Retention Policy** (via API or web UI):

```bash
curl -X POST "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Standard 90-day retention",
    "retentionDays": 90,
    "appliesTo": "ALL",
    "campaignId": null,
    "agentId": null,
    "isDefault": true
  }'
```

**Campaign-Specific Policy Example:**

```bash
# High-value sales: Keep for 1 year
curl -X POST "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "High-Value Sales - 1 year",
    "retentionDays": 365,
    "appliesTo": "CAMPAIGN",
    "campaignId": 5,
    "agentId": null,
    "isDefault": false
  }'
```

**Compliance Considerations:**
- **GDPR** (EU): Typically 30-90 days unless legitimate business need
- **TCPA** (US): No specific requirement, but 90-180 days common
- **FINRA** (Financial): 3-6 years for securities firms
- **HIPAA** (Healthcare): 6 years minimum
- **PCI-DSS** (Payment): 1 year minimum

### 4. Configure Consent Announcements (Optional)

Set consent announcements per campaign to comply with call recording laws:

1. Record a consent message (e.g., "This call may be recorded for quality assurance")
2. Save as WAV file (8kHz, 16-bit PCM, mono or stereo)
3. Upload via API or store in accessible location
4. Configure campaign with consent announcement path

**Example via SQL:**
```sql
UPDATE campaigns
SET consent_announcement = '/path/to/consent_message.wav',
    recording_enabled = true
WHERE id = 1;
```

**Legal Requirements by Region:**
- **One-Party Consent** (US: 38 states): Only one party must consent (can be agent)
- **Two-Party Consent** (US: CA, CT, FL, IL, MD, MA, MT, NH, PA, WA): Both parties must consent
- **EU/UK**: Clear notice required; GDPR compliance mandatory
- **Canada**: One-party consent federally; varies by province

### 5. Set Up Automated Cleanup

The system automatically deletes expired recordings daily at 2:00 AM local time.

**Verify Scheduler is Running:**
```bash
# Check logs for scheduler initialization
tail -f logs/voipcrm.log | grep "retention scheduler"
```

**Manual Cleanup (if needed):**
```sql
-- Find expired recordings (not under compliance hold)
SELECT id, file_path, retention_until
FROM call_recordings
WHERE retention_until < NOW()
  AND compliance_hold = false;

-- Manual deletion (use with caution!)
-- The scheduler handles this automatically
```

### 6. Configure Storage Alerts

Email alerts are sent to admins when storage exceeds the threshold:

```bash
# .env configuration
STORAGE_ALERT_THRESHOLD=80  # Alert at 80% capacity

# Alerts are throttled to once per 24 hours
# Daily storage reports sent at 8:00 AM to all admin users
```

**Alert Triggers:**
- **80-90%**: Warning (amber alert)
- **90-95%**: Critical (orange alert)
- **95%+**: Emergency (red alert)

**Alert Email Example:**
```
Subject: ⚠️ Storage Alert: 85% capacity reached

Current Usage: 85.3 GB / 100 GB (85%)

Recommendations:
1. Review retention policies
2. Delete unnecessary recordings
3. Expand storage capacity
4. Enable compliance holds only when necessary

View Storage Dashboard: http://localhost:3000/recordings?tab=storage
```

### 7. Backup Recommendations

**⚠️ IMPORTANT**: Recordings are encrypted. You must backup both files AND encryption keys.

#### Backup Strategy

**Option 1: File-Level Backup (Simple)**

```bash
#!/bin/bash
# Daily backup script
BACKUP_DIR="/mnt/backup/voipcrm-recordings"
DATE=$(date +%Y%m%d)

# Backup recordings
rsync -av --delete /mnt/recordings/ "$BACKUP_DIR/$DATE/"

# Backup database (includes retention policies, metadata)
pg_dump voipcrm > "$BACKUP_DIR/$DATE/database.sql"

# Keep backups for 30 days
find "$BACKUP_DIR" -type d -mtime +30 -exec rm -rf {} \;
```

**Option 2: Incremental Backup (Recommended for large volumes)**

```bash
#!/bin/bash
# Incremental backup using rsnapshot or restic

# Install restic
# sudo dnf install restic

# Initialize repository
restic -r /mnt/backup/voipcrm init

# Backup with deduplication
restic -r /mnt/backup/voipcrm backup /mnt/recordings \
  --tag voipcrm-recordings

# Backup database
pg_dump voipcrm | restic -r /mnt/backup/voipcrm backup \
  --stdin --stdin-filename database.sql \
  --tag voipcrm-database

# Prune old backups (keep 7 daily, 4 weekly, 6 monthly)
restic -r /mnt/backup/voipcrm forget \
  --keep-daily 7 --keep-weekly 4 --keep-monthly 6 \
  --prune
```

**Option 3: Cloud Backup (S3-compatible)**

```bash
#!/bin/bash
# Backup to S3 (or compatible: MinIO, Backblaze B2, Wasabi)

# Install rclone
# curl https://rclone.org/install.sh | sudo bash

# Configure S3 remote
# rclone config

# Sync to cloud
rclone sync /mnt/recordings remote:voipcrm-recordings \
  --transfers 10 \
  --checkers 10 \
  --backup-dir remote:voipcrm-recordings-archive/$(date +%Y%m%d)

# Backup database to cloud
pg_dump voipcrm | gzip | rclone rcat remote:voipcrm-backups/database-$(date +%Y%m%d).sql.gz
```

#### Encryption Key Backup

**CRITICAL**: Store encryption keys separately from recordings!

```bash
# Export encryption key to secure location
echo "ENCRYPTION_KEY=$ENCRYPTION_KEY" > /secure/location/encryption-keys.env
chmod 600 /secure/location/encryption-keys.env

# Encrypt key file with GPG (recommended)
gpg --symmetric --cipher-algo AES256 /secure/location/encryption-keys.env

# Store encrypted key file in:
# - Password manager (1Password, LastPass, Bitwarden)
# - Hardware security module (HSM)
# - Cloud secrets manager (AWS Secrets Manager, HashiCorp Vault)
# - Offline secure storage (encrypted USB drive in safe)
```

#### Disaster Recovery Testing

Test your backups regularly:

```bash
# Restore test
mkdir -p /tmp/restore-test
rsync -av /mnt/backup/voipcrm-recordings/latest/ /tmp/restore-test/

# Verify file integrity
cd /tmp/restore-test
find . -name "*.wav.enc" | head -n 10 | while read file; do
  echo "Checking: $file"
  # Attempt to list file (will fail if corrupted)
  ls -lh "$file"
done

# Test database restore
createdb voipcrm_restore_test
psql voipcrm_restore_test < /mnt/backup/voipcrm-recordings/latest/database.sql

# Cleanup
dropdb voipcrm_restore_test
rm -rf /tmp/restore-test
```

### 8. Key Rotation (Advanced)

For maximum security, rotate encryption keys periodically:

**Steps:**
1. Generate new encryption key
2. Deploy new key with new `encryption_key_id`
3. New recordings use new key
4. Optionally re-encrypt old recordings (see `docs/KEY-ROTATION.md`)

**Note**: Key rotation requires planning. Old recordings remain encrypted with old keys unless explicitly re-encrypted.

### 9. Monitoring & Maintenance

**Daily Checklist (Automated):**
- ✅ Automatic retention cleanup (2:00 AM)
- ✅ Storage usage tracking
- ✅ Daily storage report email (8:00 AM)

**Weekly Checklist (Manual):**
- Review storage usage trends
- Check for failed recordings (audit log)
- Verify backup success
- Review compliance holds

**Monthly Checklist:**
- Review retention policies
- Audit recording access logs
- Test backup restoration
- Review storage capacity planning

**Query Storage Usage:**
```sql
-- Current storage stats
SELECT
  COUNT(*) as total_recordings,
  ROUND(SUM(file_size)::numeric / 1024 / 1024 / 1024, 2) as total_gb,
  ROUND(AVG(duration_seconds), 2) as avg_duration_seconds
FROM call_recordings
WHERE uploaded_at >= NOW() - INTERVAL '30 days';

-- Recordings by campaign
SELECT
  c.name as campaign,
  COUNT(cr.id) as recordings,
  ROUND(SUM(cr.file_size)::numeric / 1024 / 1024, 2) as size_mb
FROM call_recordings cr
JOIN calls ca ON cr.call_id = ca.id
JOIN campaigns c ON ca.campaign_id = c.id
GROUP BY c.name
ORDER BY size_mb DESC;
```

## Development

### Project Structure

```
voip-crm/
├── src/
│   ├── main.rs                    # Application entry point
│   ├── api/                       # API client (WASM)
│   │   ├── auth.rs
│   │   ├── campaigns.rs
│   │   ├── calls.rs
│   │   ├── leads.rs
│   │   ├── recordings.rs         # Recording API client
│   │   └── mod.rs
│   ├── components/                # Dioxus UI components
│   │   ├── auth/
│   │   ├── campaigns/
│   │   ├── leads/
│   │   ├── recordings/           # Recording UI components
│   │   └── supervisor/
│   ├── models/                    # Shared data models
│   │   ├── recording.rs          # Recording models
│   │   └── ...
│   └── server/                    # Backend (native)
│       ├── auth.rs
│       ├── campaigns.rs
│       ├── recordings_api.rs     # Recording REST API
│       ├── automation.rs         # Retention scheduler
│       ├── db/
│       │   ├── recordings.rs     # Recording database queries
│       │   └── ...
│       ├── sip/                  # SIP/RTP stack
│       │   ├── audio_mixer.rs    # Audio mixing
│       │   ├── audio_converter.rs # PCM to WAV
│       │   └── rtp.rs            # RTP recording
│       ├── storage/              # File storage system
│       │   ├── encryption.rs     # AES-256-GCM encryption
│       │   └── mod.rs            # Storage backend
│       └── mod.rs
├── migrations/                    # SQL migrations
│   ├── 004_call_recordings.sql
│   ├── 005_recording_retention_policies.sql
│   ├── 006_storage_usage_log.sql
│   ├── 007_add_recording_fields_to_campaigns.sql
│   └── 008_recording_audit_log.sql
├── docs/                         # Documentation
│   ├── RECORDING-API.md          # Recording API documentation
│   └── API-AUTHENTICATION.md
├── Cargo.toml                    # Rust dependencies
├── Dioxus.toml                   # Dioxus configuration
├── docker-compose.yml            # Docker services
└── .env.example                  # Environment template
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test storage::tests

# Run with output
cargo test -- --nocapture

# Test recording features
cargo test recording
cargo test retention
cargo test audio
```

### Building for Production

```bash
# Build optimized backend
cargo build --release

# Build optimized frontend
dx build --release --platform web

# The frontend will be in dist/ and served by the backend
```

### Docker Deployment

```bash
# Build and run all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## API Documentation

- **Recording API**: See [docs/RECORDING-API.md](./docs/RECORDING-API.md)
- **Authentication**: See [docs/API-AUTHENTICATION.md](./docs/API-AUTHENTICATION.md)

### Quick API Examples

**Search Recordings:**
```bash
curl -X GET "http://localhost:3000/api/recordings?startDate=2026-01-01T00:00:00Z&limit=10" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Download Recording:**
```bash
curl -X GET "http://localhost:3000/api/recordings/123/download" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -o recording.wav
```

**Get Storage Stats:**
```bash
curl -X GET "http://localhost:3000/api/recordings/storage/stats" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Troubleshooting

### Recording Issues

**Problem**: Recordings not being created

**Solutions**:
1. Check campaign has `recording_enabled = true`
2. Verify `RECORDINGS_PATH` directory exists and is writable
3. Check encryption key is configured correctly
4. Review logs for RTP packet capture errors

**Problem**: Storage quota exceeded

**Solutions**:
1. Increase `MAX_STORAGE_GB` in .env
2. Review retention policies (shorten retention period)
3. Delete old recordings manually
4. Expand disk space

**Problem**: Playback not working

**Solutions**:
1. Verify recording file exists in storage
2. Check browser console for errors
3. Ensure JWT token is valid
4. Test download endpoint directly

### Database Issues

**Problem**: Migration failed

**Solution**:
```bash
# Check which migrations have run
psql voipcrm -c "SELECT * FROM schema_migrations;"

# Manually run missing migration
psql voipcrm < migrations/004_call_recordings.sql
```

**Problem**: Connection refused

**Solution**:
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Verify connection string in .env
psql $DATABASE_URL
```

## Security Considerations

### Call Recording Security

- **Encryption**: All recordings encrypted with AES-256-GCM
- **Access Control**: Role-based permissions (Agent/Supervisor/Admin)
- **Audit Logging**: All access logged with user ID, timestamp, IP address
- **Compliance Holds**: Prevent deletion of sensitive recordings
- **Key Management**: Separate encryption keys from data storage

### Production Checklist

- [ ] Change default `JWT_SECRET`
- [ ] Generate secure `ENCRYPTION_KEY` (never use example value)
- [ ] Enable HTTPS/TLS for all API endpoints
- [ ] Configure firewall rules (restrict port 3000 access)
- [ ] Set up regular database backups
- [ ] Set up recording file backups
- [ ] Configure SMTP with proper SPF/DKIM records
- [ ] Review retention policies for compliance
- [ ] Enable storage alerts
- [ ] Set up monitoring and logging
- [ ] Restrict admin access
- [ ] Use secrets manager for sensitive environment variables

## License

MIT License - See LICENSE file for details

## Support

- **Documentation**: See `docs/` directory
- **Issues**: Submit via GitHub Issues
- **Email**: support@voipcrm.local

## Credits

Built with:
- [Dioxus](https://dioxuslabs.com/) - Rust UI framework
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- [ftth-rsipstack](https://crates.io/crates/ftth-rsipstack) - Pure Rust SIP stack
- [hound](https://crates.io/crates/hound) - WAV audio encoding
- [aes-gcm](https://crates.io/crates/aes-gcm) - Encryption

---

**VoIP CRM** - Production-ready VoIP CRM with AI-powered features and enterprise-grade call recording.
