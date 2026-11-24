use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct WrapArgs {
    /// Amount of stETH to wrap into wstETH (e.g. "1.5")
    #[arg(long)]
    pub amount_steth: f64,
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

pub async fn run(args: WrapArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    let wallet = match args.from.clone() {
        Some(f) => f,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let amount_wei = (args.amount_steth * 1e18_f64).round() as u128;
    if amount_wei == 0 {
        anyhow::bail!("Amount must be greater than 0.");
    }

    // Preview: expected wstETH output — non-fatal, shows "N/A" on RPC failure
    let wsteth_expected = preview_wsteth_out(chain_id, amount_wei).await;

    // Pre-flight: check stETH balance
    if !args.dry_run {
        let balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &wallet);
        let balance_result = onchainos::eth_call(chain_id, config::STETH_ADDRESS, &balance_calldata)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch stETH balance: {}", e))?;
        let balance_wei = rpc::extract_return_data(&balance_result)
            .and_then(|h| rpc::decode_uint256(&h))
            .map_err(|e| anyhow::anyhow!("Failed to decode stETH balance: {}", e))?;
        if balance_wei < amount_wei {
            anyhow::bail!(
                "Insufficient stETH balance. Have {:.6} stETH ({} wei), need {:.6} stETH ({} wei).",
                balance_wei as f64 / 1e18, balance_wei,
                args.amount_steth, amount_wei,
            );
        }
    }

    // Calldata: wstETH.wrap(uint256 _stETHAmount)
    let wrap_calldata = format!(
        "0x{}{}",
        config::SEL_WSTETH_WRAP,
        rpc::encode_uint256_u128(amount_wei)
    );
    // Calldata: stETH.approve(wstETH, amount)
    let approve_calldata = rpc::calldata_approve(config::WSTETH_ADDRESS, amount_wei);

    println!("=== Lido Wrap (stETH → wstETH) ===");
    println!("From:          {}", wallet);
    println!("Amount:        {:.6} stETH ({} wei)", args.amount_steth, amount_wei);
    println!("Expected out:  {} wstETH", wsteth_expected);
    println!("stETH:         {}", config::STETH_ADDRESS);
    println!("wstETH:        {}", config::WSTETH_ADDRESS);

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": "wrap",
            "stETHAmount":   format!("{:.6}", args.amount_steth),
            "stETHWei":      amount_wei.to_string(),
            "wstETHExpected": wsteth_expected,
            "approve_calldata": approve_calldata,
            "wrap_calldata": wrap_calldata,
        }));
        return Ok(());
    }

    if !args.confirm {
        println!("\nAdd --confirm to execute this transaction.");
        return Ok(());
    }

    // Step 1: Approve stETH → wstETH contract
    println!("\nStep 1/2: Approving stETH spend for wstETH contract...");
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        config::STETH_ADDRESS,
        &approve_calldata,
        Some(&wallet),
        None,
        args.confirm,
        args.dry_run,
    )
    .await?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result, "Approve")?;
    println!("Approve tx: {} — waiting for confirmation...", approve_tx);
    onchainos::wait_for_receipt(chain_id, &approve_tx, 120)
        .await
        .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
    println!("Approve confirmed.");

    // Step 2: Wrap
    println!("Step 2/2: Wrapping stETH → wstETH...");
    let wrap_result = onchainos::wallet_contract_call(
        chain_id,
        config::WSTETH_ADDRESS,
        &wrap_calldata,
        Some(&wallet),
        None,
        args.confirm,
        args.dry_run,
    )
    .await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&wrap_result, "wrap")?;
    onchainos::wait_for_receipt(chain_id, &tx_hash, 120).await?;

    println!(
        "{}",
        serde_json::json!({
            "ok":            true,
            "txHash":        tx_hash,
            "action":        "wrap",
            "stETHWrapped":  format!("{:.6}", args.amount_steth),
            "stETHWei":      amount_wei.to_string(),
            "wstETHExpected": wsteth_expected,
        })
    );

    Ok(())
}

/// Preview: estimate wstETH output by deriving rate from getStETHByWstETH(1e18).
/// wstETH_out = stETH_in / rate (where rate = stETH per 1 wstETH).
/// Non-fatal — returns "N/A" if RPC call fails.
async fn preview_wsteth_out(chain_id: u64, steth_wei: u128) -> String {
    // getStETHByWstETH(1e18) → stETH per wstETH (exchange rate)
    let one_wsteth: u128 = 1_000_000_000_000_000_000;
    let calldata = format!(
        "0x{}{}",
        config::SEL_GET_STETH_BY_WSTETH,
        rpc::encode_uint256_u128(one_wsteth)
    );
    match onchainos::eth_call(chain_id, config::WSTETH_ADDRESS, &calldata).await {
        Ok(result) => match rpc::extract_return_data(&result).and_then(|h| rpc::decode_uint256(&h)) {
            Ok(rate_wei) if rate_wei > 0 => {
                let wsteth_out = steth_wei as f64 / (rate_wei as f64 / 1e18) / 1e18;
                format!("{:.6}", wsteth_out)
            }
            _ => "N/A".to_string(),
        },
        Err(_) => "N/A".to_string(),
    }
}
