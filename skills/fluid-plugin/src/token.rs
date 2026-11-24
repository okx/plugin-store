use std::collections::HashMap;
use crate::abi::{selector, calldata, decode_string, word, word_to_u8};
use crate::chain::eth_call;
use crate::contracts::NATIVE_ETH;

#[derive(Clone)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

/// Lookup token symbol and decimals. Returns cached results for known tokens.
pub async fn token_info(chain_id: u64, address: &str) -> TokenInfo {
    let addr_lower = address.to_lowercase();

    // Native ETH sentinel
    if addr_lower == NATIVE_ETH {
        return TokenInfo {
            address: NATIVE_ETH.to_string(),
            symbol: "ETH".to_string(),
            decimals: 18,
        };
    }

    // Well-known tokens (hardcoded for speed)
    if let Some(info) = known_token(&addr_lower) {
        return info;
    }

    // On-chain lookup
    let sym = fetch_symbol(chain_id, address).await.unwrap_or_else(|_| "?".to_string());
    let dec = fetch_decimals(chain_id, address).await.unwrap_or(18);
    TokenInfo { address: addr_lower, symbol: sym, decimals: dec }
}

/// Fetch multiple token infos, deduplicating on address.
pub async fn token_infos(chain_id: u64, addresses: &[String]) -> HashMap<String, TokenInfo> {
    let mut map = HashMap::new();
    for addr in addresses {
        let lower = addr.to_lowercase();
        if !map.contains_key(&lower) {
            let info = token_info(chain_id, addr).await;
            map.insert(lower, info);
        }
    }
    map
}

async fn fetch_symbol(chain_id: u64, addr: &str) -> anyhow::Result<String> {
    let sel = selector("symbol()");
    let data = calldata(sel, &[]);
    let result = eth_call(chain_id, addr, &data).await?;
    Ok(decode_string(&format!("0x{}", result)))
}

async fn fetch_decimals(chain_id: u64, addr: &str) -> anyhow::Result<u8> {
    let sel = selector("decimals()");
    let data = calldata(sel, &[]);
    let result = eth_call(chain_id, addr, &data).await?;
    let w = word(&result, 0).ok_or_else(|| anyhow::anyhow!("No result"))?;
    Ok(word_to_u8(&w))
}

fn known_token(addr: &str) -> Option<TokenInfo> {
    let (sym, dec) = match addr {
        // Ethereum mainnet
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => ("USDC", 6),
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => ("USDT", 6),
        "0x6b175474e89094c44da98b954eedeac495271d0f" => ("DAI", 18),
        "0x7f39c581f595b53c5cb19bd0b3f8da6c935e2ca0" => ("wstETH", 18),
        "0xae7ab96520de3a18e5e111b5eaab095312d7fe84" => ("stETH", 18),
        "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => ("WBTC", 8),
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => ("WETH", 18),
        "0xae78736cd615f374d3085123a210448e74fc6393" => ("rETH", 18),
        "0xac3e018457b222d93114458476f3e3416abbe38f" => ("sfrxETH", 18),
        "0xbe9895146f7af43049ca1c1ae358b0541ea49704" => ("cbETH", 18),
        "0xcd5fe23c85820f7b72d0926fc9b05b43e359b7ee" => ("weETH", 18),
        "0xbf5495efe5db9ce00f80364c8b423567e58d2110" => ("ezETH", 18),
        "0xd5f7838f5c461feff7fe49ea5ebaf7728bb0adfa" => ("mETH", 18),
        // Arbitrum
        "0xaf88d065e77c8cc2239327c5edb3a432268e5831" => ("USDC", 6),
        "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9" => ("USDT", 6),
        "0x82af49447d8a07e3bd95bd0d56f35241523fbab1" => ("WETH", 18),
        "0x5979d7b546e38e414f7e9822514be443a4800529" => ("wstETH", 18),
        "0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f" => ("WBTC", 8),
        _ => return None,
    };
    Some(TokenInfo {
        address: addr.to_string(),
        symbol: sym.to_string(),
        decimals: dec,
    })
}
