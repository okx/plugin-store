use clap::Args;
use crate::config::{factory_address, pad_address, resolve_token_address, rpc_url};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{factory_get_pool, gauge_earned, voter_get_gauge};

const CHAIN_ID: u64 = 10;

#[derive(Args)]
pub struct ClaimRewardsArgs {
    /// Token A of the pool (symbol or hex address)
    #[arg(long)]
    pub token_a: Option<String>,
    /// Token B of the pool (symbol or hex address)
    #[arg(long)]
    pub token_b: Option<String>,
    /// Pool type: volatile (false) or stable (true)
    #[arg(long, default_value_t = false)]
    pub stable: bool,
    /// Direct gauge address (alternative to token_a/token_b lookup)
    #[arg(long)]
    pub gauge: Option<String>,
    /// Dry run -- build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: ClaimRewardsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let voter = crate::config::voter_address();
    let factory = factory_address();

    // --- 1. Resolve gauge address ---
    let gauge_addr = if let Some(g) = args.gauge {
        g
    } else if args.token_a.is_some() && args.token_b.is_some() {
        let token_a = resolve_token_address(&args.token_a.unwrap());
        let token_b = resolve_token_address(&args.token_b.unwrap());
        let pool_addr = factory_get_pool(&token_a, &token_b, args.stable, factory, rpc).await?;
        if pool_addr == "0x0000000000000000000000000000000000000000" {
            anyhow::bail!("Pool not found for {}/{} stable={}", token_a, token_b, args.stable);
        }
        println!("Pool: {}", pool_addr);
        let gauge = voter_get_gauge(voter, &pool_addr, rpc).await?;
        if gauge == "0x0000000000000000000000000000000000000000" {
            anyhow::bail!("No gauge found for pool {}. The pool may not have gauge rewards.", pool_addr);
        }
        gauge
    } else {
        anyhow::bail!("Provide --token-a and --token-b, or --gauge <address>");
    };

    println!("Gauge: {}", gauge_addr);

    // --- 2. Resolve wallet ---
    let wallet = if args.dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        resolve_wallet(CHAIN_ID)?
    };

    // --- 3. Check earned rewards ---
    let earned = if args.dry_run {
        0u128
    } else {
        gauge_earned(&gauge_addr, &wallet, rpc).await?
    };

    println!("VELO earned: {}", earned);

    if !args.dry_run && earned == 0 {
        println!("{{\"ok\":true,\"message\":\"No VELO rewards to claim\",\"gauge\":\"{}\",\"earned\":0}}", gauge_addr);
        return Ok(());
    }

    println!("Please confirm claiming {} VELO from gauge {}. (Proceeding automatically in non-interactive mode)", earned, gauge_addr);

    // --- 4. Build getReward(address account) calldata ---
    // Selector: 0xc00007b0
    let calldata = format!("0xc00007b0{}", pad_address(&wallet));

    let result = wallet_contract_call(CHAIN_ID, &gauge_addr, &calldata, true, args.dry_run).await?;

    let tx_hash = extract_tx_hash(&result);
    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"gauge\":\"{}\",\"wallet\":\"{}\",\"earnedVelo\":\"{}\"}}",
        tx_hash, gauge_addr, wallet, earned
    );

    Ok(())
}
