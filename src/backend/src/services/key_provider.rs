use base64::{engine::general_purpose, Engine as _};
use std::env;

use crate::error::{AppError, Result};

pub trait KeyProvider: Send + Sync {
    /// Returns the active (latest) encryption key and its version identifier.
    fn get_active_key(&self) -> Result<(Vec<u8>, String)>;

    /// Returns a specific encryption key by its version identifier.
    fn get_key_by_version(&self, version: &str) -> Result<Vec<u8>>;
}

/// A simple environment-based key provider for development/testing or
/// environments where a KMS/Vault is injected via env vars.
pub struct EnvKeyProvider {
    /// The default version to use if none is specified or for new encryptions
    pub active_version: String,
}

impl EnvKeyProvider {
    pub fn new() -> Self {
        // We use a simplified versioning scheme for the env provider.
        // It reads from CTC_ENCRYPTION_KEY_V1, CTC_ENCRYPTION_KEY_V2, etc.
        // And CTC_ACTIVE_KEY_VERSION defines the current active version.
        let active_version =
            env::var("CTC_ACTIVE_KEY_VERSION").unwrap_or_else(|_| "v1".to_string());
        Self { active_version }
    }

    fn retrieve_key_from_env(&self, version: &str) -> Result<Vec<u8>> {
        let env_var_name = format!("CTC_ENCRYPTION_KEY_{}", version.to_uppercase());
        let key_b64 = env::var(&env_var_name).map_err(|_| {
            AppError::Internal(format!(
                "Decryption key for version {} not found in environment",
                version
            ))
        })?;

        let key_bytes = general_purpose::STANDARD
            .decode(key_b64.trim())
            .map_err(|e| {
                AppError::Internal(format!(
                    "Failed to decode base64 key for version {}: {}",
                    version, e
                ))
            })?;

        if key_bytes.len() != 32 {
            return Err(AppError::Internal(format!(
                "Invalid key length for version {}: expected 32 bytes (256 bits), got {}",
                version,
                key_bytes.len()
            )));
        }

        Ok(key_bytes)
    }
}

impl KeyProvider for EnvKeyProvider {
    fn get_active_key(&self) -> Result<(Vec<u8>, String)> {
        let key = self.retrieve_key_from_env(&self.active_version)?;
        Ok((key, self.active_version.clone()))
    }

    fn get_key_by_version(&self, version: &str) -> Result<Vec<u8>> {
        self.retrieve_key_from_env(version)
    }
}
