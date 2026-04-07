/// Chain configuration and contract addresses for the Fluid plugin.

#[allow(dead_code)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: &'static str,
    pub lending_resolver: &'static str,
    pub dex_resolver: &'static str,
    pub liquidity_resolver: &'static str,
}

pub const CHAIN_BASE: ChainConfig = ChainConfig {
    chain_id: 8453,
    rpc_url: "https://base-rpc.publicnode.com",
    lending_resolver: "0x48D32f49aFeAEC7AE66ad7B9264f446fc11a1569",
    dex_resolver: "0x11D80CfF056Cef4F9E6d23da8672fE9873e5cC07",
    liquidity_resolver: "0xca13A15de31235A37134B4717021C35A3CF25C60",
};

pub const CHAIN_ETHEREUM: ChainConfig = ChainConfig {
    chain_id: 1,
    rpc_url: "https://eth.llamarpc.com",
    lending_resolver: "0x48D32f49aFeAEC7AE66ad7B9264f446fc11a1569",
    dex_resolver: "0x11D80CfF056Cef4F9E6d23da8672fE9873e5cC07",
    liquidity_resolver: "0xca13A15de31235A37134B4717021C35A3CF25C60",
};

pub const CHAIN_ARBITRUM: ChainConfig = ChainConfig {
    chain_id: 42161,
    rpc_url: "https://arbitrum-one-rpc.publicnode.com",
    lending_resolver: "0x48D32f49aFeAEC7AE66ad7B9264f446fc11a1569",
    dex_resolver: "0x11D80CfF056Cef4F9E6d23da8672fE9873e5cC07",
    liquidity_resolver: "0xca13A15de31235A37134B4717021C35A3CF25C60",
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        1 => Ok(&CHAIN_ETHEREUM),
        8453 => Ok(&CHAIN_BASE),
        42161 => Ok(&CHAIN_ARBITRUM),
        _ => anyhow::bail!("Unsupported chain ID: {}. Use 1 (Ethereum), 8453 (Base), or 42161 (Arbitrum)", chain_id),
    }
}

pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "Ethereum Mainnet",
        8453 => "Base",
        42161 => "Arbitrum",
        _ => "Unknown",
    }
}

/// Known fToken addresses per chain.
/// Returns (ftoken_address, underlying_asset_address, symbol, decimals)
pub fn get_ftoken_info(symbol: &str, chain_id: u64) -> anyhow::Result<(&'static str, &'static str, u8)> {
    // Returns (ftoken_addr, underlying_addr, decimals)
    let info = match (chain_id, symbol.to_uppercase().as_str()) {
        // Base (8453)
        (8453, "FUSDC") | (8453, "FTOKEN_FUSDC") => (
            "0xf42f5795D9ac7e9D757dB633D693cD548Cfd9169",
            "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
            6u8,
        ),
        (8453, "FWETH") | (8453, "FTOKEN_FWETH") => (
            "0x9272D6153133175175Bc276512B2336BE3931CE9",
            "0x4200000000000000000000000000000000000006",
            18u8,
        ),
        (8453, "FGHO") | (8453, "FTOKEN_FGHO") => (
            "0x8DdbfFA3CFda2355a23d6B11105AC624BDbE3631",
            "0x6Bb7a212910682DCFdbd5BCBb3e28FB4E8da10Ee",
            18u8,
        ),
        (8453, "FEURC") | (8453, "FTOKEN_FEURC") => (
            "0x1943FA26360f038230442525Cf1B9125b5DCB401",
            "0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1aDb42",
            6u8,
        ),
        // Ethereum (1)
        (1, "FUSDC") | (1, "FTOKEN_FUSDC") => (
            "0x9Fb7b4477576Fe5B32be4C1843aFB1e55F251B33",
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            6u8,
        ),
        (1, "FWETH") | (1, "FTOKEN_FWETH") => (
            "0x90551c1795392094FE6D29B758EcCD233cFAa260",
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            18u8,
        ),
        (1, "FUSDT") | (1, "FTOKEN_FUSDT") => (
            "0x5C20B550819128074FD538Edf79791733ccEdd18",
            "0xdac17f958d2ee523a2206206994597c13d831ec7",
            6u8,
        ),
        // Arbitrum (42161)
        (42161, "FUSDC") | (42161, "FTOKEN_FUSDC") => (
            "0x1A996cb54bb95462040408C06122D45D6Cdb6096",
            "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
            6u8,
        ),
        (42161, "FWETH") | (42161, "FTOKEN_FWETH") => (
            "0x45Df0656F8aDf017590009d2f1898eeca4F0a205",
            "0x82af49447d8a07e3bd95bd0d56f35241523fbab1",
            18u8,
        ),
        (42161, "FUSDT") | (42161, "FTOKEN_FUSDT") => (
            "0x4A03F37e7d3fC243e3f99341d36f4b829BEe5E03",
            "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
            6u8,
        ),
        _ => anyhow::bail!(
            "Unknown fToken '{}' on chain {}. Use symbols like fUSDC, fWETH, or provide address with --ftoken",
            symbol, chain_id
        ),
    };
    Ok(info)
}

/// Known DEX pool addresses per chain.
/// Returns (pool_address, token0_address, token1_address, token0_decimals, token1_decimals, token0_symbol, token1_symbol)
pub struct DexPool {
    pub address: &'static str,
    pub token0: &'static str,
    pub token1: &'static str,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub token0_symbol: &'static str,
    pub token1_symbol: &'static str,
    pub token0_is_eth: bool,
    pub token1_is_eth: bool,
}

pub fn get_dex_pool(token_in: &str, token_out: &str, chain_id: u64) -> anyhow::Result<(&'static DexPool, bool)> {
    let pools = get_dex_pools(chain_id);
    let ti = token_in.to_uppercase();
    let to = token_out.to_uppercase();
    for pool in pools {
        if pool.token0_symbol.to_uppercase() == ti && pool.token1_symbol.to_uppercase() == to {
            return Ok((pool, true)); // swap0to1 = true
        }
        if pool.token1_symbol.to_uppercase() == ti && pool.token0_symbol.to_uppercase() == to {
            return Ok((pool, false)); // swap0to1 = false
        }
    }
    anyhow::bail!(
        "No Fluid DEX pool found for {}/{} on chain {}. Available pools: EURC/USDC, USDe/USDC, wstETH/ETH, weETH/ETH, FLUID/ETH",
        token_in, token_out, chain_id
    )
}

static BASE_POOLS: &[DexPool] = &[
    DexPool {
        address: "0x2886a01a0645390872a9eb99dAe1283664b0c524",
        token0: "0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1aDb42",
        token1: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        token0_decimals: 6,
        token1_decimals: 6,
        token0_symbol: "EURC",
        token1_symbol: "USDC",
        token0_is_eth: false,
        token1_is_eth: false,
    },
    DexPool {
        address: "0x836951EB21F3Df98273517B7249dCEFF270d34bf",
        token0: "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34",
        token1: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        token0_decimals: 18,
        token1_decimals: 6,
        token0_symbol: "USDE",
        token1_symbol: "USDC",
        token0_is_eth: false,
        token1_is_eth: false,
    },
    DexPool {
        address: "0x667701e51B4D1Ca244F17C78F7aB8744B4C99F9B",
        token0: "0xc1cba3fcea344f92d9239c08c0568f6f2f0ee452",
        token1: "0x4200000000000000000000000000000000000006",
        token0_decimals: 18,
        token1_decimals: 18,
        token0_symbol: "WSTETH",
        token1_symbol: "WETH",
        token0_is_eth: false,
        token1_is_eth: true,
    },
    DexPool {
        address: "0x3C0441B42195F4aD6aa9a0978E06096ea616CDa7",
        token0: "0x04C0599Ae5A44757c0af6F9eC3b93da8976c150A",
        token1: "0x4200000000000000000000000000000000000006",
        token0_decimals: 18,
        token1_decimals: 18,
        token0_symbol: "WEETH",
        token1_symbol: "WETH",
        token0_is_eth: false,
        token1_is_eth: true,
    },
    DexPool {
        address: "0xdE632C3a214D5f14C1d8ddF0b92F8BCd188fee45",
        token0: "0xf73CF2BE6d553a2bBe48Cba4D0Ae6a72bD46E0D0",
        token1: "0x4200000000000000000000000000000000000006",
        token0_decimals: 18,
        token1_decimals: 18,
        token0_symbol: "FLUID",
        token1_symbol: "WETH",
        token0_is_eth: false,
        token1_is_eth: true,
    },
];

static ETHEREUM_POOLS: &[DexPool] = &[];
static ARBITRUM_POOLS: &[DexPool] = &[];

pub fn get_dex_pools(chain_id: u64) -> &'static [DexPool] {
    match chain_id {
        8453 => BASE_POOLS,
        1 => ETHEREUM_POOLS,
        42161 => ARBITRUM_POOLS,
        _ => &[],
    }
}

/// Resolve token symbol to address on a given chain
#[allow(dead_code)]
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> anyhow::Result<String> {
    if symbol.starts_with("0x") && symbol.len() == 42 {
        return Ok(symbol.to_lowercase());
    }
    let addr = match (chain_id, symbol.to_uppercase().as_str()) {
        (8453, "USDC") => "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        (8453, "WETH") => "0x4200000000000000000000000000000000000006",
        (8453, "EURC") => "0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1aDb42",
        (8453, "USDE") => "0x5d3a1Ff2b6BAb83b63cd9AD0787074081a52ef34",
        (8453, "WSTETH") => "0xc1cba3fcea344f92d9239c08c0568f6f2f0ee452",
        (8453, "WEETH") => "0x04C0599Ae5A44757c0af6F9eC3b93da8976c150A",
        (8453, "FLUID") => "0xf73CF2BE6d553a2bBe48Cba4D0Ae6a72bD46E0D0",
        (8453, "GHO") => "0x6Bb7a212910682DCFdbd5BCBb3e28FB4E8da10Ee",
        (1, "USDC") => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        (1, "WETH") => "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        (1, "USDT") => "0xdac17f958d2ee523a2206206994597c13d831ec7",
        (1, "WSTETH") => "0x7f39c581f595b53c5cb19bd0b3f8da6c935e2ca0",
        (42161, "USDC") => "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        (42161, "WETH") => "0x82af49447d8a07e3bd95bd0d56f35241523fbab1",
        (42161, "USDT") => "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
        _ => anyhow::bail!("Unknown token '{}' on chain {}. Please provide the ERC-20 address.", symbol, chain_id),
    };
    Ok(addr.to_string())
}
