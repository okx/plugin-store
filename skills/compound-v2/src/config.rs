// src/config.rs — Compound V2 contract addresses and asset metadata

/// Known Compound V2 market info
#[derive(Debug, Clone)]
pub struct Market {
    pub symbol: &'static str,
    pub ctoken: &'static str,
    pub underlying: Option<&'static str>, // None for cETH (native ETH)
    pub underlying_decimals: u8,
    pub ctoken_decimals: u8,
    pub is_eth: bool,
}

pub const COMPTROLLER: &str = "0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3b";

pub const MARKETS: &[Market] = &[
    Market {
        symbol: "ETH",
        ctoken: "0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5",
        underlying: None,
        underlying_decimals: 18,
        ctoken_decimals: 8,
        is_eth: true,
    },
    Market {
        symbol: "USDT",
        ctoken: "0xf650C3d88D12dB855b8bf7D11Be6C55A4e07dCC9",
        underlying: Some("0xdAC17F958D2ee523a2206206994597C13D831ec7"),
        underlying_decimals: 6,
        ctoken_decimals: 8,
        is_eth: false,
    },
    Market {
        symbol: "USDC",
        ctoken: "0x39AA39c021dfbaE8faC545936693aC917d5E7563",
        underlying: Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        underlying_decimals: 6,
        ctoken_decimals: 8,
        is_eth: false,
    },
    Market {
        symbol: "DAI",
        ctoken: "0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643",
        underlying: Some("0x6B175474E89094C44Da98b954EedeAC495271d0F"),
        underlying_decimals: 18,
        ctoken_decimals: 8,
        is_eth: false,
    },
];

/// Blocks per year (Ethereum ~15s/block)
pub const BLOCKS_PER_YEAR: u128 = 2_102_400;

/// Mainnet public RPC
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

pub fn find_market(symbol: &str) -> Option<&'static Market> {
    let sym = symbol.to_uppercase();
    MARKETS.iter().find(|m| m.symbol == sym.as_str())
}

/// Scale a human-readable amount (e.g. 0.01) to raw integer units
pub fn to_raw(amount: f64, decimals: u8) -> u128 {
    let factor = 10f64.powi(decimals as i32);
    (amount * factor).round() as u128
}
