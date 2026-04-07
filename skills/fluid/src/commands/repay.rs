/// Repay Fluid Vault debt (dry-run only — liquidation risk).
pub async fn run(
    _vault: &str,
    _amount: Option<&str>,
    _all: bool,
    _chain_id: u64,
    _from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !dry_run {
        anyhow::bail!(
            "Repay is only supported in --dry-run mode. \
             Re-run with --dry-run to simulate. \
             Full repay execution requires careful handling of borrow shares to avoid dust."
        );
    }

    let output = serde_json::json!({
        "ok": true,
        "operation": "repay",
        "dryRun": true,
        "note": "Repay is dry-run only. Fluid Vault repay requires: 1) approve vault for repay token, 2) call vault.payback(). Due to vault state complexity, live execution is disabled.",
        "documentation": "https://docs.fluid.instadapp.io/",
        "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
