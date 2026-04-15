/// Kamino Lend configuration constants

pub const API_BASE: &str = "https://api.kamino.finance";
pub const MAIN_MARKET: &str = "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF";
/// JLP market (Jupiter LP token as collateral)
pub const JLP_MARKET: &str = "DxXdAyU3kCjnyggvHmY5nAwg5cRbbmdyX3npfDMjjMek";
/// Altcoin market (SOL, mSOL, jitoSOL, etc.)
pub const ALTCOIN_MARKET: &str = "ByYiZxp8QrdN9qbdtaAiePN8AAr3qvTPppNJDpf5DVJ5";
pub const KLEND_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const SOLANA_CHAIN_ID: u64 = 501;

/// Known reserve addresses for the Main Market
pub fn reserve_address(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "USDC" => Some("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"),
        "SOL" => Some("d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q"),
        _ => None,
    }
}

pub fn reserve_symbol(reserve_addr: &str) -> &'static str {
    match reserve_addr {
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59" => "USDC",
        "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q" => "SOL",
        _ => "UNKNOWN",
    }
}

/// Native token decimals for each reserve (used to convert raw amounts to UI units).
pub fn reserve_decimals(reserve_addr: &str) -> u32 {
    match reserve_addr {
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59" => 6,  // USDC
        "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q" => 9,  // SOL
        _ => 9,
    }
}
