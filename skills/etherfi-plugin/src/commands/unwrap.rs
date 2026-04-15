use clap::Args;
use crate::calldata::build_unwrap_calldata;
use crate::config::{format_units, parse_units, rpc_url, weeth_address, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::get_balance;

#[derive(Args)]
pub struct UnwrapArgs {
    /// Amount of weETH to redeem back to eETH (e.g. "0.5", "1.0")
    #[arg(long)]
    pub amount: String,
    /// Dry run — build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: UnwrapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let weeth = weeth_address();

    // Parse weETH amount to wei (18 decimals)
    let weeth_wei = parse_units(&args.amount, 18)?;

    if weeth_wei == 0 {
        anyhow::bail!("Amount must be greater than zero.");
    }

    // Resolve wallet address
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Preview: how much eETH will be returned.
    // weETH.convertToAssets() reverts on this contract; use getRate() instead.
    let rate = crate::rpc::weeth_get_rate(weeth, rpc).await
        .map_err(|e| anyhow::anyhow!(
            "Failed to fetch weETH exchange rate: {}. Check RPC connectivity and retry.",
            e
        ))?;
    if rate == 0.0 {
        anyhow::bail!(
            "weETH exchange rate returned 0 — RPC may be unavailable or contract unresponsive. \
             Retry in a moment or check https://ethereum-rpc.publicnode.com connectivity."
        );
    }
    let eeth_expected = (weeth_wei as f64 * rate) as u128;

    println!(
        "Unwrapping {} weETH ({} wei) → eETH",
        args.amount, weeth_wei
    );
    println!("  weETH contract: {}", weeth);
    println!("  Wallet: {}", wallet);
    println!(
        "  Expected eETH to receive: {} ({}  wei)",
        format_units(eeth_expected, 18),
        eeth_expected
    );
    println!("  Run with --confirm to broadcast. (Proceeding automatically in non-interactive mode.)");

    // Check weETH balance
    if !args.dry_run {
        let weeth_balance = get_balance(weeth, &wallet, rpc).await?;
        if weeth_balance < weeth_wei {
            anyhow::bail!(
                "Insufficient weETH balance. Have {} wei ({} weETH), need {} wei ({} weETH).",
                weeth_balance,
                format_units(weeth_balance, 18),
                weeth_wei,
                args.amount,
            );
        }
    }

    // Build weETH.unwrap(uint256 _weETHAmount) calldata
    // No approve needed: unwrap() burns caller's weETH directly
    let calldata = build_unwrap_calldata(weeth_wei, &wallet);

    let result = wallet_contract_call(
        CHAIN_ID,
        weeth,
        &calldata,
        0,  // no ETH value
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result);

    // Fetch updated eETH balance if live transaction
    let eeth_balance_str = if !args.dry_run && args.confirm {
        match get_balance(crate::config::eeth_address(), &wallet, rpc).await {
            Ok(bal) => format_units(bal, 18),
            Err(_) => "N/A".to_string(),
        }
    } else {
        "N/A".to_string()
    };

    println!(
        "{}",
        serde_json::json!({
            "ok":           true,
            "txHash":       tx_hash,
            "action":       "unwrap",
            "weETHRedeemed": args.amount,
            "weETHWei":     weeth_wei.to_string(),
            "eETHExpected": format!("{:.6}", eeth_expected as f64 / 1e18),
            "eETHBalance":  eeth_balance_str,
        })
    );

    Ok(())
}
