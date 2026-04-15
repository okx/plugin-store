use clap::Args;
use crate::calldata::build_wrap_calldata;
use crate::config::{
    build_approve_calldata, eeth_address, format_units, parse_units,
    rpc_url, weeth_address, CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_contract_call};
use crate::rpc::{get_allowance, get_balance};

#[derive(Args)]
pub struct WrapArgs {
    /// Amount of eETH to wrap into weETH (e.g. "0.5", "1.0")
    #[arg(long)]
    pub amount: String,
    /// Dry run — build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: WrapArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let eeth = eeth_address();
    let weeth = weeth_address();

    // Parse eETH amount to wei (18 decimals)
    let eeth_wei = parse_units(&args.amount, 18)?;

    if eeth_wei == 0 {
        anyhow::bail!("Amount must be greater than zero.");
    }

    // Resolve wallet address
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Preview: expected weETH output via getRate() — 1 weETH = rate eETH → weETH = eETH / rate
    let weeth_expected_str = match crate::rpc::weeth_get_rate(weeth, rpc).await {
        Ok(rate) if rate > 0.0 => {
            let expected = (eeth_wei as f64 / rate) as u128;
            format!("{:.6}", expected as f64 / 1e18)
        }
        _ => "N/A".to_string(),
    };

    println!(
        "Wrapping {} eETH ({} wei) → weETH",
        args.amount, eeth_wei
    );
    println!("  eETH contract:  {}", eeth);
    println!("  weETH contract: {}", weeth);
    println!("  Wallet: {}", wallet);
    println!("  Expected weETH to receive: {}", weeth_expected_str);
    println!("  Run with --confirm to broadcast. (Proceeding automatically in non-interactive mode.)");

    // Step 1: Check eETH balance
    if !args.dry_run {
        let eeth_balance = get_balance(eeth, &wallet, rpc).await?;
        if eeth_balance < eeth_wei {
            anyhow::bail!(
                "Insufficient eETH balance. Have {} wei ({} eETH), need {} wei ({} eETH).",
                eeth_balance,
                format_units(eeth_balance, 18),
                eeth_wei,
                args.amount,
            );
        }
    }

    // Step 2: Approve weETH contract to spend eETH (ERC-20 approve)
    if !args.dry_run {
        let allowance = get_allowance(eeth, &wallet, weeth, rpc).await?;
        if allowance < eeth_wei {
            println!(
                "WARNING: This approval grants the weETH contract unlimited (u128::MAX) spending                  access to your eETH. To revoke later, call approve(weETH, 0)."
            );
            println!("Approving weETH contract to spend eETH (unlimited allowance)...");
            let approve_data = build_approve_calldata(weeth, u128::MAX);
            let approve_result = wallet_contract_call(
                CHAIN_ID,
                eeth,
                &approve_data,
                0,  // no ETH value for approve
                args.confirm,
                false,
            )
            .await?;

            if approve_result["preview"].as_bool() == Some(true) {
                println!("Preview (approve): {}", serde_json::to_string_pretty(&approve_result)?);
                println!("Re-run with --confirm to execute approve + wrap.");
                return Ok(());
            }

            let approve_tx = extract_tx_hash(&approve_result).to_string();
            println!("Approve tx: {} — waiting for confirmation...", approve_tx);
            wait_for_tx(approve_tx, wallet.clone()).await
                .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
            println!("Approve confirmed.");
        }
    }

    // Step 3: Call weETH.deposit(assets, receiver) — ERC-4626 wrap
    let calldata = build_wrap_calldata(eeth_wei, &wallet);

    let result = wallet_contract_call(
        CHAIN_ID,
        weeth,
        &calldata,
        0,  // no ETH value — eETH is an ERC-20 transfer
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result);

    // Fetch updated weETH balance if live transaction
    let weeth_balance_str = if !args.dry_run && args.confirm {
        match get_balance(weeth, &wallet, rpc).await {
            Ok(bal) => format_units(bal, 18),
            Err(_) => "N/A".to_string(),
        }
    } else {
        "N/A".to_string()
    };

    println!(
        "{}",
        serde_json::json!({
            "ok":            true,
            "txHash":        tx_hash,
            "action":        "wrap",
            "eETHWrapped":   args.amount,
            "eETHWei":       eeth_wei.to_string(),
            "weETHExpected": weeth_expected_str,
            "weETHBalance":  weeth_balance_str,
        })
    );

    Ok(())
}
