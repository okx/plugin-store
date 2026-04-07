// Venus Core Pool — Configuration (BSC chain 56)

pub const BSC_CHAIN_ID: u64 = 56;
pub const BSC_RPC_URL: &str = "https://bsc-rpc.publicnode.com";

// ~3s block time on BSC; used for APY calculation
pub const BLOCKS_PER_YEAR: u64 = 10_512_000;

// Venus Core Pool Comptroller (BSC mainnet)
pub const COMPTROLLER: &str = "0xfD36E2c2a6789Db23113685031d7F16329158384";

// Known vToken addresses (BSC mainnet)
pub const VBNB: &str = "0xa07c5b74c9b40447a954e1466938b865b6bbea36";
pub const VUSDT: &str = "0xfd5840cd36d94d7229439859c0112a4185bc0255";
pub const VBTC: &str = "0x882c173bc7ff3b7786ca16dfed3dfffb9ee7847b";
pub const VETH: &str = "0xf508fcd89b8bd15579dc79a6827cb4686a3592c8";
pub const VUSDC: &str = "0xeca88125a5adbe82614ffc12d0db554e2e2867c8";
pub const VXVS: &str = "0x151b1e2635a717bcdc836ecd6fbb62b674fe3e1d";

// Underlying ERC-20 token addresses
pub const USDT_BSC: &str = "0x55d398326f99059ff775485246999027b3197955";
pub const BTCB_BSC: &str = "0x7130d2a12b9bcbfae4f2634d864a1ee1ce3ead9c";
pub const ETH_BSC: &str = "0x2170ed0880ac9a755fd29b2688956bd959f933f8";
pub const USDC_BSC: &str = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d";

/// Resolve vToken address from asset symbol.
/// Returns (vtoken_addr, underlying_addr, decimals, is_native_bnb)
pub fn resolve_asset(symbol: &str) -> anyhow::Result<(&'static str, &'static str, u32, bool)> {
    match symbol.to_uppercase().as_str() {
        "BNB" => Ok((VBNB, "native", 18, true)),
        "USDT" => Ok((VUSDT, USDT_BSC, 18, false)),
        "BTC" | "BTCB" => Ok((VBTC, BTCB_BSC, 18, false)),
        "ETH" => Ok((VETH, ETH_BSC, 18, false)),
        "USDC" => Ok((VUSDC, USDC_BSC, 18, false)),
        _ => anyhow::bail!("Unsupported asset: {}. Supported: BNB, USDT, BTC, ETH, USDC", symbol),
    }
}

pub fn get_rpc(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        56 => Ok(BSC_RPC_URL),
        _ => anyhow::bail!("Unsupported chain ID: {}. Venus Core Pool is only on BSC (56).", chain_id),
    }
}
