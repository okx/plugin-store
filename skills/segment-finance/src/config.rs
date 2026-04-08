// Segment Finance — Configuration (BSC chain 56)
// Compound V2 fork on BNB Chain with Diamond proxy Comptroller

pub const BSC_CHAIN_ID: u64 = 56;
pub const BSC_RPC_URL: &str = "https://bsc-rpc.publicnode.com";

// ~3s block time on BSC; ~10.5M blocks per year
pub const BLOCKS_PER_YEAR: u64 = 10_512_000;

// Segment Finance Unitroller (Diamond proxy Comptroller) — BSC mainnet
pub const COMPTROLLER: &str = "0x57E09c96DAEE58B77dc771B017de015C38060173";

// Segment Finance Oracle
pub const ORACLE: &str = "0x763217cFeFac3B26191b1DCaE1926F65157B9A05";

// Known seToken addresses (BSC mainnet)
pub const SEBNB: &str = "0x5fceA94B96858048433359BB5278a402363328C3";
pub const SEUSDT: &str = "0x44B1E0f4533FD155B9859a9DB292C90E5B300119";
pub const SEUSDC: &str = "0x8969b89D5f38359fBE95Bbe392f5ad82dd93e226";
pub const SEBTC: &str = "0x12CD46B96fe0D86E396248a623B81fD84dD0F61d";
pub const SEETH: &str = "0x3821175E59CD0acDa6c5Fd3eBB618b204e5D7eed";

// Underlying ERC-20 token addresses (BSC mainnet)
// BSC USDT is BEP-20 with 18 decimals (NOT 6 decimals like Ethereum USDT)
pub const USDT_BSC: &str = "0x55d398326f99059ff775485246999027b3197955";
pub const USDC_BSC: &str = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d";
pub const BTCB_BSC: &str = "0x7130d2a12b9bcbfae4f2634d864a1ee1ce3ead9c";
pub const ETH_BSC: &str = "0x2170ed0880ac9a755fd29b2688956bd959f933f8";

/// Resolve seToken address from asset symbol.
/// Returns (setoken_addr, underlying_addr, decimals, is_native_bnb)
pub fn resolve_asset(symbol: &str) -> anyhow::Result<(&'static str, &'static str, u32, bool)> {
    match symbol.to_uppercase().as_str() {
        "BNB" => Ok((SEBNB, "native", 18, true)),
        "USDT" => Ok((SEUSDT, USDT_BSC, 18, false)),
        "USDC" => Ok((SEUSDC, USDC_BSC, 18, false)),
        "BTC" | "BTCB" => Ok((SEBTC, BTCB_BSC, 18, false)),
        "ETH" => Ok((SEETH, ETH_BSC, 18, false)),
        _ => anyhow::bail!(
            "Unsupported asset: {}. Supported: BNB, USDT, USDC, BTC, ETH",
            symbol
        ),
    }
}

pub fn get_rpc(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        56 => Ok(BSC_RPC_URL),
        _ => anyhow::bail!(
            "Unsupported chain ID: {}. Segment Finance is only on BSC (56).",
            chain_id
        ),
    }
}
