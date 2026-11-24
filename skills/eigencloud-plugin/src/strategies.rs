/// EigenLayer strategy table — verified on-chain via underlyingToken() calls.
pub struct Strategy {
    pub symbol: &'static str,
    pub strategy: &'static str,   // EigenLayer strategy contract
    pub token: &'static str,      // Underlying LST token contract
    pub decimals: u8,
    pub description: &'static str,
}

pub const STRATEGIES: &[Strategy] = &[
    Strategy {
        symbol: "stETH",
        strategy: "0x93c4b944d05dfe6df7645a86cd2206016c51564d",
        token: "0xae7ab96520de3a18e5e111b5eaab095312d7fe84",
        decimals: 18,
        description: "Lido Staked ETH",
    },
    Strategy {
        symbol: "rETH",
        strategy: "0x1bee69b7dfffA4e2d53c2a2df135c388ad25dcd2",
        token: "0xae78736cd615f374d3085123a210448e74fc6393",
        decimals: 18,
        description: "Rocket Pool ETH",
    },
    Strategy {
        symbol: "cbETH",
        strategy: "0x54945180db7943c0ed0fee7edab2bd24620256bc",
        token: "0xbe9895146f7af43049ca1c1ae358b0541ea49704",
        decimals: 18,
        description: "Coinbase Wrapped Staked ETH",
    },
    Strategy {
        symbol: "mETH",
        strategy: "0x298afb19a105d59e74658c4c334ff360bade6dd2",
        token: "0xd5f7838f5c461feff7fe49ea5ebaf7728bb0adfa",
        decimals: 18,
        description: "Mantle Staked ETH",
    },
    Strategy {
        symbol: "swETH",
        strategy: "0x0fe4f44bee93503346a3ac9ee5a26b130a5796d6",
        token: "0xf951e335afb289353dc249e82926178eac7ded78",
        decimals: 18,
        description: "Swell ETH",
    },
    Strategy {
        symbol: "wBETH",
        strategy: "0x7ca911e83dabf90c90dd3de5411a10f1a6112184",
        token: "0xa2e3356610840701bdf5611a53974510ae27e2e1",
        decimals: 18,
        description: "Wrapped Beacon ETH (Binance)",
    },
    Strategy {
        symbol: "sfrxETH",
        strategy: "0x8ca7a5d6f3acd3a7a8bc468a8cd0fb14b6bd28b6",
        token: "0xac3e018457b222d93114458476f3e3416abbe38f",
        decimals: 18,
        description: "Staked Frax ETH",
    },
    Strategy {
        symbol: "osETH",
        strategy: "0x57ba429517c3473b6d34ca9acd56c0e735b94c02",
        token: "0xf1c9acdc66974dfb6decb12aa385b9cd01190e38",
        decimals: 18,
        description: "StakeWise Staked ETH",
    },
    Strategy {
        symbol: "ETHx",
        strategy: "0x9d7ed45ee2e8fc5482fa2428f15c971e6369011d",
        token: "0xa35b1b31ce002fbf2058d22f30f95d405200a15b",
        decimals: 18,
        description: "Stader ETHx",
    },
    Strategy {
        symbol: "ankrETH",
        strategy: "0x13760f50a9d7377e4f20cb8cf9e4c26586c658ff",
        token: "0xe95a203b1a91a908f9b9ce46459d101078c2c3cb",
        decimals: 18,
        description: "Ankr Staked ETH",
    },
    Strategy {
        symbol: "EIGEN",
        strategy: "0xacb55c530acdb2849e6d4f36992cd8c9d50ed8f7",
        token: "0x83e9115d334d248ce39a6f36144aeab5b3456e75",
        decimals: 18,
        description: "EigenLayer Token",
    },
];

pub fn by_symbol(sym: &str) -> Option<&'static Strategy> {
    STRATEGIES.iter().find(|s| s.symbol.eq_ignore_ascii_case(sym))
}

pub fn by_token(addr: &str) -> Option<&'static Strategy> {
    let addr = addr.to_lowercase();
    STRATEGIES.iter().find(|s| s.token.eq_ignore_ascii_case(&addr))
}

pub fn by_strategy(addr: &str) -> Option<&'static Strategy> {
    STRATEGIES.iter().find(|s| s.strategy.eq_ignore_ascii_case(addr))
}
