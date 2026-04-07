use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct GetPositionArgs {
    /// Wallet address to query (resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub fn run(args: GetPositionArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    // Resolve address
    let addr = match args.address {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id)?,
    };
    if addr.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // 1. STONE balance
    let calldata_balance = format!(
        "0x{}{}",
        config::SEL_BALANCE_OF,
        rpc::encode_address(&addr)
    );
    let balance_hex = rpc::eth_call(config::STONE_TOKEN, &calldata_balance)?;
    let stone_balance = rpc::decode_uint256(&balance_hex)?;
    let stone_f = stone_balance as f64 / 1e18;

    // 2. currentSharePrice for ETH value
    let price_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_CURRENT_SHARE_PRICE),
    )?;
    let price_raw = rpc::decode_uint256(&price_hex)?;
    let price_eth = price_raw as f64 / 1e18;
    let eth_value = stone_f * price_eth;

    // 3. userReceipts for pending withdrawal
    let calldata_receipts = format!(
        "0x{}{}",
        config::SEL_USER_RECEIPTS,
        rpc::encode_address(&addr)
    );
    let receipts_hex = rpc::eth_call(config::STONE_VAULT, &calldata_receipts)?;
    let (withdraw_round, withdraw_shares, withdrawable_amount) =
        rpc::decode_tuple3_u128(&receipts_hex)?;

    // 4. Latest round for context
    let round_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_LATEST_ROUND_ID),
    )?;
    let current_round = rpc::decode_uint256(&round_hex)?;

    println!("=== StakeStone Position ===");
    println!("Address:          {}", addr);
    println!();
    println!("STONE balance:    {:.6} STONE", stone_f);
    println!("ETH value:        {:.6} ETH (at {:.6} ETH/STONE)", eth_value, price_eth);
    println!();

    if withdraw_shares > 0 || withdrawable_amount > 0 {
        let shares_f = withdraw_shares as f64 / 1e18;
        let withdrawable_f = withdrawable_amount as f64 / 1e18;
        println!("Pending Withdrawal:");
        println!("  Queued shares:  {:.6} STONE", shares_f);
        println!("  Withdrawable:   {:.6} ETH", withdrawable_f);
        println!("  Queued round:   {} (current: {})", withdraw_round, current_round);
        if withdraw_round > 0 && withdraw_round > current_round {
            println!("  Status:         Pending settlement");
        } else if withdrawable_amount > 0 {
            println!("  Status:         Ready to claim (contact StakeStone UI to claim)");
        }
    } else {
        println!("Pending Withdrawal: None");
    }

    Ok(())
}
