use clap::Args;
use crate::abi::{selector, calldata, encode_uint256, encode_int256, encode_address, format_amount, validate_address};
use crate::chain::{CHAIN_ETH, chain_name, eth_call_simulate};
use crate::contracts::NATIVE_ETH;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};
use crate::token::token_infos;
use crate::vault::{vault_info_single, vault_for_nft};
use crate::commands::positions::fetch_positions;

#[derive(Args)]
pub struct CloseArgs {
    /// Chain ID (1 = Ethereum, 42161 = Arbitrum)
    #[arg(long, default_value_t = CHAIN_ETH)]
    pub chain: u64,
    /// NFT position ID to close
    #[arg(long)]
    pub nft_id: u64,
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

pub async fn run(args: CloseArgs) -> anyhow::Result<()> {
    let wallet = match &args.wallet {
        Some(w) => w.clone(),
        None => resolve_wallet(args.chain)?,
    };

    eprintln!("[fluid] Fetching position data for NFT #{} on {}...", args.nft_id, chain_name(args.chain));

    // Fetch current col_raw and debt_raw from the positions resolver
    let positions = fetch_positions(args.chain, &[args.nft_id]).await?;
    let pos = positions.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!(
            "No position found for NFT #{} on chain {}. \
             Verify the NFT ID and chain with `fluid positions`.",
            args.nft_id, args.chain
        ))?;

    // pos = [nft_id, owner, col_raw, debt_raw]
    let col_raw  = pos[2];
    let debt_raw = pos[3];

    if col_raw == 0 && debt_raw == 0 {
        anyhow::bail!(
            "Position NFT #{} appears to be empty (no collateral or debt). \
             It may have already been closed.",
            args.nft_id
        );
    }

    // Get vault address for this NFT
    let vault_addr = vault_for_nft(args.chain, args.nft_id, &wallet).await?;
    validate_address(&vault_addr, "vault")?;

    let vault_info = vault_info_single(args.chain, &vault_addr).await?;

    let token_addrs = vec![vault_info.col_token.clone(), vault_info.debt_token.clone()];
    let tokens = token_infos(args.chain, &token_addrs).await;

    let col_tok  = tokens.get(&vault_info.col_token);
    let debt_tok = tokens.get(&vault_info.debt_token);
    let col_dec  = col_tok.map(|t| t.decimals).unwrap_or(18);
    let debt_dec = debt_tok.map(|t| t.decimals).unwrap_or(6);
    let col_sym  = col_tok.map(|t| t.symbol.as_str()).unwrap_or("?").to_string();
    let debt_sym = debt_tok.map(|t| t.symbol.as_str()).unwrap_or("?").to_string();

    let is_native_debt = vault_info.debt_token.to_lowercase() == NATIVE_ETH.to_lowercase();

    // Build operate(nft_id, -col_raw, -debt_raw, wallet) — withdraw collateral + repay debt atomically
    let op_sel = selector("operate(uint256,int256,int256,address)");
    let op_data = calldata(op_sel, &[
        encode_uint256(args.nft_id as u128),
        encode_int256(-(col_raw as i128)),
        encode_int256(-(debt_raw as i128)),
        encode_address(&wallet),
    ]);

    let approval_note = if debt_raw == 0 {
        "No debt to repay — no ERC-20 approval needed".to_string()
    } else if is_native_debt {
        "Native ETH debt — no approval needed".to_string()
    } else {
        format!("ERC-20 debt ({}) — approval tx will fire first", debt_sym)
    };

    let preview = serde_json::json!({
        "preview":      true,
        "action":       "close",
        "vault":        vault_addr,
        "nft_id":       args.nft_id,
        "col_symbol":   col_sym,
        "col":          format_amount(col_raw, col_dec),
        "col_raw":      col_raw.to_string(),
        "debt_symbol":  debt_sym,
        "debt":         format_amount(debt_raw, debt_dec),
        "debt_raw":     debt_raw.to_string(),
        "wallet":       wallet,
        "chain":        args.chain,
        "note":         approval_note,
        "confirm_hint": "Add --confirm to broadcast"
    });

    if !args.confirm && !args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // Simulate before spending any gas — catches vault reverts early
    eprintln!("[fluid] Simulating close via eth_call...");
    if let Err(sim_err) = eth_call_simulate(args.chain, &vault_addr, &op_data, &wallet).await {
        anyhow::bail!(
            "Close simulation failed (the vault would revert on-chain): {}\n\
             The position data may have changed since the preview was generated. \
             Re-run `fluid close --nft-id {}` to fetch fresh balances.",
            sim_err, args.nft_id
        );
    }

    let mut approve_hash: Option<String> = None;

    // Approve ERC-20 debt token for the vault (if debt > 0 and not native ETH)
    if debt_raw > 0 && !is_native_debt {
        let approve_sel = selector("approve(address,uint256)");
        let approve_data = calldata(approve_sel, &[
            encode_address(&vault_addr),
            encode_uint256(debt_raw),
        ]);
        eprintln!("[fluid] Approving {} {} for vault {}...",
            format_amount(debt_raw, debt_dec), debt_sym, vault_addr);
        let approve_resp = wallet_contract_call(
            args.chain, &vault_info.debt_token, &approve_data, "0", args.dry_run, Some(&wallet),
        )?;
        approve_hash = Some(extract_tx_hash(&approve_resp));
        eprintln!("[fluid] Approval tx: {}", approve_hash.as_deref().unwrap_or("?"));
    }

    // For native ETH debt, pass debt_raw as --amt; otherwise 0
    let amt_eth = if is_native_debt && debt_raw > 0 { debt_raw.to_string() } else { "0".to_string() };
    let resp = wallet_contract_call(
        args.chain, &vault_addr, &op_data, &amt_eth, args.dry_run, Some(&wallet),
    )?;
    let tx_hash = extract_tx_hash(&resp);

    let out = serde_json::json!({
        "ok":             true,
        "action":         "close",
        "vault":          vault_addr,
        "nft_id":         args.nft_id,
        "col_symbol":     col_sym,
        "col_withdrawn":  format_amount(col_raw, col_dec),
        "debt_symbol":    debt_sym,
        "debt_repaid":    format_amount(debt_raw, debt_dec),
        "approve_tx_hash": approve_hash,
        "tx_hash":        tx_hash,
        "wallet":         wallet,
        "chain":          args.chain,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
