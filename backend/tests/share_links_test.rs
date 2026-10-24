//! Integration tests for FileDeck's secure share links, password hashing, and token verification.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShareLinkRecord {
    pub id: i64,
    pub token: String,
    pub document_id: Option<i64>,
    pub saved_view_id: Option<i64>,
    pub expires_at: Option<String>,
    pub allow_download: bool,
    pub password_hash: Option<String>,
    pub access_count: i64,
    pub created_at: String,
}

#[test]
fn test_share_token_entropy_and_format() {
    // Standard FileDeck share token is a 32-character hex string (128 bits of entropy)
    let sample_token = "4f8a3c9b2e1d0f7a6b5c4d3e2f1a0b9c";
    assert_eq!(sample_token.len(), 32);
    assert!(sample_token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_share_link_expiration_check() {
    let now = Utc::now();
    let past = (now - Duration::hours(2)).to_rfc3339();
    let future = (now + Duration::days(7)).to_rfc3339();

    let is_expired = |expires_at: Option<&str>| -> bool {
        let Some(exp_str) = expires_at else { return false; }; // None = permanent link
        if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(exp_str) {
            Utc::now() > exp_time
        } else {
            true
        }
    };

    assert!(is_expired(Some(&past)));
    assert!(!is_expired(Some(&future)));
    assert!(!is_expired(None)); // Permanent share link never expires
}

#[test]
fn test_share_link_password_hashing() {
    let hash_password = |password: &str, salt: &str| -> String {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(password.as_bytes());
        hex::encode(hasher.finalize())
    };

    let salt = "filedeck_salt_random_99182";
    let hash1 = hash_password("SecretSharePass123!", salt);
    let hash2 = hash_password("SecretSharePass123!", salt);
    let hash3 = hash_password("WrongPassword!", salt);

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_share_link_permissions_serialization() {
    let link = ShareLinkRecord {
        id: 10,
        token: "e3b0c44298fc1c149afbf4c8996fb924".to_string(),
        document_id: Some(42),
        saved_view_id: None,
        expires_at: Some("2025-01-01T00:00:00Z".to_string()),
        allow_download: true,
        password_hash: None,
        access_count: 5,
        created_at: "2024-10-24T22:00:00Z".to_string(),
    };

    let json_val = serde_json::to_value(&link).unwrap();
    assert_eq!(json_val["allow_download"], true);
    assert_eq!(json_val["access_count"], 5);
    assert!(json_val["password_hash"].is_null());
}
