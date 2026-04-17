/// Kamino Lend configuration constants

pub const API_BASE: &str = "https://api.kamino.finance";
pub const MAIN_MARKET: &str = "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF";
pub const KLEND_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const SOLANA_CHAIN_ID: u64 = 501;

/// Known reserve addresses for the Main Market.
/// Derived via: getProgramAccounts(KLend program, filters=[market@32, tokenMint@128])
/// wSOL maps to the same reserve as SOL — Kamino handles wrapping/unwrapping automatically.
pub fn reserve_address(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        // Stablecoins
        "USDC"              => Some("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"),
        "USDT"              => Some("H3t6qZ1JkguCNTi9uzVKqQ7dvt2cum4XiXWom6Gn5e5S"),
        "PYUSD"             => Some("2gc9Dm1eB6UgVYFBUN9bWks6Kes9PbWSaPaa9DqyvEiN"),
        "USDS"              => Some("BHUi32TrEsfN2U821G4FprKrR4hTeK4LCWtA3BFetuqA"),
        // Native & liquid staking
        "SOL" | "WSOL"      => Some("d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q"),
        "JITOSOL"           => Some("EVbyPKrHG6WBfm4dLxLMJpUDY43cCAcHSpV3KYjKsktW"),
        "MSOL"              => Some("FBSyPnxtHKLBZ4UeeUyAnbtFuAmTHLtso9YtsqRDRWpM"),
        "JUPSOL"            => Some("DGQZWCY17gGtBUgdaFs1VreJWsodkjFxndPsskwFKGpp"),
        "BSOL"              => Some("H9vmCVd77N1HZa36eBn3UnftYmg4vQzPfm1RxabHAMER"),
        // Cross-chain
        "ETH" | "WETH"      => Some("febGYTnFX4GbSGoFHFeJXUHgNaK53fB23uDins9Jp1E"),
        "CBBTC"             => Some("37Jk2zkz23vkAYBT66HM2gaqJuNg2nYLsCreQAVt5MWK"),
        _ => None,
    }
}

pub fn reserve_symbol(reserve_addr: &str) -> &'static str {
    match reserve_addr {
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59" => "USDC",
        "H3t6qZ1JkguCNTi9uzVKqQ7dvt2cum4XiXWom6Gn5e5S" => "USDT",
        "2gc9Dm1eB6UgVYFBUN9bWks6Kes9PbWSaPaa9DqyvEiN" => "PYUSD",
        "BHUi32TrEsfN2U821G4FprKrR4hTeK4LCWtA3BFetuqA" => "USDS",
        "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q" => "SOL",
        "EVbyPKrHG6WBfm4dLxLMJpUDY43cCAcHSpV3KYjKsktW" => "JitoSOL",
        "FBSyPnxtHKLBZ4UeeUyAnbtFuAmTHLtso9YtsqRDRWpM" => "mSOL",
        "DGQZWCY17gGtBUgdaFs1VreJWsodkjFxndPsskwFKGpp" => "JupSOL",
        "H9vmCVd77N1HZa36eBn3UnftYmg4vQzPfm1RxabHAMER" => "bSOL",
        "febGYTnFX4GbSGoFHFeJXUHgNaK53fB23uDins9Jp1E" => "ETH",
        "37Jk2zkz23vkAYBT66HM2gaqJuNg2nYLsCreQAVt5MWK" => "cbBTC",
        _ => "UNKNOWN",
    }
}

/// SPL token mint address for each reserve.
/// Used to fetch wallet balances and call Jupiter for auto-swap on interest shortfall.
pub fn reserve_mint(reserve_addr: &str) -> Option<&'static str> {
    match reserve_addr {
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59" => Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
        "H3t6qZ1JkguCNTi9uzVKqQ7dvt2cum4XiXWom6Gn5e5S" => Some("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"), // USDT
        "2gc9Dm1eB6UgVYFBUN9bWks6Kes9PbWSaPaa9DqyvEiN" => Some("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo"), // PYUSD
        "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q"  => Some("So11111111111111111111111111111111111111112"),   // wSOL
        "EVbyPKrHG6WBfm4dLxLMJpUDY43cCAcHSpV3KYjKsktW" => Some("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn"), // JitoSOL
        "FBSyPnxtHKLBZ4UeeUyAnbtFuAmTHLtso9YtsqRDRWpM" => Some("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"),  // mSOL
        "DGQZWCY17gGtBUgdaFs1VreJWsodkjFxndPsskwFKGpp" => Some("jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v"),  // JupSOL
        "H9vmCVd77N1HZa36eBn3UnftYmg4vQzPfm1RxabHAMER" => Some("bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1"),  // bSOL
        "febGYTnFX4GbSGoFHFeJXUHgNaK53fB23uDins9Jp1E"  => Some("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs"), // ETH (Wormhole)
        "37Jk2zkz23vkAYBT66HM2gaqJuNg2nYLsCreQAVt5MWK" => Some("cbbtcf3aa214zXHbiAZQwf4122FBYbraNdFqgw4iMij"),  // cbBTC
        _ => None,
    }
}

/// Native token decimals for each reserve (used to convert raw amounts to UI units).
pub fn reserve_decimals(reserve_addr: &str) -> u32 {
    match reserve_addr {
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59" => 6,  // USDC
        "H3t6qZ1JkguCNTi9uzVKqQ7dvt2cum4XiXWom6Gn5e5S" => 6,  // USDT
        "2gc9Dm1eB6UgVYFBUN9bWks6Kes9PbWSaPaa9DqyvEiN" => 6,  // PYUSD
        "BHUi32TrEsfN2U821G4FprKrR4hTeK4LCWtA3BFetuqA" => 6,  // USDS
        "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q" => 9,   // SOL
        "EVbyPKrHG6WBfm4dLxLMJpUDY43cCAcHSpV3KYjKsktW" => 9,  // JitoSOL
        "FBSyPnxtHKLBZ4UeeUyAnbtFuAmTHLtso9YtsqRDRWpM" => 9,  // mSOL
        "DGQZWCY17gGtBUgdaFs1VreJWsodkjFxndPsskwFKGpp" => 9,  // JupSOL
        "H9vmCVd77N1HZa36eBn3UnftYmg4vQzPfm1RxabHAMER" => 9,  // bSOL
        "febGYTnFX4GbSGoFHFeJXUHgNaK53fB23uDins9Jp1E" => 8,   // ETH (Wormhole, 8 dec)
        "37Jk2zkz23vkAYBT66HM2gaqJuNg2nYLsCreQAVt5MWK" => 8,  // cbBTC
        _ => 9,
    }
}
