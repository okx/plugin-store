use solana_hash::Hash;
use solana_instruction::{AccountMeta, Instruction};
use solana_message::Message;
use solana_pubkey::Pubkey;

// ── Program / sysvar IDs ─────────────────────────────────────────────────────

pub const DLMM_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");

const TOKEN_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const SYSTEM_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("11111111111111111111111111111111");

const RENT_SYSVAR: Pubkey =
    solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111");

const ATA_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe8bXh");

// ── PDA helpers ──────────────────────────────────────────────────────────────

/// PDA for logging events: seeds=["__event_authority"]
pub fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &DLMM_PROGRAM).0
}

/// Position PDA: seeds=["position", lb_pair, base, lower_bin_id_le4, width_le4]
pub fn position_pda(lb_pair: &Pubkey, base: &Pubkey, lower_bin_id: i32, width: i32) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"position",
            lb_pair.as_ref(),
            base.as_ref(),
            &lower_bin_id.to_le_bytes(),
            &width.to_le_bytes(),
        ],
        &DLMM_PROGRAM,
    )
    .0
}

/// Bin array PDA: seeds=["bin_array", lb_pair, index_le8]
pub fn bin_array_pda(lb_pair: &Pubkey, index: i64) -> Pubkey {
    Pubkey::find_program_address(
        &[b"bin_array", lb_pair.as_ref(), &index.to_le_bytes()],
        &DLMM_PROGRAM,
    )
    .0
}

/// Python-style floor division for bin array index (handles negative bin IDs)
pub fn bin_array_index(bin_id: i32) -> i64 {
    (bin_id as f64 / 70.0).floor() as i64
}

/// Associated token account address
pub fn get_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), TOKEN_PROGRAM.as_ref(), mint.as_ref()],
        &ATA_PROGRAM,
    )
    .0
}

/// Build `create_idempotent` ATA instruction (no-op if ATA already exists).
pub fn ix_create_ata_idempotent(
    payer: &Pubkey,
    ata: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: ATA_PROGRAM,
        accounts: vec![
            AccountMeta::new(*payer, true),          // payer (writable signer)
            AccountMeta::new(*ata, false),            // ATA (writable)
            AccountMeta::new_readonly(*wallet, false),// wallet
            AccountMeta::new_readonly(*mint, false),  // mint
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        ],
        data: vec![1u8], // instruction variant 1 = CreateIdempotent
    }
}

// ── Instruction builders ─────────────────────────────────────────────────────

/// Build `initialize_bin_array` instruction.
///
/// Must be called before `add_liquidity_by_strategy` if the bin array account
/// for the given index does not yet exist on-chain.
pub fn ix_initialize_bin_array(
    lb_pair: &Pubkey,
    bin_array: &Pubkey,
    funder: &Pubkey,
    index: i64,
) -> Instruction {
    // discriminator = sha256("global:initialize_bin_array")[:8]
    let discriminator: [u8; 8] = [35, 86, 19, 185, 78, 212, 75, 211];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&index.to_le_bytes());

    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*lb_pair, false),
            AccountMeta::new(*bin_array, false),
            AccountMeta::new(*funder, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

/// Build `initialize_position_pda` instruction.
///
/// The wallet serves as payer, base, AND owner (three signer roles,
/// one key → Solana deduplicates, requires only one signature).
pub fn ix_initialize_position_pda(
    wallet: &Pubkey,
    lb_pair: &Pubkey,
    position: &Pubkey,
    lower_bin_id: i32,
    width: i32,
) -> Instruction {
    let discriminator: [u8; 8] = [46, 82, 125, 146, 85, 141, 228, 153];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&lower_bin_id.to_le_bytes());
    data.extend_from_slice(&width.to_le_bytes());

    let ev_auth = event_authority();

    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*wallet, true),             // payer (writable signer)
            AccountMeta::new_readonly(*wallet, true),    // base (signer)
            AccountMeta::new(*position, false),          // position (writable PDA)
            AccountMeta::new_readonly(*lb_pair, false),  // lb_pair
            AccountMeta::new_readonly(*wallet, true),    // owner (signer)
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(RENT_SYSVAR, false),
            AccountMeta::new_readonly(ev_auth, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false), // program self-ref
        ],
        data,
    }
}

/// Borsh-serialize `LiquidityParameterByStrategy` with `SpotBalanced` strategy.
///
/// Layout (all LE):
///   amount_x          u64   8 bytes
///   amount_y          u64   8 bytes
///   active_id         i32   4 bytes
///   max_active_bin_slippage i32 4 bytes
///   StrategyParameters:
///     min_bin_id      i32   4 bytes
///     max_bin_id      i32   4 bytes
///     strategy_type   u8    1 byte  (SpotBalanced = 3)
///     parameteres     [u8;64] 64 bytes (zeroed)
fn serialize_liquidity_params(
    amount_x: u64,
    amount_y: u64,
    active_id: i32,
    max_active_bin_slippage: i32,
    min_bin_id: i32,
    max_bin_id: i32,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(97);
    data.extend_from_slice(&amount_x.to_le_bytes());
    data.extend_from_slice(&amount_y.to_le_bytes());
    data.extend_from_slice(&active_id.to_le_bytes());
    data.extend_from_slice(&max_active_bin_slippage.to_le_bytes());
    data.extend_from_slice(&min_bin_id.to_le_bytes());
    data.extend_from_slice(&max_bin_id.to_le_bytes());
    data.push(3u8); // StrategyType::SpotBalanced
    data.extend_from_slice(&[0u8; 64]); // parameteres (unused for SpotBalanced)
    data
}

/// Build `add_liquidity_by_strategy` instruction.
#[allow(clippy::too_many_arguments)]
pub fn ix_add_liquidity_by_strategy(
    position: &Pubkey,
    lb_pair: &Pubkey,
    user_token_x: &Pubkey,
    user_token_y: &Pubkey,
    reserve_x: &Pubkey,
    reserve_y: &Pubkey,
    token_x_mint: &Pubkey,
    token_y_mint: &Pubkey,
    bin_array_lower: &Pubkey,
    bin_array_upper: &Pubkey,
    sender: &Pubkey,
    amount_x: u64,
    amount_y: u64,
    active_id: i32,
    max_active_bin_slippage: i32,
    min_bin_id: i32,
    max_bin_id: i32,
) -> Instruction {
    let discriminator: [u8; 8] = [7, 3, 150, 127, 148, 40, 61, 200];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&serialize_liquidity_params(
        amount_x,
        amount_y,
        active_id,
        max_active_bin_slippage,
        min_bin_id,
        max_bin_id,
    ));

    let ev_auth = event_authority();

    // bin_array_bitmap_extension is OPTIONAL; pass DLMM_PROGRAM as sentinel (unused).
    // Must be readonly — passing an executable account as writable causes ProgramAccountNotFound.
    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*position, false),
            AccountMeta::new(*lb_pair, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false), // bin_array_bitmap_extension (optional sentinel)
            AccountMeta::new(*user_token_x, false),
            AccountMeta::new(*user_token_y, false),
            AccountMeta::new(*reserve_x, false),
            AccountMeta::new(*reserve_y, false),
            AccountMeta::new_readonly(*token_x_mint, false),
            AccountMeta::new_readonly(*token_y_mint, false),
            AccountMeta::new(*bin_array_lower, false),
            AccountMeta::new(*bin_array_upper, false),
            AccountMeta::new_readonly(*sender, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false), // token_x_program
            AccountMeta::new_readonly(TOKEN_PROGRAM, false), // token_y_program
            AccountMeta::new_readonly(ev_auth, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false), // program self-ref
        ],
        data,
    }
}

/// Build `remove_liquidity_by_range` instruction.
/// `bps_to_remove`: 10000 = 100% (remove all liquidity).
#[allow(clippy::too_many_arguments)]
pub fn ix_remove_liquidity_by_range(
    position: &Pubkey,
    lb_pair: &Pubkey,
    user_token_x: &Pubkey,
    user_token_y: &Pubkey,
    reserve_x: &Pubkey,
    reserve_y: &Pubkey,
    token_x_mint: &Pubkey,
    token_y_mint: &Pubkey,
    bin_array_lower: &Pubkey,
    bin_array_upper: &Pubkey,
    sender: &Pubkey,
    from_bin_id: i32,
    to_bin_id: i32,
    bps_to_remove: u16,
) -> Instruction {
    // sha256("global:remove_liquidity_by_range")[:8]
    let discriminator: [u8; 8] = [26, 82, 102, 152, 240, 74, 105, 26];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&from_bin_id.to_le_bytes());
    data.extend_from_slice(&to_bin_id.to_le_bytes());
    data.extend_from_slice(&bps_to_remove.to_le_bytes());

    let ev_auth = event_authority();

    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*position, false),
            AccountMeta::new(*lb_pair, false),
            AccountMeta::new(DLMM_PROGRAM, false), // bin_array_bitmap_extension (optional sentinel)
            AccountMeta::new(*user_token_x, false),
            AccountMeta::new(*user_token_y, false),
            AccountMeta::new(*reserve_x, false),
            AccountMeta::new(*reserve_y, false),
            AccountMeta::new_readonly(*token_x_mint, false),
            AccountMeta::new_readonly(*token_y_mint, false),
            AccountMeta::new(*bin_array_lower, false),
            AccountMeta::new(*bin_array_upper, false),
            AccountMeta::new_readonly(*sender, true),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false), // token_x_program
            AccountMeta::new_readonly(TOKEN_PROGRAM, false), // token_y_program
            AccountMeta::new_readonly(ev_auth, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false), // program self-ref
        ],
        data,
    }
}

/// Build `claim_fee` instruction.
/// Claims accumulated trading fees for a position into the owner's token accounts.
/// Account order from IDL: lb_pair, position, bin_array_lower, bin_array_upper,
///   sender(signer), reserve_x, reserve_y, user_token_x, user_token_y,
///   token_x_mint, token_y_mint, token_program, event_authority, program
#[allow(clippy::too_many_arguments)]
pub fn ix_claim_fee(
    lb_pair: &Pubkey,
    position: &Pubkey,
    bin_array_lower: &Pubkey,
    bin_array_upper: &Pubkey,
    sender: &Pubkey,
    reserve_x: &Pubkey,
    reserve_y: &Pubkey,
    user_token_x: &Pubkey,
    user_token_y: &Pubkey,
    token_x_mint: &Pubkey,
    token_y_mint: &Pubkey,
) -> Instruction {
    // sha256("global:claim_fee")[:8]
    let discriminator: [u8; 8] = [169, 32, 79, 137, 136, 232, 70, 137];
    let ev_auth = event_authority();

    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*lb_pair, false),
            AccountMeta::new(*position, false),
            AccountMeta::new(*bin_array_lower, false),
            AccountMeta::new(*bin_array_upper, false),
            AccountMeta::new_readonly(*sender, true),       // sender (signer, not writable per IDL)
            AccountMeta::new(*reserve_x, false),
            AccountMeta::new(*reserve_y, false),
            AccountMeta::new(*user_token_x, false),
            AccountMeta::new(*user_token_y, false),
            AccountMeta::new_readonly(*token_x_mint, false),
            AccountMeta::new_readonly(*token_y_mint, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(ev_auth, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build `close_position_if_empty` instruction.
/// Closes a position account that has no liquidity and no pending fees.
/// Simpler than `close_position`: does not require bin array accounts.
/// Returns rent (~0.057 SOL) to rent_receiver.
pub fn ix_close_position_if_empty(
    sender: &Pubkey,
    position: &Pubkey,
) -> Instruction {
    // sha256("global:close_position_if_empty")[:8]
    let discriminator: [u8; 8] = [59, 124, 212, 118, 91, 152, 110, 157];
    let ev_auth = event_authority();

    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*position, false),          // position (writable)
            AccountMeta::new_readonly(*sender, true),    // sender (signer)
            AccountMeta::new(*sender, false),            // rent_receiver (writable)
            AccountMeta::new_readonly(ev_auth, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false),
        ],
        data: discriminator.to_vec(),
    }
}

/// Build `close_position` instruction.
/// Must be called after removing ALL liquidity (bps=10000).
/// Closes the position account and returns rent (~0.057 SOL) to the sender.
///
/// Account order (matches Meteora DLMM IDL):
///   position, lb_pair, bin_array_lower, bin_array_upper, owner(signer),
///   rent_receiver(writable), event_authority, program
pub fn ix_close_position(
    sender: &Pubkey,
    position: &Pubkey,
    lb_pair: &Pubkey,
    bin_array_lower: &Pubkey,
    bin_array_upper: &Pubkey,
) -> Instruction {
    // sha256("global:close_position")[:8]
    let discriminator: [u8; 8] = [123, 134, 81, 0, 49, 68, 98, 98];
    let ev_auth = event_authority();

    Instruction {
        program_id: DLMM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*position, false),          // position (writable)
            AccountMeta::new(*lb_pair, false),           // lb_pair (writable)
            AccountMeta::new(*bin_array_lower, false),   // bin_array_lower (writable)
            AccountMeta::new(*bin_array_upper, false),   // bin_array_upper (writable)
            AccountMeta::new_readonly(*sender, true),    // owner (signer)
            AccountMeta::new(*sender, false),            // rent_receiver (writable)
            AccountMeta::new_readonly(ev_auth, false),
            AccountMeta::new_readonly(DLMM_PROGRAM, false), // program self-ref
        ],
        data: discriminator.to_vec(),
    }
}

/// Build `SetComputeUnitLimit` instruction (ComputeBudget program).
/// Call this as the FIRST instruction in any transaction that may exceed the
/// default 200k CU limit (e.g. remove_liquidity_by_range over wide bin ranges).
pub fn ix_set_compute_unit_limit(units: u32) -> Instruction {
    const COMPUTE_BUDGET: Pubkey =
        solana_pubkey::pubkey!("ComputeBudget111111111111111111111111111111");
    let mut data = vec![0x02u8]; // SetComputeUnitLimit discriminator (compact wire format: 0x02 = SetComputeUnitLimit, 0x03 = SetComputeUnitPrice)
    data.extend_from_slice(&units.to_le_bytes());
    Instruction {
        program_id: COMPUTE_BUDGET,
        accounts: vec![],
        data,
    }
}

/// Transfer native SOL from `from` to `to` via the System program.
/// Used to fund a WSOL ATA before syncing, ensuring the token balance reflects
/// the deposited SOL (required when token_x is the native SOL mint).
pub fn ix_sol_transfer(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    // SystemInstruction::Transfer discriminant = 2 (u32 LE) + lamports (u64 LE)
    let mut data = vec![2u8, 0, 0, 0];
    data.extend_from_slice(&lamports.to_le_bytes());
    Instruction {
        program_id: SYSTEM_PROGRAM,
        accounts: vec![
            AccountMeta::new(*from, true),
            AccountMeta::new(*to, false),
        ],
        data,
    }
}

/// Sync a WSOL (native SOL) token account so its token balance matches its lamport balance.
/// Must be called after `ix_sol_transfer` to make newly deposited SOL spendable as tokens.
pub fn ix_sync_native(wsol_account: &Pubkey) -> Instruction {
    // SyncNative SPL token instruction discriminant = 17
    Instruction {
        program_id: TOKEN_PROGRAM,
        accounts: vec![AccountMeta::new(*wsol_account, false)],
        data: vec![17],
    }
}

// ── Transaction serialization ─────────────────────────────────────────────────

/// Build a Legacy Transaction with placeholder signatures, serialize to base58.
/// `onchainos wallet contract-call --chain 501 --unsigned-tx <result>` will sign and broadcast.
pub fn build_tx_b58(
    instructions: &[Instruction],
    payer: &Pubkey,
    blockhash_bytes: [u8; 32],
) -> anyhow::Result<String> {
    let blockhash = Hash::new_from_array(blockhash_bytes);
    let msg = Message::new_with_blockhash(instructions, Some(payer), &blockhash);

    let num_sigs = msg.header.num_required_signatures as usize;

    // Manually serialize: compact_u16(num_sigs) + num_sigs*[0u8;64] + bincode(message)
    // This matches the Solana wire format for an unsigned legacy transaction.
    let mut tx_bytes = Vec::new();
    encode_compact_u16(num_sigs as u16, &mut tx_bytes);
    for _ in 0..num_sigs {
        tx_bytes.extend_from_slice(&[0u8; 64]);
    }
    tx_bytes.extend_from_slice(&bincode::serialize(&msg)?);

    Ok(bs58::encode(&tx_bytes).into_string())
}

/// Solana compact_u16 encoding (1-3 bytes, LSB first, high bit = more bytes).
fn encode_compact_u16(mut val: u16, out: &mut Vec<u8>) {
    loop {
        let byte = (val & 0x7f) as u8;
        val >>= 7;
        if val == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}
