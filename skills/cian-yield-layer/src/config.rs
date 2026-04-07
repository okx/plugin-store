/// CIAN Yield Layer — contract addresses and configuration for Ethereum Mainnet (chain 1)

pub const CHAIN_ID: u64 = 1;

/// Primary Ethereum RPC — mevblocker (no MEV, reliable)
pub const ETH_FALLBACK_RPCS: &[&str] = &[
    "https://ethereum.publicnode.com",
    "https://rpc.mevblocker.io",
    "https://eth.llamarpc.com",
];

/// stETH Yield Layer Vault Proxy (ylstETH)
pub const VAULT_YLSTETH: &str = "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d";

/// pumpBTC Yield Layer Vault Proxy (ylpumpBTC)
pub const VAULT_YLPUMPBTC: &str = "0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b";

/// Token addresses on Ethereum Mainnet
pub const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
pub const STETH: &str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";
pub const WSTETH: &str = "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0";
pub const EETH: &str = "0x35fA164735182de50811E8e2E824cFb9B6118ac2";
pub const WEETH: &str = "0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee";
pub const PUMPBTC: &str = "0xF469fBD2abcd6B9de8E169d128226C0Fc90a6Ff9";
pub const WBTC: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";

/// CIAN REST API base URL
pub const API_BASE_URL: &str = "https://yieldlayer.cian.app/ethereum";

/// Zero address (used for ETH deposits and no-referral)
pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// Async withdrawal wait period (informational)
pub const WITHDRAWAL_WAIT_DAYS: u64 = 5;

/// Supported tokens per vault (for user guidance)
pub const YLSTETH_TOKENS: &[(&str, &str, u32)] = &[
    ("ETH",    ZERO_ADDRESS, 18),
    ("WETH",   WETH,   18),
    ("stETH",  STETH,  18),
    ("wstETH", WSTETH, 18),
    ("eETH",   EETH,   18),
    ("weETH",  WEETH,  18),
];

pub const YLPUMPBTC_TOKENS: &[(&str, &str, u32)] = &[
    ("pumpBTC", PUMPBTC, 8),
    ("WBTC",    WBTC,    8),
];

/// Format a raw uint256 with given decimals into a human-readable string (6 decimal places).
pub fn format_units(raw: u128, decimals: u32) -> String {
    let divisor = 10u128.pow(decimals);
    let frac_divisor = if decimals >= 6 {
        10u128.pow(decimals - 6)
    } else {
        1u128
    };
    let whole = raw / divisor;
    let frac = (raw % divisor) / frac_divisor;
    let frac_digits = if decimals >= 6 { 6 } else { decimals as usize };
    format!("{}.{:0>width$}", whole, frac, width = frac_digits)
}

/// Shorthand for 18-decimal formatting
pub fn format_18(raw: u128) -> String {
    format_units(raw, 18)
}

/// Parse a human-readable decimal string to u128 with given decimals.
pub fn parse_units(s: &str, decimals: u32) -> anyhow::Result<u128> {
    let s = s.trim();
    let multiplier = 10u128.pow(decimals);
    if let Some((whole, frac)) = s.split_once('.') {
        let whole_val: u128 = whole.parse().map_err(|_| anyhow::anyhow!("invalid number: {}", s))?;
        let frac_padded = format!("{:0<width$}", frac, width = decimals as usize);
        let frac_str = if frac_padded.len() >= decimals as usize {
            &frac_padded[..decimals as usize]
        } else {
            &frac_padded
        };
        let frac_val: u128 = frac_str.parse().map_err(|_| anyhow::anyhow!("invalid fraction: {}", s))?;
        Ok(whole_val * multiplier + frac_val)
    } else {
        let whole_val: u128 = s.parse().map_err(|_| anyhow::anyhow!("invalid number: {}", s))?;
        Ok(whole_val * multiplier)
    }
}

/// Encode an address parameter padded to 32 bytes (without 0x prefix).
pub fn encode_address(addr: &str) -> String {
    let stripped = addr.strip_prefix("0x").unwrap_or(addr);
    format!("{:0>64}", stripped.to_lowercase())
}

/// Encode a uint256 as 32-byte hex (without 0x prefix).
pub fn encode_uint256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Vault info struct for display
pub struct VaultInfo {
    pub name: &'static str,
    pub address: &'static str,
    pub symbol: &'static str,
    pub asset_decimals: u32,
}

pub const VAULTS: &[VaultInfo] = &[
    VaultInfo {
        name: "stETH Yield Layer",
        address: VAULT_YLSTETH,
        symbol: "ylstETH",
        asset_decimals: 18,
    },
    VaultInfo {
        name: "pumpBTC Yield Layer",
        address: VAULT_YLPUMPBTC,
        symbol: "ylpumpBTC",
        asset_decimals: 8,
    },
];

/// Resolve vault address from user input (name or address)
pub fn resolve_vault(input: &str) -> anyhow::Result<&'static VaultInfo> {
    let lower = input.to_lowercase();
    for v in VAULTS {
        if v.address.to_lowercase() == lower
            || v.symbol.to_lowercase() == lower
            || v.name.to_lowercase().contains(&lower)
        {
            return Ok(v);
        }
    }
    anyhow::bail!(
        "Unknown vault '{}'. Use: ylsteth, ylpumpbtc, or full address.\nAvailable: ylstETH ({}), ylpumpBTC ({})",
        input, VAULT_YLSTETH, VAULT_YLPUMPBTC
    )
}

/// Resolve token address and decimals from symbol or address
pub fn resolve_token(input: &str, vault_addr: &str) -> anyhow::Result<(&'static str, u32)> {
    let lower = input.to_lowercase();
    let tokens: &[(&str, &str, u32)] = if vault_addr.to_lowercase() == VAULT_YLSTETH.to_lowercase() {
        YLSTETH_TOKENS
    } else {
        YLPUMPBTC_TOKENS
    };
    for (sym, addr, dec) in tokens {
        if sym.to_lowercase() == lower || addr.to_lowercase() == lower {
            return Ok((addr, *dec));
        }
    }
    // fallback: raw address input
    if input.starts_with("0x") && input.len() == 42 {
        // unknown token, assume 18 decimals
        return Ok((Box::leak(input.to_string().into_boxed_str()), 18));
    }
    anyhow::bail!(
        "Unknown token '{}' for this vault. ylstETH accepts: ETH, WETH, stETH, wstETH, eETH, weETH. ylpumpBTC accepts: pumpBTC, WBTC.",
        input
    )
}
