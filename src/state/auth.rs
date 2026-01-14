use dioxus::prelude::*;
use crate::models::UserInfo;

/// Global authentication state
pub static AUTH_STATE: GlobalSignal<AuthState> = Signal::global(AuthState::default);

#[derive(Clone, Default)]
pub struct AuthState {
    pub user: Option<UserInfo>,
    pub token: Option<String>,
    pub is_loading: bool,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some() && self.user.is_some()
    }

    pub fn is_supervisor_or_above(&self) -> bool {
        self.user.as_ref().map(|u| u.role.is_supervisor_or_above()).unwrap_or(false)
    }

    pub fn user_id(&self) -> Option<i64> {
        self.user.as_ref().map(|u| u.id)
    }

    pub fn username(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.username.as_str())
    }
}

pub fn set_auth(user: UserInfo, token: String) {
    let mut state = AUTH_STATE.write();
    state.user = Some(user);
    state.token = Some(token);
    state.is_loading = false;
}

pub fn clear_auth() {
    let mut state = AUTH_STATE.write();
    state.user = None;
    state.token = None;
    state.is_loading = false;
}
