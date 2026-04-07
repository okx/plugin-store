use serde_json::Value;

/// Unstake sSOL to receive SOL.
///
/// ⚠️  The Solayer REST API does not provide an unstake endpoint
/// (`/api/partner/unrestake/ssol` returns HTTP 500).
/// Unstaking requires complex multi-instruction on-chain transactions
/// (unrestake + create stake account + withdrawStake + deactivate).
///
/// This command returns dry-run guidance only.
/// For actual unstaking, use the Solayer UI: https://app.solayer.org
pub async fn execute(amount: f64, dry_run: bool) -> anyhow::Result<Value> {
    let message = format!(
        "Unstaking {} sSOL requires multi-step on-chain instructions not available via REST API. \
         Please use the Solayer app at https://app.solayer.org to unstake your sSOL.",
        amount
    );

    let result = serde_json::json!({
        "ok": true,
        "dry_run": dry_run,
        "data": {
            "amount_ssol": amount,
            "status": "not_available_via_api",
            "message": message,
            "ui_url": "https://app.solayer.org",
            "description": "Unstaking sSOL involves: (1) unrestake instruction, (2) approve token access, (3) create stake account, (4) withdrawStake, (5) deactivate stake account."
        }
    });
    Ok(result)
}
