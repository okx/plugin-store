use clap::Args;
use crate::abi::{selector, calldata, encode_uint256, encode_int256, encode_address, parse_amount, validate_address};
use crate::chain::{CHAIN_ETH, chain_name};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::token::token_infos;
use crate::vault::vault_info_single;

#[derive(Args)]
pub struct WithdrawArgs {
    /// Chain ID (1 = Ethereum, 42161 = Arbitrum)
    #[arg(long, default_value_t = CHAIN_ETH)]
    pub chain: u64,
    /// Vault address of the position to withdraw from
    #[arg(long)]
    pub vault: String,
    /// NFT position ID to withdraw collateral from
    #[arg(long)]
    pub nft_id: u64,
    /// Amount of collateral to withdraw (human-readable, e.g. 0.5 or 1000)
    #[arg(long)]
    pub amount: String,
    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
    /// Simulate without broadcasting (returns stub hashes)
    #[arg(long)]
    pub dry_run: bool,
    /// Broadcast the transaction (required to execute)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    validate_address(&args.vault, "--vault")?;

    let wallet = match &args.wallet {
        Some(w) => w.clone(),
        None => resolve_wallet(args.chain)?,
    };

    let vault_info = vault_info_single(args.chain, &args.vault).await?;
    let token_addrs = vec![vault_info.col_token.clone()];
    let tokens = token_infos(args.chain, &token_addrs).await;

    let col_tok = tokens.get(&vault_info.col_token);
    let col_dec = col_tok.map(|t| t.decimals).unwrap_or(18);
    let col_sym = col_tok.map(|t| t.symbol.as_str()).unwrap_or("?").to_string();

    let col_raw = parse_amount(&args.amount, col_dec)?;
    if col_raw == 0 {
        anyhow::bail!("Withdraw amount must be greater than 0");
    }

    eprintln!("[fluid] Withdraw {} {} from vault {} NFT #{} on {}...",
        args.amount, col_sym, args.vault, args.nft_id, chain_name(args.chain));

    // Build operate(nft_id, -newCol, 0, wallet) — negative newCol = withdraw
    let op_sel = selector("operate(uint256,int256,int256,address)");
    let op_data = calldata(op_sel, &[
        encode_uint256(args.nft_id as u128),
        encode_int256(-(col_raw as i128)),
        encode_int256(0),
        encode_address(&wallet),
    ]);

    let preview = serde_json::json!({
        "preview": true,
        "action": "withdraw",
        "vault": args.vault,
        "nft_id": args.nft_id,
        "col_token": vault_info.col_token,
        "col_symbol": col_sym,
        "amount": args.amount,
        "amount_raw": col_raw.to_string(),
        "wallet": wallet,
        "chain": args.chain,
        "note": "No approval needed — collateral is returned to your wallet",
        "confirm_hint": "Add --confirm to broadcast"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // No approval needed for withdrawal
    let resp = wallet_contract_call(
        args.chain, &args.vault, &op_data, "0", args.dry_run, Some(&wallet),
    )?;
    let tx_hash = extract_tx_hash(&resp);

    let out = serde_json::json!({
        "ok": true,
        "action": "withdraw",
        "vault": args.vault,
        "nft_id": args.nft_id,
        "col_symbol": col_sym,
        "amount": args.amount,
        "tx_hash": tx_hash,
        "wallet": wallet,
        "chain": args.chain,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
