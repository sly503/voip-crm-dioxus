use crate::api::{api_client, ApiError};
use crate::models::{
    LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
    VerifyEmailRequest, VerifyEmailResponse, ResendVerificationRequest,
    ResendVerificationResponse, InviteUserRequest, InviteUserResponse,
    AcceptInvitationRequest, AcceptInvitationResponse, UserRole,
};

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

pub async fn register(username: &str, email: &str, password: &str) -> Result<RegisterResponse, ApiError> {
    let request = RegisterRequest {
        username: username.to_string(),
        email: email.to_string(),
        password: password.to_string(),
        role: None,
    };

    let response: RegisterResponse = api_client()
        .post("/api/auth/register", &request)
        .await?;

    Ok(response)
}

pub async fn verify_email(token: &str) -> Result<VerifyEmailResponse, ApiError> {
    let request = VerifyEmailRequest {
        token: token.to_string(),
    };

    let response: VerifyEmailResponse = api_client()
        .post("/api/auth/verify-email", &request)
        .await?;

    // Store the token for automatic login after verification
    api_client().set_token(Some(response.token.clone()));

    Ok(response)
}

pub async fn resend_verification(email: &str) -> Result<ResendVerificationResponse, ApiError> {
    let request = ResendVerificationRequest {
        email: email.to_string(),
    };

    let response: ResendVerificationResponse = api_client()
        .post("/api/auth/resend-verification", &request)
        .await?;

    Ok(response)
}

pub async fn invite_user(email: &str, role: UserRole) -> Result<InviteUserResponse, ApiError> {
    let request = InviteUserRequest {
        email: email.to_string(),
        role,
    };

    let response: InviteUserResponse = api_client()
        .post("/api/auth/invite", &request)
        .await?;

    Ok(response)
}

pub async fn accept_invitation(token: &str, username: &str, password: &str) -> Result<AcceptInvitationResponse, ApiError> {
    let request = AcceptInvitationRequest {
        token: token.to_string(),
        username: username.to_string(),
        password: password.to_string(),
    };

    let response: AcceptInvitationResponse = api_client()
        .post("/api/auth/register-invitation", &request)
        .await?;

    // Store the token for automatic login after accepting invitation
    api_client().set_token(Some(response.token.clone()));

    Ok(response)
}
