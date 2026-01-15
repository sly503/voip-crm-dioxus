//! File encryption/decryption module using AES-256-GCM
//!
//! This module provides secure encryption for call recordings at rest.
//! Uses AES-256-GCM which provides both confidentiality and authenticity.
//!
//! Key features:
//! - AES-256-GCM authenticated encryption
//! - Unique nonce (IV) generated for each encryption
//! - Nonce prepended to ciphertext for storage efficiency
//! - Master key derived from environment variable

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use thiserror::Error;

/// Encryption-related errors
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Invalid ciphertext: {0}")]
    InvalidCiphertext(String),
}

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Size of the nonce/IV for AES-GCM (96 bits / 12 bytes)
const NONCE_SIZE: usize = 12;

/// Encryption context holding the cipher
pub struct EncryptionContext {
    cipher: Aes256Gcm,
    key_id: String,
}

impl EncryptionContext {
    /// Create a new encryption context from a master key
    ///
    /// # Arguments
    /// * `key_bytes` - 32-byte master key
    /// * `key_id` - Identifier for this key (for key rotation)
    ///
    /// # Returns
    /// * `EncryptionResult<Self>` - New encryption context or error
    pub fn new(key_bytes: &[u8; 32], key_id: impl Into<String>) -> EncryptionResult<Self> {
        let cipher = Aes256Gcm::new(key_bytes.into());

        Ok(Self {
            cipher,
            key_id: key_id.into(),
        })
    }

    /// Create encryption context from hex-encoded key string
    ///
    /// # Arguments
    /// * `key_hex` - 64-character hex string representing 32 bytes
    /// * `key_id` - Identifier for this key
    ///
    /// # Returns
    /// * `EncryptionResult<Self>` - New encryption context or error
    pub fn from_hex(key_hex: &str, key_id: impl Into<String>) -> EncryptionResult<Self> {
        if key_hex.len() != 64 {
            return Err(EncryptionError::InvalidKey(
                "Key must be 64 hex characters (32 bytes)".to_string(),
            ));
        }

        let mut key_bytes = [0u8; 32];
        hex::decode_to_slice(key_hex, &mut key_bytes).map_err(|e| {
            EncryptionError::InvalidKey(format!("Invalid hex encoding: {}", e))
        })?;

        Self::new(&key_bytes, key_id)
    }

    /// Get the key ID for this context
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Encrypt data
    ///
    /// The output format is: [nonce (12 bytes)][ciphertext][auth tag (16 bytes)]
    /// The nonce is prepended so we can decrypt without needing to store it separately.
    ///
    /// # Arguments
    /// * `plaintext` - Data to encrypt
    ///
    /// # Returns
    /// * `EncryptionResult<Vec<u8>>` - Encrypted data with nonce prepended
    pub fn encrypt(&self, plaintext: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Generate a random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        tracing::debug!(
            "Encrypted {} bytes to {} bytes (including nonce and auth tag)",
            plaintext.len(),
            result.len()
        );

        Ok(result)
    }

    /// Decrypt data
    ///
    /// Expects data in the format: [nonce (12 bytes)][ciphertext][auth tag (16 bytes)]
    ///
    /// # Arguments
    /// * `ciphertext_with_nonce` - Encrypted data with nonce prepended
    ///
    /// # Returns
    /// * `EncryptionResult<Vec<u8>>` - Decrypted data
    pub fn decrypt(&self, ciphertext_with_nonce: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Check minimum size (nonce + auth tag)
        if ciphertext_with_nonce.len() < NONCE_SIZE + 16 {
            return Err(EncryptionError::InvalidCiphertext(
                "Ciphertext too short".to_string(),
            ));
        }

        // Extract nonce
        let (nonce_bytes, ciphertext) = ciphertext_with_nonce.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the data
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        tracing::debug!(
            "Decrypted {} bytes to {} bytes",
            ciphertext_with_nonce.len(),
            plaintext.len()
        );

        Ok(plaintext)
    }
}

/// Generate a new random encryption key
///
/// # Returns
/// * `String` - 64-character hex string representing a 32-byte key
pub fn generate_key() -> String {
    let mut key_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut key_bytes);
    hex::encode(key_bytes)
}

// We need the hex crate for encoding/decoding
// Since it's a common utility, it should already be available, but if not,
// we'll need to add it to Cargo.toml
mod hex {
    use super::EncryptionError;

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    pub fn decode_to_slice(
        hex: &str,
        output: &mut [u8],
    ) -> Result<(), EncryptionError> {
        if hex.len() != output.len() * 2 {
            return Err(EncryptionError::InvalidKey(
                "Hex string length mismatch".to_string(),
            ));
        }

        for i in 0..output.len() {
            let byte_str = &hex[i * 2..i * 2 + 2];
            output[i] = u8::from_str_radix(byte_str, 16).map_err(|e| {
                EncryptionError::InvalidKey(format!("Invalid hex digit: {}", e))
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key_hex = generate_key();
        let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

        let plaintext = b"This is a test recording file";
        let ciphertext = ctx.encrypt(plaintext).unwrap();

        // Ciphertext should be larger (nonce + auth tag)
        assert!(ciphertext.len() > plaintext.len());

        let decrypted = ctx.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces() {
        let key_hex = generate_key();
        let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

        let plaintext = b"Same plaintext";
        let ciphertext1 = ctx.encrypt(plaintext).unwrap();
        let ciphertext2 = ctx.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertexts (different nonces)
        assert_ne!(ciphertext1, ciphertext2);

        // Both should decrypt to the same plaintext
        assert_eq!(ctx.decrypt(&ciphertext1).unwrap(), plaintext);
        assert_eq!(ctx.decrypt(&ciphertext2).unwrap(), plaintext);
    }

    #[test]
    fn test_invalid_key_length() {
        let result = EncryptionContext::from_hex("too_short", "test-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_ciphertext() {
        let key_hex = generate_key();
        let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

        // Too short
        let result = ctx.decrypt(&[0u8; 10]);
        assert!(result.is_err());

        // Invalid ciphertext (wrong key/corrupted data)
        let plaintext = b"Test data";
        let ciphertext = ctx.encrypt(plaintext).unwrap();

        let other_key_hex = generate_key();
        let other_ctx = EncryptionContext::from_hex(&other_key_hex, "other-key").unwrap();

        let result = other_ctx.decrypt(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_id() {
        let key_hex = generate_key();
        let ctx = EncryptionContext::from_hex(&key_hex, "my-key-v1").unwrap();
        assert_eq!(ctx.key_id(), "my-key-v1");
    }

    #[test]
    fn test_large_data() {
        let key_hex = generate_key();
        let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

        // Test with a large file (1MB)
        let plaintext = vec![0x42u8; 1024 * 1024];
        let ciphertext = ctx.encrypt(&plaintext).unwrap();
        let decrypted = ctx.decrypt(&ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_generate_key() {
        let key1 = generate_key();
        let key2 = generate_key();

        // Keys should be 64 hex characters
        assert_eq!(key1.len(), 64);
        assert_eq!(key2.len(), 64);

        // Keys should be different
        assert_ne!(key1, key2);

        // Should be valid hex
        assert!(EncryptionContext::from_hex(&key1, "test").is_ok());
        assert!(EncryptionContext::from_hex(&key2, "test").is_ok());
    }
}
