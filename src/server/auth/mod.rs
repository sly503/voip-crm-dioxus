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
) -> Result<Json<LoginResponse>, (StatusCode, Json<AuthError>)> {
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

    // Create user
    let role = req.role.unwrap_or(UserRole::Agent);
    let user = db::users::create(&state.db, &req.username, &req.email, &password_hash, role)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthError { message: "Failed to create user".to_string() }),
            )
        })?;

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
