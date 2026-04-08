pub struct ChainConfig {
    pub router: &'static str,
    pub position_router: &'static str,
    pub glp_manager: &'static str,
    pub reward_router: &'static str,
    pub api_base_url: &'static str,
}

pub const ARBITRUM: ChainConfig = ChainConfig {
    router: "0xaBBc5F99639c9B6bCb58544ddf04CF3C176D2B00",
    position_router: "0xb87a436B93fE243ff3BC3ff12dA8dcFF7A5a36a7",
    // GlpManager V2 (updated): the original V1 GlpManager no longer accepts new deposits
    glp_manager: "0x3963ffc9dff443c2a94f21b129d429891e32ec18",
    // RewardRouter V2 (updated): routes through updated GlpManager
    reward_router: "0xB95DB5B167D75e6d04227CfFFA61069348d271F5",
    api_base_url: "https://arbitrum-api.gmxinfra.io",
};

pub const AVALANCHE: ChainConfig = ChainConfig {
    router: "0x5F719c2F1095F7B9fc68a68e35B51194f4b6abe8",
    position_router: "0x195256074192170d1809527d3c462CF0430Bb4d7",
    glp_manager: "0xe1ae4d4b06A5Fe1fc288f6B4CD72f9F8323B107F",
    reward_router: "0x82147C5A7E850eA4E28155DF107F2590fD4ba327",
    api_base_url: "https://avalanche-api.gmxinfra.io",
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        42161 => Ok(&ARBITRUM),
        43114 => Ok(&AVALANCHE),
        _ => anyhow::bail!(
            "Unsupported chain ID: {}. Use 42161 (Arbitrum) or 43114 (Avalanche)",
            chain_id
        ),
    }
}

/// GMX V1 position execution fee: 0.0001 ETH = 100_000_000_000_000 wei
pub const EXECUTION_FEE_WEI: u64 = 100_000_000_000_000;
