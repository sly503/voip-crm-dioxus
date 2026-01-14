# Self-Registration Flow - Quick Test Checklist

## Pre-Test Setup
- [ ] PostgreSQL running
- [ ] Migrations applied (check: `psql $DATABASE_URL -c "\dt"`)
- [ ] SMTP configured in `.env`
- [ ] App running on http://localhost:3000
- [ ] Test email account accessible

## Quick Test Steps

### 1. Registration Page Access ✓
- [ ] Navigate to /register
- [ ] All form fields visible
- [ ] UI matches design

### 2. Form Validation ✓
- [ ] Empty form → error
- [ ] Invalid email → error
- [ ] Weak password → error
- [ ] Password mismatch → error

### 3. Successful Registration ✓
- [ ] Fill form with valid data
- [ ] Submit succeeds
- [ ] Success message shown
- [ ] DB: User created with email_verified=false
- [ ] DB: Verification token created

### 4. Email Delivery ✓
- [ ] Email received in inbox
- [ ] Correct from address
- [ ] Correct subject line
- [ ] HTML formatting looks good
- [ ] Verification link present and correct

### 5. Email Verification ✓
- [ ] Click verification link
- [ ] Page loads at /verify-email?token=XXX
- [ ] Success message shown
- [ ] Auto-redirect to dashboard
- [ ] Auto-login successful
- [ ] DB: email_verified=true
- [ ] DB: token marked as used

### 6. Verified Login ✓
- [ ] Logout
- [ ] Login with verified user
- [ ] Login succeeds
- [ ] Dashboard loads

### 7. Unverified Login Block ✓
- [ ] Register new user (don't verify)
- [ ] Try to login
- [ ] Login fails with 403
- [ ] Error message displayed
- [ ] Resend option shown

### 8. Resend Verification ✓
- [ ] Click resend from login page
- [ ] Enter email address
- [ ] New email received
- [ ] New token in database
- [ ] Rate limit: 3 resends max per hour

### 9. Duplicate Prevention ✓
- [ ] Try to register with existing email
- [ ] Registration fails
- [ ] Error message shown
- [ ] No new user created

### 10. Error Cases ✓
- [ ] Invalid token → error message + resend form
- [ ] Expired token → error message + resend form
- [ ] Used token → error message

## Test Data

### User 1 (Full Flow)
```
Email: test-user-1@example.com
Username: testuser1
First Name: Test
Last Name: User
Password: TestPass123
```

### User 2 (Unverified)
```
Email: test-user-2@example.com
Username: testuser2
First Name: Test
Last Name: User2
Password: TestPass456
```

## Database Check Queries

```sql
-- Check user status
SELECT email, username, email_verified FROM users WHERE email LIKE 'test-user-%';

-- Check verification tokens
SELECT email, expires_at, used_at, created_at FROM verification_tokens
WHERE email LIKE 'test-user-%' ORDER BY created_at DESC;

-- Count recent tokens (rate limiting)
SELECT email, COUNT(*) as count FROM verification_tokens
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY email;
```

## Success Criteria
All checkboxes above must be ✅ for subtask-9.1 completion.

---

# Invitation Flow - Quick Test Checklist (subtask-9.2)

## Pre-Test Setup
- [ ] PostgreSQL running
- [ ] Migrations applied (invitations table exists)
- [ ] SMTP configured in `.env`
- [ ] App running on http://localhost:3000
- [ ] Supervisor account available (verified, role = Supervisor or Admin)
- [ ] Test email account accessible

## Quick Test Steps

### 1. Access Invite Dialog (Supervisor) ✓
- [ ] Login as supervisor
- [ ] Navigate to /agents
- [ ] "Invite User" button visible
- [ ] Click button → dialog opens
- [ ] Form shows email + role fields

### 2. Permission Check ✓
- [ ] Login as agent (role = Agent)
- [ ] Try to send invitation
- [ ] API returns 403 Forbidden
- [ ] Error message shown

### 3. Send Agent Invitation ✓
- [ ] Login as supervisor
- [ ] Open invite dialog
- [ ] Enter email: test-invite-agent@example.com
- [ ] Select role: Agent
- [ ] Submit succeeds
- [ ] Success message shown
- [ ] DB: Invitation created

### 4. Send Supervisor Invitation ✓
- [ ] Open invite dialog
- [ ] Enter email: test-invite-supervisor@example.com
- [ ] Select role: Supervisor
- [ ] Submit succeeds
- [ ] DB: Role = Supervisor

### 5. Invitation Email Delivery ✓
- [ ] Email received in inbox
- [ ] Correct from address
- [ ] Subject mentions invitation
- [ ] Body shows inviter username
- [ ] Body shows role
- [ ] Accept invitation link present
- [ ] Link format: /accept-invitation?token=XXX
- [ ] Expiration notice (7 days)

### 6. Invitation Link Click ✓
- [ ] Click link in email
- [ ] Page loads at /accept-invitation?token=XXX
- [ ] Invitation details displayed:
  - Inviter username
  - Role
  - Email
- [ ] Registration form shown:
  - Username input
  - Password input
  - Confirm password input

### 7. Complete Registration via Invitation ✓
- [ ] Fill username: invitedagent1
- [ ] Fill password: InvitedPass123
- [ ] Fill confirm password: InvitedPass123
- [ ] Submit succeeds
- [ ] Auto-login successful
- [ ] Redirect to dashboard
- [ ] DB: User created with email_verified=true
- [ ] DB: User has correct role
- [ ] DB: Invitation marked as used

### 8. Invited User Role Verification ✓
- [ ] User has correct role in DB
- [ ] UI reflects correct permissions
- [ ] Agent: Cannot invite users
- [ ] Supervisor: Can invite users

### 9. Invited User Immediate Login ✓
- [ ] Logout
- [ ] Login with invited user credentials
- [ ] Login succeeds (no verification required)
- [ ] Dashboard loads
- [ ] email_verified = true in DB

### 10. Used Invitation Cannot Be Reused ✓
- [ ] Copy used invitation URL
- [ ] Open in new incognito window
- [ ] Navigate to URL
- [ ] Error message: "invitation already used"
- [ ] Registration form not shown
- [ ] DB: used_at IS NOT NULL

### 11. Expired Invitation Shows Error ✓
- [ ] Create expired invitation in DB
- [ ] Navigate to invitation URL
- [ ] Error message: "invitation expired"
- [ ] Registration form not shown

### 12. Invalid Token Shows Error ✓
- [ ] Navigate to /accept-invitation?token=00000000-0000-0000-0000-000000000000
- [ ] Error message: "invalid token" or "not found"
- [ ] Registration form not shown

### 13. Duplicate Email Prevention ✓
- [ ] Try to invite email that already exists
- [ ] API returns error
- [ ] Error message shown
- [ ] No invitation created

### 14. Form Validation ✓
- [ ] Empty form → error
- [ ] Weak password → error
- [ ] Password mismatch → error
- [ ] Duplicate username → error (backend)

### 15. Integration with Agents Page ✓
- [ ] Invitation sent from agents page
- [ ] After registration, new user appears in list (may need refresh)

## Test Data

### Supervisor Account
```
Email: supervisor@voipcrm.local
Username: supervisor1
Password: SuperPass123
Role: Supervisor
```

### Agent Account (Permission Test)
```
Email: agent@voipcrm.local
Username: agent1
Password: AgentPass123
Role: Agent
```

### Invited Users
```
# Invited Agent
Email: test-invite-agent@example.com
Username: invitedagent1
Password: InvitedPass123

# Invited Supervisor
Email: test-invite-supervisor@example.com
Username: invitedsup1
Password: InvitedPass456
```

## Database Check Queries

```sql
-- Check invitations
SELECT id, email, role, invited_by, expires_at, used_at, created_at
FROM invitations
WHERE email LIKE 'test-invite-%'
ORDER BY created_at DESC;

-- Check invitation status
SELECT
  i.email,
  i.role,
  u.username as invited_by,
  i.expires_at,
  i.used_at,
  CASE
    WHEN i.used_at IS NOT NULL THEN 'Used'
    WHEN i.expires_at < NOW() THEN 'Expired'
    ELSE 'Valid'
  END as status
FROM invitations i
LEFT JOIN users u ON i.invited_by = u.id
WHERE i.email LIKE 'test-invite-%';

-- Check invited users
SELECT
  u.username,
  u.email,
  u.role,
  u.email_verified,
  u.created_at
FROM users u
WHERE u.email LIKE 'test-invite-%';

-- Create expired invitation for testing
INSERT INTO invitations (token, email, role, invited_by, expires_at, created_at)
VALUES (
  gen_random_uuid()::text,
  'test-expired@example.com',
  'Agent',
  (SELECT id FROM users WHERE role = 'Supervisor' LIMIT 1),
  NOW() - INTERVAL '1 day',
  NOW() - INTERVAL '8 days'
)
RETURNING token;
```

## Success Criteria - Invitation Flow
All checkboxes above must be ✅ for subtask-9.2 completion.

---

## Notes
- Use different email for each test
- Clear browser cache between tests if issues occur
- Check browser console for any JS errors
- Monitor server logs during testing
- Keep track of invitation tokens for reuse tests
- Verify both supervisor and agent permission levels

---

# Edge Cases and Validation - Quick Test Checklist (subtask-9.3)

## Pre-Test Setup
- [ ] PostgreSQL running with test accounts (agent, supervisor, admin)
- [ ] SMTP configured in `.env`
- [ ] App running on http://localhost:3000
- [ ] Test email account accessible

## Acceptance Criteria Tests

### 1. Duplicate Email Prevention ✓
- [ ] Try to register with existing email
- [ ] Registration fails with error
- [ ] No new user created in DB

### 2. Duplicate Username Prevention ✓
- [ ] Try to register with existing username
- [ ] Registration fails with error
- [ ] No new user created in DB

### 3. Weak Password Rejection ✓
- [ ] Password < 8 chars → error
- [ ] Password without letters → error
- [ ] Password without numbers → error
- [ ] Client-side validation prevents submission

### 4. Invalid Email Format Rejection ✓
- [ ] Email without @ → error
- [ ] Email without domain → error
- [ ] Email without username → error
- [ ] Client-side validation prevents submission

### 5. Expired Token Handling ✓
- [ ] Expired verification token → error message + resend form
- [ ] Expired invitation token → error message
- [ ] User cannot verify/register with expired tokens
- [ ] Helpful error messages displayed

### 6. Non-Supervisor Cannot Invite ✓
- [ ] Login as Agent
- [ ] Try to send invitation (API or UI)
- [ ] API returns 403 Forbidden
- [ ] No invitation created

### 7. Admin Bypass Email Verification ✓
- [ ] Create admin with email_verified = false
- [ ] Admin can login without verification
- [ ] Agent/Supervisor cannot login without verification

## Additional Edge Cases

### 8. Used Token Prevention ✓
- [ ] Used verification token → error
- [ ] Used invitation token → error
- [ ] Tokens marked as used in DB

### 9. Invalid Token Format ✓
- [ ] Invalid verification token → error
- [ ] Invalid invitation token → error
- [ ] Missing token parameter → error
- [ ] Application remains stable

### 10. Security Tests ✓
- [ ] SQL injection attempts prevented
- [ ] XSS payloads do not execute
- [ ] CSRF protection in place

### 11. Rate Limiting ✓
- [ ] Max 3 verification resends per hour
- [ ] 4th request fails with rate limit error
- [ ] Error message displayed

### 12. Other Validation ✓
- [ ] Password mismatch → error
- [ ] Empty fields → error
- [ ] Whitespace-only fields → error

## Test Accounts (All password: TestPass123 variants)

```
Agent:      agent1 / AgentPass123
Supervisor: supervisor1 / SuperPass123
Admin:      admin / AdminPass123
```

## Quick Database Checks

```sql
-- Check for duplicate email/username
SELECT username, email FROM users WHERE email = 'test@example.com';

-- Check token expiration
SELECT email, expires_at, used_at,
       CASE WHEN expires_at < NOW() THEN 'Expired' ELSE 'Valid' END as status
FROM verification_tokens WHERE email = 'test@example.com';

-- Check invitation status
SELECT email, role, expires_at, used_at,
       CASE WHEN used_at IS NOT NULL THEN 'Used'
            WHEN expires_at < NOW() THEN 'Expired'
            ELSE 'Valid' END as status
FROM invitations WHERE email = 'test@example.com';

-- Check rate limiting
SELECT email, COUNT(*) FROM verification_tokens
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY email;
```

## Success Criteria - Edge Cases
All checkboxes above must be ✅ for subtask-9.3 completion.

---

## Overall Test Summary

### Subtask Coverage
- **subtask-9.1**: Self-Registration Flow (10 test cases)
- **subtask-9.2**: Invitation Flow (15 test cases)
- **subtask-9.3**: Edge Cases and Validation (20 test cases)

### Total Test Cases: 45

All three subtasks must be completed and all tests must pass before the User Registration Flow feature is considered complete and ready for production deployment.

