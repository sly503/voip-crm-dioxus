//! Authentication module with JWT

use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    Json,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::{UserRole, LoginRequest, LoginResponse, RegisterRequest};
use crate::server::{AppState, db};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,        // user id
    pub username: String,
    pub role: String,
    pub exp: usize,      // expiration timestamp
}

#[derive(Debug, Serialize)]
pub struct AuthError {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub message: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub message: String,
    pub token: String,
    pub user: crate::models::UserInfo,
}

#[derive(Debug, Deserialize)]
pub struct ResendVerificationRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct ResendVerificationResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize)]
pub struct InviteUserResponse {
    pub message: String,
    pub email: String,
    pub role: UserRole,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterInvitationRequest {
    pub token: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterInvitationResponse {
    pub message: String,
    pub token: String,
    pub user: crate::models::UserInfo,
}

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

/// Create a JWT token for a user
pub fn create_token(user_id: i64, username: &str, role: &str, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate a JWT token and extract claims
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

/// JWT Auth extractor - extracts Claims from Authorization header
impl FromRequestParts<Arc<AppState>> for Claims {
    type Rejection = (StatusCode, Json<AuthError>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract the Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthError { message: "Missing authorization header".to_string() }),
                )
            })?;

        // Validate the token
        let claims = validate_token(bearer.token(), &state.jwt_secret)
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthError { message: "Invalid token".to_string() }),
                )
            })?;

        Ok(claims)
    }
}

/// Login handler
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<AuthError>)> {
    // Find user by username
    let user = db::users::get_by_username(&state.db, &req.username)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Database error".to_string() }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError { message: "Invalid credentials".to_string() }),
            )
        })?;

    // Verify password
    let valid = verify_password(&req.password, &user.password_hash)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Password verification error".to_string() }),
            )
        })?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthError { message: "Invalid credentials".to_string() }),
        ));
    }

    // Check email verification status (admins bypass this check)
    if !user.email_verified && user.role != UserRole::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(AuthError { message: "Please verify your email before logging in. Check your inbox for a verification link.".to_string() }),
        ));
    }

    // Create JWT token
    let role_str = format!("{:?}", user.role);
    let token = create_token(user.id, &user.username, &role_str, &state.jwt_secret)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Token generation error".to_string() }),
            )
        })?;

    Ok(Json(LoginResponse {
        token,
        user: user.to_info(),
    }))
}

/// Register handler
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, Json<AuthError>)> {
    // Check if email already exists
    if let Ok(Some(_)) = db::users::get_by_email(&state.db, &req.email).await {
        return Err((
            StatusCode::CONFLICT,
            Json(AuthError { message: "Email already exists".to_string() }),
        ));
    }

    // Check if username already exists
    if let Ok(Some(_)) = db::users::get_by_username(&state.db, &req.username).await {
        return Err((
            StatusCode::CONFLICT,
            Json(AuthError { message: "Username already exists".to_string() }),
        ));
    }

    // Hash password
    let password_hash = hash_password(&req.password)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Password hashing error".to_string() }),
            )
        })?;

    // Create user (email_verified defaults to false)
    let role = req.role.unwrap_or(UserRole::Agent);
    let user = db::users::create(&state.db, &req.username, &req.email, &password_hash, role)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to create user".to_string() }),
            )
        })?;

    // Generate verification token
    let verification_token = uuid::Uuid::new_v4().to_string();

    // Store verification token in database
    db::users::create_verification_token(&state.db, user.id, &user.email, &verification_token)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to create verification token".to_string() }),
            )
        })?;

    // Send verification email
    state.email
        .send_verification_email(&user.email, Some(&user.username), &verification_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to send verification email".to_string() }),
            )
        })?;

    Ok(Json(RegisterResponse {
        message: "Registration successful. Please check your email to verify your account.".to_string(),
        email: user.email,
    }))
}

/// Verify email handler
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyEmailRequest>,
) -> Result<Json<VerifyEmailResponse>, (StatusCode, Json<AuthError>)> {
    // Get verification token from database
    let token_data = db::users::get_verification_token(&state.db, &req.token)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Database error".to_string() }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(AuthError { message: "Invalid verification token".to_string() }),
            )
        })?;

    let (user_id, _email, is_valid) = token_data;

    // Check if token is valid (not used and not expired)
    if !is_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthError { message: "Verification token has expired or already been used".to_string() }),
        ));
    }

    // Mark email as verified
    db::users::verify_email(&state.db, user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to verify email".to_string() }),
            )
        })?;

    // Mark token as used
    db::users::mark_token_used(&state.db, &req.token)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to mark token as used".to_string() }),
            )
        })?;

    // Get user details
    let user = db::users::get_by_id(&state.db, user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to retrieve user".to_string() }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "User not found".to_string() }),
            )
        })?;

    // Create JWT token for automatic login
    let role_str = format!("{:?}", user.role);
    let token = create_token(user.id, &user.username, &role_str, &state.jwt_secret)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Token generation error".to_string() }),
            )
        })?;

    Ok(Json(VerifyEmailResponse {
        message: "Email verified successfully".to_string(),
        token,
        user: user.to_info(),
    }))
}

/// Resend verification email handler
pub async fn resend_verification(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResendVerificationRequest>,
) -> Result<Json<ResendVerificationResponse>, (StatusCode, Json<AuthError>)> {
    // Find user by email
    let user = db::users::get_by_email(&state.db, &req.email)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Database error".to_string() }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthError { message: "Email not found".to_string() }),
            )
        })?;

    // Check if email is already verified
    if user.email_verified {
        return Ok(Json(ResendVerificationResponse {
            message: "Email is already verified. You can now login.".to_string(),
        }));
    }

    // Rate limiting: Check how many verification tokens were created in the last hour
    let recent_token_count = db::users::count_recent_verification_tokens(&state.db, &req.email)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Database error".to_string() }),
            )
        })?;

    if recent_token_count >= 3 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(AuthError {
                message: "Too many verification emails sent. Please wait an hour before requesting another.".to_string()
            }),
        ));
    }

    // Generate new verification token
    let verification_token = uuid::Uuid::new_v4().to_string();

    // Store verification token in database
    db::users::create_verification_token(&state.db, user.id, &user.email, &verification_token)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to create verification token".to_string() }),
            )
        })?;

    // Send verification email
    state.email
        .send_verification_email(&user.email, Some(&user.username), &verification_token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to send verification email".to_string() }),
            )
        })?;

    Ok(Json(ResendVerificationResponse {
        message: "Verification email sent. Please check your inbox.".to_string(),
    }))
}

/// Invite user handler - allows supervisors and admins to invite new users
pub async fn invite_user(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<InviteUserRequest>,
) -> Result<Json<InviteUserResponse>, (StatusCode, Json<AuthError>)> {
    // Authorization: Only supervisors and admins can send invitations
    let claims_role = match claims.role.as_str() {
        "Admin" => UserRole::Admin,
        "Supervisor" => UserRole::Supervisor,
        "Agent" => UserRole::Agent,
        _ => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(AuthError { message: "Invalid role in token".to_string() }),
            ));
        }
    };

    if !claims_role.is_supervisor_or_above() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(AuthError { message: "Only supervisors and admins can send invitations".to_string() }),
        ));
    }

    // Validate invited role: Cannot invite admins
    if matches!(req.role, UserRole::Admin) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthError { message: "Cannot invite admin users. Only Agent and Supervisor roles are allowed.".to_string() }),
        ));
    }

    // Check if email already exists
    if let Ok(Some(_)) = db::users::get_by_email(&state.db, &req.email).await {
        return Err((
            StatusCode::CONFLICT,
            Json(AuthError { message: "Email already registered".to_string() }),
        ));
    }

    // Generate invitation token
    let invitation_token = uuid::Uuid::new_v4().to_string();

    // Store invitation in database
    db::invitations::create_invitation(
        &state.db,
        &invitation_token,
        &req.email,
        req.role.clone(),
        claims.sub,
    )
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthError { message: "Failed to create invitation".to_string() }),
        )
    })?;

    // Send invitation email
    state.email
        .send_invitation_email(&req.email, &invitation_token, &req.role)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send invitation email: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to send invitation email".to_string() }),
            )
        })?;

    Ok(Json(InviteUserResponse {
        message: "Invitation sent successfully".to_string(),
        email: req.email,
        role: req.role,
        token: invitation_token,
    }))
}

/// Register with invitation handler - allows users to register using an invitation token
pub async fn register_invitation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterInvitationRequest>,
) -> Result<Json<RegisterInvitationResponse>, (StatusCode, Json<AuthError>)> {
    // Get invitation by token
    let invitation = db::invitations::get_invitation_by_token(&state.db, &req.token)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Database error".to_string() }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(AuthError { message: "Invalid invitation token".to_string() }),
            )
        })?;

    let (email, role, _invited_by, is_valid) = invitation;

    // Check if invitation is valid (not used and not expired)
    if !is_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthError { message: "Invitation has expired or already been used".to_string() }),
        ));
    }

    // Check if username already exists
    if let Ok(Some(_)) = db::users::get_by_username(&state.db, &req.username).await {
        return Err((
            StatusCode::CONFLICT,
            Json(AuthError { message: "Username already exists".to_string() }),
        ));
    }

    // Hash password
    let password_hash = hash_password(&req.password)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Password hashing error".to_string() }),
            )
        })?;

    // Create user with invited role
    let user = db::users::create(&state.db, &req.username, &email, &password_hash, role.clone())
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to create user".to_string() }),
            )
        })?;

    // Mark email as verified (skip email verification for invited users)
    db::users::verify_email(&state.db, user.id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to verify email".to_string() }),
            )
        })?;

    // Mark invitation as used
    db::invitations::mark_invitation_used(&state.db, &req.token, user.id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to mark invitation as used".to_string() }),
            )
        })?;

    // Create JWT token for automatic login
    let role_str = format!("{:?}", role);
    let token = create_token(user.id, &user.username, &role_str, &state.jwt_secret)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Token generation error".to_string() }),
            )
        })?;

    Ok(Json(RegisterInvitationResponse {
        message: "Registration successful".to_string(),
        token,
        user: user.to_info(),
    }))
}
