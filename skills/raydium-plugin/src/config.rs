#[allow(dead_code)]
pub const SOLANA_CHAIN_ID: &str = "501";
#[allow(dead_code)]
pub const SOL_NATIVE_MINT: &str = "So11111111111111111111111111111111111111112";
/// Native SOL system program address — treated as SOL with 9 decimals; rewritten to WSOL for API calls.
#[allow(dead_code)]
pub const SOL_SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
#[allow(dead_code)]
pub const USDC_SOLANA: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

pub const DATA_API_BASE: &str = "https://api-v3.raydium.io";
pub const TX_API_BASE: &str = "https://transaction-v1.raydium.io";
pub const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

// Raydium AMM V4 program (standard pools — used as --to for onchainos contract-call)
pub const RAYDIUM_AMM_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

// Default compute unit price in micro-lamports (avoid "auto" which the API rejects)
pub const DEFAULT_COMPUTE_UNIT_PRICE: &str = "1000";

pub const DEFAULT_SLIPPAGE_BPS: u32 = 50;
pub const DEFAULT_TX_VERSION: &str = "V0";

pub const PRICE_IMPACT_WARN_PCT: f64 = 5.0;
pub const PRICE_IMPACT_BLOCK_PCT: f64 = 20.0;

/// Parse a human-readable decimal amount string into raw token units (u64 for Solana).
///
/// Examples:
///   parse_human_amount("1",     9) -> 1_000_000_000  (1 SOL)
///   parse_human_amount("0.1",   9) -> 100_000_000    (0.1 SOL)
///   parse_human_amount("1.5",   6) -> 1_500_000      (1.5 USDC)
pub fn parse_human_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u64> {
    let s = amount_str.trim();
    let factor = 10u64.pow(decimals as u32);
    if let Some(dot_pos) = s.find('.') {
        let int_part: u64 = if dot_pos == 0 {
            0
        } else {
            s[..dot_pos]
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_str = &s[dot_pos + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!(
                "Amount '{}' has {} decimal places but token only supports {}",
                s,
                frac_str.len(),
                decimals
            );
        }
        let frac: u64 = if frac_str.is_empty() {
            0
        } else {
            frac_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_factor = 10u64.pow(decimals as u32 - frac_str.len() as u32);
        Ok(int_part * factor + frac * frac_factor)
    } else {
        let int_val: u64 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        Ok(int_val * factor)
    }
}

/// Validate a Solana mint/wallet address: base58, 32-44 chars, no 0/O/I/l.
pub fn validate_solana_address(addr: &str) -> anyhow::Result<()> {
    let len = addr.len();
    if len < 32 || len > 44 {
        anyhow::bail!("Invalid Solana address '{}': expected 32-44 chars, got {}", addr, len);
    }
    let invalid = addr.chars().find(|c| {
        !matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z')
    });
    if let Some(c) = invalid {
        anyhow::bail!("Invalid Solana address '{}': contains invalid base58 character '{}'", addr, c);
    }
    Ok(())
}
