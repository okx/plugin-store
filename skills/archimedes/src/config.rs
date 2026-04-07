/// Archimedes Finance V2 — Ethereum mainnet configuration

pub const CHAIN_ID: u64 = 1;

/// RPC endpoints for Ethereum mainnet (in priority order)
pub const RPC_URLS: &[&str] = &[
    "https://rpc.mevblocker.io",
    "https://mainnet.gateway.tenderly.co",
    "https://ethereum-rpc.publicnode.com",
];

/// Known Archimedes V2 MultiPoolStrategy vaults (hardcoded — factory is inactive)
pub struct VaultInfo {
    pub name: &'static str,
    pub address: &'static str,
    pub underlying_symbol: &'static str,
    pub underlying_address: &'static str,
    pub underlying_decimals: u32,
}

pub const VAULTS: &[VaultInfo] = &[
    VaultInfo {
        name: "WETH ETH+ Strategy (Convex)",
        address: "0xfA364CBca915f17fEc356E35B61541fC6D4D8269",
        underlying_symbol: "WETH",
        underlying_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        underlying_decimals: 18,
    },
    VaultInfo {
        name: "WETH Aura Weighted Strategy",
        address: "0x83FeD5139eD14162198Bd0a54637c22cA854E2f6",
        underlying_symbol: "WETH",
        underlying_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        underlying_decimals: 18,
    },
    VaultInfo {
        name: "alUSD FRAXBP Strategy (Convex)",
        address: "0x2E04e0aEa173F95A23043576138539fBa60D930a",
        underlying_symbol: "crvFRAX",
        underlying_address: "0x3175Df0976dFA876431C2E9eE6Bc45b65d3473CC",
        underlying_decimals: 18,
    },
];

/// Default slippage in basis points (50 = 0.5%)
pub const DEFAULT_SLIPPAGE_BPS: u64 = 50;

/// Delay between approve and deposit/withdraw (seconds)
pub const APPROVE_DELAY_SECS: u64 = 3;

/// Look up vault info by address (case-insensitive)
pub fn find_vault(address: &str) -> Option<&'static VaultInfo> {
    let lower = address.to_lowercase();
    VAULTS.iter().find(|v| v.address.to_lowercase() == lower)
}

/// Format raw amount (u128 in smallest unit) to human-readable string
pub fn format_amount(amount: u128, decimals: u32) -> String {
    let divisor = 10u128.pow(decimals);
    let integer = amount / divisor;
    let frac = amount % divisor;
    if frac == 0 {
        format!("{}", integer)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", integer, trimmed)
    }
}

/// Parse human-readable amount to raw u128 in smallest unit
pub fn parse_amount(amount_str: &str, decimals: u32) -> anyhow::Result<u128> {
    let parts: Vec<&str> = amount_str.splitn(2, '.').collect();
    let integer_part = parts[0]
        .parse::<u128>()
        .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount_str))?;
    let frac_str = if parts.len() > 1 { parts[1] } else { "" };
    let frac_len = frac_str.len() as u32;
    if frac_len > decimals {
        anyhow::bail!(
            "Too many decimal places: {} (max {})",
            frac_len,
            decimals
        );
    }
    let frac_part = if frac_str.is_empty() {
        0u128
    } else {
        frac_str
            .parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid fractional part: {}", frac_str))?
    };
    let multiplier = 10u128.pow(decimals);
    let frac_multiplier = 10u128.pow(decimals - frac_len);
    Ok(integer_part * multiplier + frac_part * frac_multiplier)
}
