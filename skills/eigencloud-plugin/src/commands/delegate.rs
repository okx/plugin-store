use clap::Args;
use crate::abi::{selector, encode_address, zero32, calldata};
use crate::chain::{CHAIN_ID, eth_call, decode_word, decode_address};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};

const DELEGATION_MANAGER: &str = "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A";

#[derive(Args)]
pub struct DelegateArgs {
    /// Operator address to delegate to
    #[arg(long)]
    pub operator: String,
    /// Broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
    /// Dry-run: build calldata without calling onchainos
    #[arg(long, conflicts_with = "confirm")]
    pub dry_run: bool,
}

pub async fn run(args: DelegateArgs) -> anyhow::Result<()> {
    // Validate operator address format
    if !args.operator.starts_with("0x") || args.operator.len() != 42
        || !args.operator[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        anyhow::bail!("Invalid operator address '{}': must be a 42-character hex address (0x...)", args.operator);
    }
    // N3: Reject the zero address — it is never a registered operator
    if args.operator[2..].chars().all(|c| c == '0') {
        anyhow::bail!("Invalid operator address: 0x000...000 is not a registered EigenLayer operator");
    }

    let wallet = if args.dry_run {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    } else if args.confirm {
        resolve_wallet(CHAIN_ID)?
    } else {
        resolve_wallet(CHAIN_ID).unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
    };

    // Check current delegation status before preview or execution.
    // Skip for --dry-run: calldata inspection should work regardless of on-chain state.
    if !args.dry_run {
        let current_operator = {
            let delegated_sel = selector("delegatedTo(address)");
            let check_data = calldata(delegated_sel, &[encode_address(&wallet)]);
            eth_call(DELEGATION_MANAGER, &check_data).await
                .ok()
                .and_then(|r| decode_word(&r, 0))
                .map(|w| decode_address(&w))
                .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
        };
        let is_already_delegated = current_operator != "0x0000000000000000000000000000000000000000";
        if is_already_delegated {
            anyhow::bail!(
                "Already delegated to operator {}. Run `eigencloud undelegate --confirm` first, then wait 7 days before re-delegating.",
                current_operator
            );
        }
    }

    // Build delegateTo calldata.
    // Signature: delegateTo(address operator, (bytes signature, uint256 expiry), bytes32 salt)
    // For operators with no approver (most public operators), pass empty signature and expiry=0.
    //
    // ABI encoding:
    //   slot 0: operator (address, static)
    //   slot 1: offset to (bytes, uint256) tuple = 0x60 (3 slots from data start)
    //   slot 2: salt (bytes32) = 0x00...
    //   slot 3: offset to `bytes` within tuple = 0x40 (after 2 words: this offset + expiry)
    //   slot 4: expiry = 0
    //   slot 5: length of bytes = 0
    let sel = selector("delegateTo(address,(bytes,uint256),bytes32)");
    let op_slot = encode_address(&args.operator);
    let tuple_offset = {
        let mut s = [0u8; 32];
        s[31] = 0x60; // 3 * 32 = 96 bytes from data start
        s
    };
    let salt = zero32();
    let bytes_offset_in_tuple = {
        let mut s = [0u8; 32];
        s[31] = 0x40; // 2 * 32 = 64 bytes from tuple start
        s
    };
    let expiry = zero32();
    let bytes_length = zero32(); // empty signature

    let mut data = sel.to_vec();
    data.extend_from_slice(&op_slot);
    data.extend_from_slice(&tuple_offset);
    data.extend_from_slice(&salt);
    data.extend_from_slice(&bytes_offset_in_tuple);
    data.extend_from_slice(&expiry);
    data.extend_from_slice(&bytes_length);
    let calldata = format!("0x{}", hex::encode(data));

    let preview = serde_json::json!({
        "preview":           !args.confirm && !args.dry_run,
        "action":            "delegate",
        "operator":          args.operator,
        "delegation_manager": DELEGATION_MANAGER,
        "wallet":            wallet,
        "note":              "Delegates all current and future restaked positions to the specified operator.",
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("\nAdd --confirm to broadcast this delegation transaction.");
        return Ok(());
    }

    eprintln!("[eigencloud] Delegating to operator {}...", args.operator);
    let result = wallet_contract_call(CHAIN_ID, DELEGATION_MANAGER, &calldata, "0", args.dry_run, Some(&wallet))?;
    let tx_hash = extract_tx_hash(&result);
    eprintln!("[eigencloud] delegate tx: {}", tx_hash);

    let mut out = serde_json::json!({
        "ok":       true,
        "action":   "delegate",
        "operator": args.operator,
        "wallet":   wallet,
        "tx_hash":  tx_hash,
    });
    if args.dry_run { out["dry_run"] = serde_json::json!(true); }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
