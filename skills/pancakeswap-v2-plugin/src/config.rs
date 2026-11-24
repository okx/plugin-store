// src/config.rs — Chain config: RPC URLs, contract addresses, token maps

pub struct ChainConfig {
    pub router02: &'static str,
    pub factory: &'static str,
    pub weth: &'static str,     // WBNB on BSC, WETH on Base
    pub rpc_url: &'static str,
    pub explorer: &'static str,
}

pub fn chain_config(chain_id: u64) -> anyhow::Result<ChainConfig> {
    match chain_id {
        56 => Ok(ChainConfig {
            router02: "0x10ED43C718714eb63d5aA57B78B54704E256024E",
            factory: "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73",
            weth: "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
            rpc_url: "https://bsc-rpc.publicnode.com",
            explorer: "https://bscscan.com",
        }),
        8453 => Ok(ChainConfig {
            router02: "0x8cFe327CEc66d1C090Dd72bd0FF11d690C33a2Eb",
            factory: "0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E",
            weth: "0x4200000000000000000000000000000000000006",
            rpc_url: "https://base-rpc.publicnode.com",
            explorer: "https://basescan.org",
        }),
        42161 => Ok(ChainConfig {
            router02: "0x8cFe327CEc66d1C090Dd72bd0FF11d690C33a2Eb",
            factory: "0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E",
            weth: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            rpc_url: "https://arbitrum-one-rpc.publicnode.com",
            explorer: "https://arbiscan.io",
        }),
        _ => anyhow::bail!("Unsupported chain ID {}. Supported: 56 (BSC), 8453 (Base), 42161 (Arbitrum)", chain_id),
    }
}

/// Resolve common token symbols to addresses.
/// Returns Ok(address) for known symbols or valid hex addresses.
/// Returns Err if the symbol is not recognised on this chain and is not a hex address.
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> anyhow::Result<String> {
    let resolved = match (symbol.to_uppercase().as_str(), chain_id) {
        // BSC (56)
        ("WBNB", 56) | ("BNB", 56) => "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".to_string(),
        ("CAKE", 56) => "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82".to_string(),
        ("USDT", 56) => "0x55d398326f99059fF775485246999027B3197955".to_string(),
        ("USDC", 56) => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".to_string(),
        ("BUSD", 56) => "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56".to_string(),
        ("ETH", 56) => "0x2170Ed0880ac9A755fd29B2688956BD959F933F8".to_string(),
        ("BTCB", 56) => "0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c".to_string(),
        // Base (8453)
        ("WETH", 8453) | ("ETH", 8453) => "0x4200000000000000000000000000000000000006".to_string(),
        ("USDC", 8453) => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
        // Arbitrum One (42161)
        ("WETH", 42161) | ("ETH", 42161) => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".to_string(),
        ("USDC", 42161) => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string(),
        ("USDT", 42161) => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string(),
        _ => symbol.to_string(), // pass through — may be a raw hex address
    };
    // Validate the resolved value is a hex address
    if resolved.starts_with("0x") && resolved.len() >= 40 {
        Ok(resolved)
    } else {
        anyhow::bail!(
            "Token '{}' is not supported on chain {}. \
            Use a hex address (0x...) or a supported symbol \
            (BSC: USDT, USDC, CAKE, BNB/WBNB, BUSD, ETH, BTCB; \
             Base: ETH/WETH, USDC; Arbitrum: ETH/WETH, USDC, USDT).",
            symbol, chain_id
        )
    }
}

/// Returns true if the symbol is native BNB/ETH (not an ERC-20)
pub fn is_native(symbol: &str) -> bool {
    matches!(symbol.to_uppercase().as_str(), "BNB" | "ETH")
}
