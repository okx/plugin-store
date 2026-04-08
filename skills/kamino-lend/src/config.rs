/// Kamino Lend configuration constants

pub const API_BASE: &str = "https://api.kamino.finance";
pub const MAIN_MARKET: &str = "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF";
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
