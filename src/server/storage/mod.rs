//! Storage module for call recordings
//!
//! This module provides secure file storage for call recordings with encryption at rest.
//! Features:
//! - Trait-based storage abstraction for multiple backends
//! - Local filesystem storage with encryption support
//! - Storage quota management and tracking
//! - Automatic directory structure organization (YYYY/MM/DD)

pub mod encryption;

use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use chrono::{DateTime, Utc};
use thiserror::Error;
use encryption::EncryptionContext;

/// Storage configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub recordings_path: PathBuf,
    pub max_storage_gb: f64,
    pub default_retention_days: i32,
    pub encryption_key: String,
}

impl StorageConfig {
    /// Load storage configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let recordings_path = std::env::var("RECORDINGS_PATH")
            .unwrap_or_else(|_| "./recordings".to_string());

        let max_storage_gb = std::env::var("MAX_STORAGE_GB")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(100.0);

        let default_retention_days = std::env::var("DEFAULT_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(90);

        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| "ENCRYPTION_KEY environment variable not set")?;

        // Validate encryption key format (should be 64 hex characters for 32 bytes)
        if encryption_key.len() != 64 {
            return Err(format!(
                "ENCRYPTION_KEY must be 64 hex characters (32 bytes), got {} characters",
                encryption_key.len()
            ));
        }

        Ok(Self {
            recordings_path: PathBuf::from(recordings_path),
            max_storage_gb,
            default_retention_days,
            encryption_key,
        })
    }

    /// Initialize the storage system
    /// Creates the base directory and returns a LocalFileStorage instance
    pub async fn initialize(self) -> StorageResult<LocalFileStorage> {
        // Create encryption context
        let encryption_ctx = EncryptionContext::from_hex(&self.encryption_key, "default")
            .map_err(|e| StorageError::Encryption(e.to_string()))?;

        // Create storage instance
        let storage = LocalFileStorage::new(
            &self.recordings_path,
            self.max_storage_gb,
            encryption_ctx,
        );

        // Initialize directory structure
        storage.init().await?;

        tracing::info!(
            "Storage initialized: path={:?}, quota={}GB, retention={}days",
            self.recordings_path,
            self.max_storage_gb,
            self.default_retention_days
        );

        Ok(storage)
    }
}

/// Storage-related errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Storage quota exceeded: {used}GB used of {quota}GB")]
    QuotaExceeded { used: f64, quota: f64 },

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Encryption error: {0}")]
    EncryptionError(#[from] encryption::EncryptionError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Storage operation failed: {0}")]
    OperationFailed(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub available_space_bytes: u64,
}

/// Metadata for a stored recording
#[derive(Debug, Clone)]
pub struct RecordingFile {
    pub file_path: String,
    pub file_size: u64,
    pub encryption_key_id: String,
    pub uploaded_at: DateTime<Utc>,
}

/// Trait defining the storage interface for call recordings
#[async_trait::async_trait]
pub trait RecordingStorage: Send + Sync {
    /// Store a recording file
    /// Returns the file path and encryption key ID
    async fn store_recording(
        &self,
        call_id: i64,
        data: Vec<u8>,
        format: &str,
    ) -> StorageResult<RecordingFile>;

    /// Retrieve a recording file
    /// Returns the decrypted file data
    async fn get_recording(&self, file_path: &str) -> StorageResult<Vec<u8>>;

    /// Delete a recording file
    async fn delete_recording(&self, file_path: &str) -> StorageResult<()>;

    /// Get storage statistics
    async fn get_storage_info(&self) -> StorageResult<StorageInfo>;

    /// Check if storage quota allows storing a file of given size
    async fn check_quota(&self, file_size: u64) -> StorageResult<bool>;
}

/// Local filesystem storage implementation
pub struct LocalFileStorage {
    base_path: PathBuf,
    max_storage_bytes: u64,
    encryption_ctx: EncryptionContext,
}

impl LocalFileStorage {
    /// Create a new local file storage with encryption
    ///
    /// # Arguments
    /// * `base_path` - Base directory for storing recordings
    /// * `max_storage_gb` - Maximum storage quota in gigabytes
    /// * `encryption_ctx` - Encryption context for securing recordings
    ///
    /// # Returns
    /// * `Self` - New local file storage instance
    pub fn new(
        base_path: impl AsRef<Path>,
        max_storage_gb: f64,
        encryption_ctx: EncryptionContext,
    ) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            max_storage_bytes: (max_storage_gb * 1024.0 * 1024.0 * 1024.0) as u64,
            encryption_ctx,
        }
    }

    /// Initialize storage directory structure
    pub async fn init(&self) -> StorageResult<()> {
        fs::create_dir_all(&self.base_path).await?;
        tracing::info!("Initialized recording storage at: {:?}", self.base_path);
        Ok(())
    }

    /// Generate a storage path for a recording
    /// Format: base_path/YYYY/MM/DD/call_id_timestamp.format
    fn generate_path(&self, call_id: i64, format: &str) -> PathBuf {
        let now = Utc::now();
        let date_path = now.format("%Y/%m/%d").to_string();
        let timestamp = now.timestamp();
        let filename = format!("{}_{}.{}", call_id, timestamp, format);

        self.base_path
            .join(date_path)
            .join(filename)
    }

    /// Convert absolute path to relative path for storage
    fn to_relative_path(&self, absolute_path: &Path) -> StorageResult<String> {
        absolute_path
            .strip_prefix(&self.base_path)
            .map_err(|_| StorageError::InvalidPath(format!("{:?}", absolute_path)))?
            .to_str()
            .ok_or_else(|| StorageError::InvalidPath(format!("{:?}", absolute_path)))
            .map(|s| s.to_string())
    }

    /// Convert relative path to absolute path
    fn to_absolute_path(&self, relative_path: &str) -> PathBuf {
        self.base_path.join(relative_path)
    }

    /// Calculate total storage usage
    async fn calculate_usage(&self) -> StorageResult<(u64, u64)> {
        let mut total_files = 0u64;
        let mut total_size = 0u64;

        if !self.base_path.exists() {
            return Ok((0, 0));
        }

        // Recursively walk the directory tree
        let mut entries = vec![self.base_path.clone()];

        while let Some(path) = entries.pop() {
            let mut read_dir = fs::read_dir(&path).await?;

            while let Some(entry) = read_dir.next_entry().await? {
                let path = entry.path();
                let metadata = entry.metadata().await?;

                if metadata.is_dir() {
                    entries.push(path);
                } else if metadata.is_file() {
                    total_files += 1;
                    total_size += metadata.len();
                }
            }
        }

        Ok((total_files, total_size))
    }
}

#[async_trait::async_trait]
impl RecordingStorage for LocalFileStorage {
    async fn store_recording(
        &self,
        call_id: i64,
        data: Vec<u8>,
        format: &str,
    ) -> StorageResult<RecordingFile> {
        let file_size = data.len() as u64;

        // Check quota before storing
        if !self.check_quota(file_size).await? {
            let (_, used_bytes) = self.calculate_usage().await?;
            let used_gb = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let quota_gb = self.max_storage_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            return Err(StorageError::QuotaExceeded {
                used: used_gb,
                quota: quota_gb,
            });
        }

        // Generate storage path
        let absolute_path = self.generate_path(call_id, format);

        // Create parent directories
        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Encrypt data before writing
        let encrypted_data = self.encryption_ctx.encrypt(&data)?;
        let encryption_key_id = self.encryption_ctx.key_id().to_string();

        // Write file
        let mut file = fs::File::create(&absolute_path).await?;
        file.write_all(&encrypted_data).await?;
        file.sync_all().await?;

        let relative_path = self.to_relative_path(&absolute_path)?;
        let uploaded_at = Utc::now();

        tracing::info!(
            "Stored recording for call {} at {} (size: {} bytes)",
            call_id,
            relative_path,
            file_size
        );

        Ok(RecordingFile {
            file_path: relative_path,
            file_size,
            encryption_key_id,
            uploaded_at,
        })
    }

    async fn get_recording(&self, file_path: &str) -> StorageResult<Vec<u8>> {
        let absolute_path = self.to_absolute_path(file_path);

        if !absolute_path.exists() {
            return Err(StorageError::FileNotFound(file_path.to_string()));
        }

        // Read file
        let mut file = fs::File::open(&absolute_path).await?;
        let mut encrypted_data = Vec::new();
        file.read_to_end(&mut encrypted_data).await?;

        // Decrypt data before returning
        let decrypted_data = self.encryption_ctx.decrypt(&encrypted_data)?;

        tracing::debug!("Retrieved recording from {} ({} bytes)", file_path, decrypted_data.len());

        Ok(decrypted_data)
    }

    async fn delete_recording(&self, file_path: &str) -> StorageResult<()> {
        let absolute_path = self.to_absolute_path(file_path);

        if !absolute_path.exists() {
            return Err(StorageError::FileNotFound(file_path.to_string()));
        }

        fs::remove_file(&absolute_path).await?;

        tracing::info!("Deleted recording at {}", file_path);

        // Clean up empty parent directories
        if let Some(parent) = absolute_path.parent() {
            let _ = Self::cleanup_empty_dirs(parent, &self.base_path).await;
        }

        Ok(())
    }

    async fn get_storage_info(&self) -> StorageResult<StorageInfo> {
        let (total_files, total_size_bytes) = self.calculate_usage().await?;
        let available_space_bytes = self.max_storage_bytes.saturating_sub(total_size_bytes);

        Ok(StorageInfo {
            total_files,
            total_size_bytes,
            available_space_bytes,
        })
    }

    async fn check_quota(&self, file_size: u64) -> StorageResult<bool> {
        let (_, used_bytes) = self.calculate_usage().await?;
        let would_use = used_bytes + file_size;
        Ok(would_use <= self.max_storage_bytes)
    }
}

impl LocalFileStorage {
    /// Clean up empty directories recursively up to base_path
    async fn cleanup_empty_dirs(dir: &Path, base_path: &Path) -> std::io::Result<()> {
        // Don't delete the base path itself
        if dir == base_path {
            return Ok(());
        }

        // Check if directory is empty
        let mut entries = fs::read_dir(dir).await?;
        if entries.next_entry().await?.is_none() {
            // Directory is empty, remove it
            fs::remove_dir(dir).await?;

            // Recursively clean parent
            if let Some(parent) = dir.parent() {
                let _ = Self::cleanup_empty_dirs(parent, base_path).await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_from_env() {
        // Set up test environment variables
        std::env::set_var("RECORDINGS_PATH", "/tmp/test_recordings");
        std::env::set_var("MAX_STORAGE_GB", "50");
        std::env::set_var("DEFAULT_RETENTION_DAYS", "60");
        std::env::set_var("ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");

        let config = StorageConfig::from_env().unwrap();

        assert_eq!(config.recordings_path, PathBuf::from("/tmp/test_recordings"));
        assert_eq!(config.max_storage_gb, 50.0);
        assert_eq!(config.default_retention_days, 60);
        assert_eq!(config.encryption_key.len(), 64);

        // Cleanup
        std::env::remove_var("RECORDINGS_PATH");
        std::env::remove_var("MAX_STORAGE_GB");
        std::env::remove_var("DEFAULT_RETENTION_DAYS");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn test_storage_config_defaults() {
        // Ensure ENCRYPTION_KEY is set (required)
        std::env::set_var("ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");

        // Remove optional vars to test defaults
        std::env::remove_var("RECORDINGS_PATH");
        std::env::remove_var("MAX_STORAGE_GB");
        std::env::remove_var("DEFAULT_RETENTION_DAYS");

        let config = StorageConfig::from_env().unwrap();

        assert_eq!(config.recordings_path, PathBuf::from("./recordings"));
        assert_eq!(config.max_storage_gb, 100.0);
        assert_eq!(config.default_retention_days, 90);

        // Cleanup
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn test_storage_config_invalid_key() {
        std::env::set_var("ENCRYPTION_KEY", "too_short");

        let result = StorageConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("64 hex characters"));

        // Cleanup
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[tokio::test]
    async fn test_local_storage_init() {
        let temp_dir = std::env::temp_dir().join("voip_crm_test_storage");
        let key_hex = encryption::generate_key();
        let encryption_ctx = encryption::EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
        let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx); // 1GB quota

        storage.init().await.unwrap();
        assert!(temp_dir.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_directory_structure_creation() {
        let temp_dir = std::env::temp_dir().join("voip_crm_test_storage_dirs");
        let key_hex = encryption::generate_key();
        let encryption_ctx = encryption::EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
        let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

        storage.init().await.unwrap();

        // Store a file - this should create YYYY/MM/DD directories
        let test_data = b"test recording data".to_vec();
        let result = storage.store_recording(99999, test_data, "wav").await.unwrap();

        // Verify the path contains date structure
        let now = Utc::now();
        let expected_date_path = now.format("%Y/%m/%d").to_string();
        assert!(result.file_path.contains(&expected_date_path));

        // Verify the physical directory exists
        let full_path = temp_dir.join(&result.file_path);
        assert!(full_path.exists());

        // Verify parent directories exist
        let parent = full_path.parent().unwrap();
        assert!(parent.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_recording() {
        let temp_dir = std::env::temp_dir().join("voip_crm_test_storage_2");
        let key_hex = encryption::generate_key();
        let encryption_ctx = encryption::EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
        let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

        storage.init().await.unwrap();

        // Store a recording
        let test_data = b"test audio data".to_vec();
        let result = storage.store_recording(12345, test_data.clone(), "wav").await.unwrap();

        assert!(result.file_size == test_data.len() as u64);
        assert!(result.file_path.contains("12345"));
        assert_eq!(result.encryption_key_id, "test-key");

        // Retrieve the recording
        let retrieved = storage.get_recording(&result.file_path).await.unwrap();
        assert_eq!(retrieved, test_data);

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_delete_recording() {
        let temp_dir = std::env::temp_dir().join("voip_crm_test_storage_3");
        let key_hex = encryption::generate_key();
        let encryption_ctx = encryption::EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
        let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

        storage.init().await.unwrap();

        // Store and then delete
        let test_data = b"test audio data".to_vec();
        let result = storage.store_recording(12345, test_data.clone(), "wav").await.unwrap();

        storage.delete_recording(&result.file_path).await.unwrap();

        // Verify it's deleted
        let retrieve_result = storage.get_recording(&result.file_path).await;
        assert!(retrieve_result.is_err());

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_storage_quota() {
        let temp_dir = std::env::temp_dir().join("voip_crm_test_storage_4");
        let key_hex = encryption::generate_key();
        let encryption_ctx = encryption::EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
        // Very small quota for testing
        let storage = LocalFileStorage::new(&temp_dir, 0.000001, encryption_ctx); // ~1KB

        storage.init().await.unwrap();

        // Try to store data that exceeds quota
        let large_data = vec![0u8; 10000]; // 10KB
        let result = storage.store_recording(12345, large_data, "wav").await;

        assert!(result.is_err());
        if let Err(StorageError::QuotaExceeded { .. }) = result {
            // Expected error
        } else {
            panic!("Expected QuotaExceeded error");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_storage_info() {
        let temp_dir = std::env::temp_dir().join("voip_crm_test_storage_5");
        let key_hex = encryption::generate_key();
        let encryption_ctx = encryption::EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
        let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

        storage.init().await.unwrap();

        // Initial state should be empty
        let info = storage.get_storage_info().await.unwrap();
        assert_eq!(info.total_files, 0);
        assert_eq!(info.total_size_bytes, 0);

        // Store a recording
        let test_data = b"test audio data".to_vec();
        let _ = storage.store_recording(12345, test_data.clone(), "wav").await.unwrap();

        // Check updated stats
        let info = storage.get_storage_info().await.unwrap();
        assert_eq!(info.total_files, 1);
        // Note: encrypted size will be larger than plaintext due to nonce and auth tag
        assert!(info.total_size_bytes > test_data.len() as u64);

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
