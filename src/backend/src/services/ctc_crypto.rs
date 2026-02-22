use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use rand::RngCore;

use crate::error::{AppError, Result};
use crate::services::key_provider::KeyProvider;

pub struct EncryptedPayload {
    pub ciphertext: String, // Stored as base64 string
    pub key_version: String,
    pub encryption_version: String,
    pub algorithm: String,
    pub encrypted_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait CtcCryptoService: Send + Sync {
    async fn encrypt_components(&self, components_json: &serde_json::Value) -> Result<EncryptedPayload>;
    async fn decrypt_components(&self, payload: &EncryptedPayload) -> Result<serde_json::Value>;
}

pub struct DefaultCtcCryptoService<K: KeyProvider> {
    key_provider: K,
}

impl<K: KeyProvider> DefaultCtcCryptoService<K> {
    pub fn new(key_provider: K) -> Self {
        Self { key_provider }
    }
}

#[async_trait::async_trait]
impl<K: KeyProvider> CtcCryptoService for DefaultCtcCryptoService<K> {
    async fn encrypt_components(&self, components_json: &serde_json::Value) -> Result<EncryptedPayload> {
        let (key_bytes, key_version) = self.key_provider.get_active_key()?;
        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(cipher_key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes); // 96-bits

        let plaintext = serde_json::to_vec(components_json).map_err(|e| {
            AppError::Internal(format!("Failed to serialize components for encryption: {}", e))
        })?;

        // Authenticated encryption
        let mut encrypted_data = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|e| {
            AppError::Internal(format!("Encryption failed: {}", e))
        })?;

        // Prepend nonce to the encrypted data
        let mut final_payload = nonce_bytes.to_vec();
        final_payload.append(&mut encrypted_data);

        let ciphertext_b64 = general_purpose::STANDARD.encode(final_payload);

        Ok(EncryptedPayload {
            ciphertext: ciphertext_b64,
            key_version,
            encryption_version: "v1".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            encrypted_at: Utc::now(),
        })
    }

    async fn decrypt_components(&self, payload: &EncryptedPayload) -> Result<serde_json::Value> {
        if payload.algorithm != "AES-256-GCM" {
            return Err(AppError::Internal(format!(
                "Unsupported encryption algorithm: {}",
                payload.algorithm
            )));
        }

        let key_bytes = self.key_provider.get_key_by_version(&payload.key_version)?;
        if key_bytes.len() != 32 {
            return Err(AppError::Internal("Invalid decryption key size".to_string()));
        }

        let cipher_key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(cipher_key);

        let final_payload = general_purpose::STANDARD.decode(&payload.ciphertext).map_err(|e| {
            AppError::Internal(format!("Failed to decode base64 ciphertext: {}", e))
        })?;

        if final_payload.len() < 12 {
            return Err(AppError::Internal("Invalid ciphertext: too short".to_string()));
        }

        let nonce_bytes = &final_payload[..12];
        let encrypted_data = &final_payload[12..];

        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, encrypted_data).map_err(|e| {
            AppError::Internal(format!("Decryption failed: possible tampering or incorrect key: {}", e))
        })?;

        let components_json: serde_json::Value = serde_json::from_slice(&plaintext).map_err(|e| {
            AppError::Internal(format!("Failed to deserialize decrypted components: {}", e))
        })?;

        Ok(components_json)
    }
}

pub async fn backfill_plaintext_ctc_records(pool: &sqlx::PgPool, crypto_svc: &impl CtcCryptoService) -> Result<usize> {
    // Backfill either missing component encryption or missing encrypted daily-rate payload.
    let unencrypted_records = sqlx::query(
        "SELECT resource_id, components, daily_rate, encrypted_components, encrypted_daily_rate,
                key_version, encryption_version, encryption_algorithm, encrypted_at
         FROM ctc_records
         WHERE encrypted_components IS NULL OR encrypted_daily_rate IS NULL"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let mut count = 0;
    for record in unencrypted_records {
        use sqlx::Row;
        let plaintext_components: serde_json::Value = record
            .try_get("components")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let plaintext_daily_rate: sqlx::types::BigDecimal = record
            .try_get("daily_rate")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let existing_encrypted_components: Option<String> = record
            .try_get("encrypted_components")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let existing_encrypted_daily_rate: Option<String> = record
            .try_get("encrypted_daily_rate")
            .map_err(|e| AppError::Database(e.to_string()))?;
        let resource_id: uuid::Uuid = record.try_get("resource_id").map_err(|e| AppError::Database(e.to_string()))?;

        let (encrypted_components_ciphertext, key_version, encryption_version, algorithm, encrypted_at) =
            if let Some(existing_ciphertext) = existing_encrypted_components {
                let existing_key_version: String = record
                    .try_get("key_version")
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let existing_encryption_version: String = record
                    .try_get("encryption_version")
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let existing_algorithm: String = record
                    .try_get("encryption_algorithm")
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let existing_encrypted_at: Option<DateTime<Utc>> = record
                    .try_get("encrypted_at")
                    .map_err(|e| AppError::Database(e.to_string()))?;

                (
                    existing_ciphertext,
                    existing_key_version,
                    existing_encryption_version,
                    existing_algorithm,
                    existing_encrypted_at.unwrap_or_else(Utc::now),
                )
            } else {
                let encrypted_payload = crypto_svc.encrypt_components(&plaintext_components).await?;
                (
                    encrypted_payload.ciphertext,
                    encrypted_payload.key_version,
                    encrypted_payload.encryption_version,
                    encrypted_payload.algorithm,
                    encrypted_payload.encrypted_at,
                )
            };

        let encrypted_daily_rate_ciphertext = if let Some(existing_daily_ciphertext) = existing_encrypted_daily_rate {
            existing_daily_ciphertext
        } else {
            let encrypted_daily_rate_payload = crypto_svc
                .encrypt_components(&serde_json::json!({
                    "daily_rate": plaintext_daily_rate.to_string(),
                }))
                .await?;
            encrypted_daily_rate_payload.ciphertext
        };

        sqlx::query(
            "UPDATE ctc_records 
             SET encrypted_components = $1, 
                 encrypted_daily_rate = $2,
                 components = '{}'::jsonb,
                 base_salary = 0,
                 hra_allowance = 0,
                 medical_allowance = 0,
                 transport_allowance = 0,
                 meal_allowance = 0,
                 bpjs_kesehatan = 0,
                 bpjs_ketenagakerjaan = 0,
                 thr_monthly_accrual = 0,
                 total_monthly_ctc = 0,
                 daily_rate = 0,
                 key_version = $3, 
                 encryption_version = $4, 
                 encryption_algorithm = $5,
                 encrypted_at = $6
             WHERE resource_id = $7"
        )
        .bind(&encrypted_components_ciphertext)
        .bind(&encrypted_daily_rate_ciphertext)
        .bind(&key_version)
        .bind(&encryption_version)
        .bind(&algorithm)
        .bind(&encrypted_at)
        .bind(&resource_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        
        count += 1;
    }

    Ok(count)
}
