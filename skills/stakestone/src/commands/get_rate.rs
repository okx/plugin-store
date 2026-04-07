use crate::{config, rpc};

pub fn run() -> anyhow::Result<()> {
    // 1. currentSharePrice()
    let price_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_CURRENT_SHARE_PRICE),
    )?;
    let price_raw = rpc::decode_uint256(&price_hex)?;
    let price_eth = price_raw as f64 / 1e18;

    // 2. latestRoundID()
    let round_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_LATEST_ROUND_ID),
    )?;
    let round = rpc::decode_uint256(&round_hex)?;

    // 3. withdrawFeeRate()
    let fee_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_WITHDRAW_FEE_RATE),
    )?;
    let fee_raw = rpc::decode_uint256(&fee_hex)?;
    let fee_pct = fee_raw as f64 / 1e6 * 100.0;

    // 4. getVaultAvailableAmount()
    let avail_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_GET_VAULT_AVAILABLE),
    )?;
    let (idle_raw, invested_raw) = rpc::decode_tuple2_u128(&avail_hex)?;
    let idle_eth = idle_raw as f64 / 1e18;
    let invested_eth = invested_raw as f64 / 1e18;
    let total_eth = idle_eth + invested_eth;

    println!("=== StakeStone STONE Rate ===");
    println!("STONE price:      {:.6} ETH per STONE", price_eth);
    println!("Settlement round: {}", round);
    println!("Withdrawal fee:   {:.4}%", fee_pct);
    println!();
    println!("Vault TVL:        {:.4} ETH total", total_eth);
    println!("  Idle:           {:.4} ETH", idle_eth);
    println!("  Deployed:       {:.4} ETH", invested_eth);
    println!();
    println!(
        "1 ETH stakes to approximately {:.6} STONE",
        1.0 / price_eth
    );

    Ok(())
}
