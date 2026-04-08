/// EigenLayer / EigenCloud contract addresses on Ethereum mainnet (chain ID 1).

/// Ethereum mainnet JSON-RPC (public node, no key required).
pub fn rpc_url() -> &'static str {
    "https://ethereum-rpc.publicnode.com"
}

pub fn chain_id() -> u64 {
    1
}

// --- Core protocol contracts ---

/// StrategyManager: handles deposits into LST strategies.
/// depositIntoStrategy(address strategy, address token, uint256 amount) -> uint256 shares
pub fn strategy_manager() -> &'static str {
    "0x858646372CC42E1A627fcE94aa7A7033e7CF075A"
}

/// DelegationManager: handles delegation and withdrawal queueing.
pub fn delegation_manager() -> &'static str {
    "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A"
}

/// RewardsCoordinator: claim EIGEN/operator rewards.
#[allow(dead_code)]
pub fn rewards_coordinator() -> &'static str {
    "0x7750d328b314EfFa365A0402CcfD489B80B0adda"
}

/// EIGEN Token (ERC-20).
#[allow(dead_code)]
pub fn eigen_token() -> &'static str {
    "0xec53bf9167f50cdeb3ae105f56099aaab9061f83"
}

/// EigenStrategy for EIGEN token staking.
#[allow(dead_code)]
pub fn eigen_strategy() -> &'static str {
    "0xaCB55C530Acdb2849e6d4f36992Cd8c9D50ED8F7"
}

// --- LST Strategy addresses ---

pub struct StrategyInfo {
    pub symbol: &'static str,
    pub token: &'static str,
    pub strategy: &'static str,
    pub decimals: u8,
}

/// All supported LST strategies on EigenLayer mainnet.
pub fn lst_strategies() -> &'static [StrategyInfo] {
    &[
        StrategyInfo { symbol: "stETH",   token: "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84", strategy: "0x93c4b944D05dfe6df7645A86cd2206016c51564D", decimals: 18 },
        StrategyInfo { symbol: "rETH",    token: "0xae78736Cd615f374D3085123A210448E74Fc6393", strategy: "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2", decimals: 18 },
        StrategyInfo { symbol: "cbETH",   token: "0xBe9895146f7AF43049ca1c1AE358B0541Ea49704", strategy: "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc", decimals: 18 },
        StrategyInfo { symbol: "ETHx",    token: "0xA35b1B31Ce002FBF2058D22F30f95D405200A15b", strategy: "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d", decimals: 18 },
        StrategyInfo { symbol: "osETH",   token: "0xf1C9acDc66974dFB6dEcB12aA385b9cD01190E38", strategy: "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02", decimals: 18 },
        StrategyInfo { symbol: "wBETH",   token: "0xa2E3356610840701BDf5611a53974510Ae27E2e1", strategy: "0x7CA911E83dabf90C90dD3De5411a10F1A6112184", decimals: 18 },
        StrategyInfo { symbol: "mETH",    token: "0xd5F7838F5C461fefF7FE49ea5ebaF7728bB0ADfa", strategy: "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2", decimals: 18 },
        StrategyInfo { symbol: "OETH",    token: "0x856c4Efb76C1D1AE02e20CEB03A2A6a08b0b8dC3", strategy: "0xa4C637e0F704745D182e4D38cAb7E7485321d059", decimals: 18 },
        StrategyInfo { symbol: "sfrxETH", token: "0xac3E018457B222d93114458476f3E3416Abbe38F", strategy: "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6", decimals: 18 },
        StrategyInfo { symbol: "lsETH",   token: "0x8c1BEd5b9a0928467c9B1341Da1D7BD5e10b6549", strategy: "0xAe60d8180437b5C34bB956822ac2710972584473", decimals: 18 },
        StrategyInfo { symbol: "EIGEN",   token: "0xec53bf9167f50cdeb3ae105f56099aaab9061f83", strategy: "0xaCB55C530Acdb2849e6d4f36992Cd8c9D50ED8F7", decimals: 18 },
    ]
}

/// Resolve a token symbol to (token_address, strategy_address, decimals).
/// Returns None if symbol is unknown.
pub fn resolve_token(symbol: &str) -> Option<&'static StrategyInfo> {
    let upper = symbol.to_uppercase();
    for s in lst_strategies() {
        if s.symbol.to_uppercase() == upper {
            return Some(s);
        }
    }
    None
}

/// ABI helper: pad address to 32 bytes (no 0x prefix).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", clean)
}

/// ABI helper: pad u128 to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// Build ERC-20 approve calldata.
/// approve(address spender, uint256 amount) — selector 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    format!(
        "0x095ea7b3{}{}",
        pad_address(spender),
        pad_u256(amount)
    )
}

/// Build depositIntoStrategy calldata.
/// depositIntoStrategy(address strategy, address token, uint256 amount) — selector 0xe7a050aa
pub fn build_deposit_calldata(strategy: &str, token: &str, amount: u128) -> String {
    format!(
        "0xe7a050aa{}{}{}",
        pad_address(strategy),
        pad_address(token),
        pad_u256(amount)
    )
}

/// Build delegateTo calldata.
/// delegateTo(address operator, SignatureWithExpiry approverSignatureAndExpiry, bytes32 approverSalt)
/// For unregistered delegation (no approver): empty signature, zero expiry, zero salt.
/// Selector: 0xeea9064b
pub fn build_delegate_calldata(operator: &str) -> String {
    // approverSignatureAndExpiry: struct { bytes signature; uint256 expiry }
    //   ABI: offset_to_struct(32) + salt(32) + struct_offset_inner(32) + expiry(32) + sig_offset(32) + sig_len(32)
    // Simplified: empty signature (bytes = ""), expiry = 0, salt = 0
    let op = pad_address(operator);
    // struct offset: 0x60 (after op + struct_offset + salt = 3 words)
    let struct_offset = pad_u256(0x60);
    let salt = pad_u256(0); // zero bytes32 approverSalt
    // inner struct: signature bytes offset = 0x40 (2 words), expiry = 0
    let sig_offset = pad_u256(0x40);
    let expiry = pad_u256(0);
    let sig_len = pad_u256(0); // empty bytes
    format!(
        "0xeea9064b{}{}{}{}{}{}",
        op, struct_offset, salt, sig_offset, expiry, sig_len
    )
}
