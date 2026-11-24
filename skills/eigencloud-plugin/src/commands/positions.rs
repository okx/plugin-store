use clap::Args;
use crate::abi::{selector, calldata, encode_address};
use crate::chain::{CHAIN_ID, eth_call, decode_word, decode_address, decode_uint};
use crate::onchainos::resolve_wallet;
use crate::strategies::by_strategy;

// EigenLayer mainnet contracts
const STRATEGY_MANAGER: &str = "0x858646372CC42E1A627fcE94aa7A7033e7CF075A";
const DELEGATION_MANAGER: &str = "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A";

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to active onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let wallet = match args.wallet {
        Some(w) => w,
        None => resolve_wallet(CHAIN_ID)
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string()),
    };

    // 1. getDeposits(address) on StrategyManager
    let get_deposits_sel = selector("getDeposits(address)");
    let deposits_data = calldata(get_deposits_sel, &[encode_address(&wallet)]);
    let deposits_result = eth_call(STRATEGY_MANAGER, &deposits_data).await
        .unwrap_or_default();

    let positions = parse_deposits(&deposits_result);

    // 2. delegatedTo(address) on DelegationManager
    let delegated_sel = selector("delegatedTo(address)");
    let delegated_data = calldata(delegated_sel, &[encode_address(&wallet)]);
    let delegated_result = eth_call(DELEGATION_MANAGER, &delegated_data).await
        .unwrap_or_default();

    let operator = decode_word(&delegated_result, 0)
        .map(|w| decode_address(&w))
        .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string());

    let is_delegated = operator != "0x0000000000000000000000000000000000000000";

    let out = serde_json::json!({
        "wallet":       wallet,
        "positions":    positions,
        "delegated":    is_delegated,
        "operator":     if is_delegated { operator } else { "none".to_string() },
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Parse the ABI-encoded response from getDeposits(address).
/// Returns (IStrategy[], uint256[]) — two dynamic arrays.
fn parse_deposits(result: &str) -> Vec<serde_json::Value> {
    let result = result.trim_start_matches("0x");
    if result.len() < 128 {
        return vec![];
    }
    // Word 0: offset to strategies array
    // Word 1: offset to shares array
    let strat_offset = usize::from_str_radix(&result[0..64], 16).unwrap_or(0) * 2;
    let shares_offset = usize::from_str_radix(&result[64..128], 16).unwrap_or(0) * 2;

    if result.len() < strat_offset + 64 || result.len() < shares_offset + 64 {
        return vec![];
    }

    let strat_count = usize::from_str_radix(&result[strat_offset..strat_offset + 64], 16).unwrap_or(0);
    let shares_count = usize::from_str_radix(&result[shares_offset..shares_offset + 64], 16).unwrap_or(0);

    let count = strat_count.min(shares_count);
    let mut out = Vec::new();

    for i in 0..count {
        let strat_word_start = strat_offset + 64 + i * 64;
        let shares_word_start = shares_offset + 64 + i * 64;

        if result.len() < strat_word_start + 64 || result.len() < shares_word_start + 64 {
            break;
        }

        let strat_addr = decode_address(&result[strat_word_start..strat_word_start + 64]);
        let shares_hex = &result[shares_word_start..shares_word_start + 64];
        let shares_raw = decode_uint(shares_hex);

        let (symbol, decimals) = by_strategy(&strat_addr)
            .map(|s| (s.symbol.to_string(), s.decimals))
            .unwrap_or_else(|| ("UNKNOWN".to_string(), 18));

        let shares_fmt = format_shares(shares_raw, decimals);

        out.push(serde_json::json!({
            "symbol":     symbol,
            "strategy":   strat_addr,
            "shares_raw": shares_raw.to_string(),
            "shares":     shares_fmt,
        }));
    }
    out
}

fn format_shares(raw: u128, decimals: u8) -> String {
    let scale = 10u128.pow(decimals as u32);
    let whole = raw / scale;
    let frac = raw % scale;
    if frac == 0 {
        whole.to_string()
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        format!("{}.{}", whole, frac_str.trim_end_matches('0'))
    }
}
