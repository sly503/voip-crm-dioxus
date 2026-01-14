# Manual Test Plan: Edge Cases and Validation

**Date:** 2026-01-14
**Subtask:** subtask-9.3
**Tester:** Auto-Claude

## Test Environment Setup

### Prerequisites
1. **Database**: PostgreSQL running with migrations applied
2. **SMTP Configuration**: Valid SMTP credentials in `.env` file
3. **Application**: Backend server running on http://localhost:3000
4. **Email Access**: Ability to check emails sent to test addresses
5. **Test Accounts**:
   - At least one verified agent account
   - At least one verified supervisor account
   - At least one verified admin account

### Required Environment Variables
```bash
# Add to .env file before testing
SMTP_HOST=smtp.gmail.com  # or your SMTP provider
SMTP_PORT=587
SMTP_USERNAME=your-email@example.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=noreply@voipcrm.local
SMTP_FROM_NAME=VoIP CRM
APP_URL=http://localhost:3000
REGISTRATION_ENABLED=true
```

### Database Migrations
Ensure migration `003_email_verification.sql` has been applied:
```bash
# Check if tables exist
psql $DATABASE_URL -c "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name IN ('users', 'verification_tokens', 'invitations');"
```

---

## Test Cases

### Test Case 1: Duplicate Email Registration Prevention
**Objective:** Verify that the system prevents registration with an email that already exists

**Steps:**
1. Navigate to registration page: `http://localhost:3000/register`
2. Fill in the form with an email that already exists in the system:
   - Email: `existing-user@example.com` (use an email from database)
   - Username: `newusername123`
   - First Name: `Duplicate`
   - Last Name: `Test`
   - Password: `DupTest123`
   - Confirm Password: `DupTest123`
3. Click "Create Account" button
4. Observe the error message

**Expected Results:**
- ✓ Registration fails with error message
- ✓ Error message indicates email already exists (e.g., "Email already registered" or similar)
- ✓ No new user is created in the database
- ✓ No verification email is sent
- ✓ HTTP status code 409 (Conflict) or 400 (Bad Request) is returned

**Database Verification:**
```sql
-- Count users with this email (should be 1, not 2)
SELECT COUNT(*) as count FROM users WHERE email = 'existing-user@example.com';

-- Check that no new user was created
SELECT username, email, created_at FROM users
WHERE email = 'existing-user@example.com'
ORDER BY created_at DESC;
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 2: Duplicate Username Registration Prevention
**Objective:** Verify that the system prevents registration with a username that already exists

**Steps:**
1. Navigate to registration page: `http://localhost:3000/register`
2. Fill in the form with a username that already exists in the system:
   - Email: `newemail@example.com`
   - Username: `existingusername` (use a username from database)
   - First Name: `Duplicate`
   - Last Name: `Test`
   - Password: `DupTest123`
   - Confirm Password: `DupTest123`
3. Click "Create Account" button
4. Observe the error message

**Expected Results:**
- ✓ Registration fails with error message
- ✓ Error message indicates username already exists (e.g., "Username already taken" or similar)
- ✓ No new user is created in the database
- ✓ No verification email is sent
- ✓ HTTP status code 409 (Conflict) or 400 (Bad Request) is returned

**Database Verification:**
```sql
-- Count users with this username (should be 1, not 2)
SELECT COUNT(*) as count FROM users WHERE username = 'existingusername';

-- Check that no new user was created with the new email
SELECT username, email, created_at FROM users
WHERE email = 'newemail@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 3: Weak Password Rejection - Client-Side
**Objective:** Verify that client-side validation rejects weak passwords

#### Test 3.1: Password Too Short (< 8 characters)
**Steps:**
1. Navigate to registration page
2. Fill in the form with a password that is too short:
   - Email: `shortpass@example.com`
   - Username: `shortpassuser`
   - First Name: `Short`
   - Last Name: `Pass`
   - Password: `Test123` (7 characters)
   - Confirm Password: `Test123`
3. Click "Create Account" button

**Expected Results:**
- ✓ Error message appears: "Password must be at least 8 characters and contain letters and numbers"
- ✓ Form is not submitted to the server
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 3.2: Password Without Letters
**Steps:**
1. Navigate to registration page
2. Fill in the form with a password that contains only numbers:
   - Email: `numpass@example.com`
   - Username: `numpassuser`
   - First Name: `Num`
   - Last Name: `Pass`
   - Password: `12345678` (no letters)
   - Confirm Password: `12345678`
3. Click "Create Account" button

**Expected Results:**
- ✓ Error message appears: "Password must be at least 8 characters and contain letters and numbers"
- ✓ Form is not submitted to the server
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 3.3: Password Without Numbers
**Steps:**
1. Navigate to registration page
2. Fill in the form with a password that contains only letters:
   - Email: `letterpass@example.com`
   - Username: `letterpassuser`
   - First Name: `Letter`
   - Last Name: `Pass`
   - Password: `TestPassword` (no numbers)
   - Confirm Password: `TestPassword`
3. Click "Create Account" button

**Expected Results:**
- ✓ Error message appears: "Password must be at least 8 characters and contain letters and numbers"
- ✓ Form is not submitted to the server
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 4: Invalid Email Format Rejection - Client-Side
**Objective:** Verify that client-side validation rejects invalid email formats

#### Test 4.1: Email Without @ Symbol
**Steps:**
1. Navigate to registration page
2. Fill in the form with an email without @ symbol:
   - Email: `notanemail.com`
   - Username: `testuser`
   - First Name: `Test`
   - Last Name: `User`
   - Password: `TestPass123`
   - Confirm Password: `TestPass123`
3. Click "Create Account" button

**Expected Results:**
- ✓ Error message appears: "Invalid email format"
- ✓ Form is not submitted to the server
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 4.2: Email Without Domain
**Steps:**
1. Navigate to registration page
2. Fill in the form with an email without domain:
   - Email: `test@`
   - Username: `testuser2`
   - First Name: `Test`
   - Last Name: `User`
   - Password: `TestPass123`
   - Confirm Password: `TestPass123`
3. Click "Create Account" button

**Expected Results:**
- ✓ Error message appears: "Invalid email format"
- ✓ Form is not submitted to the server
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 4.3: Email Without Username
**Steps:**
1. Navigate to registration page
2. Fill in the form with an email without username:
   - Email: `@example.com`
   - Username: `testuser3`
   - First Name: `Test`
   - Last Name: `User`
   - Password: `TestPass123`
   - Confirm Password: `TestPass123`
3. Click "Create Account" button

**Expected Results:**
- ✓ Error message appears: "Invalid email format"
- ✓ Form is not submitted to the server
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 5: Expired Verification Token Handling
**Objective:** Verify that expired verification tokens are handled properly

**Steps:**
1. Create a user with an expired verification token directly in the database:
   ```sql
   -- Insert a user
   INSERT INTO users (username, email, first_name, last_name, password_hash, role, email_verified)
   VALUES ('expiredtokenuser', 'expiredtoken@example.com', 'Expired', 'Token', '$2b$12$dummyhash', 'Agent', false)
   RETURNING id;

   -- Insert an expired verification token (expires 25 hours ago)
   INSERT INTO verification_tokens (token, user_id, email, expires_at, created_at)
   VALUES (
     'expired-token-12345',
     (SELECT id FROM users WHERE username = 'expiredtokenuser'),
     'expiredtoken@example.com',
     NOW() - INTERVAL '25 hours',
     NOW() - INTERVAL '26 hours'
   );
   ```
2. Navigate to: `http://localhost:3000/verify-email?token=expired-token-12345`
3. Observe the error message and UI

**Expected Results:**
- ✓ Error message displayed: "Verification link has expired" or similar
- ✓ User email remains unverified in database (email_verified = false)
- ✓ Token is NOT marked as used
- ✓ Resend verification form is displayed
- ✓ User can request a new verification email
- ✓ New token has fresh expiration time (24 hours from creation)

**Database Verification:**
```sql
-- Check user verification status (should still be false)
SELECT email, email_verified FROM users WHERE username = 'expiredtokenuser';

-- Check token status (should not be marked as used)
SELECT token, expires_at, used_at,
       CASE WHEN expires_at < NOW() THEN 'Expired' ELSE 'Valid' END as status
FROM verification_tokens
WHERE email = 'expiredtoken@example.com'
ORDER BY created_at DESC;
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 6: Expired Invitation Token Handling
**Objective:** Verify that expired invitation tokens are handled properly

**Steps:**
1. Create an expired invitation directly in the database:
   ```sql
   -- Insert an expired invitation (expires 8 days ago)
   INSERT INTO invitations (token, email, role, invited_by, expires_at, created_at)
   VALUES (
     'expired-invitation-12345',
     'expiredinvite@example.com',
     'Agent',
     (SELECT id FROM users WHERE role IN ('Supervisor', 'Admin') LIMIT 1),
     NOW() - INTERVAL '8 days',
     NOW() - INTERVAL '9 days'
   )
   RETURNING token;
   ```
2. Navigate to: `http://localhost:3000/accept-invitation?token=expired-invitation-12345`
3. Observe the error message and UI

**Expected Results:**
- ✓ Error message displayed: "Invitation has expired" or similar
- ✓ Registration form is NOT displayed
- ✓ Helpful message suggests contacting supervisor for new invitation
- ✓ No user can be created
- ✓ Invitation remains unused in database (used_at = NULL)

**Database Verification:**
```sql
-- Check invitation status (should be expired and unused)
SELECT token, email, expires_at, used_at,
       CASE
         WHEN used_at IS NOT NULL THEN 'Used'
         WHEN expires_at < NOW() THEN 'Expired'
         ELSE 'Valid'
       END as status
FROM invitations
WHERE email = 'expiredinvite@example.com';

-- Verify no user was created
SELECT COUNT(*) as count FROM users WHERE email = 'expiredinvite@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 7: Non-Supervisor Cannot Send Invitations
**Objective:** Verify that users with Agent role cannot send invitations

**Steps:**
1. Login with an Agent role account:
   - Navigate to `http://localhost:3000`
   - Login with credentials:
     - Username: `agent1` (or any agent account)
     - Password: `AgentPass123`
2. Navigate to the Agents page: `http://localhost:3000/agents`
3. Observe the UI - "Invite User" button should not be visible (or disabled)
4. Attempt to send an invitation via API directly (using browser console or API client):
   ```javascript
   // In browser console
   fetch('http://localhost:3000/api/auth/invite', {
     method: 'POST',
     headers: {
       'Content-Type': 'application/json',
       'Authorization': 'Bearer ' + localStorage.getItem('token')
     },
     body: JSON.stringify({
       email: 'test-agent-invite@example.com',
       role: 'Agent'
     })
   }).then(r => r.json()).then(console.log);
   ```
5. Observe the API response

**Expected Results:**
- ✓ UI: "Invite User" button is not visible or is disabled for Agent users
- ✓ API call returns 403 Forbidden status
- ✓ Error message indicates insufficient permissions
- ✓ No invitation is created in the database
- ✓ No invitation email is sent

**Database Verification:**
```sql
-- Verify no invitation was created
SELECT COUNT(*) as count FROM invitations
WHERE email = 'test-agent-invite@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 8: Supervisor Can Send Invitations
**Objective:** Verify that users with Supervisor role CAN send invitations

**Steps:**
1. Login with a Supervisor role account:
   - Navigate to `http://localhost:3000`
   - Login with credentials:
     - Username: `supervisor1` (or any supervisor account)
     - Password: `SuperPass123`
2. Navigate to the Agents page: `http://localhost:3000/agents`
3. Observe the UI - "Invite User" button should be visible
4. Click "Invite User" button
5. Fill in the invitation form:
   - Email: `supervisor-can-invite@example.com`
   - Role: Agent
6. Click "Send Invitation"
7. Observe the success message

**Expected Results:**
- ✓ UI: "Invite User" button is visible and enabled
- ✓ Dialog opens with invitation form
- ✓ Form submission succeeds
- ✓ Success message is displayed
- ✓ Invitation is created in the database
- ✓ Invitation email is sent
- ✓ Dialog closes after success

**Database Verification:**
```sql
-- Verify invitation was created
SELECT i.email, i.role, u.username as invited_by, i.expires_at, i.created_at
FROM invitations i
JOIN users u ON i.invited_by = u.id
WHERE i.email = 'supervisor-can-invite@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 9: Admin Can Send Invitations
**Objective:** Verify that users with Admin role CAN send invitations

**Steps:**
1. Login with an Admin role account:
   - Navigate to `http://localhost:3000`
   - Login with credentials:
     - Username: `admin` (or any admin account)
     - Password: `AdminPass123`
2. Navigate to the Agents page: `http://localhost:3000/agents`
3. Observe the UI - "Invite User" button should be visible
4. Click "Invite User" button
5. Fill in the invitation form:
   - Email: `admin-can-invite@example.com`
   - Role: Supervisor
6. Click "Send Invitation"
7. Observe the success message

**Expected Results:**
- ✓ UI: "Invite User" button is visible and enabled
- ✓ Dialog opens with invitation form
- ✓ Admin can invite Supervisors (not just Agents)
- ✓ Form submission succeeds
- ✓ Success message is displayed
- ✓ Invitation is created in the database
- ✓ Invitation email is sent

**Database Verification:**
```sql
-- Verify invitation was created with Supervisor role
SELECT i.email, i.role, u.username as invited_by, i.expires_at, i.created_at
FROM invitations i
JOIN users u ON i.invited_by = u.id
WHERE i.email = 'admin-can-invite@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 10: Admin Bypass Email Verification
**Objective:** Verify that Admin users can bypass email verification if needed

**Note:** This test assumes that Admin users created manually (e.g., via database seeding) should be able to bypass email verification.

**Steps:**
1. Create an Admin user directly in the database with email_verified = true:
   ```sql
   INSERT INTO users (username, email, first_name, last_name, password_hash, role, email_verified)
   VALUES (
     'adminbypass',
     'adminbypass@example.com',
     'Admin',
     'Bypass',
     '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5NU7TW9fVWFsS',  -- password: "AdminPass123"
     'Admin',
     false  -- Not verified initially
   )
   RETURNING id;
   ```
2. Navigate to login page: `http://localhost:3000`
3. Attempt to login with:
   - Username: `adminbypass`
   - Password: `AdminPass123`
4. Observe if login succeeds or fails

**Expected Results (Design Decision Needed):**

**Option A: Admins Bypass Verification**
- ✓ Login succeeds even though email_verified = false
- ✓ Admin is logged in and can access the dashboard
- ✓ No error message about unverified email

**Option B: Admins Still Require Verification (Recommended)**
- ✓ Login fails with unverified email error
- ✓ Admin must verify email before logging in
- ✓ This is the current implementation based on code review

**Current Implementation Analysis:**
Based on the code in `src/server/auth/mod.rs`, the login handler checks:
```rust
if user.role != UserRole::Admin && !user.email_verified {
    return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "...".to_string() })));
}
```

This means **Admins CAN bypass email verification** in the current implementation.

**Testing Steps:**
1. Create admin with email_verified = false
2. Attempt login
3. Verify login succeeds for Admin even without verification

**Database Verification:**
```sql
-- Check admin user status
SELECT username, email, role, email_verified FROM users WHERE username = 'adminbypass';

-- Test login success even with email_verified = false
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 11: Used Verification Token Cannot Be Reused
**Objective:** Verify that verification tokens can only be used once

**Steps:**
1. Register a new user: `usedtokentest@example.com`
2. Check email and copy the verification link
3. Click the verification link - verification should succeed
4. Try to use the same verification link again (open in new incognito window or clear session)
5. Navigate to the same verification URL
6. Observe the error message

**Expected Results:**
- ✓ First use: Verification succeeds
- ✓ Second use: Error message displayed (e.g., "This verification link has already been used")
- ✓ No ability to verify again with the same token
- ✓ Token is marked as used in database (used_at IS NOT NULL)
- ✓ Resend verification form may be shown (but not needed since already verified)

**Database Verification:**
```sql
-- Check token status (should show used_at timestamp)
SELECT token, email, expires_at, used_at,
       CASE
         WHEN used_at IS NOT NULL THEN 'Already Used'
         WHEN expires_at < NOW() THEN 'Expired'
         ELSE 'Valid'
       END as status
FROM verification_tokens
WHERE email = 'usedtokentest@example.com'
ORDER BY created_at DESC;

-- Check user is verified
SELECT email, email_verified FROM users WHERE email = 'usedtokentest@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 12: Used Invitation Cannot Be Reused
**Objective:** Verify that invitation tokens can only be used once

**Steps:**
1. Login as Supervisor
2. Send an invitation to: `usedinvitetest@example.com`
3. Check email and copy the invitation link
4. Click the invitation link and complete registration
5. Try to use the same invitation link again (open in new incognito window)
6. Navigate to the same invitation URL
7. Observe the error message

**Expected Results:**
- ✓ First use: Registration succeeds, user created
- ✓ Second use: Error message displayed (e.g., "This invitation has already been used")
- ✓ Registration form is NOT displayed on second use
- ✓ No ability to create another user with the same invitation
- ✓ Invitation is marked as used in database (used_at IS NOT NULL, used_by = user_id)

**Database Verification:**
```sql
-- Check invitation status (should show used_at timestamp and used_by user_id)
SELECT i.token, i.email, i.expires_at, i.used_at, i.used_by, u.username,
       CASE
         WHEN i.used_at IS NOT NULL THEN 'Already Used'
         WHEN i.expires_at < NOW() THEN 'Expired'
         ELSE 'Valid'
       END as status
FROM invitations i
LEFT JOIN users u ON i.used_by = u.id
WHERE i.email = 'usedinvitetest@example.com'
ORDER BY i.created_at DESC;

-- Check user was created
SELECT username, email, role, email_verified FROM users WHERE email = 'usedinvitetest@example.com';
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 13: Invalid Token Format Handling
**Objective:** Verify that invalid or malformed tokens are handled gracefully

#### Test 13.1: Invalid Verification Token
**Steps:**
1. Navigate to: `http://localhost:3000/verify-email?token=invalid-token-format-12345`
2. Observe the error message

**Expected Results:**
- ✓ Error message displayed (e.g., "Invalid verification link")
- ✓ No error thrown in browser console
- ✓ Resend verification form is displayed
- ✓ Application remains stable

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 13.2: Invalid Invitation Token
**Steps:**
1. Navigate to: `http://localhost:3000/accept-invitation?token=invalid-invitation-12345`
2. Observe the error message

**Expected Results:**
- ✓ Error message displayed (e.g., "Invalid invitation link")
- ✓ No error thrown in browser console
- ✓ Registration form is NOT displayed
- ✓ Helpful message suggests contacting supervisor
- ✓ Application remains stable

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 13.3: Empty Token Parameter
**Steps:**
1. Navigate to: `http://localhost:3000/verify-email` (no token parameter)
2. Observe the error message
3. Navigate to: `http://localhost:3000/accept-invitation` (no token parameter)
4. Observe the error message

**Expected Results:**
- ✓ Both pages handle missing token gracefully
- ✓ Appropriate error messages displayed
- ✓ No application crash or blank page
- ✓ User can navigate back to login/register

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 14: SQL Injection Prevention
**Objective:** Verify that the application is protected against SQL injection attacks

#### Test 14.1: SQL Injection in Email Field
**Steps:**
1. Navigate to registration page
2. Attempt registration with SQL injection in email field:
   - Email: `'; DROP TABLE users; --@example.com`
   - Username: `sqltest1`
   - First Name: `SQL`
   - Last Name: `Test`
   - Password: `SqlTest123`
   - Confirm Password: `SqlTest123`
3. Click "Create Account"
4. Check database to ensure no damage

**Expected Results:**
- ✓ Registration either fails validation (invalid email) or creates user safely
- ✓ No SQL injection occurs
- ✓ All database tables remain intact
- ✓ Application uses parameterized queries (sqlx) to prevent injection

**Database Verification:**
```sql
-- Verify tables still exist
SELECT table_name FROM information_schema.tables
WHERE table_schema = 'public'
ORDER BY table_name;

-- Check if user was created (should not be created due to email validation)
SELECT COUNT(*) as count FROM users WHERE username = 'sqltest1';
```

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 14.2: SQL Injection in Username Field
**Steps:**
1. Navigate to registration page
2. Attempt registration with SQL injection in username field:
   - Email: `sqltest2@example.com`
   - Username: `admin' OR '1'='1`
   - First Name: `SQL`
   - Last Name: `Test`
   - Password: `SqlTest123`
   - Confirm Password: `SqlTest123`
3. Click "Create Account"

**Expected Results:**
- ✓ Registration processes normally (username is stored as-is, no injection)
- ✓ No SQL injection occurs
- ✓ sqlx parameterized queries prevent injection
- ✓ Login with exact username works normally

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 15: Rate Limiting - Resend Verification
**Objective:** Verify that rate limiting works for resend verification requests

**Steps:**
1. Register a user: `ratelimit@example.com` (do not verify)
2. Attempt to login - observe unverified email error
3. Use the resend verification form to request new verification email
4. Immediately request resend again (2nd time)
5. Immediately request resend again (3rd time)
6. Immediately request resend again (4th time - should fail)
7. Observe the error message

**Expected Results:**
- ✓ First 3 resend requests succeed
- ✓ 4th resend request fails with rate limit error
- ✓ Error message indicates too many requests (e.g., "Too many verification emails sent. Please try again later.")
- ✓ Database shows exactly 4 tokens created (1 original + 3 resends)
- ✓ After 1 hour, user can request resend again

**Database Verification:**
```sql
-- Count verification tokens created in last hour
SELECT email, COUNT(*) as token_count
FROM verification_tokens
WHERE email = 'ratelimit@example.com'
  AND created_at > NOW() - INTERVAL '1 hour'
GROUP BY email;

-- View all tokens with timestamps
SELECT token, email, created_at, expires_at, used_at
FROM verification_tokens
WHERE email = 'ratelimit@example.com'
ORDER BY created_at DESC;
```

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 16: Password Mismatch Validation
**Objective:** Verify that password confirmation field is validated

**Steps:**
1. Navigate to registration page
2. Fill in the form with mismatched passwords:
   - Email: `mismatch@example.com`
   - Username: `mismatchuser`
   - First Name: `Mismatch`
   - Last Name: `Test`
   - Password: `TestPass123`
   - Confirm Password: `DifferentPass456`
3. Click "Create Account"

**Expected Results:**
- ✓ Error message displayed: "Passwords do not match"
- ✓ Form is not submitted
- ✓ No API call is made
- ✓ No user is created

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 17: Empty Fields Validation
**Objective:** Verify that all required fields are validated

#### Test 17.1: Empty Email
**Steps:**
1. Fill all fields except email
2. Submit form

**Expected Results:**
- ✓ Error message: "All fields are required"
- ✓ Form not submitted

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 17.2: Empty Username
**Steps:**
1. Fill all fields except username
2. Submit form

**Expected Results:**
- ✓ Error message: "All fields are required"
- ✓ Form not submitted

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 17.3: Empty First Name
**Steps:**
1. Fill all fields except first name
2. Submit form

**Expected Results:**
- ✓ Error message: "All fields are required"
- ✓ Form not submitted

**Status:** ⏳ PENDING MANUAL TEST

---

#### Test 17.4: Empty Last Name
**Steps:**
1. Fill all fields except last name
2. Submit form

**Expected Results:**
- ✓ Error message: "All fields are required"
- ✓ Form not submitted

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 18: Whitespace-Only Fields Validation
**Objective:** Verify that fields with only whitespace are rejected

**Steps:**
1. Navigate to registration page
2. Fill in the form with whitespace-only values:
   - Email: `whitespace@example.com`
   - Username: `   ` (spaces only)
   - First Name: `Test`
   - Last Name: `User`
   - Password: `TestPass123`
   - Confirm Password: `TestPass123`
3. Click "Create Account"

**Expected Results:**
- ✓ Client-side validation catches whitespace-only fields
- ✓ Error message displayed (e.g., "All fields are required" or "Invalid username")
- ✓ Form is not submitted OR
- ✓ Backend validation rejects the request if client-side passes

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 19: XSS Prevention in Input Fields
**Objective:** Verify that the application is protected against XSS attacks

**Steps:**
1. Navigate to registration page
2. Attempt registration with XSS payload in first name:
   - Email: `xsstest@example.com`
   - Username: `xsstestuser`
   - First Name: `<script>alert('XSS')</script>`
   - Last Name: `Test`
   - Password: `TestPass123`
   - Confirm Password: `TestPass123`
3. Complete registration and login
4. Navigate to a page that displays the user's name
5. Observe if the script executes

**Expected Results:**
- ✓ Registration succeeds (or sanitized)
- ✓ Script does NOT execute
- ✓ First name is displayed as plain text: `<script>alert('XSS')</script>` OR sanitized
- ✓ No alert popup appears
- ✓ Dioxus framework provides automatic XSS protection

**Status:** ⏳ PENDING MANUAL TEST

---

### Test Case 20: CSRF Protection Verification
**Objective:** Verify that the application has CSRF protection for state-changing operations

**Note:** This test checks if the application properly uses CSRF tokens or SameSite cookies.

**Steps:**
1. Login to the application
2. Open browser developer tools → Application/Storage → Cookies
3. Check the JWT token cookie settings
4. Verify cookie attributes

**Expected Results:**
- ✓ JWT token is stored securely (localStorage or httpOnly cookie)
- ✓ If using cookies: SameSite=Lax or SameSite=Strict attribute is set
- ✓ If using cookies: Secure flag is set (for HTTPS)
- ✓ API endpoints validate the Authorization header
- ✓ External sites cannot make authenticated requests on behalf of the user

**Current Implementation:**
Based on code review, the JWT token is stored in localStorage and sent via Authorization header, which provides CSRF protection by design (cookies are not used for authentication).

**Status:** ⏳ PENDING MANUAL TEST

---

## Test Summary

### Acceptance Criteria Coverage

| Criterion | Test Cases | Status |
|-----------|------------|--------|
| Duplicate email registration prevented | TC1 | ⏳ |
| Duplicate username registration prevented | TC2 | ⏳ |
| Weak passwords rejected | TC3.1, TC3.2, TC3.3 | ⏳ |
| Invalid email format rejected | TC4.1, TC4.2, TC4.3 | ⏳ |
| Expired tokens handled properly | TC5, TC6 | ⏳ |
| Non-supervisors cannot send invitations | TC7 | ⏳ |
| Admins can bypass email verification | TC10 | ⏳ |

### Additional Edge Cases Tested

- Used tokens cannot be reused (TC11, TC12)
- Invalid token format handling (TC13.1, TC13.2, TC13.3)
- SQL injection prevention (TC14.1, TC14.2)
- Rate limiting (TC15)
- Password mismatch validation (TC16)
- Empty fields validation (TC17.1-17.4)
- Whitespace-only fields (TC18)
- XSS prevention (TC19)
- CSRF protection (TC20)

---

## Test Execution Guide

### 1. Pre-Test Database Setup

```bash
# Connect to database
psql $DATABASE_URL

# Create test accounts if they don't exist
-- Agent account
INSERT INTO users (username, email, first_name, last_name, password_hash, role, email_verified)
VALUES ('agent1', 'agent@voipcrm.local', 'Test', 'Agent', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5NU7TW9fVWFsS', 'Agent', true)
ON CONFLICT (username) DO NOTHING;

-- Supervisor account
INSERT INTO users (username, email, first_name, last_name, password_hash, role, email_verified)
VALUES ('supervisor1', 'supervisor@voipcrm.local', 'Test', 'Supervisor', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5NU7TW9fVWFsS', 'Supervisor', true)
ON CONFLICT (username) DO NOTHING;

-- Admin account
INSERT INTO users (username, email, first_name, last_name, password_hash, role, email_verified)
VALUES ('admin', 'admin@voipcrm.local', 'Test', 'Admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5NU7TW9fVWFsS', 'Admin', true)
ON CONFLICT (username) DO NOTHING;

-- All test accounts use password: "AdminPass123" or "AgentPass123" or "SuperPass123"
```

### 2. Test Execution Order

**Recommended execution order:**

1. **Validation Tests First** (TC3, TC4, TC16, TC17, TC18)
   - These are quick client-side tests
   - No database changes
   - Can be run rapidly

2. **Duplicate Prevention** (TC1, TC2)
   - Test with existing accounts
   - Requires database with existing users

3. **Token Expiration** (TC5, TC6)
   - Requires database manipulation
   - Clean up tokens after testing

4. **Permission Tests** (TC7, TC8, TC9)
   - Test with different role accounts
   - Verify authorization logic

5. **Admin Bypass** (TC10)
   - Important security test
   - Verify current implementation behavior

6. **Token Reuse Prevention** (TC11, TC12)
   - End-to-end flow tests
   - Requires email verification

7. **Error Handling** (TC13)
   - Test edge cases
   - Quick tests

8. **Security Tests** (TC14, TC19, TC20)
   - SQL injection, XSS, CSRF
   - Important for production readiness

9. **Rate Limiting** (TC15)
   - Time-sensitive test
   - May take several minutes

### 3. Test Data Cleanup

After testing, clean up test data:

```sql
-- Remove test users
DELETE FROM users WHERE email LIKE '%example.com';
DELETE FROM users WHERE email LIKE '%test%';

-- Remove test tokens
DELETE FROM verification_tokens WHERE email LIKE '%example.com';
DELETE FROM verification_tokens WHERE email LIKE '%test%';

-- Remove test invitations
DELETE FROM invitations WHERE email LIKE '%example.com';
DELETE FROM invitations WHERE email LIKE '%test%';
```

### 4. Test Logging

Create a test log file to track results:

```
# test-results-edge-cases.log

Date: 2026-01-14
Tester: [Your Name]
Environment: Local Development

TC1: Duplicate Email Prevention - ✅ PASS
TC2: Duplicate Username Prevention - ✅ PASS
TC3.1: Password Too Short - ✅ PASS
...
```

---

## Troubleshooting

### Issue: SMTP emails not received
**Solution:**
- Check SMTP configuration in `.env`
- Verify SMTP credentials are correct
- Check spam/junk folder
- Use a reliable SMTP provider (Gmail, SendGrid, etc.)
- Check server logs for email errors

### Issue: Database constraints fail
**Solution:**
- Ensure migrations are applied: `psql $DATABASE_URL -f migrations/003_email_verification.sql`
- Check for existing test data conflicts
- Clear test data before re-running tests

### Issue: Token expiration tests don't work
**Solution:**
- Verify database system time: `SELECT NOW();`
- Ensure intervals are calculated correctly in SQL
- Use proper PostgreSQL interval syntax: `INTERVAL '25 hours'`

### Issue: Permission tests fail
**Solution:**
- Verify test account roles in database
- Check JWT token contains correct role claim
- Ensure authorization middleware is active
- Check browser console for 403 errors

---

## Success Criteria

All test cases (TC1-TC20) must pass for subtask-9.3 to be considered complete.

**Checklist:**
- [ ] All validation tests pass (client-side and server-side)
- [ ] All duplicate prevention tests pass
- [ ] All token expiration tests pass
- [ ] All permission tests pass
- [ ] All security tests pass (SQL injection, XSS, CSRF)
- [ ] Rate limiting works correctly
- [ ] Error messages are user-friendly
- [ ] No console errors during testing
- [ ] Application remains stable under all edge cases
