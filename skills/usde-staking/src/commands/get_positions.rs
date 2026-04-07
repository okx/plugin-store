use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct GetPositionsArgs {
    /// Wallet address to query (resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: GetPositionsArgs) -> anyhow::Result<()> {
    // Resolve wallet address
    let wallet = match args.address {
        Some(ref addr) => addr.clone(),
        None => onchainos::resolve_wallet(config::CHAIN_ID)?,
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // sUSDe balance
    let bal_calldata = rpc::calldata_balance_of(&wallet);
    let bal_result = onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &bal_calldata)?;
    let bal_hex = rpc::extract_return_data(&bal_result).unwrap_or_default();
    let susde_balance = rpc::decode_uint256(&bal_hex).unwrap_or(0);
    let susde_float = rpc::wei_to_float(susde_balance);

    // USDe value of sUSDe balance
    let usde_equivalent = if susde_balance > 0 {
        let convert_calldata = rpc::calldata_convert_to_assets(susde_balance);
        let convert_result =
            onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &convert_calldata)?;
        let convert_hex = rpc::extract_return_data(&convert_result).unwrap_or_default();
        rpc::decode_uint256(&convert_hex).unwrap_or(susde_balance)
    } else {
        0
    };
    let usde_equivalent_float = rpc::wei_to_float(usde_equivalent);

    // USDe balance (staked and available)
    let usde_bal_calldata = rpc::calldata_balance_of(&wallet);
    let usde_bal_result = onchainos::eth_call(config::CHAIN_ID, config::USDE_ADDRESS, &usde_bal_calldata)?;
    let usde_bal_hex = rpc::extract_return_data(&usde_bal_result).unwrap_or_default();
    let usde_balance = rpc::decode_uint256(&usde_bal_hex).unwrap_or(0);
    let usde_balance_float = rpc::wei_to_float(usde_balance);

    // Pending cooldown/unstake
    let cd_calldata = rpc::calldata_cooldowns(&wallet);
    let cd_result = onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &cd_calldata)?;
    let cd_hex = rpc::extract_return_data(&cd_result).unwrap_or_default();
    let (cooldown_end, underlying_amount) = rpc::decode_two_uint256(&cd_hex).unwrap_or((0, 0));
    let underlying_float = rpc::wei_to_float(underlying_amount);

    println!("=== sUSDe Staking Position ===");
    println!();
    println!("Wallet: {}", wallet);
    println!();
    println!("Balances:");
    println!("  USDe (unstaked):      {:.6} USDe", usde_balance_float);
    println!("  sUSDe (staked):       {:.6} sUSDe", susde_float);
    println!("  USDe equivalent:      {:.6} USDe", usde_equivalent_float);

    if cooldown_end > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u128;
        println!();
        println!("Pending Unstake:");
        println!("  Amount:               {:.6} USDe", underlying_float);
        if cooldown_end > now {
            let remaining = cooldown_end - now;
            let hours = remaining / 3600;
            let mins = (remaining % 3600) / 60;
            println!("  Cooldown ends:        {} hours {} minutes remaining", hours, mins);
            println!("  Status:               COOLING DOWN (not yet claimable)");
        } else {
            println!("  Cooldown ends:        COMPLETED");
            println!("  Status:               READY TO CLAIM");
            println!("  Run: usde-staking claim-unstake");
        }
    } else {
        println!();
        println!("Pending Unstake:      None");
    }

    println!();
    println!("Chain: Ethereum mainnet (ID: {})", config::CHAIN_ID);

    Ok(())
}
