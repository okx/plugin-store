use clap::Args;
use sha3::{Digest, Keccak256};
use crate::config::{CHAIN_ID, HYPER_EVM_RPC, info_url};
use crate::onchainos::{resolve_wallet, wallet_contract_call};
use crate::api::get_clearinghouse_state;
use crate::rpc::wait_tx_mined;

/// CoreWriter precompile on HyperEVM — executes HyperCore actions via msg.sender
const CORE_WRITER: &str = "0x3333333333333333333333333333333333333333";
/// USDC token index in HyperCore spot system (index 0)
const USDC_SPOT_TOKEN: u64 = 0;

#[derive(Args)]
pub struct EvmSendArgs {
    /// USDC amount to move from HyperCore perp → HyperEVM address
    #[arg(long)]
    pub amount: f64,

    /// Destination address on HyperEVM (defaults to your wallet address)
    #[arg(long)]
    pub to: Option<String>,

    /// Dry run — show calldata without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and execute both steps (requires --confirm)
    #[arg(long)]
    pub confirm: bool,
}

/// Encode a u64 as ABI uint256 (32 bytes, big-endian, left-zero-padded).
fn abi_u64(val: u64) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[24..32].copy_from_slice(&val.to_be_bytes());
    b
}

/// Encode an Ethereum address as ABI bytes32 (12 zero bytes + 20 address bytes).
fn abi_address(addr: &str) -> [u8; 32] {
    let hex = addr.trim_start_matches("0x");
    let decoded = hex::decode(hex).unwrap_or_default();
    let mut b = [0u8; 32];
    let offset = 32usize.saturating_sub(decoded.len());
    b[offset..offset + decoded.len().min(20)].copy_from_slice(&decoded[..decoded.len().min(20)]);
    b
}

/// Encode a bool as ABI bytes32 (0x00...00 or 0x00...01).
fn abi_bool(val: bool) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[31] = val as u8;
    b
}

/// Build calldata for CoreWriter.sendRawAction(bytes).
///
/// The data layout:
///   [0x01]             — encoding version
///   [3 bytes BE]       — action ID
///   [abi_params...]    — ABI-encoded action parameters
///
/// Wrapped as: sendRawAction(bytes) with standard ABI encoding.
fn core_writer_calldata(action_id: u32, abi_params: &[u8]) -> String {
    // Assemble the raw action payload
    let mut action_bytes: Vec<u8> = vec![
        0x01,
        ((action_id >> 16) & 0xff) as u8,
        ((action_id >> 8) & 0xff) as u8,
        (action_id & 0xff) as u8,
    ];
    action_bytes.extend_from_slice(abi_params);

    // sendRawAction(bytes) selector
    let mut h = Keccak256::new();
    h.update(b"sendRawAction(bytes)");
    let selector = h.finalize();

    // ABI-encode: selector[4] + offset(32) + length(32) + data(padded to 32n)
    let data_len = action_bytes.len();
    let padded_len = ((data_len + 31) / 32) * 32;

    let mut out = Vec::with_capacity(4 + 32 + 32 + padded_len);
    out.extend_from_slice(&selector[..4]);

    // offset = 32 (one dynamic argument, starts right after offset word)
    let mut offset_word = [0u8; 32];
    offset_word[31] = 32;
    out.extend_from_slice(&offset_word);

    // length
    let mut len_word = [0u8; 32];
    len_word[24..32].copy_from_slice(&(data_len as u64).to_be_bytes());
    out.extend_from_slice(&len_word);

    // data + right-padding
    out.extend_from_slice(&action_bytes);
    out.extend(std::iter::repeat(0u8).take(padded_len - data_len));

    format!("0x{}", hex::encode(out))
}

pub async fn run(args: EvmSendArgs) -> anyhow::Result<()> {
    if args.amount <= 0.0 {
        anyhow::bail!("--amount must be positive");
    }

    let usdc_units = (args.amount * 1_000_000.0).round() as u64;
    let wallet = resolve_wallet(CHAIN_ID)?;
    let destination = args.to.clone().unwrap_or_else(|| wallet.clone());

    // Check HyperCore perp withdrawable balance
    let state = get_clearinghouse_state(info_url(), &wallet).await?;
    let withdrawable: f64 = state["withdrawable"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    if args.amount > withdrawable {
        anyhow::bail!(
            "Insufficient perp balance: requested {:.4} USDC, withdrawable {:.4} USDC",
            args.amount, withdrawable
        );
    }

    // ── Build calldata ────────────────────────────────────────────────────
    //
    // Action 7 — USD class transfer: perp → spot
    //   params: (uint64 ntl, bool toPerp)  where toPerp=false means "to spot"
    let mut p7 = Vec::new();
    p7.extend_from_slice(&abi_u64(usdc_units));
    p7.extend_from_slice(&abi_bool(false));
    let calldata_perp_to_spot = core_writer_calldata(7, &p7);

    // Action 6 — Spot send: spot → HyperEVM address
    //   params: (address destination, uint64 token, uint64 wei)
    //   token = 0 (USDC is spot token index 0)
    let mut p6 = Vec::new();
    p6.extend_from_slice(&abi_address(&destination));
    p6.extend_from_slice(&abi_u64(USDC_SPOT_TOKEN));
    p6.extend_from_slice(&abi_u64(usdc_units));
    let calldata_spot_to_evm = core_writer_calldata(6, &p6);

    if args.dry_run || !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": !args.confirm,
            "wallet": wallet,
            "destination": destination,
            "amount_usdc": args.amount,
            "withdrawable": format!("{:.4}", withdrawable),
            "steps": [
                {
                    "step": 1,
                    "action": "perp → spot (CoreWriter Action 7)",
                    "to": CORE_WRITER,
                    "calldata": calldata_perp_to_spot
                },
                {
                    "step": 2,
                    "action": "spot → HyperEVM (CoreWriter Action 6)",
                    "to": CORE_WRITER,
                    "calldata": calldata_spot_to_evm
                }
            ],
            "note": if args.confirm { "" } else { "Add --confirm to execute" }
        }));
        return Ok(());
    }

    // ── Execute ───────────────────────────────────────────────────────────

    // Step 1: Move perp → spot
    eprintln!("Step 1/2  Transferring {} USDC from perp → spot via CoreWriter...", args.amount);
    let result1 = wallet_contract_call(CHAIN_ID, CORE_WRITER, &calldata_perp_to_spot, None, false)?;

    // Wait for step 1 to be mined before submitting step 2 (HyperCore needs the tx on-chain)
    eprintln!("  Waiting for HyperCore to process...");
    let tx1_hash = result1["data"]["txHash"].as_str().unwrap_or("");
    if !tx1_hash.is_empty() {
        let confirmed = wait_tx_mined(tx1_hash, HYPER_EVM_RPC).await;
        if !confirmed {
            eprintln!("  Warning: step 1 tx confirmation timed out. Proceeding with step 2.");
        }
    }

    // Step 2: Spot → HyperEVM address
    eprintln!("Step 2/2  Sending {} USDC from spot → HyperEVM {}...", args.amount, &destination[..10]);
    wallet_contract_call(CHAIN_ID, CORE_WRITER, &calldata_spot_to_evm, None, false)?;

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "evm-send",
        "wallet": wallet,
        "destination": destination,
        "amount_usdc": args.amount,
        "note": "USDC sent to HyperEVM. Verify with 'hyperliquid address'."
    }));

    Ok(())
}
