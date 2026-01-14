use crate::api::{api_client, ApiError};
use crate::models::{LoginRequest, LoginResponse};

pub async fn login(username: &str, password: &str) -> Result<LoginResponse, ApiError> {
    let request = LoginRequest {
        username: username.to_string(),
        password: password.to_string(),
    };

    let response: LoginResponse = api_client()
        .post("/api/auth/login", &request)
        .await?;

    // Store the token for future requests
    api_client().set_token(Some(response.token.clone()));

    Ok(response)
}

pub async fn logout() {
    api_client().set_token(None);
}
