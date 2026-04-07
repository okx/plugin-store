/// Dynamic contract address resolution via RocketStorage.
/// RocketStorage.getAddress(bytes32) — selector 0x21f8a721.
use crate::{config, onchainos, rpc};

pub struct RocketPoolContracts {
    pub deposit_pool: String,
    pub token_reth: String,
    pub network_balances: String,
    pub node_manager: String,
    pub minipool_manager: String,
}

impl RocketPoolContracts {
    /// Resolve all relevant contract addresses from RocketStorage.
    pub fn resolve(chain_id: u64) -> anyhow::Result<Self> {
        let deposit_pool = resolve_address(chain_id, config::KEY_DEPOSIT_POOL)?;
        let token_reth = resolve_address(chain_id, config::KEY_TOKEN_RETH)?;
        let network_balances = resolve_address(chain_id, config::KEY_NETWORK_BALANCES)?;
        let node_manager = resolve_address(chain_id, config::KEY_NODE_MANAGER)?;
        // For minipool manager, use known stable address as fallback
        let minipool_manager = resolve_address(chain_id, config::KEY_MINIPOOL_MANAGER)
            .unwrap_or_else(|_| "0xe54b8c641fd96de5d6747f47c19964c6b824d62c".to_string());
        Ok(Self {
            deposit_pool,
            token_reth,
            network_balances,
            node_manager,
            minipool_manager,
        })
    }
}

fn resolve_address(chain_id: u64, key_hex: &str) -> anyhow::Result<String> {
    let calldata = format!("0x{}{}", config::SEL_GET_ADDRESS, key_hex);
    let result = onchainos::eth_call(chain_id, config::ROCKET_STORAGE, &calldata)?;
    let data = rpc::extract_return_data(&result)?;
    let addr = rpc::decode_address(&data)?;
    if addr == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("RocketStorage returned zero address for key {}", key_hex);
    }
    Ok(addr)
}
