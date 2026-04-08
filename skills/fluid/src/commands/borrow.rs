/// Borrow from Fluid Vault (dry-run only — liquidation risk).
pub async fn run(
    _vault: &str,
    _amount: &str,
    _chain_id: u64,
    _from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !dry_run {
        anyhow::bail!(
            "Borrow is only supported in --dry-run mode due to liquidation risk. \
             Re-run with --dry-run to simulate. \
             To borrow, supply collateral to a Fluid Vault first and ensure adequate collateral ratio."
        );
    }

    let output = serde_json::json!({
        "ok": true,
        "operation": "borrow",
        "dryRun": true,
        "note": "Borrow is dry-run only. Fluid Vault borrow requires: 1) supply collateral to vault, 2) call vault.borrow(). Due to liquidation risk, live execution is disabled.",
        "documentation": "https://docs.fluid.instadapp.io/",
        "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
