use xynergy_backend::services::ctc_crypto::{backfill_plaintext_ctc_records, DefaultCtcCryptoService};
use xynergy_backend::services::key_provider::EnvKeyProvider;
use xynergy_backend::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::new().await?;
    let crypto = DefaultCtcCryptoService::new(EnvKeyProvider::new());
    let count = backfill_plaintext_ctc_records(db.pool(), &crypto).await?;
    println!("backfilled_rows={}", count);
    Ok(())
}
