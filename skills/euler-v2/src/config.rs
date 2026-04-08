/// Chain configuration and contract addresses for the Euler V2 plugin.

#[allow(dead_code)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: &'static str,
    pub rpc_url: &'static str,
    pub evc: &'static str,
    pub evault_factory: &'static str,
    pub account_lens: &'static str,
    pub vault_lens: &'static str,
    pub governed_perspective: &'static str,
}

pub const CHAIN_ETHEREUM: ChainConfig = ChainConfig {
    chain_id: 1,
    name: "Ethereum",
    rpc_url: "https://eth.llamarpc.com",
    evc: "0x0C9a3dd6b8F28529d72d7f9cE918D493519EE383",
    evault_factory: "0x29a56a1b8214D9Cf7c5561811750D5cBDb45CC8e",
    account_lens: "0xA60c4257c809353039A71527dfe701B577e34bc7",
    vault_lens: "0xA18D79deB85C414989D7297F23e5391703Ea66aB",
    governed_perspective: "0xC0121817FF224a018840e4D15a864747d36e6Eb2",
};

pub const CHAIN_BASE: ChainConfig = ChainConfig {
    chain_id: 8453,
    name: "Base",
    rpc_url: "https://base-rpc.publicnode.com",
    evc: "0x5301c7dD20bD945D2013b48ed0DEE3A284ca8989",
    evault_factory: "0x7F321498A801A191a93C840750ed637149dDf8D0",
    account_lens: "0xe6b05A38D6a29D2C8277fA1A8BA069F1693b780C",
    vault_lens: "0x601F023CD063324DdbCADa69460e969fb97e98b9",
    governed_perspective: "0xafC8545c49DF2c8216305922D9753Bf60bf8c14A",
};

pub const CHAIN_ARBITRUM: ChainConfig = ChainConfig {
    chain_id: 42161,
    name: "Arbitrum",
    rpc_url: "https://arbitrum-one-rpc.publicnode.com",
    evc: "0x6302ef0F34100CDDFb5489fbcB6eE1AA95CD1066",
    evault_factory: "0x78Df1CF5bf06a7f27f2ACc580B934238C1b80D50",
    account_lens: "0x90a52DDcb232e7bb003DD9258fA1235c553eC956",
    vault_lens: "0x19ff0fD1c4bC5aD5D9ad75EA7303DEaAA6286814",
    governed_perspective: "0x0000000000000000000000000000000000000000",
};

pub const CHAIN_AVALANCHE: ChainConfig = ChainConfig {
    chain_id: 43114,
    name: "Avalanche",
    rpc_url: "https://api.avax.network/ext/bc/C/rpc",
    evc: "0xddcbe30A761Edd2e19bba930A977475265F36Fa1",
    evault_factory: "0xaf4B4c18B17F6a2B32F6c398a3910bdCD7f26181",
    account_lens: "0x08bb803D19e5E2F006C87FEe77c232Dc481cB735",
    vault_lens: "0x7a2A57a0ed6807c7dbF846cc74aa04eE9DFa7F57",
    governed_perspective: "0x0000000000000000000000000000000000000000",
};

pub const CHAIN_BSC: ChainConfig = ChainConfig {
    chain_id: 56,
    name: "BSC",
    rpc_url: "https://bsc-rpc.publicnode.com",
    evc: "0xb2E5a73CeE08593d1a076a2AE7A6e02925a640ea",
    evault_factory: "0x7F53E2755eB3c43824E162F7F6F087832B9C9Df6",
    account_lens: "0x9578D17d2e1AA70EA6f9eC8A39967bfD1c6F6217",
    vault_lens: "0xA5A9486CaF3155123f8846b5478b72bDd6560BF7",
    governed_perspective: "0x0000000000000000000000000000000000000000",
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        1 => Ok(&CHAIN_ETHEREUM),
        8453 => Ok(&CHAIN_BASE),
        42161 => Ok(&CHAIN_ARBITRUM),
        43114 => Ok(&CHAIN_AVALANCHE),
        56 => Ok(&CHAIN_BSC),
        _ => anyhow::bail!(
            "Unsupported chain ID: {}. Use 1 (Ethereum), 8453 (Base), 42161 (Arbitrum), 43114 (Avalanche), 56 (BSC)",
            chain_id
        ),
    }
}

/// Known EVault addresses for common assets on Base (8453).
/// Returns (vault_address, underlying_address, decimals).
pub fn get_known_vault(symbol: &str, chain_id: u64) -> Option<(&'static str, &'static str, u8)> {
    match (chain_id, symbol.to_uppercase().as_str()) {
        // Base (8453)
        (8453, "USDC") => Some((
            "0x0a1a3b5f2041f33522c4efc754a7d096f880ee16",
            "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
            6,
        )),
        (8453, "WETH") | (8453, "ETH") => Some((
            "0x859160db5841e5cfb8d3f144c6b3381a85a4b410",
            "0x4200000000000000000000000000000000000006",
            18,
        )),
        (8453, "CBBTC") | (8453, "BTC") => Some((
            "0x7b181d6509deabfbd1a23af1e65fd46e89572609",
            "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
            8,
        )),
        // Ethereum (1)
        (1, "USDC") => Some((
            "0x797DD80692c3b2dAdabCe8e30C07fDE5307D48a9",
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            6,
        )),
        (1, "WETH") | (1, "ETH") => Some((
            "0xb3b36220fA7d12f7055dab5dc857592743B6151F",
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            18,
        )),
        _ => None,
    }
}
