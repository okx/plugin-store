/// Chain configuration: RPC URLs and contract addresses

#[allow(dead_code)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: &'static str,
    pub nonfungible_position_manager: &'static str,
    pub masterchef_v3: &'static str,
    pub factory: &'static str,
}

pub const CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 56,
        rpc_url: "https://bsc-rpc.publicnode.com",
        nonfungible_position_manager: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
        masterchef_v3: "0x556B9306565093C855AEA9AE92A594704c2Cd59e",
        factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    },
    ChainConfig {
        chain_id: 1,
        rpc_url: "https://ethereum.publicnode.com",
        nonfungible_position_manager: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
        masterchef_v3: "0x556B9306565093C855AEA9AE92A594704c2Cd59e",
        factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    },
    ChainConfig {
        chain_id: 8453,
        rpc_url: "https://base-rpc.publicnode.com",
        nonfungible_position_manager: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
        masterchef_v3: "0xC6A2Db661D5a5690172d8eB0a7DEA2d3008665A3",
        factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    },
    ChainConfig {
        chain_id: 42161,
        rpc_url: "https://arb1.arbitrum.io/rpc",
        nonfungible_position_manager: "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364",
        masterchef_v3: "0x5e09ACf80C0296740eC5d6F643005a4ef8DaA694",
        factory: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865",
    },
];

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported chain ID: {}. Supported: 56 (BSC), 1 (Ethereum), 8453 (Base), 42161 (Arbitrum)",
                chain_id
            )
        })
}

pub fn get_rpc_url(chain_id: u64, override_rpc: Option<&str>) -> anyhow::Result<String> {
    if let Some(url) = override_rpc {
        return Ok(url.to_string());
    }
    get_chain_config(chain_id).map(|c| c.rpc_url.to_string())
}
