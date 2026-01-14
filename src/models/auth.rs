use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<UserRole>,
}

/// User info returned to client (no password)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub role: UserRole,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
}

/// Full user struct for server-side use (includes password_hash)
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email_verified: bool,
}

impl User {
    pub fn to_info(&self) -> UserInfo {
        UserInfo {
            id: self.id,
            username: self.username.clone(),
            email: Some(self.email.clone()),
            role: self.role.clone(),
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(not(target_arch = "wasm32"), sqlx(type_name = "user_role", rename_all = "PascalCase"))]
pub enum UserRole {
    Admin,
    Supervisor,
    Agent,
}

impl UserRole {
    pub fn is_supervisor_or_above(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Supervisor)
    }
}
