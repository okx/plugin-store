/// Ethereum mainnet chain ID.
pub const CHAIN_ID: u64 = 1;

/// Puffer PufferVault (pufETH, ERC-4626) on Ethereum mainnet.
/// Accepts ETH via `depositETH(receiver)` and WETH via `deposit(assets, receiver)`.
/// Exit paths: `withdraw()` / `redeem()` apply an exit fee (see `getTotalExitFeeBasisPoints`).
pub fn puffer_vault_address() -> &'static str {
    "0xD9A442856C234a39a81a089C06451EBAa4306a72"
}

/// Alias for pufETH ERC-20 token (same contract as PufferVault).
pub fn pufeth_address() -> &'static str {
    puffer_vault_address()
}

/// PufferWithdrawalManager — 2-step queued withdraw path (no fee, ~14 days).
/// `requestWithdrawal(uint128 pufETHAmount, address recipient)` → `completeQueuedWithdrawal(uint256 idx)`.
pub fn withdrawal_manager_address() -> &'static str {
    "0xDdA0483184E75a5579ef9635ED14BacCf9d50283"
}

/// PufferDepositor — one-step adapter to mint pufETH from stETH / wstETH with permit.
/// Reserved for v0.2.x (wstETH / stETH deposit commands). Not wired to a command yet.
#[allow(dead_code)]
pub fn puffer_depositor_address() -> &'static str {
    "0x4aa799C5dfc01ee7d790e3bf1a7C2257CE1DcefF"
}

/// WETH on Ethereum mainnet (returned by both exit paths).
pub fn weth_address() -> &'static str {
    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
}

/// Ethereum mainnet public RPC endpoint.
pub fn rpc_url() -> &'static str {
    "https://ethereum-rpc.publicnode.com"
}

/// Minimum withdrawal amount enforced by PufferWithdrawalManager (0.01 pufETH in wei).
pub const MIN_WITHDRAWAL_AMOUNT_WEI: u128 = 10_000_000_000_000_000; // 0.01 * 1e18

/// Batch size used by PufferWithdrawalManager; needed to compute batch index from withdrawal index.
pub const WITHDRAWAL_BATCH_SIZE: u64 = 10;

/// Parse a decimal string amount into the raw u128 integer in smallest units.
/// Uses only integer arithmetic — no f64.
///
/// Examples:
///   parse_units("1.5", 18) = 1_500_000_000_000_000_000
///   parse_units("0.01", 6)  = 10_000
///   parse_units("100", 18)  = 100_000_000_000_000_000_000
pub fn parse_units(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    let (integer_part, frac_part) = if let Some(dot_pos) = s.find('.') {
        let int_s = &s[..dot_pos];
        let frac_s = &s[dot_pos + 1..];
        (int_s, frac_s)
    } else {
        (s, "")
    };

    let int_val: u128 = if integer_part.is_empty() {
        0
    } else {
        integer_part
            .parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid integer part in amount: {}", amount_str))?
    };

    let scale: u128 = 10u128
        .checked_pow(decimals as u32)
        .ok_or_else(|| anyhow::anyhow!("Decimals too large: {}", decimals))?;

    let int_wei = int_val
        .checked_mul(scale)
        .ok_or_else(|| anyhow::anyhow!("Overflow in integer part of amount: {}", amount_str))?;

    let frac_wei = if frac_part.is_empty() {
        0u128
    } else {
        let frac_len = frac_part.len() as u32;
        if frac_len > decimals as u32 {
            let truncated = &frac_part[..decimals as usize];
            truncated
                .parse::<u128>()
                .map_err(|_| anyhow::anyhow!("Invalid fractional part in amount: {}", amount_str))?
        } else {
            let frac_val: u128 = frac_part
                .parse::<u128>()
                .map_err(|_| anyhow::anyhow!("Invalid fractional part in amount: {}", amount_str))?;
            let remaining = decimals as u32 - frac_len;
            let frac_scale: u128 = 10u128
                .checked_pow(remaining)
                .ok_or_else(|| anyhow::anyhow!("Decimals too large: {}", remaining))?;
            frac_val
                .checked_mul(frac_scale)
                .ok_or_else(|| anyhow::anyhow!("Overflow in fractional part: {}", amount_str))?
        }
    };

    int_wei
        .checked_add(frac_wei)
        .ok_or_else(|| anyhow::anyhow!("Overflow combining integer and fractional: {}", amount_str))
}

/// Format a wei u128 value as a human-readable string with `decimals` decimal places.
/// Trims trailing zeros after the decimal point.
pub fn format_units(wei: u128, decimals: u8) -> String {
    let scale: u128 = 10u128.pow(decimals as u32);
    let int_part = wei / scale;
    let frac_part = wei % scale;
    if frac_part == 0 {
        return format!("{}", int_part);
    }
    let frac_str = format!("{:0>width$}", frac_part, width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    format!("{}.{}", int_part, trimmed)
}

/// Pad an address to 32 bytes (no 0x prefix in output).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 value to 32 bytes hex.
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}
