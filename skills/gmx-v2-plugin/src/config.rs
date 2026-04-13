/// Chain configuration for GMX V2
pub struct ChainConfig {
    pub chain_id: u64,
    pub exchange_router: &'static str,
    pub router: &'static str,
    pub order_vault: &'static str,
    pub deposit_vault: &'static str,
    pub withdrawal_vault: &'static str,
    pub reader: &'static str,
    pub datastore: &'static str,
    pub api_base: &'static str,
    pub api_fallback: &'static str,
    pub rpc_url: &'static str,
    pub execution_fee_wei: u128,
}

pub static ARBITRUM: ChainConfig = ChainConfig {
    chain_id: 42161,
    exchange_router: "0x1C3fa76e6E1088bCE750f23a5BFcffa1efEF6A41",
    router: "0x7452c558d45f8afC8c83dAe62C3f8A5BE19c71f6",
    order_vault: "0x31eF83a530Fde1B38EE9A18093A333D8Bbbc40D5",
    deposit_vault: "0xF89e77e8Dc11691C9e8757e84aaFbCD8A67d7A55",
    withdrawal_vault: "0x0628D46b5D145f183AdB6Ef1f2c97eD1C4701C55",
    reader: "0x470fbC46bcC0f16532691Df360A07d8Bf5ee0789",
    datastore: "0xFD70de6b91282D8017aA4E741e9Ae325CAb992d8",
    api_base: "https://arbitrum-api.gmxinfra.io",
    api_fallback: "https://arbitrum-api.gmxinfra2.io",
    rpc_url: "https://arbitrum.publicnode.com",
    execution_fee_wei: 1_000_000_000_000_000, // 0.001 ETH
};

pub static AVALANCHE: ChainConfig = ChainConfig {
    chain_id: 43114,
    exchange_router: "0x8f550E53DFe96C055D5Bdb267c21F268fCAF63B2",
    router: "0x820F5FfC5b525cD4d88Cd91aCf2c28F16530Cc68",
    order_vault: "0xD3D60D22d415aD43b7e64b510D86A30f19B1B12C",
    deposit_vault: "0x90c670825d0C62ede1c5ee9571d6d9a17A722DFF",
    withdrawal_vault: "0xf5F30B10141E1F63FC11eD772931A8294a591996",
    reader: "0x62Cb8740E6986B29dC671B2EB596676f60590A5B",
    datastore: "0x2F0b22339414ADeD7D5F06f9D604c7fF5b2fe3f6",
    api_base: "https://avalanche-api.gmxinfra.io",
    api_fallback: "https://avalanche-api.gmxinfra2.io",
    rpc_url: "https://avalanche-c-chain-rpc.publicnode.com",
    execution_fee_wei: 12_000_000_000_000_000, // 0.012 AVAX
};

pub fn get_chain_config(chain: &str) -> anyhow::Result<&'static ChainConfig> {
    match chain.to_lowercase().as_str() {
        "arbitrum" | "arb" | "42161" => Ok(&ARBITRUM),
        "avalanche" | "avax" | "43114" => Ok(&AVALANCHE),
        _ => anyhow::bail!("Unsupported chain '{}'. Use 'arbitrum' or 'avalanche'.", chain),
    }
}

/// GMX V2 price precision: 1 USD = 10^30
pub const PRICE_PRECISION: u128 = 1_000_000_000_000_000_000_000_000_000_000; // 10^30

/// Default slippage in basis points (100 = 1%)
pub const DEFAULT_SLIPPAGE_BPS: u32 = 100;
