use crate::config;
use crate::onchainos;
use clap::Args;

#[derive(Args)]
pub struct StakeFrxArgs {
    /// Amount of frxETH to stake (e.g. 0.00005)
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (only Ethereum mainnet supported)
    #[arg(long, default_value = "1")]
    pub chain: u64,

    /// Simulate without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

/// Stake frxETH to receive sfrxETH via ERC-4626 deposit.
/// Step 1: ERC-20 approve frxETH → sfrxETH
/// Step 2: ERC-4626 deposit(assets, receiver)
pub async fn run(args: StakeFrxArgs) -> anyhow::Result<()> {
    // Dry-run guard must be before resolve_wallet
    if args.dry_run {
        let amt_wei = (args.amount * 1e18) as u128;
        // approve calldata
        let spender_padded = format!("{:0>64}", &config::SFRXETH_VAULT[2..]);
        let amount_hex = format!("{:064x}", amt_wei);
        let approve_calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
        // deposit(uint256,address) calldata — receiver = zero address placeholder
        let amt_hex = format!("{:064x}", amt_wei);
        let deposit_calldata = format!(
            "0x{}{}{}",
            config::SEL_DEPOSIT,
            amt_hex,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "approve_calldata": approve_calldata,
                "deposit_calldata": deposit_calldata,
                "amount_frxeth": args.amount,
                "amount_wei": amt_wei.to_string(),
                "contracts": {
                    "frxETH": config::FRXETH_TOKEN,
                    "sfrxETH": config::SFRXETH_VAULT
                }
            })
        );
        return Ok(());
    }

    let wallet = onchainos::resolve_wallet(args.chain)?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Ensure onchainos is logged in.");
    }

    let amt_wei = (args.amount * 1e18) as u128;
    if amt_wei == 0 {
        anyhow::bail!("Amount too small (rounds to 0 wei)");
    }

    // Step 1: approve frxETH to sfrxETH vault
    let approve_result = onchainos::erc20_approve(
        args.chain,
        config::FRXETH_TOKEN,
        config::SFRXETH_VAULT,
        amt_wei,
        Some(&wallet),
        false,
    )
    .await?;

    let approve_hash = onchainos::extract_tx_hash(&approve_result);
    eprintln!("Approve tx: {}", approve_hash);

    // Wait a moment for the approval to be mined
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    // Step 2: deposit frxETH to sfrxETH vault
    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);
    let amt_hex = format!("{:064x}", amt_wei);
    let deposit_calldata = format!("0x{}{}{}", config::SEL_DEPOSIT, amt_hex, wallet_padded);

    let result = onchainos::wallet_contract_call(
        args.chain,
        config::SFRXETH_VAULT,
        &deposit_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "action": "stake frxETH → sfrxETH",
                "approve_txHash": approve_hash,
                "amount_frxeth": args.amount,
                "amount_wei": amt_wei.to_string(),
                "from": wallet,
                "contract": config::SFRXETH_VAULT,
                "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
            }
        })
    );
    Ok(())
}
