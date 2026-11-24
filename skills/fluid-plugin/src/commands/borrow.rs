use clap::Args;
use crate::abi::{selector, calldata, encode_uint256, encode_int256, encode_address, parse_amount, validate_address};
use crate::chain::{CHAIN_ETH, chain_name, eth_call_simulate};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::token::token_infos;
use crate::vault::vault_info_single;

#[derive(Args)]
pub struct BorrowArgs {
    /// Chain ID (1 = Ethereum, 42161 = Arbitrum)
    #[arg(long, default_value_t = CHAIN_ETH)]
    pub chain: u64,
    /// Vault address to borrow from
    #[arg(long)]
    pub vault: String,
    /// Existing NFT position ID (must have collateral supplied)
    #[arg(long)]
    pub nft_id: u64,
    /// Amount of debt token to borrow (human-readable, e.g. 500 or 1000.5)
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

pub async fn run(args: BorrowArgs) -> anyhow::Result<()> {
    validate_address(&args.vault, "--vault")?;

    let wallet = match &args.wallet {
        Some(w) => w.clone(),
        None => resolve_wallet(args.chain)?,
    };

    let vault_info = vault_info_single(args.chain, &args.vault).await?;
    let token_addrs = vec![vault_info.debt_token.clone()];
    let tokens = token_infos(args.chain, &token_addrs).await;

    let debt_tok = tokens.get(&vault_info.debt_token);
    let debt_dec = debt_tok.map(|t| t.decimals).unwrap_or(6);
    let debt_sym = debt_tok.map(|t| t.symbol.as_str()).unwrap_or("?").to_string();

    let debt_raw = parse_amount(&args.amount, debt_dec)?;
    if debt_raw == 0 {
        anyhow::bail!("Borrow amount must be greater than 0");
    }

    eprintln!("[fluid] Borrow {} {} from vault {} NFT #{} on {}...",
        args.amount, debt_sym, args.vault, args.nft_id, chain_name(args.chain));

    // Build operate(nft_id, 0, +newDebt, wallet) calldata
    let op_sel = selector("operate(uint256,int256,int256,address)");
    let op_data = calldata(op_sel, &[
        encode_uint256(args.nft_id as u128),
        encode_int256(0),
        encode_int256(debt_raw as i128),
        encode_address(&wallet),
    ]);

    let borrow_rate_pct = vault_info.borrow_rate_vault as f64 / 100.0;
    let preview = serde_json::json!({
        "preview": true,
        "action": "borrow",
        "vault": args.vault,
        "nft_id": args.nft_id,
        "debt_token": vault_info.debt_token,
        "debt_symbol": debt_sym,
        "amount": args.amount,
        "amount_raw": debt_raw.to_string(),
        "borrow_rate": format!("{:.2}%", borrow_rate_pct),
        "wallet": wallet,
        "chain": args.chain,
        "confirm_hint": "Add --confirm to broadcast"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // Simulate operate() before broadcasting — catches minimum-position violations
    // (Fluid error 0x60121cca) before any on-chain action, saving gas.
    eprintln!("[fluid] Simulating borrow via eth_call...");
    if let Err(sim_err) = eth_call_simulate(args.chain, &args.vault, &op_data, &wallet).await {
        anyhow::bail!(
            "Borrow simulation failed (the vault would revert on-chain): {}\n\
             This usually means the borrow amount is below the vault minimum, \
             or your position lacks sufficient collateral. \
             Check `fluid positions` for your current collateral balance.",
            sim_err
        );
    }

    // Borrow: no approval needed — debt token comes out of the vault to the wallet
    let resp = wallet_contract_call(
        args.chain, &args.vault, &op_data, "0", args.dry_run, Some(&wallet),
    )?;
    let tx_hash = extract_tx_hash(&resp);

    let out = serde_json::json!({
        "ok": true,
        "action": "borrow",
        "vault": args.vault,
        "nft_id": args.nft_id,
        "debt_symbol": debt_sym,
        "amount": args.amount,
        "borrow_rate": format!("{:.2}%", borrow_rate_pct),
        "tx_hash": tx_hash,
        "wallet": wallet,
        "chain": args.chain,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
