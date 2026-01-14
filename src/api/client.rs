use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Authentication required")]
    Unauthorized,
    #[error("Access denied")]
    Forbidden,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Server error: {0}")]
    Server(String),
    #[error("Invalid response: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::Network(err.to_string())
    }
}

#[derive(Clone)]
pub struct ApiClient {
    inner: Arc<ApiClientInner>,
}

struct ApiClientInner {
    base_url: String,
    client: Client,
    token: RwLock<Option<String>>,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        // On wasm, we can't use timeout
        #[cfg(target_arch = "wasm32")]
        let client = Client::new();

        #[cfg(not(target_arch = "wasm32"))]
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        ApiClient {
            inner: Arc::new(ApiClientInner {
                base_url: base_url.trim_end_matches('/').to_string(),
                client,
                token: RwLock::new(None),
            }),
        }
    }

    pub fn set_token(&self, token: Option<String>) {
        let mut guard = self.inner.token.write().unwrap();
        *guard = token;
    }

    pub fn get_token(&self) -> Option<String> {
        self.inner.token.read().unwrap().clone()
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.get(&url);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.post(&url).json(body);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.post(&url);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    pub async fn put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.put(&url).json(body);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    #[allow(dead_code)]
    pub async fn patch<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.patch(&url).json(body);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    #[allow(dead_code)]
    pub async fn delete(&self, path: &str) -> Result<(), ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.delete(&url);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_empty_response(response).await
    }

    #[allow(dead_code)]
    pub async fn post_no_response(&self, path: &str) -> Result<(), ApiError> {
        let url = format!("{}{}", self.inner.base_url, path);
        let mut request = self.inner.client.post(&url);

        if let Some(token) = self.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        self.handle_empty_response(response).await
    }

    async fn handle_empty_response(&self, response: reqwest::Response) -> Result<(), ApiError> {
        let status = response.status();

        match status {
            StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            StatusCode::FORBIDDEN => Err(ApiError::Forbidden),
            StatusCode::NOT_FOUND => {
                let text = response.text().await.unwrap_or_default();
                Err(ApiError::NotFound(text))
            }
            _ => {
                let text = response.text().await.unwrap_or_default();
                Err(ApiError::Server(format!("{}: {}", status, text)))
            }
        }
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T, ApiError> {
        let status = response.status();

        match status {
            StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED => {
                response.json::<T>().await.map_err(|e| ApiError::Parse(e.to_string()))
            }
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized),
            StatusCode::FORBIDDEN => Err(ApiError::Forbidden),
            StatusCode::NOT_FOUND => {
                let text = response.text().await.unwrap_or_default();
                Err(ApiError::NotFound(text))
            }
            _ => {
                let text = response.text().await.unwrap_or_default();
                Err(ApiError::Server(format!("{}: {}", status, text)))
            }
        }
    }
}

// Global API client instance
static API_CLIENT: std::sync::OnceLock<ApiClient> = std::sync::OnceLock::new();

pub fn init_api_client(base_url: &str) {
    let _ = API_CLIENT.set(ApiClient::new(base_url));
}

pub fn api_client() -> &'static ApiClient {
    API_CLIENT.get().expect("API client not initialized. Call init_api_client first.")
}
