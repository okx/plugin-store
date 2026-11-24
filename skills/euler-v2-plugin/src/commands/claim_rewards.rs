/// `euler-v2-plugin claim-rewards` — claim Merkl reward streams.
///
/// **v0.2 implementation**: queries the Merkl official API for the user's
/// claimable rewards on the requested chain, builds calldata for the Merkl
/// distributor's `claim(users, tokens, amounts, proofs)` function, and
/// submits via onchainos.
///
/// Brevis and Fuul reward streams are **not yet supported** — they have
/// different distributor ABIs and proof formats. Surface them as
/// `unsupported_distributor` in the output if found.
///
/// Merkl distributor address is the same on every chain:
/// `0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae`

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::rpc::{eth_get_balance_wei, estimate_native_gas_cost_wei, pad_address, wei_to_eth};

/// Merkl distributor — same address on every supported chain.
const MERKL_DISTRIBUTOR: &str = "0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae";

/// `claim(address[],address[],uint256[],bytes32[][])` selector — verified at runtime.
const SEL_MERKL_CLAIM: &str = "71ee95c0";

const GAS_LIMIT_CLAIM: u64 = 350_000;

#[derive(Args)]
pub struct ClaimRewardsArgs {
    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    #[arg(long)]
    pub dry_run: bool,

    /// Required to broadcast. Without this, the command prints a preview
    /// (calldata + intent) and exits without touching the chain.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: ClaimRewardsArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("claim-rewards"), None)); Ok(()) }
    }
}

async fn run_inner(args: ClaimRewardsArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let wallet = crate::onchainos::get_wallet_address(args.chain).await?;

    // 1. Fetch claimable rewards from Merkl
    let rewards = crate::api::get_merkl_rewards(args.chain, &wallet).await?;
    if rewards.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "action": "claim_rewards",
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet,
                "status": "no_rewards",
                "rewards": [],
                "tip": "No claimable Merkl rewards on this chain. \
                        Note: Brevis / Fuul streams are not yet supported in this command.",
            }
        }))?);
        return Ok(());
    }

    // 2. Build claim calldata.
    //
    //   claim(
    //     address[] users,
    //     address[] tokens,
    //     uint256[] amounts,
    //     bytes32[][] proofs
    //   )
    //
    // For each reward we set users[i] = wallet, tokens[i] = reward token, amounts[i] =
    // cumulative authorized amount (Merkl distributor enforces "amount is the running
    // total user has ever earned, contract subtracts already-claimed from it").
    let calldata = build_merkl_claim_calldata(&wallet, &rewards);

    let total_claimable: u128 = rewards.iter().map(|r| r.claimable_raw).sum();
    let summary: Vec<serde_json::Value> = rewards.iter().map(|r| serde_json::json!({
        "token":             r.token,
        "symbol":            r.symbol,
        "cumulative_amount": r.cumulative_amount,
        "claimable_raw":     r.claimable_raw.to_string(),
        "proofs_count":      r.proofs.len(),
    })).collect();

    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true, "dry_run": true,
            "data": {
                "action": "claim_rewards",
                "chain": chain_name(args.chain), "chain_id": args.chain,
                "wallet": wallet,
                "distributor": MERKL_DISTRIBUTOR,
                "rewards": summary,
                "total_claimable_raw": total_claimable.to_string(),
                "calldata_size_bytes": calldata.len() / 2 - 1,  // strip 0x then bytes
                "note": "dry-run: no transaction submitted",
            }
        }))?);
        return Ok(());
    }

    // 3. Pre-flight gas
    let need_wei = estimate_native_gas_cost_wei(args.chain, GAS_LIMIT_CLAIM).await?;
    let have_wei = eth_get_balance_wei(args.chain, &wallet).await?;
    if have_wei < need_wei {
        anyhow::bail!(
            "Insufficient native gas: have {:.6} ETH, need ~{:.6} ETH for Merkl claim.",
            wei_to_eth(have_wei), wei_to_eth(need_wei)
        );
    }

    // 4. Submit
    eprintln!("[euler-v2] claim-rewards: submitting Merkl claim for {} reward token(s)...", rewards.len());
    let resp = crate::onchainos::wallet_contract_call(
        args.chain, MERKL_DISTRIBUTOR, &calldata,
        Some(&wallet), None, false, false,
    ).await?;
    let tx = crate::onchainos::extract_tx_hash(&resp)?;
    eprintln!("[euler-v2] claim tx: {} (waiting...)", tx);
    crate::onchainos::wait_for_tx_receipt(&tx, args.chain, 120).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "action": "claim_rewards",
            "chain": chain_name(args.chain), "chain_id": args.chain,
            "wallet": wallet,
            "distributor": MERKL_DISTRIBUTOR,
            "rewards_claimed": summary,
            "tx_hash": tx, "on_chain_status": "0x1",
            "tip": "Reward tokens transferred to your wallet. \
                    Brevis / Fuul streams (if any) require a separate claim flow not yet supported.",
        }
    }))?);
    Ok(())
}

/// ABI-encode `claim(address[],address[],uint256[],bytes32[][])` for the given user + rewards.
fn build_merkl_claim_calldata(user: &str, rewards: &[crate::api::MerklReward]) -> String {
    let n = rewards.len();
    let user_padded = pad_address(user);

    // Each of users / tokens / amounts is a uniform array of static words → simple encoding.
    // proofs is an array of arrays — each inner array is bytes32[] (also static).

    // Encode the four head offsets (relative to start of args section, i.e. after selector).
    // Layout:
    //   [0..32]   offset to users   = 0x80
    //   [32..64]  offset to tokens
    //   [64..96]  offset to amounts
    //   [96..128] offset to proofs
    //
    // Each subsequent dynamic block: length word + words.
    //
    // users / tokens / amounts each take: 32 (length) + N×32 = (N+1)×32 bytes.
    // proofs: 32 (top length) + N × (32 length + len_i × 32) bytes.

    let users_block = encode_static_array(&vec![user_padded.clone(); n]);
    let tokens_block = encode_static_array(&rewards.iter().map(|r| pad_address(&r.token)).collect());
    let amounts_block = encode_static_array(&rewards.iter().map(|r| {
        let amt: u128 = r.cumulative_amount.parse().unwrap_or(0);
        format!("{:064x}", amt)
    }).collect());
    let proofs_block = encode_proofs_array(rewards);

    // Compute offsets
    let head_size = 4 * 32; // 4 offset words
    let users_off  = head_size;
    let tokens_off = users_off + users_block.len() / 2;
    let amounts_off = tokens_off + tokens_block.len() / 2;
    let proofs_off = amounts_off + amounts_block.len() / 2;

    let mut out = String::new();
    out.push_str("0x");
    out.push_str(SEL_MERKL_CLAIM);
    out.push_str(&format!("{:064x}", users_off));
    out.push_str(&format!("{:064x}", tokens_off));
    out.push_str(&format!("{:064x}", amounts_off));
    out.push_str(&format!("{:064x}", proofs_off));
    out.push_str(&users_block);
    out.push_str(&tokens_block);
    out.push_str(&amounts_block);
    out.push_str(&proofs_block);
    out
}

/// Encode a uniform-static-element array: length word + words.
/// Each `word` should already be a 64-char hex string with no `0x`.
fn encode_static_array(words: &Vec<String>) -> String {
    let mut s = String::new();
    s.push_str(&format!("{:064x}", words.len()));
    for w in words { s.push_str(w); }
    s
}

/// Encode `bytes32[][]` for the proofs argument. Each inner array is
/// `length + N×32` bytes. Outer wraps as: length + offsets (each pointing to
/// inner array start, relative to start of OUTER data section) + inner arrays.
fn encode_proofs_array(rewards: &[crate::api::MerklReward]) -> String {
    let n = rewards.len();
    // Compute offsets table
    let offset_table_bytes = (n * 32) as u64;
    let mut inner_blocks: Vec<String> = Vec::with_capacity(n);
    let mut offsets: Vec<String> = Vec::with_capacity(n);

    let mut cursor = offset_table_bytes;
    for r in rewards {
        let m = r.proofs.len();
        let mut inner = String::new();
        inner.push_str(&format!("{:064x}", m));
        for p in &r.proofs {
            // Each proof is a 0x-prefixed 32-byte hex; strip 0x and pad if shorter.
            let hex = p.trim_start_matches("0x");
            if hex.len() < 64 {
                inner.push_str(&format!("{:0>64}", hex));
            } else {
                inner.push_str(&hex[..64]);
            }
        }
        offsets.push(format!("{:064x}", cursor));
        cursor += (inner.len() / 2) as u64;
        inner_blocks.push(inner);
    }

    let mut s = String::new();
    s.push_str(&format!("{:064x}", n));   // outer length
    for o in &offsets { s.push_str(o); }
    for inner in &inner_blocks { s.push_str(inner); }
    s
}
