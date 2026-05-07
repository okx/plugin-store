use anyhow::Result;
use reqwest::Client;

use crate::api::get_clob_version;
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
    match run_inner().await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("create-readonly-key"), None)); Ok(()) }
    }
}

async fn run_inner() -> Result<()> {
    let client = Client::new();

    // create-readonly-key is a CLOB v2-only endpoint — fail early with a clear message
    // rather than an opaque "Unauthorized" from the server.
    let clob_version = get_clob_version(&client).await.unwrap_or(1);
    if clob_version < 2 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "create-readonly-key requires CLOB v2 (server is currently v1)",
                "suggestion": "The /auth/readonly-api-key endpoint is only available after the \
                               Polymarket CLOB v2 upgrade. Check again once the upgrade is live."
            }))?
        );
        return Ok(());
    }

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
