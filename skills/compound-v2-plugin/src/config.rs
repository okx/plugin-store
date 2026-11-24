/// Static config for compound-v2-plugin: chain whitelist + Comptroller + cToken markets.
///
/// v0.1.0 scope: Ethereum mainnet only. Compound V2 was deployed only on Ethereum;
/// "V2 on BSC/Polygon" instances are non-official forks (e.g. Venus, CREAM) which
/// have their own contracts and are out of scope.
///
/// All contract addresses verified directly on-chain via:
///   - Comptroller.getAllMarkets() enumeration
///   - cToken.symbol() lookup
///   - cToken.underlying() resolution to ERC-20 address (cETH excepted — payable native)

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub id: u64,
    pub key: &'static str,
    pub name: &'static str,
    pub rpc: &'static str,
    pub native_symbol: &'static str,
    /// Comptroller (Unitroller proxy). User-facing for enterMarkets / exitMarket / claimComp / pause flags.
    pub comptroller: &'static str,
    /// COMP governance token.
    pub comp_token: &'static str,
    /// Approximate Ethereum mainnet blocks per year (12s avg block time).
    /// Compound V2 interest is per-block, so APR = ratePerBlock × blocks_per_year.
    pub blocks_per_year: u128,
}

pub const SUPPORTED_CHAINS: &[ChainInfo] = &[
    ChainInfo {
        id: 1,
        key: "ETH",
        name: "Ethereum",
        rpc: "https://ethereum-rpc.publicnode.com",
        native_symbol: "ETH",
        comptroller:    "0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3B",
        comp_token:     "0xc00e94Cb662C3520282E6f5717214004A7f26888",
        blocks_per_year: 2_102_400, // 365.25 × 86400 / 12s
    },
];

/// Well-known Compound V2 cToken markets on Ethereum.
/// Tuple: (cToken_address, symbol, underlying_address (or "" for cETH), underlying_decimals, ctoken_decimals)
///
/// All entries verified on-chain:
///   - Comptroller.getAllMarkets() returns these as part of the 20-market list
///   - cToken.symbol() matches the listed symbol
///   - cToken.underlying() returns the underlying_address (cETH has no underlying() — payable)
///
/// **2026 status note**: All 6 markets have `mintGuardianPaused = true` (governance-imposed
/// wind-down). borrow / redeem / repay / claim still work for users with legacy positions.
pub const ETH_KNOWN_MARKETS: &[CTokenInfo] = &[
    CTokenInfo {
        ctoken: "0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643",
        symbol: "cDAI",
        underlying: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
        underlying_symbol: "DAI",
        underlying_decimals: 18,
        is_native: false,
    },
    CTokenInfo {
        ctoken: "0x39AA39c021dfbaE8faC545936693aC917d5E7563",
        symbol: "cUSDC",
        underlying: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        underlying_symbol: "USDC",
        underlying_decimals: 6,
        is_native: false,
    },
    CTokenInfo {
        ctoken: "0xf650C3d88D12dB855b8bf7D11Be6C55A4e07dCC9",
        symbol: "cUSDT",
        underlying: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        underlying_symbol: "USDT",
        underlying_decimals: 6,
        is_native: false,
    },
    CTokenInfo {
        ctoken: "0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5",
        symbol: "cETH",
        underlying: "",                                  // native ETH; payable mint(); no underlying()
        underlying_symbol: "ETH",
        underlying_decimals: 18,
        is_native: true,
    },
    CTokenInfo {
        ctoken: "0xccF4429DB6322D5C611ee964527D42E5d685DD6a",
        symbol: "cWBTC2",
        underlying: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
        underlying_symbol: "WBTC",
        underlying_decimals: 8,
        is_native: false,
    },
    CTokenInfo {
        ctoken: "0x70e36f6BF80a52b3B46b3aF8e106CC0ed743E8e4",
        symbol: "cCOMP",
        underlying: "0xc00e94Cb662C3520282E6f5717214004A7f26888",
        underlying_symbol: "COMP",
        underlying_decimals: 18,
        is_native: false,
    },
];

#[derive(Debug, Clone, Copy)]
pub struct CTokenInfo {
    pub ctoken: &'static str,
    pub symbol: &'static str,             // e.g. "cDAI"
    pub underlying: &'static str,         // ERC-20 address; "" for cETH
    pub underlying_symbol: &'static str,  // e.g. "DAI"
    pub underlying_decimals: u32,         // underlying token decimals
    pub is_native: bool,                  // true only for cETH
}

/// All cToken contracts use 8 decimals (cToken supply uses fixed 8 dec; balanceOfUnderlying
/// scales it to underlying decimals via exchangeRate).
pub const CTOKEN_DECIMALS: u32 = 8;

/// Resolve a symbol-or-address to a CTokenInfo.
/// Accepted inputs: "DAI" / "cDAI" (case-insensitive), or 0x cToken address, or 0x underlying address.
pub fn resolve_market(token: &str) -> Option<&'static CTokenInfo> {
    let trimmed = token.trim();
    let upper = trimmed.to_uppercase();
    for info in ETH_KNOWN_MARKETS {
        if upper == info.symbol.to_uppercase()
            || upper == info.underlying_symbol.to_uppercase()
            || trimmed.eq_ignore_ascii_case(info.ctoken)
            || (!info.underlying.is_empty() && trimmed.eq_ignore_ascii_case(info.underlying))
        {
            return Some(info);
        }
    }
    None
}
