// src/commands/token_info.rs — query on-chain token info + price for a Clanker token
use crate::onchainos;
use anyhow::Result;

pub fn run(chain_id: u64, token_address: &str) -> Result<()> {
    // Validate address format before querying
    let is_valid_addr = token_address.starts_with("0x")
        && token_address.len() == 42
        && token_address[2..].chars().all(|c| c.is_ascii_hexdigit());
    if !is_valid_addr {
        anyhow::bail!(
            "Invalid token address: '{}'. Must be a 42-character hex address (0x...).",
            token_address
        );
    }

    let info = onchainos::token_info(chain_id, token_address)?;
    let price = onchainos::token_price_info(chain_id, token_address)?;

    // price["data"] is [] when no price oracle covers this token (common for new/illiquid tokens).
    // Surface a clear status rather than a bare empty array.
    let price_data = &price["data"];
    let price_available = !price_data.is_null()
        && !(price_data.is_array() && price_data.as_array().map_or(true, |a| a.is_empty()));

    let price_field = if price_available {
        price_data.clone()
    } else {
        serde_json::json!(null)
    };

    let info_value = {
        let d = &info["data"];
        if let Some(arr) = d.as_array() {
            arr.first().cloned().unwrap_or(serde_json::Value::Null)
        } else {
            d.clone()
        }
    };

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "token_address": token_address,
            "chain_id": chain_id,
            "info": info_value,
            "price": price_field,
            "price_available": price_available,
            "price_note": if price_available {
                serde_json::json!(null)
            } else {
                serde_json::json!("No price data available — token is not yet tracked by any price oracle. This is common for newly deployed or low-liquidity Clanker tokens.")
            }
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
