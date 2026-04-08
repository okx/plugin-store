/// Sui mainnet JSON-RPC endpoint
pub const SUI_RPC_URL: &str = "https://fullnode.mainnet.sui.io";

/// Scallop REST API base
pub const SCALLOP_API_URL: &str = "https://sui.apis.scallop.io";

/// Scallop protocol package (mainnet)
pub const PROTOCOL_PACKAGE: &str =
    "0xd971609b7feb6230585831e7aeb3c121fb21b9431337a30fc99185eb459a05ee";

/// Scallop market object (mainnet)
pub const MARKET_OBJECT: &str =
    "0xed80ed898df1e0b7a14b78c92527b47ef88591d5722ded16050d7e101687bb20";

/// Scallop version object (mainnet)
pub const VERSION_OBJECT: &str =
    "0x72bc09c4ce413d76d07f6e712413aebbe3ce3747eadfbc2331fbdb1dbde2d43a";

/// Well-known assets on Scallop (symbol -> coin type)
pub fn known_coin_type(symbol: &str) -> Option<&'static str> {
    match symbol.to_lowercase().as_str() {
        "sui" => Some("0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI"),
        "usdc" => Some("0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC"),
        "sca" => Some("0x7016aae72cfc67f2fadf55769c0a7dd54291a583b63051a5ed71081cce836ac6::sca::SCA"),
        "deep" => Some("0xdeeb7a4662eec9f2f3def03fb937a663dddaa2e215b8078a284d026b7946c270::deep::DEEP"),
        "hasui" => Some("0xbde4ba4c2e274a60ce15c1cfff9e5c42e41654ac8b6d906a57efa4bd3c29f47d::hasui::HASUI"),
        "cetus" => Some("0x06864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS"),
        "weth" => Some("0xaf8cd5edc19c4512f4259f0bee101a40d41ebed738ade5874359610ef8eeced5::coin::COIN"),
        "usdt" | "sbusdt" => Some("0x375f70cf2ae4c00bf37117d0c85a2c71545e6ee05c4a5c7d282cd66a4504b068::coin::COIN"),
        "wsol" => Some("0xb7844e289a8410e50fb3ca48d69eb9cf29e27d223ef90353fe1bd8e27ff8f3f8::coin::COIN"),
        _ => None,
    }
}
