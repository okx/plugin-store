// src/config.rs — Moonwell contract addresses and asset metadata

#[derive(Debug, Clone)]
pub struct Market {
    pub symbol: &'static str,
    pub mtoken: &'static str,
    pub underlying: &'static str,
    pub underlying_decimals: u8,
    pub mtoken_decimals: u8,
}

// ── Base (8453) ──────────────────────────────────────────────────────────────

pub const COMPTROLLER_BASE: &str = "0xfBb21d0380beE3312B33c4353c8936a0F13EF26C";
#[allow(dead_code)]
pub const WELL_TOKEN_BASE: &str = "0xA88594D404727625A9437C3f886C7643872296AE";
pub const RPC_BASE: &str = "https://base.publicnode.com";

pub const MARKETS_BASE: &[Market] = &[
    Market {
        symbol: "USDC",
        mtoken: "0xEdc817A28E8B93B03976FBd4a3dDBc9f7D176c22",
        underlying: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        underlying_decimals: 6,
        mtoken_decimals: 8,
    },
    Market {
        symbol: "WETH",
        mtoken: "0x628ff693426583D9a7FB391E54366292F509D457",
        underlying: "0x4200000000000000000000000000000000000006",
        underlying_decimals: 18,
        mtoken_decimals: 8,
    },
    Market {
        symbol: "cbETH",
        mtoken: "0x3bf93770f2d4a794c3d9EBEfBAeBAE2a8f09A5E5",
        underlying: "0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22",
        underlying_decimals: 18,
        mtoken_decimals: 8,
    },
    Market {
        symbol: "USDbC",
        mtoken: "0x703843C3379b52F9FF486c9f5892218d2a065cC8",
        underlying: "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA",
        underlying_decimals: 6,
        mtoken_decimals: 8,
    },
    Market {
        symbol: "DAI",
        mtoken: "0x73b06D8d18De422E269645eaCe15400DE7462417",
        underlying: "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb",
        underlying_decimals: 18,
        mtoken_decimals: 8,
    },
];

// ── Optimism (10) ────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub const COMPTROLLER_OPTIMISM: &str = "0x6e3Aa75dce2E3Bb2a7a5F0bfd2Dd6e08e1B2C3D4"; // placeholder
#[allow(dead_code)]
pub const RPC_OPTIMISM: &str = "https://optimism.publicnode.com";

// ── Moonbeam (1284) ──────────────────────────────────────────────────────────

#[allow(dead_code)]
pub const RPC_MOONBEAM: &str = "https://moonbeam.publicnode.com";

// ── Seconds per year for APR calculation ─────────────────────────────────────

pub const SECONDS_PER_YEAR: u128 = 31_536_000;

// ── Chain config helper ───────────────────────────────────────────────────────

pub struct ChainConfig {
    pub comptroller: &'static str,
    pub markets: &'static [Market],
    pub rpc: &'static str,
}

pub fn chain_config(chain_id: u64) -> anyhow::Result<ChainConfig> {
    match chain_id {
        8453 => Ok(ChainConfig {
            comptroller: COMPTROLLER_BASE,
            markets: MARKETS_BASE,
            rpc: RPC_BASE,
        }),
        _ => anyhow::bail!(
            "Chain {} is not supported. Supported chains: Base (8453)",
            chain_id
        ),
    }
}

pub fn find_market(chain_id: u64, symbol: &str) -> anyhow::Result<&'static Market> {
    let cfg = chain_config(chain_id)?;
    let sym = symbol.to_uppercase();
    cfg.markets
        .iter()
        .find(|m| m.symbol.to_uppercase() == sym)
        .ok_or_else(|| anyhow::anyhow!("Unknown asset '{}' on chain {}", symbol, chain_id))
}

/// Scale a human-readable amount (e.g. 0.01) to raw integer units
pub fn to_raw(amount: f64, decimals: u8) -> u128 {
    let factor = 10f64.powi(decimals as i32);
    (amount * factor).round() as u128
}
