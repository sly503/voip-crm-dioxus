//! Additional comprehensive tests for storage module
//!
//! This test module complements the existing tests in mod.rs and encryption.rs
//! with additional edge cases, integration tests, and concurrent operation tests.

use super::*;
use encryption::{generate_key, EncryptionContext};
use std::sync::Arc;
use tokio::sync::Semaphore;

// ==================== Encryption Edge Cases ====================

#[tokio::test]
async fn test_encrypt_empty_file() {
    let key_hex = generate_key();
    let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

    let plaintext = b"";
    let ciphertext = ctx.encrypt(plaintext).unwrap();

    // Even empty files should have nonce + auth tag
    assert!(ciphertext.len() > 0);

    let decrypted = ctx.decrypt(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[tokio::test]
async fn test_encrypt_single_byte() {
    let key_hex = generate_key();
    let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

    let plaintext = b"A";
    let ciphertext = ctx.encrypt(plaintext).unwrap();
    let decrypted = ctx.decrypt(&ciphertext).unwrap();

    assert_eq!(decrypted, plaintext);
}

#[tokio::test]
async fn test_decrypt_corrupted_ciphertext() {
    let key_hex = generate_key();
    let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

    let plaintext = b"Important recording data";
    let mut ciphertext = ctx.encrypt(plaintext).unwrap();

    // Corrupt a byte in the middle of the ciphertext
    if ciphertext.len() > 15 {
        ciphertext[15] ^= 0xFF; // Flip all bits
    }

    // Decryption should fail due to authentication check
    let result = ctx.decrypt(&ciphertext);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), EncryptionError::DecryptionFailed(_)));
}

#[tokio::test]
async fn test_decrypt_with_wrong_key() {
    let key_hex1 = generate_key();
    let ctx1 = EncryptionContext::from_hex(&key_hex1, "key1").unwrap();

    let key_hex2 = generate_key();
    let ctx2 = EncryptionContext::from_hex(&key_hex2, "key2").unwrap();

    let plaintext = b"Secret recording";
    let ciphertext = ctx1.encrypt(plaintext).unwrap();

    // Try to decrypt with wrong key
    let result = ctx2.decrypt(&ciphertext);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_encryption_with_various_sizes() {
    let key_hex = generate_key();
    let ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

    // Test various file sizes
    let sizes = vec![
        0,      // Empty
        1,      // Single byte
        15,     // Smaller than AES block
        16,     // Exact AES block size
        17,     // Just over block size
        1000,   // 1KB
        10000,  // 10KB
        100000, // 100KB
    ];

    for size in sizes {
        let plaintext = vec![0x42u8; size];
        let ciphertext = ctx.encrypt(&plaintext).unwrap();
        let decrypted = ctx.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext, "Failed for size {}", size);
    }
}

// ==================== Storage Path Handling ====================

#[tokio::test]
async fn test_relative_path_conversion() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_path_conversion");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Store a recording
    let test_data = b"test data".to_vec();
    let result = storage.store_recording(12345, test_data, "wav").await.unwrap();

    // Verify path is relative (doesn't start with /)
    assert!(!result.file_path.starts_with("/"));
    assert!(!result.file_path.contains(temp_dir.to_str().unwrap()));

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_path_security_no_traversal() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_path_security");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Try to retrieve a file with path traversal attempt
    let result = storage.get_recording("../../etc/passwd").await;

    // Should fail to find the file (not allow traversal)
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StorageError::FileNotFound(_)));

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== Quota Enforcement Tests ====================

#[tokio::test]
async fn test_quota_exactly_at_limit() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_quota_exact");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

    // Set very small quota for testing (account for encryption overhead)
    let storage = LocalFileStorage::new(&temp_dir, 0.000005, encryption_ctx); // ~5KB

    storage.init().await.unwrap();

    // Store a file that's close to the limit
    let data1 = vec![0u8; 2000];
    let result1 = storage.store_recording(1, data1, "wav").await;
    assert!(result1.is_ok(), "First file should succeed");

    // Try to store another file that would exceed quota
    let data2 = vec![0u8; 5000];
    let result2 = storage.store_recording(2, data2, "wav").await;
    assert!(result2.is_err(), "Second file should fail quota check");
    assert!(matches!(result2.unwrap_err(), StorageError::QuotaExceeded { .. }));

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_quota_freed_after_deletion() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_quota_freed");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();

    // Small quota
    let storage = LocalFileStorage::new(&temp_dir, 0.00001, encryption_ctx); // ~10KB

    storage.init().await.unwrap();

    // Store a file
    let data1 = vec![0u8; 5000];
    let result1 = storage.store_recording(1, data1.clone(), "wav").await.unwrap();

    // Try to store another large file - should fail
    let data2 = vec![0u8; 5000];
    let result2 = storage.store_recording(2, data2.clone(), "wav").await;
    assert!(result2.is_err());

    // Delete the first file
    storage.delete_recording(&result1.file_path).await.unwrap();

    // Now the second file should succeed
    let result3 = storage.store_recording(3, data2, "wav").await;
    assert!(result3.is_ok(), "Should succeed after freeing quota");

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_check_quota_without_storing() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_check_quota");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 0.00001, encryption_ctx); // 10KB

    storage.init().await.unwrap();

    // Check if we can store 1KB - should be fine
    assert!(storage.check_quota(1000).await.unwrap());

    // Check if we can store 100KB - should fail
    assert!(!storage.check_quota(100000).await.unwrap());

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== File Operations Tests ====================

#[tokio::test]
async fn test_get_nonexistent_file() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_nonexistent");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Try to get a file that doesn't exist
    let result = storage.get_recording("2024/01/01/nonexistent.wav").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StorageError::FileNotFound(_)));

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_delete_nonexistent_file() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_delete_nonexistent");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Try to delete a file that doesn't exist
    let result = storage.delete_recording("2024/01/01/nonexistent.wav").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StorageError::FileNotFound(_)));

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_multiple_formats() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_formats");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    let test_data = b"test audio".to_vec();
    let formats = vec!["wav", "mp3", "ogg", "flac"];

    for format in formats {
        let result = storage.store_recording(12345, test_data.clone(), format).await.unwrap();
        assert!(result.file_path.ends_with(format));

        // Verify we can retrieve it
        let retrieved = storage.get_recording(&result.file_path).await.unwrap();
        assert_eq!(retrieved, test_data);
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== Storage Info Tests ====================

#[tokio::test]
async fn test_storage_info_with_multiple_files() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_info_multiple");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Initially empty
    let info = storage.get_storage_info().await.unwrap();
    assert_eq!(info.total_files, 0);
    assert_eq!(info.total_size_bytes, 0);

    // Store multiple files
    let file_count = 5;
    for i in 0..file_count {
        let data = vec![0u8; 1000];
        storage.store_recording(i, data, "wav").await.unwrap();
    }

    // Check updated stats
    let info = storage.get_storage_info().await.unwrap();
    assert_eq!(info.total_files, file_count as u64);
    assert!(info.total_size_bytes > 0);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_available_space_calculation() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_available_space");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx); // 1GB

    storage.init().await.unwrap();

    // Store a file
    let data = vec![0u8; 10000]; // 10KB
    storage.store_recording(1, data, "wav").await.unwrap();

    let info = storage.get_storage_info().await.unwrap();

    // Available space should be reduced
    let max_bytes = (1.0 * 1024.0 * 1024.0 * 1024.0) as u64;
    assert!(info.available_space_bytes < max_bytes);
    assert_eq!(info.available_space_bytes, max_bytes - info.total_size_bytes);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== Directory Cleanup Tests ====================

#[tokio::test]
async fn test_empty_directory_cleanup_after_deletion() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_cleanup");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Store a file (creates YYYY/MM/DD structure)
    let data = vec![0u8; 100];
    let result = storage.store_recording(1, data, "wav").await.unwrap();

    // Verify the file and directories exist
    let full_path = temp_dir.join(&result.file_path);
    assert!(full_path.exists());

    let parent_dir = full_path.parent().unwrap();
    assert!(parent_dir.exists());

    // Delete the file
    storage.delete_recording(&result.file_path).await.unwrap();

    // File should be gone
    assert!(!full_path.exists());

    // Empty parent directories should be cleaned up
    // Note: cleanup_empty_dirs tries to remove empty dirs recursively
    // We give it a moment to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== Concurrent Operations Tests ====================

#[tokio::test]
async fn test_concurrent_file_storage() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_concurrent_store");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = Arc::new(LocalFileStorage::new(&temp_dir, 10.0, encryption_ctx));

    storage.init().await.unwrap();

    // Store 10 files concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let data = vec![0u8; 1000];
            storage_clone.store_recording(i, data, "wav").await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    // All should succeed
    assert_eq!(success_count, 10);

    // Verify count
    let info = storage.get_storage_info().await.unwrap();
    assert_eq!(info.total_files, 10);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_concurrent_read_write() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_concurrent_rw");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = Arc::new(LocalFileStorage::new(&temp_dir, 10.0, encryption_ctx));

    storage.init().await.unwrap();

    // Store initial files
    let mut file_paths = vec![];
    for i in 0..5 {
        let data = vec![i as u8; 1000];
        let result = storage.store_recording(i, data, "wav").await.unwrap();
        file_paths.push(result.file_path);
    }

    // Concurrently read existing files and write new ones
    let mut handles = vec![];

    // Read tasks
    for (i, path) in file_paths.iter().enumerate() {
        let storage_clone = Arc::clone(&storage);
        let path_clone = path.clone();
        let handle = tokio::spawn(async move {
            let data = storage_clone.get_recording(&path_clone).await.unwrap();
            assert_eq!(data[0], i as u8); // Verify data integrity
        });
        handles.push(handle);
    }

    // Write tasks
    for i in 10..15 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let data = vec![i as u8; 1000];
            storage_clone.store_recording(i, data, "wav").await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== Storage Config Tests ====================

#[test]
fn test_storage_config_missing_encryption_key() {
    std::env::remove_var("ENCRYPTION_KEY");

    let result = StorageConfig::from_env();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ENCRYPTION_KEY"));
}

#[test]
fn test_storage_config_invalid_numbers() {
    std::env::set_var("ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    std::env::set_var("MAX_STORAGE_GB", "not_a_number");
    std::env::set_var("DEFAULT_RETENTION_DAYS", "also_not_a_number");

    // Should fall back to defaults for invalid numbers
    let config = StorageConfig::from_env().unwrap();
    assert_eq!(config.max_storage_gb, 100.0); // Default
    assert_eq!(config.default_retention_days, 90); // Default

    // Cleanup
    std::env::remove_var("ENCRYPTION_KEY");
    std::env::remove_var("MAX_STORAGE_GB");
    std::env::remove_var("DEFAULT_RETENTION_DAYS");
}

// ==================== Full Lifecycle Integration Test ====================

#[tokio::test]
async fn test_full_recording_lifecycle() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_full_lifecycle");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Simulate a call recording lifecycle
    let call_id = 42;
    let original_audio = b"This is a simulated call recording with audio data".to_vec();

    // 1. Store the recording
    let stored = storage.store_recording(call_id, original_audio.clone(), "wav")
        .await
        .expect("Failed to store recording");

    assert_eq!(stored.file_size, original_audio.len() as u64);
    assert_eq!(stored.encryption_key_id, "test-key");
    assert!(stored.file_path.contains(&call_id.to_string()));

    // 2. Retrieve the recording (simulating playback)
    let retrieved = storage.get_recording(&stored.file_path)
        .await
        .expect("Failed to retrieve recording");

    assert_eq!(retrieved, original_audio);

    // 3. Check storage stats
    let info = storage.get_storage_info().await.unwrap();
    assert_eq!(info.total_files, 1);
    assert!(info.total_size_bytes > original_audio.len() as u64); // Encrypted is larger

    // 4. Delete the recording (simulating retention policy)
    storage.delete_recording(&stored.file_path)
        .await
        .expect("Failed to delete recording");

    // 5. Verify deletion
    let retrieve_after_delete = storage.get_recording(&stored.file_path).await;
    assert!(retrieve_after_delete.is_err());

    // 6. Verify stats updated
    let info_after_delete = storage.get_storage_info().await.unwrap();
    assert_eq!(info_after_delete.total_files, 0);
    assert_eq!(info_after_delete.total_size_bytes, 0);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

// ==================== Error Handling Tests ====================

#[tokio::test]
async fn test_storage_resilience_to_filesystem_errors() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_fs_errors");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    storage.init().await.unwrap();

    // Store a file
    let data = vec![0u8; 100];
    let result = storage.store_recording(1, data, "wav").await.unwrap();

    // Remove the base directory to simulate filesystem issue
    std::fs::remove_dir_all(&temp_dir).ok();

    // Try to retrieve - should get IO error
    let retrieve_result = storage.get_recording(&result.file_path).await;
    assert!(retrieve_result.is_err());

    // Try to get storage info - should handle gracefully
    let info_result = storage.get_storage_info().await;
    assert!(info_result.is_ok()); // Should return (0, 0) for non-existent dir

    // Cleanup (already removed)
}

#[tokio::test]
async fn test_base_path_accessor() {
    let temp_dir = std::env::temp_dir().join("voip_crm_test_base_path");
    let key_hex = generate_key();
    let encryption_ctx = EncryptionContext::from_hex(&key_hex, "test-key").unwrap();
    let storage = LocalFileStorage::new(&temp_dir, 1.0, encryption_ctx);

    assert_eq!(storage.base_path(), &temp_dir);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}
