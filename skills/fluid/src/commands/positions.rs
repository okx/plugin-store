use crate::calldata;
use crate::config::{get_chain_config, chain_name};
use crate::onchainos;
use crate::rpc;

/// View user's Fluid lending positions across all fTokens.
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve wallet address
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, false)?
    };

    let calldata = calldata::encode_get_user_positions(&wallet);
    let hex = rpc::eth_call(cfg.lending_resolver, &calldata, cfg.rpc_url).await?;

    // Parse user positions from raw ABI response
    let positions = parse_user_positions_hex(&hex, &wallet, chain_id, cfg.rpc_url).await;

    let output = serde_json::json!({
        "ok": true,
        "user": wallet,
        "chain": chain_name(chain_id),
        "chainId": chain_id,
        "positions": positions,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn parse_user_positions_hex(
    hex: &str,
    wallet: &str,
    chain_id: u64,
    rpc_url: &str,
) -> Vec<serde_json::Value> {
    // The getUserPositions response is a complex ABI-encoded tuple array.
    // We fall back to querying each known fToken balance individually.
    let known_ftokens = get_known_ftokens(chain_id);
    let mut positions = Vec::new();

    for (ftoken_addr, underlying_addr, symbol, decimals) in known_ftokens {
        // Get share balance
        let shares = rpc::ftoken_share_balance(ftoken_addr, wallet, rpc_url)
            .await
            .unwrap_or(0);
        if shares == 0 {
            continue;
        }
        // Convert shares to underlying assets
        let underlying = rpc::ftoken_convert_to_assets(ftoken_addr, shares, rpc_url)
            .await
            .unwrap_or(0);

        positions.push(serde_json::json!({
            "fToken": ftoken_addr,
            "symbol": symbol,
            "underlying": underlying_addr,
            "fTokenShares": shares.to_string(),
            "underlyingAssets": calldata::format_amount(underlying, decimals),
            "underlyingRaw": underlying.to_string(),
            "decimals": decimals,
        }));
    }

    if positions.is_empty() {
        // Return a message indicating no positions
        positions.push(serde_json::json!({
            "message": "No active lending positions found",
            "rawHexLength": hex.len(),
        }));
    }

    positions
}

fn get_known_ftokens(chain_id: u64) -> Vec<(&'static str, &'static str, &'static str, u8)> {
    match chain_id {
        8453 => vec![
            ("0xf42f5795D9ac7e9D757dB633D693cD548Cfd9169", "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", "fUSDC", 6u8),
            ("0x9272D6153133175175Bc276512B2336BE3931CE9", "0x4200000000000000000000000000000000000006", "fWETH", 18u8),
            ("0x8DdbfFA3CFda2355a23d6B11105AC624BDbE3631", "0x6Bb7a212910682DCFdbd5BCBb3e28FB4E8da10Ee", "fGHO", 18u8),
            ("0x1943FA26360f038230442525Cf1B9125b5DCB401", "0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1aDb42", "fEURC", 6u8),
        ],
        1 => vec![
            ("0x9Fb7b4477576Fe5B32be4C1843aFB1e55F251B33", "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", "fUSDC", 6u8),
            ("0x90551c1795392094FE6D29B758EcCD233cFAa260", "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2", "fWETH", 18u8),
            ("0x5C20B550819128074FD538Edf79791733ccEdd18", "0xdac17f958d2ee523a2206206994597c13d831ec7", "fUSDT", 6u8),
        ],
        42161 => vec![
            ("0x1A996cb54bb95462040408C06122D45D6Cdb6096", "0xaf88d065e77c8cc2239327c5edb3a432268e5831", "fUSDC", 6u8),
            ("0x45Df0656F8aDf017590009d2f1898eeca4F0a205", "0x82af49447d8a07e3bd95bd0d56f35241523fbab1", "fWETH", 18u8),
            ("0x4A03F37e7d3fC243e3f99341d36f4b829BEe5E03", "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9", "fUSDT", 6u8),
        ],
        _ => vec![],
    }
}
