use clap::Args;
use crate::calldata::build_unwrap_calldata;
use crate::config::{format_units, parse_units, rpc_url, weeth_address, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{get_balance, weeth_convert_to_assets};

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

    // Preview: how much eETH will be returned
    let eeth_expected = weeth_convert_to_assets(weeth, weeth_wei, rpc)
        .await
        .unwrap_or(0);

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

    // Build weETH.redeem(shares, receiver, owner) calldata — ERC-4626 redeem
    // No approve needed: weETH.redeem() only requires `owner == msg.sender`
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
        "{{\"ok\":true,\"txHash\":\"{}\",\"action\":\"unwrap\",\"weETHRedeemed\":\"{}\",\"weETHWei\":\"{}\",\"eETHExpected\":\"{}\",\"eETHBalance\":\"{}\"}}",
        tx_hash,
        args.amount,
        weeth_wei,
        format_units(eeth_expected, 18),
        eeth_balance_str
    );

    Ok(())
}
