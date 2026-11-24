use clap::Args;
use crate::abi::{selector, calldata_raw, encode_uint256_array, word, word_to_u128, format_amount};
use crate::chain::{CHAIN_ETH, chain_name, eth_call};
use crate::contracts::POSITIONS_RESOLVER;
use crate::nft::nft_ids_of;
use crate::onchainos::resolve_wallet;
use crate::vault::vault_for_nft;
use crate::token::token_infos;
use crate::vault::vault_info_single;

#[derive(Args)]
pub struct PositionsArgs {
    /// Chain ID (1 = Ethereum, 42161 = Arbitrum)
    #[arg(long, default_value_t = CHAIN_ETH)]
    pub chain: u64,
    /// Wallet address to query (defaults to active onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
    /// Maximum positions to display
    #[arg(long, default_value_t = 20)]
    pub limit: u64,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let wallet = match args.wallet {
        Some(w) => w,
        None => resolve_wallet(args.chain)
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string()),
    };

    eprintln!("[fluid] Fetching positions for {} on {}...", wallet, chain_name(args.chain));

    let nft_ids = nft_ids_of(args.chain, &wallet, args.limit).await?;

    if nft_ids.is_empty() {
        let out = serde_json::json!({
            "wallet":    wallet,
            "chain":     args.chain,
            "positions": [],
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Get position data for all NFTs in one call
    let positions_raw = fetch_positions(args.chain, &nft_ids).await?;

    // Get vault address for each NFT and vault info
    let mut vault_cache: std::collections::HashMap<String, crate::vault::VaultInfo> = std::collections::HashMap::new();
    let mut token_addrs: Vec<String> = Vec::new();

    for nft_id in &nft_ids {
        let vault_addr = vault_for_nft(args.chain, *nft_id, &wallet).await?;
        if !vault_cache.contains_key(&vault_addr) {
            let info = vault_info_single(args.chain, &vault_addr).await?;
            for a in [&info.col_token, &info.debt_token] {
                if !token_addrs.contains(a) { token_addrs.push(a.clone()); }
            }
            vault_cache.insert(vault_addr.clone(), info);
        }
    }

    let tokens = token_infos(args.chain, &token_addrs).await;

    let mut result = Vec::new();
    for (i, nft_id) in nft_ids.iter().enumerate() {
        let vault_addr = vault_for_nft(args.chain, *nft_id, &wallet).await?;
        let vault_info = vault_cache.get(&vault_addr).cloned().unwrap_or_else(|| crate::vault::VaultInfo {
            address: vault_addr.clone(),
            is_smart_col: false, is_smart_debt: false,
            col_token: "0x0000000000000000000000000000000000000000".to_string(),
            debt_token: "0x0000000000000000000000000000000000000000".to_string(),
            borrow_rate_vault: 0,
        });

        let col_tok = tokens.get(&vault_info.col_token);
        let debt_tok = tokens.get(&vault_info.debt_token);

        // positions_raw: each entry is 4 words (nft_id, owner, supply, debt)
        let col_raw  = positions_raw.get(i).and_then(|p| p.get(2)).copied().unwrap_or(0);
        let debt_raw = positions_raw.get(i).and_then(|p| p.get(3)).copied().unwrap_or(0);

        let col_dec  = col_tok.map(|t| t.decimals).unwrap_or(18);
        let debt_dec = debt_tok.map(|t| t.decimals).unwrap_or(6);

        result.push(serde_json::json!({
            "nft_id":      nft_id,
            "vault":       vault_addr,
            "pair":        format!("{}/{}",
                col_tok.map(|t| t.symbol.as_str()).unwrap_or("?"),
                debt_tok.map(|t| t.symbol.as_str()).unwrap_or("?")),
            "col_symbol":  col_tok.map(|t| t.symbol.as_str()).unwrap_or("?"),
            "col":         format_amount(col_raw, col_dec),
            "col_raw":     col_raw.to_string(),
            "debt_symbol": debt_tok.map(|t| t.symbol.as_str()).unwrap_or("?"),
            "debt":        format_amount(debt_raw, debt_dec),
            "debt_raw":    debt_raw.to_string(),
        }));
    }

    let out = serde_json::json!({
        "wallet":    wallet,
        "chain":     args.chain,
        "positions": result,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Call getPositionsForNftIds([ids]) on the positions resolver.
/// Returns Vec<[nft_id, owner, supply, debt]> (4 u128 values per position).
pub async fn fetch_positions(chain_id: u64, nft_ids: &[u64]) -> anyhow::Result<Vec<[u128; 4]>> {
    let sel = selector("getPositionsForNftIds(uint256[])");
    let payload = encode_uint256_array(nft_ids);
    let data = calldata_raw(sel, &payload);
    let result = eth_call(chain_id, POSITIONS_RESOLVER, &data).await?;

    // ABI: offset(32) + length + N×4 words
    if result.len() < 128 {
        return Ok(vec![]);
    }
    let length = usize::from_str_radix(&result[64..128], 16).unwrap_or(0);
    let mut positions = Vec::with_capacity(length);
    for i in 0..length {
        let base = 128 + i * 4 * 64; // 4 words per position
        let mut entry = [0u128; 4];
        for j in 0..4usize {
            let start = base + j * 64;
            if result.len() < start + 64 { break; }
            let w = word(&result, (base / 64) + j);
            entry[j] = w.map(|w| word_to_u128(&w)).unwrap_or(0);
        }
        positions.push(entry);
    }
    Ok(positions)
}
