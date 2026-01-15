# Call Recording API Documentation

**VoIP CRM - Recording API for External Integrations**

**Version:** 1.0
**Last Updated:** 2026-01-15
**Task:** 002-call-recording-and-playback

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Recording Endpoints](#recording-endpoints)
4. [Retention Policy Endpoints](#retention-policy-endpoints)
5. [Storage Management](#storage-management)
6. [Request/Response Formats](#requestresponse-formats)
7. [Code Examples](#code-examples)
8. [Error Responses](#error-responses)
9. [Best Practices](#best-practices)
10. [Webhook Integration](#webhook-integration)
11. [Rate Limiting & Performance](#rate-limiting--performance)

---

## Overview

The VoIP CRM Recording API provides programmatic access to call recordings for quality assurance, compliance, and integration with external systems. All recordings are encrypted at rest using AES-256-GCM and access is controlled through role-based permissions.

### Key Features

- **Automated Recording:** All calls are automatically recorded when enabled
- **Secure Storage:** Recordings encrypted with AES-256-GCM at rest
- **Role-Based Access:** Agents see only their recordings; Supervisors/Admins see all
- **HTTP Range Support:** Seeking and progressive download for audio players
- **Retention Policies:** Automatic deletion based on configurable policies
- **Compliance Holds:** Prevent deletion of legally sensitive recordings
- **Audit Trail:** All access logged for compliance purposes

### Supported Formats

- **Primary Format:** WAV (16-bit PCM, 8kHz, Stereo)
- **Agent Channel:** Left channel
- **Customer Channel:** Right channel
- **Future Support:** MP3, OGG (planned)

---

## Authentication

All Recording API endpoints require JWT authentication. See [API-AUTHENTICATION.md](./API-AUTHENTICATION.md) for complete details.

### Quick Reference

```bash
# Include JWT token in Authorization header
Authorization: Bearer <your_jwt_token>
```

### Required Permissions

| Endpoint | Agent | Supervisor | Admin |
|----------|-------|------------|-------|
| Search recordings | Own only | All | All |
| Get recording details | Own only | All | All |
| Download recording | Own only | All | All |
| Delete recording | ❌ No | ✅ Yes | ✅ Yes |
| Set compliance hold | ❌ No | ✅ Yes | ✅ Yes |
| Manage retention policies | ❌ No | ✅ Yes | ✅ Yes |
| View storage stats | ❌ No | ✅ Yes | ✅ Yes |

---

## Recording Endpoints

### 1. Search Recordings

Search and filter call recordings with pagination support.

**Endpoint:** `GET /api/recordings`
**Authentication:** Required (JWT)
**Permissions:** Agents see only their recordings; Supervisors/Admins see all

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `agentId` | integer | No | Filter by agent ID |
| `campaignId` | integer | No | Filter by campaign ID |
| `leadId` | integer | No | Filter by lead ID |
| `startDate` | ISO 8601 | No | Filter recordings after this date |
| `endDate` | ISO 8601 | No | Filter recordings before this date |
| `disposition` | string | No | Filter by call disposition |
| `complianceHold` | boolean | No | Filter by compliance hold status |
| `limit` | integer | No | Number of results (default: 50) |
| `offset` | integer | No | Pagination offset (default: 0) |

**Example Request:**

```bash
curl -X GET "http://localhost:3000/api/recordings?startDate=2026-01-01T00:00:00Z&limit=10" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (200 OK):**

```json
[
  {
    "id": 123,
    "callId": 456,
    "filePath": "2026/01/15/recording_123.wav",
    "fileSize": 2048000,
    "durationSeconds": 180,
    "format": "wav",
    "encryptionKeyId": "default",
    "uploadedAt": "2026-01-15T14:30:00Z",
    "retentionUntil": "2026-04-15T14:30:00Z",
    "complianceHold": false,
    "metadata": {
      "agentName": "John Doe",
      "leadName": "Jane Smith",
      "campaignName": "Q1 Sales",
      "disposition": "sale",
      "callDurationSeconds": 180
    },
    "createdAt": "2026-01-15T14:30:00Z"
  }
]
```

---

### 2. Get Recording Details

Retrieve metadata for a specific recording.

**Endpoint:** `GET /api/recordings/{id}`
**Authentication:** Required (JWT)
**Permissions:** Agents can access own recordings; Supervisors/Admins can access all

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Recording ID |

**Example Request:**

```bash
curl -X GET "http://localhost:3000/api/recordings/123" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (200 OK):**

```json
{
  "id": 123,
  "callId": 456,
  "filePath": "2026/01/15/recording_123.wav",
  "fileSize": 2048000,
  "durationSeconds": 180,
  "format": "wav",
  "encryptionKeyId": "default",
  "uploadedAt": "2026-01-15T14:30:00Z",
  "retentionUntil": "2026-04-15T14:30:00Z",
  "complianceHold": false,
  "metadata": {
    "agentName": "John Doe",
    "leadName": "Jane Smith",
    "campaignName": "Q1 Sales",
    "disposition": "sale",
    "callDurationSeconds": 180
  },
  "createdAt": "2026-01-15T14:30:00Z"
}
```

**Error Responses:**
- `404 Not Found` - Recording does not exist
- `403 Forbidden` - User does not have permission to access this recording

---

### 3. Download Recording

Download a complete recording file with support for HTTP Range requests.

**Endpoint:** `GET /api/recordings/{id}/download`
**Authentication:** Required (JWT)
**Permissions:** Agents can download own recordings; Supervisors/Admins can download all

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Recording ID |

**Request Headers:**

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes | JWT Bearer token |
| `Range` | No | Byte range for partial download (e.g., `bytes=0-1023`) |

**Example Requests:**

```bash
# Download full recording
curl -X GET "http://localhost:3000/api/recordings/123/download" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -o recording.wav

# Download with Range request (partial download)
curl -X GET "http://localhost:3000/api/recordings/123/download" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Range: bytes=0-1048575" \
  -o recording_part.wav
```

**Success Response (200 OK - Full File):**

Headers:
```
Content-Type: audio/wav
Content-Length: 2048000
Content-Disposition: attachment; filename="recording_123_1737818400.wav"
Accept-Ranges: bytes
```

Body: Binary audio data

**Success Response (206 Partial Content - Range Request):**

Headers:
```
Content-Type: audio/wav
Content-Length: 1048576
Content-Range: bytes 0-1048575/2048000
Accept-Ranges: bytes
```

Body: Binary audio data (requested range)

**HTTP Range Request Examples:**

| Range Header | Description |
|--------------|-------------|
| `bytes=0-999` | First 1000 bytes |
| `bytes=500-` | From byte 500 to end of file |
| `bytes=-500` | Last 500 bytes |

**Error Responses:**
- `404 Not Found` - Recording does not exist or file missing
- `403 Forbidden` - User does not have permission to download
- `416 Range Not Satisfiable` - Invalid range specified

**Audit Logging:**
All downloads are logged to the audit trail with user ID, timestamp, and IP address.

---

### 4. Stream Recording

Stream a recording for in-browser playback (alternative to download).

**Endpoint:** `GET /api/recordings/{id}/stream`
**Authentication:** Required (JWT)
**Permissions:** Agents can stream own recordings; Supervisors/Admins can stream all

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Recording ID |

**Example Request:**

```bash
curl -X GET "http://localhost:3000/api/recordings/123/stream" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (200 OK):**

Headers:
```
Content-Type: audio/wav
Content-Length: 2048000
Accept-Ranges: bytes
```

Body: Binary audio data

**Usage in HTML5 Audio Player:**

```html
<audio controls>
  <source src="http://localhost:3000/api/recordings/123/stream" type="audio/wav">
  Your browser does not support the audio element.
</audio>
```

**Note:** This endpoint logs access as a download event in the audit trail.

---

### 5. Delete Recording

Permanently delete a recording file and database record.

**Endpoint:** `DELETE /api/recordings/{id}`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Recording ID |

**Example Request:**

```bash
curl -X DELETE "http://localhost:3000/api/recordings/123" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (204 No Content):**

No response body.

**Error Responses:**
- `404 Not Found` - Recording does not exist
- `403 Forbidden` - Recording is under compliance hold or user lacks permission
- `401 Unauthorized` - Missing or invalid authentication

**Important Notes:**
- Recordings with `complianceHold: true` **cannot** be deleted
- Deletion is permanent and cannot be undone
- Both the encrypted file and database record are removed
- Audit trail entry is preserved even after deletion

---

### 6. Update Compliance Hold

Set or release compliance hold on a recording to prevent/allow deletion.

**Endpoint:** `PUT /api/recordings/{id}/compliance-hold`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Recording ID |

**Request Body:**

```json
{
  "complianceHold": true
}
```

**Example Requests:**

```bash
# Set compliance hold
curl -X PUT "http://localhost:3000/api/recordings/123/compliance-hold" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"complianceHold": true}'

# Release compliance hold
curl -X PUT "http://localhost:3000/api/recordings/123/compliance-hold" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"complianceHold": false}'
```

**Success Response (200 OK):**

No response body.

**Error Responses:**
- `404 Not Found` - Recording does not exist
- `403 Forbidden` - User lacks permission
- `400 Bad Request` - Invalid request body

**Audit Logging:**
All compliance hold changes are automatically logged with:
- User who made the change
- Timestamp
- Old and new hold status

---

## Retention Policy Endpoints

### 7. Get All Retention Policies

List all configured retention policies.

**Endpoint:** `GET /api/retention-policies`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Example Request:**

```bash
curl -X GET "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (200 OK):**

```json
[
  {
    "id": 1,
    "name": "Default 90-day retention",
    "retentionDays": 90,
    "appliesTo": "ALL",
    "campaignId": null,
    "agentId": null,
    "isDefault": true,
    "createdAt": "2026-01-01T00:00:00Z",
    "updatedAt": "2026-01-01T00:00:00Z"
  },
  {
    "id": 2,
    "name": "Sales Campaign - 180 days",
    "retentionDays": 180,
    "appliesTo": "CAMPAIGN",
    "campaignId": 5,
    "agentId": null,
    "isDefault": false,
    "createdAt": "2026-01-10T00:00:00Z",
    "updatedAt": "2026-01-10T00:00:00Z"
  }
]
```

---

### 8. Get Retention Policy

Retrieve details of a specific retention policy.

**Endpoint:** `GET /api/retention-policies/{id}`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Retention policy ID |

**Example Request:**

```bash
curl -X GET "http://localhost:3000/api/retention-policies/1" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (200 OK):**

```json
{
  "id": 1,
  "name": "Default 90-day retention",
  "retentionDays": 90,
  "appliesTo": "ALL",
  "campaignId": null,
  "agentId": null,
  "isDefault": true,
  "createdAt": "2026-01-01T00:00:00Z",
  "updatedAt": "2026-01-01T00:00:00Z"
}
```

**Error Responses:**
- `404 Not Found` - Retention policy does not exist
- `403 Forbidden` - User lacks permission

---

### 9. Create Retention Policy

Create a new retention policy for all recordings, specific campaigns, or specific agents.

**Endpoint:** `POST /api/retention-policies`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Request Body:**

```json
{
  "name": "Q1 Sales Campaign - 180 days",
  "retentionDays": 180,
  "appliesTo": "CAMPAIGN",
  "campaignId": 5,
  "agentId": null,
  "isDefault": false
}
```

**Field Validation:**

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `name` | string | Yes | Non-empty description |
| `retentionDays` | integer | Yes | Must be positive (> 0) |
| `appliesTo` | enum | Yes | One of: `ALL`, `CAMPAIGN`, `AGENT` |
| `campaignId` | integer | Conditional | Required if `appliesTo` is `CAMPAIGN` |
| `agentId` | integer | Conditional | Required if `appliesTo` is `AGENT` |
| `isDefault` | boolean | Yes | Only one default policy allowed |

**Retention Policy Types:**

1. **ALL** - Applies to all recordings (default fallback)
   - Must have `campaignId: null` and `agentId: null`

2. **CAMPAIGN** - Applies to recordings from a specific campaign
   - Must have `campaignId` set
   - Must have `agentId: null`

3. **AGENT** - Applies to recordings by a specific agent
   - Must have `agentId` set
   - Must have `campaignId: null`

**Priority Order:**
1. Campaign-specific policy (highest priority)
2. Agent-specific policy
3. Default policy
4. Environment variable `DEFAULT_RETENTION_DAYS`

**Example Requests:**

```bash
# Create default policy for all recordings
curl -X POST "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Standard 90-day retention",
    "retentionDays": 90,
    "appliesTo": "ALL",
    "campaignId": null,
    "agentId": null,
    "isDefault": true
  }'

# Create campaign-specific policy
curl -X POST "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "High-value Sales - 1 year",
    "retentionDays": 365,
    "appliesTo": "CAMPAIGN",
    "campaignId": 10,
    "agentId": null,
    "isDefault": false
  }'

# Create agent-specific policy
curl -X POST "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Training Agent - 30 days",
    "retentionDays": 30,
    "appliesTo": "AGENT",
    "campaignId": null,
    "agentId": 42,
    "isDefault": false
  }'
```

**Success Response (200 OK):**

```json
{
  "id": 3,
  "name": "Q1 Sales Campaign - 180 days",
  "retentionDays": 180,
  "appliesTo": "CAMPAIGN",
  "campaignId": 5,
  "agentId": null,
  "isDefault": false,
  "createdAt": "2026-01-15T10:00:00Z",
  "updatedAt": "2026-01-15T10:00:00Z"
}
```

**Error Responses:**
- `400 Bad Request` - Validation failed (see error message for details)
- `409 Conflict` - Duplicate policy or constraint violation
- `403 Forbidden` - User lacks permission

---

### 10. Update Retention Policy

Update an existing retention policy.

**Endpoint:** `PUT /api/retention-policies/{id}`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Retention policy ID |

**Request Body:**

Same format as Create Retention Policy (all fields required).

**Example Request:**

```bash
curl -X PUT "http://localhost:3000/api/retention-policies/3" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Q1 Sales Campaign - 365 days (extended)",
    "retentionDays": 365,
    "appliesTo": "CAMPAIGN",
    "campaignId": 5,
    "agentId": null,
    "isDefault": false
  }'
```

**Success Response (200 OK):**

```json
{
  "id": 3,
  "name": "Q1 Sales Campaign - 365 days (extended)",
  "retentionDays": 365,
  "appliesTo": "CAMPAIGN",
  "campaignId": 5,
  "agentId": null,
  "isDefault": false,
  "createdAt": "2026-01-15T10:00:00Z",
  "updatedAt": "2026-01-15T15:30:00Z"
}
```

**Error Responses:**
- `404 Not Found` - Retention policy does not exist
- `400 Bad Request` - Validation failed
- `409 Conflict` - Constraint violation
- `403 Forbidden` - User lacks permission

---

### 11. Delete Retention Policy

Delete a retention policy.

**Endpoint:** `DELETE /api/retention-policies/{id}`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Retention policy ID |

**Example Request:**

```bash
curl -X DELETE "http://localhost:3000/api/retention-policies/3" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (204 No Content):**

No response body.

**Error Responses:**
- `404 Not Found` - Retention policy does not exist
- `403 Forbidden` - User lacks permission

**Warning:** Be cautious when deleting the default policy. Ensure another default policy exists or recordings will fall back to the `DEFAULT_RETENTION_DAYS` environment variable.

---

## Storage Management

### 12. Get Storage Statistics

Retrieve comprehensive storage statistics including usage, quota, and daily history.

**Endpoint:** `GET /api/recordings/storage/stats`
**Authentication:** Required (JWT)
**Permissions:** Supervisors and Admins only

**Example Request:**

```bash
curl -X GET "http://localhost:3000/api/recordings/storage/stats" \
  -H "Authorization: Bearer YOUR_TOKEN"
```

**Success Response (200 OK):**

```json
{
  "totalFiles": 1523,
  "totalSizeBytes": 52428800000,
  "totalSizeGB": 48.83,
  "quotaGB": 100.0,
  "quotaPercentage": 48.83,
  "dailyUsage": [
    {
      "id": 15,
      "date": "2026-01-15",
      "totalFiles": 1523,
      "totalSizeBytes": 52428800000,
      "recordingsAdded": 45,
      "recordingsDeleted": 12,
      "createdAt": "2026-01-15T23:59:59Z"
    },
    {
      "id": 14,
      "date": "2026-01-14",
      "totalFiles": 1490,
      "totalSizeBytes": 51380224000,
      "recordingsAdded": 52,
      "recordingsDeleted": 8,
      "createdAt": "2026-01-14T23:59:59Z"
    }
  ]
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `totalFiles` | integer | Current total number of recordings |
| `totalSizeBytes` | integer | Total storage used in bytes |
| `totalSizeGB` | float | Total storage used in gigabytes |
| `quotaGB` | float | Maximum storage quota in gigabytes |
| `quotaPercentage` | float | Percentage of quota used (0-100) |
| `dailyUsage` | array | Last 30 days of usage history |

**Daily Usage Entry:**

| Field | Type | Description |
|-------|------|-------------|
| `date` | string | Date (YYYY-MM-DD) |
| `totalFiles` | integer | Total files on this date |
| `totalSizeBytes` | integer | Total size on this date |
| `recordingsAdded` | integer | Recordings added this day |
| `recordingsDeleted` | integer | Recordings deleted this day |

**Use Cases:**
- Monitor storage usage trends
- Predict when storage quota will be reached
- Display storage dashboard in external QA systems
- Trigger alerts when approaching quota limits

---

## Request/Response Formats

### Common Data Types

**CallRecording Object:**

```typescript
{
  id: number;                          // Unique recording ID
  callId: number;                      // Associated call ID
  filePath: string;                    // Relative path to encrypted file
  fileSize: number;                    // File size in bytes
  durationSeconds: number;             // Recording duration in seconds
  format: string;                      // Audio format (wav, mp3, ogg)
  encryptionKeyId: string;             // Encryption key identifier
  uploadedAt: string;                  // ISO 8601 timestamp
  retentionUntil: string;              // ISO 8601 timestamp
  complianceHold: boolean;             // True if protected from deletion
  metadata: RecordingMetadata | null;  // Optional metadata
  createdAt: string;                   // ISO 8601 timestamp
}
```

**RecordingMetadata Object:**

```typescript
{
  agentName?: string;          // Agent's full name
  leadName?: string;           // Lead's full name
  campaignName?: string;       // Campaign name
  disposition?: string;        // Call disposition (sale, no-answer, etc.)
  callDurationSeconds?: number; // Total call duration
}
```

**RecordingRetentionPolicy Object:**

```typescript
{
  id: number;                  // Unique policy ID
  name: string;                // Policy description
  retentionDays: number;       // Number of days to retain
  appliesTo: "ALL" | "CAMPAIGN" | "AGENT"; // Policy scope
  campaignId: number | null;   // Campaign ID (if applicable)
  agentId: number | null;      // Agent ID (if applicable)
  isDefault: boolean;          // True if default policy
  createdAt: string;           // ISO 8601 timestamp
  updatedAt: string;           // ISO 8601 timestamp
}
```

---

## Code Examples

### JavaScript/TypeScript Example

```typescript
// Recording API Client
class RecordingAPIClient {
  private baseURL: string;
  private token: string;

  constructor(baseURL: string, token: string) {
    this.baseURL = baseURL;
    this.token = token;
  }

  private async request(endpoint: string, options: RequestInit = {}) {
    const response = await fetch(`${this.baseURL}${endpoint}`, {
      ...options,
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Unauthorized - token may be expired');
      }
      const error = await response.json().catch(() => ({}));
      throw new Error(error.message || `HTTP ${response.status}`);
    }

    // Handle 204 No Content
    if (response.status === 204) {
      return null;
    }

    return await response.json();
  }

  // Search recordings
  async searchRecordings(params: {
    agentId?: number;
    campaignId?: number;
    startDate?: string;
    endDate?: string;
    limit?: number;
    offset?: number;
  }) {
    const queryString = new URLSearchParams(
      Object.entries(params)
        .filter(([_, v]) => v !== undefined)
        .map(([k, v]) => [k, String(v)])
    ).toString();

    return this.request(`/api/recordings?${queryString}`);
  }

  // Get recording details
  async getRecording(id: number) {
    return this.request(`/api/recordings/${id}`);
  }

  // Download recording
  async downloadRecording(id: number): Promise<Blob> {
    const response = await fetch(`${this.baseURL}/api/recordings/${id}/download`, {
      headers: {
        'Authorization': `Bearer ${this.token}`,
      },
    });

    if (!response.ok) {
      throw new Error(`Download failed: ${response.status}`);
    }

    return await response.blob();
  }

  // Get download URL for <audio> element
  getStreamURL(id: number): string {
    return `${this.baseURL}/api/recordings/${id}/stream`;
  }

  // Set compliance hold
  async setComplianceHold(id: number, hold: boolean) {
    return this.request(`/api/recordings/${id}/compliance-hold`, {
      method: 'PUT',
      body: JSON.stringify({ complianceHold: hold }),
    });
  }

  // Delete recording
  async deleteRecording(id: number) {
    return this.request(`/api/recordings/${id}`, {
      method: 'DELETE',
    });
  }

  // Get retention policies
  async getRetentionPolicies() {
    return this.request('/api/retention-policies');
  }

  // Create retention policy
  async createRetentionPolicy(policy: {
    name: string;
    retentionDays: number;
    appliesTo: 'ALL' | 'CAMPAIGN' | 'AGENT';
    campaignId?: number;
    agentId?: number;
    isDefault: boolean;
  }) {
    return this.request('/api/retention-policies', {
      method: 'POST',
      body: JSON.stringify(policy),
    });
  }

  // Get storage statistics
  async getStorageStats() {
    return this.request('/api/recordings/storage/stats');
  }
}

// Usage example
const client = new RecordingAPIClient('http://localhost:3000', 'your_jwt_token');

// Search recent recordings
const recordings = await client.searchRecordings({
  startDate: '2026-01-01T00:00:00Z',
  limit: 20
});

// Download a recording
const audioBlob = await client.downloadRecording(123);
const url = URL.createObjectURL(audioBlob);
// Use url in <audio> element or download link

// Play recording in audio element
const audioElement = document.getElementById('player') as HTMLAudioElement;
audioElement.src = client.getStreamURL(123);

// Set compliance hold
await client.setComplianceHold(123, true);

// Get storage stats
const stats = await client.getStorageStats();
console.log(`Storage: ${stats.quotaPercentage}% used`);
```

---

### Python Example

```python
import requests
from typing import Optional, Dict, Any, List
from datetime import datetime

class RecordingAPIClient:
    def __init__(self, base_url: str, token: str):
        self.base_url = base_url
        self.token = token
        self.session = requests.Session()
        self.session.headers.update({
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json'
        })

    def _handle_response(self, response: requests.Response) -> Any:
        """Handle API response and raise errors"""
        if response.status_code == 401:
            raise ValueError("Unauthorized - token may be expired")

        if response.status_code == 204:
            return None

        response.raise_for_status()
        return response.json()

    def search_recordings(
        self,
        agent_id: Optional[int] = None,
        campaign_id: Optional[int] = None,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        limit: int = 50,
        offset: int = 0
    ) -> List[Dict]:
        """Search recordings with filters"""
        params = {
            'limit': limit,
            'offset': offset
        }

        if agent_id:
            params['agentId'] = agent_id
        if campaign_id:
            params['campaignId'] = campaign_id
        if start_date:
            params['startDate'] = start_date
        if end_date:
            params['endDate'] = end_date

        response = self.session.get(
            f'{self.base_url}/api/recordings',
            params=params
        )
        return self._handle_response(response)

    def get_recording(self, recording_id: int) -> Dict:
        """Get recording details by ID"""
        response = self.session.get(
            f'{self.base_url}/api/recordings/{recording_id}'
        )
        return self._handle_response(response)

    def download_recording(self, recording_id: int, output_path: str):
        """Download recording to file"""
        response = self.session.get(
            f'{self.base_url}/api/recordings/{recording_id}/download',
            stream=True
        )
        response.raise_for_status()

        with open(output_path, 'wb') as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)

    def set_compliance_hold(self, recording_id: int, hold: bool):
        """Set or release compliance hold"""
        response = self.session.put(
            f'{self.base_url}/api/recordings/{recording_id}/compliance-hold',
            json={'complianceHold': hold}
        )
        return self._handle_response(response)

    def delete_recording(self, recording_id: int):
        """Delete a recording"""
        response = self.session.delete(
            f'{self.base_url}/api/recordings/{recording_id}'
        )
        return self._handle_response(response)

    def get_retention_policies(self) -> List[Dict]:
        """Get all retention policies"""
        response = self.session.get(
            f'{self.base_url}/api/retention-policies'
        )
        return self._handle_response(response)

    def create_retention_policy(
        self,
        name: str,
        retention_days: int,
        applies_to: str,
        campaign_id: Optional[int] = None,
        agent_id: Optional[int] = None,
        is_default: bool = False
    ) -> Dict:
        """Create a new retention policy"""
        policy = {
            'name': name,
            'retentionDays': retention_days,
            'appliesTo': applies_to,
            'campaignId': campaign_id,
            'agentId': agent_id,
            'isDefault': is_default
        }

        response = self.session.post(
            f'{self.base_url}/api/retention-policies',
            json=policy
        )
        return self._handle_response(response)

    def get_storage_stats(self) -> Dict:
        """Get storage statistics"""
        response = self.session.get(
            f'{self.base_url}/api/recordings/storage/stats'
        )
        return self._handle_response(response)


# Usage example
if __name__ == '__main__':
    client = RecordingAPIClient('http://localhost:3000', 'your_jwt_token')

    # Search recent recordings
    recordings = client.search_recordings(
        start_date='2026-01-01T00:00:00Z',
        limit=10
    )
    print(f"Found {len(recordings)} recordings")

    # Download a recording
    if recordings:
        recording_id = recordings[0]['id']
        client.download_recording(recording_id, f'recording_{recording_id}.wav')
        print(f"Downloaded recording {recording_id}")

    # Get storage stats
    stats = client.get_storage_stats()
    print(f"Storage usage: {stats['quotaPercentage']:.1f}%")
    print(f"Total files: {stats['totalFiles']}")

    # Create retention policy
    policy = client.create_retention_policy(
        name='High-priority calls - 1 year',
        retention_days=365,
        applies_to='CAMPAIGN',
        campaign_id=10
    )
    print(f"Created policy: {policy['name']}")
```

---

### cURL Examples

```bash
# Search recordings from January 2026
curl -X GET "http://localhost:3000/api/recordings?startDate=2026-01-01T00:00:00Z&limit=10" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Get specific recording details
curl -X GET "http://localhost:3000/api/recordings/123" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Download recording
curl -X GET "http://localhost:3000/api/recordings/123/download" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -o recording_123.wav

# Download with range (first 1MB)
curl -X GET "http://localhost:3000/api/recordings/123/download" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Range: bytes=0-1048575" \
  -o recording_123_part.wav

# Set compliance hold
curl -X PUT "http://localhost:3000/api/recordings/123/compliance-hold" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"complianceHold": true}'

# Delete recording
curl -X DELETE "http://localhost:3000/api/recordings/123" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Get storage statistics
curl -X GET "http://localhost:3000/api/recordings/storage/stats" \
  -H "Authorization: Bearer YOUR_TOKEN"

# Create retention policy
curl -X POST "http://localhost:3000/api/retention-policies" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Sales Team - 6 months",
    "retentionDays": 180,
    "appliesTo": "CAMPAIGN",
    "campaignId": 5,
    "agentId": null,
    "isDefault": false
  }'
```

---

## Error Responses

### Standard Error Format

All errors return a JSON object with a `message` field:

```json
{
  "message": "Error description"
}
```

### Common HTTP Status Codes

| Code | Meaning | Common Causes |
|------|---------|---------------|
| 200 | OK | Successful request |
| 204 | No Content | Successful deletion |
| 206 | Partial Content | Successful range request |
| 400 | Bad Request | Invalid request body or parameters |
| 401 | Unauthorized | Missing, invalid, or expired token |
| 403 | Forbidden | Insufficient permissions or compliance hold |
| 404 | Not Found | Recording, policy, or resource not found |
| 409 | Conflict | Duplicate policy or constraint violation |
| 416 | Range Not Satisfiable | Invalid HTTP Range header |
| 500 | Internal Server Error | Server error (check logs) |

### Error Examples

**401 Unauthorized:**
```json
{
  "message": "Missing authorization header"
}
```

**403 Forbidden - Permission Denied:**
```json
{
  "message": "User does not have permission to access this recording"
}
```

**403 Forbidden - Compliance Hold:**
```json
{
  "message": "Cannot delete recording under compliance hold"
}
```

**404 Not Found:**
```json
{
  "message": "Recording not found"
}
```

**400 Bad Request - Validation Error:**
```json
{
  "message": "retention_days must be positive"
}
```

**409 Conflict:**
```json
{
  "message": "Retention policy already exists for this campaign"
}
```

---

## Best Practices

### 1. Efficient Searching

**Use Pagination:**
```javascript
// Good - paginated requests
async function getAllRecordings() {
  const limit = 100;
  let offset = 0;
  let allRecordings = [];

  while (true) {
    const batch = await client.searchRecordings({ limit, offset });
    if (batch.length === 0) break;

    allRecordings.push(...batch);
    offset += limit;
  }

  return allRecordings;
}
```

**Filter at the API Level:**
```javascript
// Good - filter server-side
const sales = await client.searchRecordings({
  disposition: 'sale',
  startDate: '2026-01-01T00:00:00Z'
});

// Bad - filter client-side
const all = await client.searchRecordings({ limit: 10000 });
const sales = all.filter(r => r.metadata?.disposition === 'sale');
```

---

### 2. Downloading Large Files

**Use Streaming for Large Files:**
```python
# Python - stream to disk
def download_recording(client, recording_id, output_path):
    response = client.session.get(
        f'{client.base_url}/api/recordings/{recording_id}/download',
        stream=True  # Important for large files
    )

    with open(output_path, 'wb') as f:
        for chunk in response.iter_content(chunk_size=1024*1024):  # 1MB chunks
            if chunk:
                f.write(chunk)
```

**Resume Interrupted Downloads:**
```javascript
// JavaScript - resume with Range header
async function downloadWithResume(recordingId, filePath) {
  const existingSize = await getFileSize(filePath);

  const response = await fetch(`/api/recordings/${recordingId}/download`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Range': `bytes=${existingSize}-`  // Resume from existing size
    }
  });

  // Append to existing file
  await appendToFile(filePath, await response.blob());
}
```

---

### 3. Compliance Hold Management

**Set Holds Immediately for Legal Cases:**
```python
# Immediately protect recordings related to legal cases
def protect_legal_recordings(client, lead_id):
    recordings = client.search_recordings(lead_id=lead_id)

    for recording in recordings:
        client.set_compliance_hold(recording['id'], True)
        print(f"Protected recording {recording['id']}")
```

**Batch Operations:**
```javascript
// Set compliance hold on multiple recordings
async function bulkSetComplianceHold(recordingIds, hold) {
  const results = await Promise.allSettled(
    recordingIds.map(id => client.setComplianceHold(id, hold))
  );

  const succeeded = results.filter(r => r.status === 'fulfilled').length;
  const failed = results.filter(r => r.status === 'rejected').length;

  console.log(`Set compliance hold: ${succeeded} succeeded, ${failed} failed`);
  return results;
}
```

---

### 4. Monitoring Storage

**Check Storage Regularly:**
```javascript
// Monitor storage usage and alert
async function checkStorageHealth() {
  const stats = await client.getStorageStats();

  if (stats.quotaPercentage > 90) {
    console.error('⚠️ CRITICAL: Storage above 90%');
    // Trigger alert
  } else if (stats.quotaPercentage > 80) {
    console.warn('⚠️ WARNING: Storage above 80%');
  } else {
    console.log(`✅ Storage OK: ${stats.quotaPercentage.toFixed(1)}%`);
  }

  return stats;
}
```

**Predict Storage Needs:**
```python
# Calculate storage growth trend
def predict_storage_exhaustion(stats):
    usage = stats['dailyUsage']
    if len(usage) < 7:
        return None

    # Calculate average daily growth
    recent = usage[:7]
    daily_growth = sum(r['totalSizeBytes'] for r in recent) / 7

    remaining_bytes = (stats['quotaGB'] * 1e9) - stats['totalSizeBytes']
    days_until_full = remaining_bytes / daily_growth if daily_growth > 0 else float('inf')

    return {
        'dailyGrowthGB': daily_growth / 1e9,
        'daysUntilFull': int(days_until_full),
        'projectedFullDate': (datetime.now() + timedelta(days=days_until_full)).isoformat()
    }
```

---

### 5. Error Handling

**Implement Retry Logic:**
```javascript
// Retry failed requests with exponential backoff
async function requestWithRetry(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.message.includes('401')) {
        // Don't retry auth errors
        throw error;
      }

      if (i === maxRetries - 1) {
        throw error;
      }

      // Exponential backoff: 1s, 2s, 4s
      const delay = Math.pow(2, i) * 1000;
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}

// Usage
const recordings = await requestWithRetry(() =>
  client.searchRecordings({ limit: 100 })
);
```

**Handle Specific Errors:**
```typescript
try {
  await client.deleteRecording(123);
} catch (error) {
  if (error.message.includes('compliance hold')) {
    console.error('Cannot delete: Recording is under compliance hold');
    // Show user message
  } else if (error.message.includes('Unauthorized')) {
    console.error('Session expired - please login again');
    // Redirect to login
  } else {
    console.error('Delete failed:', error);
    // Show generic error
  }
}
```

---

## Webhook Integration

### Recording Completion Events

Currently, the VoIP CRM system does not send webhooks for recording completion. However, external systems can detect new recordings using **polling** or by monitoring the **audit log**.

### Polling Strategy

**Recommended Approach:**

```javascript
// Poll for new recordings every 60 seconds
class RecordingMonitor {
  constructor(client, pollInterval = 60000) {
    this.client = client;
    this.pollInterval = pollInterval;
    this.lastCheck = new Date();
    this.callbacks = [];
  }

  onNewRecording(callback) {
    this.callbacks.push(callback);
  }

  async start() {
    setInterval(async () => {
      try {
        // Search for recordings created since last check
        const recordings = await this.client.searchRecordings({
          startDate: this.lastCheck.toISOString(),
          limit: 100
        });

        if (recordings.length > 0) {
          console.log(`Found ${recordings.length} new recordings`);

          // Notify all callbacks
          for (const callback of this.callbacks) {
            for (const recording of recordings) {
              await callback(recording);
            }
          }
        }

        this.lastCheck = new Date();
      } catch (error) {
        console.error('Polling error:', error);
      }
    }, this.pollInterval);
  }
}

// Usage
const monitor = new RecordingMonitor(client, 60000); // Check every 60s

monitor.onNewRecording(async (recording) => {
  console.log(`New recording: ${recording.id}`);

  // Process new recording
  if (recording.metadata?.disposition === 'sale') {
    // Send to QA system
    await sendToQASystem(recording);
  }
});

monitor.start();
```

### Future Webhook Support

Future versions may support webhook notifications for:
- Recording completion
- Retention policy expiration
- Storage quota warnings
- Compliance hold changes

**Proposed Webhook Format:**

```json
{
  "event": "recording.completed",
  "timestamp": "2026-01-15T14:30:00Z",
  "data": {
    "recordingId": 123,
    "callId": 456,
    "agentId": 10,
    "campaignId": 5,
    "duration": 180,
    "fileSize": 2048000,
    "disposition": "sale"
  }
}
```

**To request webhook support,** contact the development team or submit a feature request.

---

## Rate Limiting & Performance

### Current Limits

The API currently does **not enforce rate limits**, but best practices should be followed to avoid overloading the server.

### Recommended Limits

| Operation | Recommended Max |
|-----------|----------------|
| Search requests | 60/minute |
| Recording downloads | 30/minute |
| Policy updates | 10/minute |
| Bulk operations | 1000 recordings/batch |

### Performance Tips

**1. Use Pagination:**
```javascript
// Good - paginated
const batch1 = await client.searchRecordings({ limit: 100, offset: 0 });
const batch2 = await client.searchRecordings({ limit: 100, offset: 100 });

// Bad - requesting thousands at once
const all = await client.searchRecordings({ limit: 10000 });
```

**2. Cache Retention Policies:**
```javascript
// Cache policies to avoid repeated requests
let policiesCache = null;
let cacheTime = 0;
const CACHE_TTL = 5 * 60 * 1000; // 5 minutes

async function getRetentionPolicies() {
  if (policiesCache && Date.now() - cacheTime < CACHE_TTL) {
    return policiesCache;
  }

  policiesCache = await client.getRetentionPolicies();
  cacheTime = Date.now();
  return policiesCache;
}
```

**3. Parallel Requests:**
```javascript
// Download multiple recordings in parallel
async function downloadMultiple(recordingIds) {
  const downloads = recordingIds.map(id =>
    client.downloadRecording(id)
  );

  // Limit concurrency to avoid overwhelming server
  const results = [];
  for (let i = 0; i < downloads.length; i += 5) {
    const batch = await Promise.all(downloads.slice(i, i + 5));
    results.push(...batch);
  }

  return results;
}
```

**4. Streaming Downloads:**
```javascript
// Use /stream endpoint for in-browser playback
// This is more efficient than downloading and then playing
audioElement.src = client.getStreamURL(123);

// Use /download endpoint only for actual file downloads
const blob = await client.downloadRecording(123);
```

---

## Summary

### Quick Reference

| Endpoint | Method | Auth | Purpose |
|----------|--------|------|---------|
| `/api/recordings` | GET | ✅ | Search recordings |
| `/api/recordings/{id}` | GET | ✅ | Get recording details |
| `/api/recordings/{id}/download` | GET | ✅ | Download recording |
| `/api/recordings/{id}/stream` | GET | ✅ | Stream recording |
| `/api/recordings/{id}` | DELETE | ✅ Supervisor+ | Delete recording |
| `/api/recordings/{id}/compliance-hold` | PUT | ✅ Supervisor+ | Set compliance hold |
| `/api/retention-policies` | GET | ✅ Supervisor+ | List policies |
| `/api/retention-policies` | POST | ✅ Supervisor+ | Create policy |
| `/api/retention-policies/{id}` | PUT | ✅ Supervisor+ | Update policy |
| `/api/retention-policies/{id}` | DELETE | ✅ Supervisor+ | Delete policy |
| `/api/recordings/storage/stats` | GET | ✅ Supervisor+ | Storage statistics |

### Key Points

- **All endpoints require JWT authentication**
- **Agents can only access their own recordings**
- **Supervisors and Admins can access all recordings**
- **Compliance holds prevent deletion**
- **Recordings are encrypted at rest with AES-256-GCM**
- **HTTP Range requests supported for streaming**
- **All access is logged to audit trail**

### Integration Checklist

- [ ] Obtain JWT token via `/api/auth/login`
- [ ] Test recording search with filters
- [ ] Verify permission model (agent vs supervisor)
- [ ] Test download with HTTP Range support
- [ ] Implement compliance hold workflow
- [ ] Configure retention policies
- [ ] Monitor storage usage
- [ ] Implement error handling and retries
- [ ] Set up polling for new recordings (if needed)
- [ ] Review audit logs for compliance

---

**Document Version:** 1.0
**Last Updated:** 2026-01-15
**Maintained By:** VoIP CRM Development Team
**Related Task:** 002-call-recording-and-playback

**Need Help?**
- **Authentication:** See [API-AUTHENTICATION.md](./API-AUTHENTICATION.md)
- **System Setup:** See README.md for environment configuration
- **Support:** Contact development team for assistance
