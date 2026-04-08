pub mod rates;
pub mod positions;
pub mod stake;
pub mod unstake;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

/// Derive an Associated Token Account (ATA) address.
/// ATA PDA = find_program_address([owner, token_program, mint], ATA_PROGRAM)
pub fn derive_ata(owner_b58: &str, mint_b58: &str) -> Result<Vec<u8>> {
    let owner = bs58_decode(owner_b58)?;
    let mint = bs58_decode(mint_b58)?;
    let token_program = bs58_decode("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
    let ata_program = bs58_decode("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bx8")?;

    find_program_address(&[&owner, &token_program, &mint], &ata_program)
}

/// Derive the withdraw authority PDA for the Jito stake pool.
/// PDA = find_program_address([pool_addr_bytes, b"withdraw"], STAKE_POOL_PROGRAM)
pub fn derive_withdraw_authority(pool_addr_b58: &str) -> Result<Vec<u8>> {
    let pool = bs58_decode(pool_addr_b58)?;
    let stake_pool_program = bs58_decode("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")?;

    find_program_address(&[&pool, b"withdraw"], &stake_pool_program)
}

fn bs58_decode(s: &str) -> Result<Vec<u8>> {
    bs58::decode(s)
        .into_vec()
        .map_err(|e| anyhow!("Invalid base58 address '{}': {}", s, e))
}

/// Solana find_program_address — iterates nonce 255..=0, returns first off-curve hash.
fn find_program_address(seeds: &[&[u8]], program_id: &[u8]) -> Result<Vec<u8>> {
    for nonce in (0u8..=255).rev() {
        let mut all_seeds: Vec<&[u8]> = seeds.to_vec();
        all_seeds.push(std::slice::from_ref(&nonce));

        let hash = create_program_address_hash(&all_seeds, program_id);
        if !is_on_ed25519_curve(&hash) {
            return Ok(hash.to_vec());
        }
    }
    Err(anyhow!("Could not find valid PDA for given seeds"))
}

/// Hash seeds + program_id + "ProgramDerivedAddress" using SHA256.
fn create_program_address_hash(seeds: &[&[u8]], program_id: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update(program_id);
    hasher.update(b"ProgramDerivedAddress");
    hasher.finalize().into()
}

/// Check if 32 bytes are a valid point on the Ed25519 curve.
///
/// Ed25519 equation: -x^2 + y^2 = 1 + d*x^2*y^2 (mod p)
/// A point is on-curve iff x^2 = (y^2 - 1) / (d*y^2 + 1) has a solution.
/// This holds iff the Legendre symbol of the numerator/denominator expression is 0 or 1.
///
/// p = 2^255 - 19
/// d = -121665 * modular_inverse(121666) mod p
///
/// Delegates to Python3 for 256-bit modular arithmetic.
/// Called at most 256 times per PDA derivation (nonce search), acceptable performance.
fn is_on_ed25519_curve(bytes: &[u8; 32]) -> bool {
    use std::process::Command;

    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

    // Python script: returns exit code 1 if on-curve, 0 if off-curve
    let script = r#"
import sys
p = 2**255 - 19
d = -121665 * pow(121666, p-2, p) % p
h = bytes.fromhex(sys.argv[1])
b = bytearray(h)
b[31] &= 0x7f
y = int.from_bytes(bytes(b), 'little')
if y >= p:
    sys.exit(0)
y2 = y * y % p
denom = (d * y2 + 1) % p
if denom == 0:
    sys.exit(1)
numer = (y2 - 1) % p
x2 = numer * pow(denom, p - 2, p) % p
if x2 == 0:
    sys.exit(1)
leg = pow(x2, (p - 1) // 2, p)
sys.exit(1 if leg == 1 else 0)
"#;

    match Command::new("python3").args(["-c", script, &hex]).output() {
        Ok(o) => o.status.code().unwrap_or(0) == 1,
        Err(_) => false, // treat as off-curve if Python unavailable
    }
}

/// Encode a Solana transaction to base64 for submission via onchainos.
pub fn encode_transaction_base64(tx_bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(tx_bytes)
}

/// Solana legacy transaction message layout
pub struct SolanaMessage {
    pub num_required_sigs: u8,
    pub num_readonly_signed: u8,
    pub num_readonly_unsigned: u8,
    pub account_keys: Vec<Vec<u8>>,
    pub recent_blockhash: Vec<u8>,
    pub instructions: Vec<SolanaInstruction>,
}

pub struct SolanaInstruction {
    pub program_id_index: u8,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

impl SolanaMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // Versioned transaction v0: prefix byte 0x80
        buf.push(0x80);
        buf.push(self.num_required_sigs);
        buf.push(self.num_readonly_signed);
        buf.push(self.num_readonly_unsigned);

        encode_compact_u16(&mut buf, self.account_keys.len() as u16);
        for key in &self.account_keys {
            buf.extend_from_slice(key);
        }

        buf.extend_from_slice(&self.recent_blockhash);

        encode_compact_u16(&mut buf, self.instructions.len() as u16);
        for ix in &self.instructions {
            buf.push(ix.program_id_index);
            encode_compact_u16(&mut buf, ix.account_indices.len() as u16);
            buf.extend_from_slice(&ix.account_indices);
            encode_compact_u16(&mut buf, ix.data.len() as u16);
            buf.extend_from_slice(&ix.data);
        }

        // v0: empty address table lookups (compact-u16 = 0)
        buf.push(0x00);

        buf
    }
}

/// Solana compact-u16 encoding
pub fn encode_compact_u16(buf: &mut Vec<u8>, val: u16) {
    if val <= 0x7f {
        buf.push(val as u8);
    } else if val <= 0x3fff {
        buf.push((val & 0x7f) as u8 | 0x80);
        buf.push(((val >> 7) & 0x7f) as u8);
    } else {
        buf.push((val & 0x7f) as u8 | 0x80);
        buf.push(((val >> 7) & 0x7f) as u8 | 0x80);
        buf.push(((val >> 14) & 0x03) as u8);
    }
}

/// Build an unsigned Solana legacy transaction (1 sig placeholder = 64 zero bytes).
pub fn build_unsigned_transaction(message_bytes: &[u8]) -> Vec<u8> {
    let mut tx = Vec::new();
    encode_compact_u16(&mut tx, 1);
    tx.extend_from_slice(&[0u8; 64]);
    tx.extend_from_slice(message_bytes);
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withdraw_authority_pda() {
        // Verified: withdraw authority = JitoSOL mint's mintAuthority on mainnet
        let result = derive_withdraw_authority("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb").unwrap();
        let addr = bs58::encode(&result).into_string();
        assert_eq!(
            addr, "6iQKfEyhr3bZMotVkW6beNZz5CPAkiwvgV2CTje9pVSS",
            "Withdraw authority PDA mismatch: got {}", addr
        );
    }
}
