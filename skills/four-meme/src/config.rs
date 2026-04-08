/// BNB Chain (BSC) chain ID
pub const CHAIN_ID: u64 = 56;

/// BSC public RPC
pub const BSC_RPC: &str = "https://bsc-dataseed.binance.org";

/// TokenManager V2 -- handles all tokens created after Sept 5, 2024
pub const TOKEN_MANAGER_V2: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";

/// TokenManagerHelper V3 -- unified query + pre-calc for V1 and V2 tokens
pub const TOKEN_MANAGER_HELPER_V3: &str = "0xF251F83e40a78868FcfA3FA4599Dad6494E46034";

/// Four.meme public config API (no auth required)
pub const API_CONFIG: &str = "https://four.meme/meme-api/v1/public/config";

/// Four.meme token info API (no auth required for reads)
pub const API_TOKEN_GET: &str = "https://four.meme/meme-api/v1/private/token/get";

/// Null address (BNB quote)
pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
