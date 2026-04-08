/// NAVI Protocol on-chain constants (Sui Mainnet)

pub const SUI_RPC_URL: &str = "https://fullnode.mainnet.sui.io";
pub const NAVI_OPEN_API: &str = "https://open-api.naviprotocol.io";

/// Default protocol package (v22, latest as of build time).
/// The binary resolves the live package at runtime via the open-api.
pub const DEFAULT_PROTOCOL_PACKAGE: &str =
    "0x1e4a13a0494d5facdbe8473e74127b838c2d446ecec0ce262e2eddafa77259cb";

/// Storage object — holds all reserve state
pub const STORAGE_ID: &str =
    "0xbb4e2f4b6205c2e2a2db47aeb4f830796ec7c005f88537ee775986639bc442fe";

/// Price oracle object
pub const PRICE_ORACLE_ID: &str =
    "0x1568865ed9a0b5ec414220e8f79b3d04c77acc82358f6e5ae4635687392ffbef";

/// Incentive V2 object
pub const INCENTIVE_V2_ID: &str =
    "0xf87a8acb8b81d14307894d12595541a73f19933f88e1326d5be349c7a6f7559c";

/// Sui clock object (always 0x6)
pub const CLOCK_ID: &str = "0x0000000000000000000000000000000000000000000000000000000000000006";

/// Pool info for the most common assets
/// (assetId, symbol, poolId, coinType)
pub const POOL_CONFIGS: &[(&str, u8, &str, &str)] = &[
    (
        "SUI",
        0,
        "0x96df0fce3c471489f4debaaa762cf960b3d97820bd1f3f025ff8190730e958c5",
        "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI",
    ),
    (
        "wUSDC",
        1,
        "0xa02a98f9c88db51c6f5efaaf2261c81f34dd56d86073387e0ef1805ca22e39c8",
        "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN",
    ),
    (
        "USDT",
        2,
        "0x0e060c3b5b8de00fb50511b7a45188c8e34b6995c01f69d98ea5a466fe10d103",
        "0xc060006111016b8a020ad5b33834984a437aaa7d3c74c18e09a95d48aceab08c::coin::COIN",
    ),
    (
        "WETH",
        3,
        "0x71b9f6e822c48ce827bceadce82201d6a7559f7b0350ed1daa1dc2ba3ac41b56",
        "0xaf8cd5edc19c4512f4259f0bee101a40d41ebed738ade5874359610ef8eeced5::coin::COIN",
    ),
    (
        "CETUS",
        4,
        "0x3c334f9d1b969b8a4c6e9b68a87b1f6e4f0cf0ced6d2e3a4f5b7c8d9e0a1b2c",
        "0x06864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS",
    ),
    (
        "NAVX",
        7,
        "0xc0e02e7a245e855dd365422faf76f87d9f5b2148a26d48dda6e8253c3fe9fa60",
        "0xa99b8952d4f7d947ea77fe0ecdcc9e5fc0bcab2841d6e2a5aa00c3044e5544b5::navx::NAVX",
    ),
    (
        "nUSDC",
        10,
        "0xa3582097b4c57630046c0c49a88bfc6b202a3ec0a9db5597c31765f7563755a8",
        "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC",
    ),
    (
        "ETH",
        11,
        "0x9d5946af61c9dd2e9c49a08df20ddf42e4b39e5e7d41d7a1d7e4b7a5c6d8e9f",
        "0xd0e89b2af5e4910726fbcd8b8dd37bb79b29e5f83f7491bca830e94f7f226d29::eth::ETH",
    ),
    (
        "suiUSDT",
        19,
        "0xa3e0471746e5d35043801bce247d3b3784cc74329d39f7ed665446ddcf22a9e2",
        "0x375f70cf2ae4c00bf37117d0c85a2c71545e6ee05c4a5c7d282cd66a4504b068::usdt::USDT",
    ),
];

/// Resolve pool config by symbol (case-insensitive)
pub fn find_pool(symbol: &str) -> Option<(&'static str, u8, &'static str, &'static str)> {
    let sym_upper = symbol.to_uppercase();
    POOL_CONFIGS.iter().find_map(|(s, id, pool, coin)| {
        if s.to_uppercase() == sym_upper {
            Some((*s, *id, *pool, *coin))
        } else {
            None
        }
    })
}
