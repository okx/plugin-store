use anyhow::Result;
use reqwest::Client;

use crate::auth::create_readonly_api_key;
use crate::onchainos::get_wallet_address;

/// Create a read-only Polymarket API key (CLOB v2 feature).
///
/// The key has the same api_key/secret/passphrase triplet as a standard key but the
/// CLOB server rejects any write operations (order placement, cancellation). Suitable
/// for monitoring scripts, dashboards, and CI pipelines that need read access without
/// exposing trading credentials.
///
/// The key is NOT saved to `~/.config/polymarket/creds.json` — it is printed to stdout
/// once. Store it securely if you intend to reuse it.
pub async fn run() -> Result<()> {
    let client = Client::new();
    let wallet_addr = get_wallet_address().await?;

    eprintln!("[polymarket] Creating read-only API key for {}...", wallet_addr);
    let key = create_readonly_api_key(&client, &wallet_addr).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "api_key": key.api_key,
                "secret": key.secret,
                "passphrase": key.passphrase,
                "wallet": wallet_addr,
                "note": "Read-only key: GET operations only. Write operations will be rejected by the CLOB server. \
                         Store securely — this key is not saved to creds.json.",
            }
        }))?
    );
    Ok(())
}
