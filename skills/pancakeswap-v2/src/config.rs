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
        _ => anyhow::bail!("Unsupported chain ID {}. Supported: 56 (BSC), 8453 (Base)", chain_id),
    }
}

/// Resolve common token symbols to addresses. Returns the symbol as-is if already an address.
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    match (symbol.to_uppercase().as_str(), chain_id) {
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
        _ => symbol.to_string(), // assume it's already a hex address
    }
}

/// Returns true if the symbol is native BNB/ETH (not an ERC-20)
pub fn is_native(symbol: &str) -> bool {
    matches!(symbol.to_uppercase().as_str(), "BNB" | "ETH")
}
