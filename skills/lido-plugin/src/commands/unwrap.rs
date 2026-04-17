use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct UnwrapArgs {
    /// Amount of wstETH to unwrap back to stETH (e.g. "1.0")
    #[arg(long)]
    pub amount_wsteth: f64,
    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,
    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: UnwrapArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    let wallet = match args.from.clone() {
        Some(f) => f,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let amount_wei = (args.amount_wsteth * 1e18_f64).round() as u128;
    if amount_wei == 0 {
        anyhow::bail!("Amount must be greater than 0.");
    }

    // Preview: expected stETH output — non-fatal, shows "N/A" on RPC failure
    let steth_expected = preview_steth_out(chain_id, amount_wei).await;

    // Pre-flight: check wstETH balance
    if !args.dry_run {
        let balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &wallet);
        let balance_result = onchainos::eth_call(chain_id, config::WSTETH_ADDRESS, &balance_calldata)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch wstETH balance: {}", e))?;
        let balance_wei = rpc::extract_return_data(&balance_result)
            .and_then(|h| rpc::decode_uint256(&h))
            .map_err(|e| anyhow::anyhow!("Failed to decode wstETH balance: {}", e))?;
        if balance_wei < amount_wei {
            anyhow::bail!(
                "Insufficient wstETH balance. Have {:.6} wstETH ({} wei), need {:.6} wstETH ({} wei).",
                balance_wei as f64 / 1e18, balance_wei,
                args.amount_wsteth, amount_wei,
            );
        }
    }

    // Calldata: wstETH.unwrap(uint256 _wstETHAmount) — burns caller's wstETH, no approve needed
    let unwrap_calldata = format!(
        "0x{}{}",
        config::SEL_WSTETH_UNWRAP,
        rpc::encode_uint256_u128(amount_wei)
    );

    println!("=== Lido Unwrap (wstETH → stETH) ===");
    println!("From:          {}", wallet);
    println!("Amount:        {:.6} wstETH ({} wei)", args.amount_wsteth, amount_wei);
    println!("Expected out:  {} stETH", steth_expected);
    println!("wstETH:        {}", config::WSTETH_ADDRESS);
    println!("Note: No approval needed — unwrap burns caller's wstETH directly.");

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": "unwrap",
            "wstETHAmount":  format!("{:.6}", args.amount_wsteth),
            "wstETHWei":     amount_wei.to_string(),
            "stETHExpected": steth_expected,
            "calldata":      unwrap_calldata,
        }));
        return Ok(());
    }

    if !args.confirm {
        println!("\nAdd --confirm to execute this transaction.");
        return Ok(());
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::WSTETH_ADDRESS,
        &unwrap_calldata,
        Some(&wallet),
        None,
        args.confirm,
        args.dry_run,
    )
    .await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result, "unwrap")?;
    onchainos::wait_for_receipt(chain_id, &tx_hash, 120).await?;

    println!(
        "{}",
        serde_json::json!({
            "ok":             true,
            "txHash":         tx_hash,
            "action":         "unwrap",
            "wstETHUnwrapped": format!("{:.6}", args.amount_wsteth),
            "wstETHWei":      amount_wei.to_string(),
            "stETHExpected":  steth_expected,
        })
    );

    Ok(())
}

/// Preview: call wstETH.getStETHByWstETH(amount) to estimate output.
/// Non-fatal — returns "N/A" if RPC call fails.
async fn preview_steth_out(chain_id: u64, wsteth_wei: u128) -> String {
    let calldata = format!(
        "0x{}{}",
        config::SEL_GET_STETH_BY_WSTETH,
        rpc::encode_uint256_u128(wsteth_wei)
    );
    match onchainos::eth_call(chain_id, config::WSTETH_ADDRESS, &calldata).await {
        Ok(result) => match rpc::extract_return_data(&result).and_then(|h| rpc::decode_uint256(&h)) {
            Ok(steth_wei) => format!("{:.6}", steth_wei as f64 / 1e18),
            Err(_) => "N/A".to_string(),
        },
        Err(_) => "N/A".to_string(),
    }
}
