/// Solana chain ID
pub const SOLANA_CHAIN_ID: u64 = 501;

/// SPL Stake Pool Program ID
pub const STAKE_POOL_PROGRAM: &str = "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy";

/// Jito Stake Pool Account (mainnet)
pub const JITO_STAKE_POOL: &str = "Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb";

/// JitoSOL token mint
pub const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";

/// SPL Associated Token Account Program
#[allow(dead_code)]
pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bx8";

/// SPL Token Program
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// System Program
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

/// Stake Program
#[allow(dead_code)]
pub const STAKE_PROGRAM: &str = "Stake11111111111111111111111111111111111111111";

/// Sysvar Clock
#[allow(dead_code)]
pub const SYSVAR_CLOCK: &str = "SysvarC1ock11111111111111111111111111111111111";

/// Solana mainnet RPC endpoint
pub const SOLANA_RPC: &str = "https://api.mainnet-beta.solana.com";

/// Lamports per SOL
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Parse a human-readable token amount string into raw integer units.
/// E.g. parse_units("1.5", 18) == 1_500_000_000_000_000_000
pub fn parse_units(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    if s.is_empty() {
        anyhow::bail!("Empty amount string");
    }
    let d = decimals as u32;
    let multiplier = 10u128.pow(d);
    if let Some(dot_pos) = s.find('.') {
        let whole: u128 = s[..dot_pos].parse().map_err(|_| anyhow::anyhow!("Invalid whole part in: {}", s))?;
        let frac_str = &s[dot_pos + 1..];
        let frac_len = frac_str.len() as u32;
        let frac: u128 = frac_str.parse().map_err(|_| anyhow::anyhow!("Invalid fractional part in: {}", s))?;
        if frac_len > d {
            anyhow::bail!("Too many decimal places (max {})", d);
        }
        let frac_scaled = frac * 10u128.pow(d - frac_len);
        Ok(whole * multiplier + frac_scaled)
    } else {
        let whole: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid integer amount: {}", s))?;
        Ok(whole * multiplier)
    }
}
