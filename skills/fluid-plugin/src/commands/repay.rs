use clap::Args;
use crate::abi::{selector, calldata, encode_uint256, encode_int256, encode_address, parse_amount, validate_address};
use crate::chain::{CHAIN_ETH, chain_name, eth_call_simulate};
use crate::contracts::NATIVE_ETH;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::token::token_infos;
use crate::vault::vault_info_single;

#[derive(Args)]
pub struct RepayArgs {
    /// Chain ID (1 = Ethereum, 42161 = Arbitrum)
    #[arg(long, default_value_t = CHAIN_ETH)]
    pub chain: u64,
    /// Vault address of the position to repay
    #[arg(long)]
    pub vault: String,
    /// NFT position ID to repay
    #[arg(long)]
    pub nft_id: u64,
    /// Amount of debt token to repay (human-readable, e.g. 500 or 1000.5)
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

pub async fn run(args: RepayArgs) -> anyhow::Result<()> {
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
    let is_native = vault_info.debt_token.to_lowercase() == NATIVE_ETH.to_lowercase();

    let debt_raw = parse_amount(&args.amount, debt_dec)?;
    if debt_raw == 0 {
        anyhow::bail!("Repay amount must be greater than 0");
    }

    eprintln!("[fluid] Repay {} {} on vault {} NFT #{} on {}...",
        args.amount, debt_sym, args.vault, args.nft_id, chain_name(args.chain));

    // Build operate(nft_id, 0, -newDebt, wallet) — negative newDebt = repay
    let op_sel = selector("operate(uint256,int256,int256,address)");
    let op_data = calldata(op_sel, &[
        encode_uint256(args.nft_id as u128),
        encode_int256(0),
        encode_int256(-(debt_raw as i128)),
        encode_address(&wallet),
    ]);

    let approval_note = if is_native {
        "Native ETH — no approval needed"
    } else {
        "ERC-20 — approval tx will fire first"
    };

    let preview = serde_json::json!({
        "preview": true,
        "action": "repay",
        "vault": args.vault,
        "nft_id": args.nft_id,
        "debt_token": vault_info.debt_token,
        "debt_symbol": debt_sym,
        "amount": args.amount,
        "amount_raw": debt_raw.to_string(),
        "wallet": wallet,
        "chain": args.chain,
        "note": approval_note,
        "confirm_hint": "Add --confirm to broadcast"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // Simulate operate() BEFORE sending any approval — catches stuck positions
    // (Fluid error 0x60121cca) and other reverts without wasting gas on approvals.
    eprintln!("[fluid] Simulating repay via eth_call...");
    if let Err(sim_err) = eth_call_simulate(args.chain, &args.vault, &op_data, &wallet).await {
        // 0xdee51a8a = residual debt below minimum floor (partial repay leaves too little)
        // 0x60121cca = vault minimum check failed (position below floor regardless of amount)
        // In both cases the position may need to be closed atomically.
        anyhow::bail!(
            "Repay simulation failed (the vault would revert on-chain): {}\n\
             The position may be below the vault minimum floor. \
             Use `fluid close --chain {} --nft-id {}` to repay debt and withdraw \
             collateral atomically in a single transaction.",
            sim_err, args.chain, args.nft_id
        );
    }

    let mut approve_hash: Option<String> = None;

    // Approve ERC-20 debt token for the vault
    if !is_native {
        let approve_sel = selector("approve(address,uint256)");
        let approve_data = calldata(approve_sel, &[
            encode_address(&args.vault),
            encode_uint256(debt_raw),
        ]);
        eprintln!("[fluid] Approving {} {} for repay on vault {}...", args.amount, debt_sym, args.vault);
        let approve_resp = wallet_contract_call(
            args.chain, &vault_info.debt_token, &approve_data, "0", args.dry_run, Some(&wallet),
        )?;
        approve_hash = Some(extract_tx_hash(&approve_resp));
        eprintln!("[fluid] Approval tx: {}", approve_hash.as_deref().unwrap_or("?"));
    }

    // Call operate — for native ETH debt, pass debt_raw as --amt
    let amt_eth = if is_native { debt_raw.to_string() } else { "0".to_string() };
    let resp = wallet_contract_call(
        args.chain, &args.vault, &op_data, &amt_eth, args.dry_run, Some(&wallet),
    )?;
    let tx_hash = extract_tx_hash(&resp);

    let out = serde_json::json!({
        "ok": true,
        "action": "repay",
        "vault": args.vault,
        "nft_id": args.nft_id,
        "debt_symbol": debt_sym,
        "amount": args.amount,
        "approve_tx_hash": approve_hash,
        "tx_hash": tx_hash,
        "wallet": wallet,
        "chain": args.chain,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
