# API Authentication Guide

**VoIP CRM API - JWT Authentication Documentation**

**Version:** 1.0
**Last Updated:** 2026-01-15
**Task:** 001-api-authentication-protection

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication Mechanism](#authentication-mechanism)
3. [Obtaining a JWT Token](#obtaining-a-jwt-token)
4. [Making Authenticated Requests](#making-authenticated-requests)
5. [Protected Routes](#protected-routes)
6. [Public Routes](#public-routes)
7. [Error Responses](#error-responses)
8. [Code Examples](#code-examples)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

---

## Overview

The VoIP CRM API uses **JSON Web Tokens (JWT)** for authentication. All API endpoints except for authentication, health check, and webhook routes require a valid JWT token in the `Authorization` header.

### Key Points

- **Authentication Type:** Bearer Token (JWT)
- **Token Location:** `Authorization` header
- **Token Format:** `Bearer <token>`
- **Protected Routes:** 49 out of 57 total routes
- **Token Expiration:** Configurable (typically 24 hours)
- **Unauthorized Response:** HTTP 401 with JSON error message

---

## Authentication Mechanism

### How It Works

1. **Login:** User submits credentials to `/api/auth/login`
2. **Token Issued:** Server validates credentials and returns a JWT token
3. **Subsequent Requests:** Client includes token in `Authorization` header
4. **Token Validation:** Server validates token signature and expiration
5. **Access Granted/Denied:** Valid token allows access; invalid token returns 401

### Token Structure

JWT tokens contain three parts (header.payload.signature):

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6InRlc3QiLCJyb2xlIjoiYWdlbnQiLCJleHAiOjE3MzcwMjE2MDB9.signature
```

**Payload includes:**
- `user_id` - Unique user identifier
- `username` - Username string
- `role` - User role (admin, supervisor, agent)
- `exp` - Expiration timestamp (Unix epoch)

---

## Obtaining a JWT Token

### Login Request

**Endpoint:** `POST /api/auth/login`
**Authentication:** None (public endpoint)

**Request Body:**
```json
{
  "username": "your_username",
  "password": "your_password"
}
```

**Success Response (200 OK):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user_id": 1,
  "username": "your_username",
  "role": "agent"
}
```

**Error Response (401 Unauthorized):**
```json
{
  "message": "Invalid credentials"
}
```

### cURL Example

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "john.doe",
    "password": "SecurePassword123!"
  }'
```

### Storing the Token

Once you receive the token from the login response:

1. **Frontend Apps:** Store in memory, sessionStorage, or localStorage
2. **Mobile Apps:** Store in secure storage (Keychain/Keystore)
3. **Server-to-Server:** Store in environment variables or secure configuration
4. **Never:** Commit tokens to version control or log them

---

## Making Authenticated Requests

### Header Format

All protected endpoints require the `Authorization` header:

```
Authorization: Bearer <your_jwt_token>
```

### cURL Example

```bash
# Store token in variable for reuse
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Make authenticated GET request
curl -X GET http://localhost:3000/api/leads \
  -H "Authorization: Bearer $TOKEN"

# Make authenticated POST request
curl -X POST http://localhost:3000/api/leads \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "John Smith",
    "phone": "+1-555-0123",
    "email": "john.smith@example.com"
  }'
```

### JavaScript Fetch Example

```javascript
const token = localStorage.getItem('jwt_token');

// GET request
const response = await fetch('http://localhost:3000/api/leads', {
  method: 'GET',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  }
});

// POST request
const createResponse = await fetch('http://localhost:3000/api/leads', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    name: 'John Smith',
    phone: '+1-555-0123',
    email: 'john.smith@example.com'
  })
});
```

### Axios Example

```javascript
import axios from 'axios';

// Configure axios instance with default headers
const api = axios.create({
  baseURL: 'http://localhost:3000',
  headers: {
    'Content-Type': 'application/json'
  }
});

// Add token to all requests via interceptor
api.interceptors.request.use(config => {
  const token = localStorage.getItem('jwt_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Make requests
const leads = await api.get('/api/leads');
const newLead = await api.post('/api/leads', {
  name: 'John Smith',
  phone: '+1-555-0123'
});
```

---

## Protected Routes

All routes listed below **require** a valid JWT token in the `Authorization` header.

### Auth Routes (1 route)

| Method | Route | Description |
|--------|-------|-------------|
| POST | `/api/auth/invite` | Invite new user to organization |

### Lead Management (8 routes)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/leads` | List all leads |
| POST | `/api/leads` | Create new lead |
| GET | `/api/leads/my` | Get leads assigned to current user |
| GET | `/api/leads/{id}` | Get specific lead details |
| PUT | `/api/leads/{id}` | Update lead information |
| DELETE | `/api/leads/{id}` | Delete lead |
| POST | `/api/leads/{id}/notes` | Add note to lead |
| PUT | `/api/leads/{id}/status` | Update lead status |
| PUT | `/api/leads/{id}/assign` | Assign lead to agent |

### Agent Management (5 routes)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/agents` | List all agents |
| POST | `/api/agents` | Create new agent |
| GET | `/api/agents/{id}` | Get specific agent details |
| PUT | `/api/agents/{id}` | Update agent information |
| PUT | `/api/agents/{id}/status` | Update agent status |

### Campaign Management (7 routes)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/campaigns` | List all campaigns |
| POST | `/api/campaigns` | Create new campaign |
| GET | `/api/campaigns/{id}` | Get campaign details |
| PUT | `/api/campaigns/{id}` | Update campaign |
| POST | `/api/campaigns/{id}/start` | Start campaign |
| POST | `/api/campaigns/{id}/pause` | Pause campaign |
| POST | `/api/campaigns/{id}/stop` | Stop campaign |

### Call Operations (7 routes)

| Method | Route | Description |
|--------|-------|-------------|
| POST | `/api/calls/dial` | Initiate outbound call |
| POST | `/api/calls/direct` | Direct dial to number |
| POST | `/api/calls/{id}/hangup` | Hang up active call |
| POST | `/api/calls/{id}/transfer` | Transfer call to another agent |
| POST | `/api/calls/{id}/hold` | Put call on hold |
| POST | `/api/calls/{id}/unhold` | Resume call from hold |
| GET | `/api/calls/{id}` | Get call details |

### Statistics & Reporting (3 routes)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/stats/realtime` | Get real-time statistics |
| GET | `/api/statistics/realtime` | Get real-time statistics (alternate) |
| GET | `/api/stats/agent/{id}` | Get agent-specific statistics |

### WebRTC Configuration (1 route)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/config/webrtc` | Get WebRTC configuration |

### SIP Trunk Operations (3 routes)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/sip/status` | Get SIP trunk status |
| POST | `/api/sip/dial` | Initiate SIP call |
| POST | `/api/sip/hangup` | Hang up SIP call |

### AI Settings (11 routes)

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/api/ai/settings` | Get all AI settings |
| GET | `/api/ai/settings/{agent_id}` | Get AI settings for agent |
| PUT | `/api/ai/settings/{agent_id}` | Update AI settings for agent |
| DELETE | `/api/ai/settings/{agent_id}` | Delete AI settings for agent |
| GET | `/api/ai/config` | Get global AI configuration |
| PUT | `/api/ai/config` | Update global AI configuration |
| GET | `/api/ai/templates` | List prompt templates |
| POST | `/api/ai/templates` | Create prompt template |
| GET | `/api/ai/templates/{id}` | Get prompt template |
| PUT | `/api/ai/templates/{id}` | Update prompt template |
| DELETE | `/api/ai/templates/{id}` | Delete prompt template |

### Campaign Automation (3 routes)

| Method | Route | Description |
|--------|-------|-------------|
| POST | `/api/campaigns/{id}/automation/start` | Start campaign automation |
| POST | `/api/campaigns/{id}/automation/stop` | Stop campaign automation |
| GET | `/api/campaigns/{id}/automation/status` | Get automation status |

**Total Protected Routes:** 49

---

## Public Routes

The following routes **do not require** authentication:

| Method | Route | Description | Purpose |
|--------|-------|-------------|---------|
| GET | `/api/health` | Health check endpoint | Monitoring |
| POST | `/api/auth/login` | User login | Obtain JWT token |
| POST | `/api/auth/register` | User registration | Create account |
| POST | `/api/auth/verify-email` | Email verification | Verify email address |
| POST | `/api/auth/resend-verification` | Resend verification email | Account activation |
| POST | `/api/auth/invitation-details` | Get invitation details | View invitation info |
| POST | `/api/auth/register-invitation` | Accept invitation | Join organization |
| POST | `/api/webhooks/telnyx` | Telnyx webhook handler | External integration |

**Total Public Routes:** 8

---

## Error Responses

### Missing Authorization Header

**HTTP Status:** `401 Unauthorized`

**Response:**
```json
{
  "message": "Missing authorization header"
}
```

**Common Causes:**
- Forgot to include `Authorization` header
- Misspelled header name
- Empty authorization header

**Solution:**
```bash
# ❌ Wrong - missing header
curl http://localhost:3000/api/leads

# ✅ Correct - includes Authorization header
curl http://localhost:3000/api/leads \
  -H "Authorization: Bearer your_token_here"
```

### Invalid Token

**HTTP Status:** `401 Unauthorized`

**Response:**
```json
{
  "message": "Invalid token"
}
```

**Common Causes:**
- Token signature doesn't match (wrong secret key)
- Token format is malformed
- Token has been tampered with
- Token has expired
- Wrong token type (not a valid JWT)

**Solutions:**
1. **Expired Token:** Request new token via `/api/auth/login`
2. **Invalid Format:** Ensure token is in correct JWT format
3. **Corrupted Token:** Get fresh token from login endpoint
4. **Wrong Environment:** Verify using correct API server/environment

### Expired Token

**HTTP Status:** `401 Unauthorized`

**Response:**
```json
{
  "message": "Invalid token"
}
```

**Note:** Expired tokens return the same error message as invalid tokens for security reasons.

**Solution:**
```javascript
// Detect 401 and re-authenticate
if (response.status === 401) {
  // Clear old token
  localStorage.removeItem('jwt_token');

  // Redirect to login or refresh token
  window.location.href = '/login';
}
```

### Malformed Authorization Header

**HTTP Status:** `401 Unauthorized`

**Response:**
```json
{
  "message": "Missing authorization header"
}
```

**Common Mistakes:**

```bash
# ❌ Wrong - missing "Bearer" prefix
curl -H "Authorization: your_token" http://localhost:3000/api/leads

# ❌ Wrong - incorrect prefix
curl -H "Authorization: Token your_token" http://localhost:3000/api/leads

# ✅ Correct - proper Bearer format
curl -H "Authorization: Bearer your_token" http://localhost:3000/api/leads
```

---

## Code Examples

### Complete Authentication Flow (JavaScript)

```javascript
class ApiClient {
  constructor(baseURL) {
    this.baseURL = baseURL;
    this.token = localStorage.getItem('jwt_token');
  }

  async login(username, password) {
    try {
      const response = await fetch(`${this.baseURL}/api/auth/login`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ username, password })
      });

      if (!response.ok) {
        throw new Error('Login failed');
      }

      const data = await response.json();
      this.token = data.token;
      localStorage.setItem('jwt_token', data.token);

      return data;
    } catch (error) {
      console.error('Login error:', error);
      throw error;
    }
  }

  async request(endpoint, options = {}) {
    if (!this.token) {
      throw new Error('Not authenticated. Please login first.');
    }

    const headers = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${this.token}`,
      ...options.headers
    };

    try {
      const response = await fetch(`${this.baseURL}${endpoint}`, {
        ...options,
        headers
      });

      // Handle 401 - token expired or invalid
      if (response.status === 401) {
        this.logout();
        throw new Error('Authentication failed. Please login again.');
      }

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return await response.json();
    } catch (error) {
      console.error('API request error:', error);
      throw error;
    }
  }

  logout() {
    this.token = null;
    localStorage.removeItem('jwt_token');
  }

  // Convenience methods
  async getLeads() {
    return this.request('/api/leads');
  }

  async createLead(leadData) {
    return this.request('/api/leads', {
      method: 'POST',
      body: JSON.stringify(leadData)
    });
  }

  async getCampaigns() {
    return this.request('/api/campaigns');
  }
}

// Usage
const api = new ApiClient('http://localhost:3000');

// Login
await api.login('john.doe', 'password123');

// Make authenticated requests
const leads = await api.getLeads();
const newLead = await api.createLead({
  name: 'Jane Smith',
  phone: '+1-555-0199'
});
```

### Python Example

```python
import requests
from typing import Optional

class VoIPCRMClient:
    def __init__(self, base_url: str):
        self.base_url = base_url
        self.token: Optional[str] = None

    def login(self, username: str, password: str) -> dict:
        """Authenticate and store JWT token"""
        response = requests.post(
            f"{self.base_url}/api/auth/login",
            json={"username": username, "password": password}
        )
        response.raise_for_status()

        data = response.json()
        self.token = data['token']
        return data

    def _headers(self) -> dict:
        """Get headers with authentication"""
        if not self.token:
            raise ValueError("Not authenticated. Call login() first.")

        return {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.token}"
        }

    def get_leads(self) -> list:
        """Get all leads"""
        response = requests.get(
            f"{self.base_url}/api/leads",
            headers=self._headers()
        )

        if response.status_code == 401:
            raise ValueError("Authentication failed. Token may be expired.")

        response.raise_for_status()
        return response.json()

    def create_lead(self, lead_data: dict) -> dict:
        """Create new lead"""
        response = requests.post(
            f"{self.base_url}/api/leads",
            headers=self._headers(),
            json=lead_data
        )

        if response.status_code == 401:
            raise ValueError("Authentication failed. Token may be expired.")

        response.raise_for_status()
        return response.json()

# Usage
client = VoIPCRMClient("http://localhost:3000")
client.login("john.doe", "password123")

leads = client.get_leads()
new_lead = client.create_lead({
    "name": "Jane Smith",
    "phone": "+1-555-0199",
    "email": "jane@example.com"
})
```

### Rust Example

```rust
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    user_id: i32,
    username: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct Lead {
    id: i32,
    name: String,
    phone: String,
    // ... other fields
}

struct VoIPCRMClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl VoIPCRMClient {
    fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token: None,
        }
    }

    async fn login(&mut self, username: String, password: String) -> Result<LoginResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/api/auth/login", self.base_url);
        let login_req = LoginRequest { username, password };

        let response = self.client
            .post(&url)
            .json(&login_req)
            .send()
            .await?;

        let login_resp: LoginResponse = response.json().await?;
        self.token = Some(login_resp.token.clone());

        Ok(login_resp)
    }

    async fn get_leads(&self) -> Result<Vec<Lead>, Box<dyn std::error::Error>> {
        let token = self.token.as_ref()
            .ok_or("Not authenticated")?;

        let url = format!("{}/api/leads", self.base_url);

        let response = self.client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await?;

        if response.status() == 401 {
            return Err("Authentication failed".into());
        }

        let leads: Vec<Lead> = response.json().await?;
        Ok(leads)
    }
}

// Usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = VoIPCRMClient::new("http://localhost:3000".to_string());

    client.login("john.doe".to_string(), "password123".to_string()).await?;

    let leads = client.get_leads().await?;
    println!("Leads: {:?}", leads);

    Ok(())
}
```

---

## Best Practices

### 1. Secure Token Storage

**✅ Do:**
- Store tokens in memory for short-lived sessions
- Use httpOnly cookies for web applications
- Use secure storage (Keychain/Keystore) for mobile apps
- Clear tokens on logout

**❌ Don't:**
- Store tokens in localStorage if XSS is a concern
- Log tokens to console or error tracking
- Commit tokens to version control
- Share tokens between users

### 2. Token Lifecycle Management

```javascript
// Implement token refresh before expiration
class TokenManager {
  constructor() {
    this.token = null;
    this.expiresAt = null;
  }

  setToken(token, expiresIn) {
    this.token = token;
    this.expiresAt = Date.now() + (expiresIn * 1000);
  }

  isExpired() {
    return this.expiresAt && Date.now() >= this.expiresAt;
  }

  shouldRefresh() {
    // Refresh if token expires in less than 5 minutes
    return this.expiresAt && (this.expiresAt - Date.now()) < 300000;
  }
}
```

### 3. Error Handling

```javascript
async function makeAuthenticatedRequest(endpoint, options) {
  try {
    const response = await fetch(endpoint, {
      ...options,
      headers: {
        'Authorization': `Bearer ${getToken()}`,
        ...options.headers
      }
    });

    if (response.status === 401) {
      // Token expired or invalid
      await refreshToken(); // or redirect to login
      return makeAuthenticatedRequest(endpoint, options); // retry
    }

    return await response.json();
  } catch (error) {
    console.error('Request failed:', error);
    throw error;
  }
}
```

### 4. Security Headers

Always use HTTPS in production:

```javascript
// ❌ Don't use HTTP in production
const apiUrl = 'http://api.example.com';

// ✅ Use HTTPS in production
const apiUrl = process.env.NODE_ENV === 'production'
  ? 'https://api.example.com'
  : 'http://localhost:3000';
```

### 5. Prevent Token Leakage

```javascript
// ✅ Good - don't log tokens
console.log('Making API request to /api/leads');

// ❌ Bad - logs contain sensitive data
console.log('Request headers:', {
  Authorization: `Bearer ${token}` // Token exposed in logs!
});

// ✅ Good - redact sensitive info
console.log('Request headers:', {
  Authorization: 'Bearer [REDACTED]'
});
```

---

## Troubleshooting

### Issue: Getting 401 on All Requests

**Symptoms:**
- All authenticated requests return 401
- Login works but subsequent requests fail

**Checklist:**
1. ✅ Verify token is being stored after login
2. ✅ Check `Authorization` header is included in requests
3. ✅ Ensure header format is `Bearer <token>` (note the space)
4. ✅ Verify token hasn't expired
5. ✅ Check for typos in header name (`Authorization` not `Authorisation`)

**Debug:**
```javascript
// Log request details
console.log('Token:', token ? 'Present' : 'Missing');
console.log('Headers:', {
  Authorization: token ? `Bearer ${token.substring(0, 20)}...` : 'Missing'
});
```

### Issue: Token Expiration

**Symptoms:**
- Requests work initially but fail after some time
- Error message: "Invalid token"

**Solutions:**

1. **Check token expiration:**
```javascript
function decodeToken(token) {
  const payload = token.split('.')[1];
  const decoded = JSON.parse(atob(payload));
  const expiresAt = new Date(decoded.exp * 1000);
  console.log('Token expires at:', expiresAt);
  return decoded;
}
```

2. **Implement automatic re-authentication:**
```javascript
api.interceptors.response.use(
  response => response,
  async error => {
    if (error.response?.status === 401) {
      // Token expired - re-login
      await api.login(savedUsername, savedPassword);
      // Retry original request
      return api.request(error.config);
    }
    return Promise.reject(error);
  }
);
```

### Issue: CORS Errors

**Symptoms:**
- Browser console shows CORS error
- Requests fail with network error
- Authorization header being stripped

**Solution:**
Ensure server has proper CORS configuration (this is already configured):

```rust
// Server-side CORS configuration (already in place)
.layer(
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
)
```

### Issue: Wrong Environment

**Symptoms:**
- Login works but other requests fail
- Inconsistent behavior

**Check:**
```javascript
// Verify all requests use same base URL
console.log('Login URL:', loginUrl);
console.log('API URL:', apiUrl);
// Should match!
```

### Issue: Special Characters in Token

**Symptoms:**
- Token works in Postman but not in browser
- Inconsistent authentication

**Solution:**
```javascript
// ❌ Don't modify the token
const token = response.token.trim(); // OK
const token = response.token.replace(/[^a-zA-Z0-9]/g, ''); // Bad!

// ✅ Use token exactly as received
const token = response.token;
```

---

## Summary

### Quick Reference

| Action | Endpoint | Auth Required | Method |
|--------|----------|---------------|--------|
| **Login** | `/api/auth/login` | ❌ No | POST |
| **Get Leads** | `/api/leads` | ✅ Yes | GET |
| **Create Lead** | `/api/leads` | ✅ Yes | POST |
| **Health Check** | `/api/health` | ❌ No | GET |

### Header Format

```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### Common Status Codes

| Code | Meaning | Action |
|------|---------|--------|
| 200 | Success | Continue |
| 401 | Unauthorized | Check token or re-login |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Check endpoint URL |
| 500 | Server Error | Contact support |

### Need Help?

- **Security Audit:** See `security-audit.md` for complete route listing
- **Testing:** See `authentication-verification.md` for test procedures
- **Token Validation:** See `expired-token-verification.md` for expiration testing

---

**Document Version:** 1.0
**Last Updated:** 2026-01-15
**Maintained By:** VoIP CRM Development Team
**Related Task:** 001-api-authentication-protection
