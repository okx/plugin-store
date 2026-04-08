use anyhow::Context;

/// Make a raw eth_call via JSON-RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("RPC request failed")?
        .json()
        .await
        .context("RPC response parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    let result = resp["result"]
        .as_str()
        .unwrap_or("0x")
        .to_string();
    Ok(result)
}

/// getNumMarkets() → uint256  selector: 0x295c39a5
pub async fn get_num_markets(dolomite_margin: &str, rpc_url: &str) -> anyhow::Result<u64> {
    let hex = eth_call(dolomite_margin, "0x295c39a5", rpc_url).await?;
    Ok(parse_u128_from_hex(&hex).unwrap_or(0) as u64)
}

/// getMarketTokenAddress(uint256 marketId) → address  selector: 0x062bd3e9
pub async fn get_market_token_address(dolomite_margin: &str, market_id: u64, rpc_url: &str) -> anyhow::Result<String> {
    let data = format!("0x062bd3e9{:064x}", market_id);
    let hex = eth_call(dolomite_margin, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 40 {
        anyhow::bail!("Invalid getMarketTokenAddress response: {}", hex);
    }
    Ok(format!("0x{}", &clean[clean.len() - 40..]))
}

/// getMarketTotalPar(uint256 marketId) → TotalPar{borrow: Par, supply: Par}
/// selector: 0xcb04a34c
/// Returns (borrow_par_value, supply_par_value) as u128 each (sign + uint128)
pub async fn get_market_total_par(dolomite_margin: &str, market_id: u64, rpc_url: &str) -> anyhow::Result<(u128, u128)> {
    let data = format!("0xcb04a34c{:064x}", market_id);
    let hex = eth_call(dolomite_margin, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    // Returns 2 x Par structs: each is (bool sign, uint128 value) packed as 32 bytes
    // ABI: borrow Par (32 bytes), supply Par (32 bytes)
    if clean.len() < 128 {
        return Ok((0, 0));
    }
    // Each Par is encoded as: sign (bool, 1 byte) + value (uint128, 16 bytes) = 32-byte slot
    // In ABI encoding, bool is padded to 32 bytes, then uint128 padded to 32 bytes
    // Actually TotalPar is a struct with two Par fields, so it's 4 slots:
    // slot0: borrow.sign (bool padded)
    // slot1: borrow.value (uint128 padded)
    // slot2: supply.sign (bool padded)
    // slot3: supply.value (uint128 padded)
    let borrow_val = if clean.len() >= 192 {
        parse_u128_from_slot(&clean[64..128])
    } else {
        0
    };
    let supply_val = if clean.len() >= 256 {
        parse_u128_from_slot(&clean[192..256])
    } else {
        0
    };
    Ok((borrow_val, supply_val))
}

/// getAccountBalances(Account.Info account) → (uint256[] marketIds, address[] tokenAddrs, Par[] pars, Wei[] weis)
/// selector: 0x6a8194e7
/// Account.Info = (address owner, uint256 number)
pub async fn get_account_balances(
    dolomite_margin: &str,
    owner: &str,
    account_number: u64,
    rpc_url: &str,
) -> anyhow::Result<Vec<AccountBalance>> {
    let owner_clean = owner.trim_start_matches("0x").to_lowercase();
    // Encode Account.Info struct: (address, uint256) = 2 slots
    let data = format!(
        "0x6a8194e7{:0>64}{:064x}",
        owner_clean, account_number
    );
    let hex = eth_call(dolomite_margin, &data, rpc_url).await?;
    decode_account_balances(&hex)
}

/// Decode the ABI-encoded getAccountBalances response.
fn decode_account_balances(hex: &str) -> anyhow::Result<Vec<AccountBalance>> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 256 {
        return Ok(vec![]);
    }

    // getAccountBalances returns (uint256[], address[], Types.Par[], Types.Wei[])
    // ABI encoding: 4 offsets (4 * 32 bytes = 128 bytes) then the arrays
    let offset0 = usize::from_str_radix(&clean[0..64], 16).unwrap_or(0) * 2;
    let offset1 = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0) * 2;
    let _offset2 = usize::from_str_radix(&clean[128..192], 16).unwrap_or(0) * 2;
    let _offset3 = usize::from_str_radix(&clean[192..256], 16).unwrap_or(0) * 2;

    if offset0 + 64 > clean.len() {
        return Ok(vec![]);
    }

    // Read market IDs array length
    let market_ids_len = usize::from_str_radix(&clean[offset0..offset0 + 64], 16).unwrap_or(0);
    if market_ids_len == 0 {
        return Ok(vec![]);
    }

    // Read market IDs
    let mut market_ids = Vec::with_capacity(market_ids_len);
    for i in 0..market_ids_len {
        let pos = offset0 + 64 + i * 64;
        if pos + 64 > clean.len() {
            break;
        }
        let mid = u64::from_str_radix(&clean[pos..pos + 64], 16).unwrap_or(0);
        market_ids.push(mid);
    }

    // Read token addresses
    let addr_len = usize::from_str_radix(&clean[offset1..offset1 + 64], 16).unwrap_or(0);
    let mut token_addrs = Vec::with_capacity(addr_len);
    for i in 0..addr_len {
        let pos = offset1 + 64 + i * 64;
        if pos + 64 > clean.len() {
            break;
        }
        let slot = &clean[pos..pos + 64];
        token_addrs.push(format!("0x{}", &slot[24..]));
    }

    // Build results from market IDs we have
    let mut balances = Vec::new();
    for (i, &mid) in market_ids.iter().enumerate() {
        let token = token_addrs.get(i).cloned().unwrap_or_default();
        balances.push(AccountBalance {
            market_id: mid,
            token_address: token,
            wei_sign: true,
            wei_value: 0, // We'll rely on separate calls if needed
        });
    }
    Ok(balances)
}

#[derive(Debug)]
pub struct AccountBalance {
    pub market_id: u64,
    pub token_address: String,
    #[allow(dead_code)]
    pub wei_sign: bool,
    #[allow(dead_code)]
    pub wei_value: u128,
}

/// getAccountWei((address,uint256),uint256) → Wei{sign, value}
/// selector: 0xc190c2ec
pub async fn get_account_wei(
    dolomite_margin: &str,
    owner: &str,
    account_number: u64,
    market_id: u64,
    rpc_url: &str,
) -> anyhow::Result<(bool, u128)> {
    let owner_clean = owner.trim_start_matches("0x").to_lowercase();
    let data = format!(
        "0xc190c2ec{:0>64}{:064x}{:064x}",
        owner_clean, account_number, market_id
    );
    let hex = eth_call(dolomite_margin, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok((true, 0));
    }
    // Wei = (bool sign, uint256 value)
    let sign = clean[63..64] == *"1";
    let value = parse_u128_from_slot(&clean[64..128]);
    Ok((sign, value))
}

/// Read ERC-20 balance of `owner` at `token`.
pub async fn erc20_balance_of(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_clean = owner.trim_start_matches("0x").to_lowercase();
    let data = format!("0x70a08231{:0>64}", owner_clean);
    let hex = eth_call(token, &data, rpc_url).await?;
    parse_u128_from_hex(&hex)
}

/// Read ERC-20 decimals.
pub async fn erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return Ok(18);
    }
    let padded = format!("{:0>64}", hex_clean);
    let val = u8::from_str_radix(&padded[padded.len().saturating_sub(2)..], 16).unwrap_or(18);
    Ok(val)
}

/// Read ERC-20 symbol.
pub async fn erc20_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    decode_string_from_hex(&hex)
}

/// Parse a u128 from a 32-byte hex eth_call result.
pub fn parse_u128_from_hex(hex: &str) -> anyhow::Result<u128> {
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.is_empty() || hex_clean == "0" {
        return Ok(0);
    }
    let padded = format!("{:0>64}", hex_clean);
    let tail = &padded[padded.len().saturating_sub(32)..];
    Ok(u128::from_str_radix(tail, 16).unwrap_or(0))
}

fn parse_u128_from_slot(slot: &str) -> u128 {
    if slot.len() < 32 {
        return 0;
    }
    let tail = &slot[slot.len().saturating_sub(32)..];
    u128::from_str_radix(tail, 16).unwrap_or(0)
}

/// Decode ABI-encoded string from eth_call result.
fn decode_string_from_hex(hex: &str) -> anyhow::Result<String> {
    let hex_clean = hex.trim_start_matches("0x");
    if hex_clean.len() < 128 {
        return Ok("UNKNOWN".to_string());
    }
    let offset = usize::from_str_radix(&hex_clean[0..64], 16).unwrap_or(32);
    let len_pos = offset * 2;
    if hex_clean.len() < len_pos + 64 {
        return Ok("UNKNOWN".to_string());
    }
    let len = usize::from_str_radix(&hex_clean[len_pos..len_pos + 64], 16).unwrap_or(0);
    if len == 0 {
        return Ok("".to_string());
    }
    let data_start = len_pos + 64;
    let data_end = data_start + len * 2;
    if data_end > hex_clean.len() {
        return Ok("UNKNOWN".to_string());
    }
    let bytes = hex::decode(&hex_clean[data_start..data_end]).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Format a raw token amount to human-readable string.
pub fn format_amount(raw: u128, decimals: u8) -> String {
    if decimals == 0 {
        return raw.to_string();
    }
    let d = decimals as u32;
    let divisor = 10u128.pow(d);
    let whole = raw / divisor;
    let frac = raw % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = d as usize);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

/// Parse human-readable amount string to raw u128.
pub fn parse_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty amount string");
    }
    let d = decimals as u32;
    let multiplier = 10u128.pow(d);
    if let Some(dot_pos) = s.find('.') {
        let whole: u128 = s[..dot_pos].parse().context("Invalid whole part")?;
        let frac_str = &s[dot_pos + 1..];
        let frac_len = frac_str.len() as u32;
        let frac: u128 = frac_str.parse().context("Invalid fractional part")?;
        if frac_len > d {
            anyhow::bail!("Too many decimal places (max {})", d);
        }
        let frac_scaled = frac * 10u128.pow(d - frac_len);
        Ok(whole * multiplier + frac_scaled)
    } else {
        let whole: u128 = s.parse().context("Invalid integer amount")?;
        Ok(whole * multiplier)
    }
}

/// Build the ABI-encoded calldata for DolomiteMargin.operate()
///
/// operate((address,uint256)[],(uint8,uint256,(bool,uint8,uint8,uint256),uint256,uint256,address,uint256,bytes)[])
/// selector: 0xa67a6a45
///
/// For a single-action operation (deposit or withdraw):
/// - accounts = [(owner, 0)]
/// - actions = [(actionType, accountId=0, AssetAmount{sign, denom=Wei(0), ref=Delta(0), value}, primaryMarketId, 0, otherAddress, 0, 0x)]
pub fn encode_operate(
    owner: &str,
    action_type: u8,  // 0=Deposit, 1=Withdraw
    amount_sign: bool,
    raw_amount: u128,
    market_id: u64,
    other_address: &str, // from (deposit) or to (withdraw)
    max_amount: bool,    // if true, use Target reference (withdraw all)
) -> String {
    // selector
    let selector = "a67a6a45";

    let owner_clean = owner.trim_start_matches("0x").to_lowercase();
    let other_clean = other_address.trim_start_matches("0x").to_lowercase();

    // AssetReference: 0=Delta, 1=Target
    let asset_ref: u8 = if max_amount { 1 } else { 0 };
    // AssetDenomination: 0=Wei, 1=Par
    let asset_denom: u8 = 0;
    // sign as 0 or 1
    let sign_val: u8 = if amount_sign { 1 } else { 0 };
    // value: 0 if max_amount (Target with 0 = full balance)
    let value: u128 = if max_amount { 0 } else { raw_amount };

    // ABI encode operate() with:
    // - accounts: 1 element = (owner, 0)
    // - actions: 1 element = (actionType, 0, AssetAmount{sign,denom,ref,value}, marketId, 0, otherAddr, 0, bytes(""))
    //
    // ABI encoding for operate(AccountInfo[], ActionArgs[]):
    // The function takes two dynamic arrays. ABI tuple encoding:
    //
    // [0..31]   = offset to accounts array = 64 (0x40)
    // [32..63]  = offset to actions array = 64 + 32 + 64 = 160 (0xa0) ... computed after
    //
    // accounts array:
    // [64..95]  = length of accounts = 1
    // [96..127] = accounts[0].owner (padded address)
    // [128..159]= accounts[0].number = 0
    //
    // actions array: ActionArgs is a struct, so it's encoded as a tuple
    // Because ActionArgs contains dynamic bytes field, the array contains offsets to each element
    //
    // Actually for simplicity with the bytes field, we need to use proper ABI encoding.
    // Let's manually build it step by step.

    // The accounts array (1 element):
    // Each AccountInfo is (address, uint256) - static tuple, 2 words
    let accounts_offset: u64 = 64; // 2 words from start
    // accounts array: length (1 word) + 1 element (2 words) = 3 words = 96 bytes
    let accounts_size: u64 = 96;
    let actions_offset: u64 = accounts_offset + accounts_size; // = 160

    // ActionArgs struct has a dynamic bytes field, so the array element is encoded with an internal offset
    // actions array: length (1 word) + offset to element[0] (1 word) + element data
    // Element data for ActionArgs:
    //   actionType:         1 word (uint8 padded)
    //   accountId:          1 word (uint256)
    //   amount.sign:        1 word (bool padded)
    //   amount.denomination:1 word (uint8 padded)
    //   amount.ref:         1 word (uint8 padded)
    //   amount.value:       1 word (uint256)
    //   primaryMarketId:    1 word (uint256)
    //   secondaryMarketId:  1 word (uint256)
    //   otherAddress:       1 word (address padded)
    //   otherAccountId:     1 word (uint256)
    //   data offset:        1 word (offset to bytes data)
    //   -- bytes data --
    //   data length:        1 word (= 0)
    // Total static part: 11 words; data: 1 word (length 0) = 12 words total per element
    // But since bytes is dynamic, the element itself has an internal offset pointer.
    // The offset for data field points to within the element encoding.
    // data offset = 11 * 32 = 352 (0x160) bytes from start of element
    // data itself = length 0 (1 word) = 0 bytes
    // element total = 12 words = 384 bytes

    // actions array encoding:
    // [0] = 1 (length)
    // [1] = 32 (offset to element 0, relative to start of array content after length) = 0x20
    // then element 0 follows

    let data_offset_in_elem: u64 = 11 * 32; // 352 = 0x160

    let mut out = String::new();
    out.push_str(selector);
    // offset to accounts (from end of selector, i.e. from word 0)
    out.push_str(&format!("{:064x}", accounts_offset));
    // offset to actions
    out.push_str(&format!("{:064x}", actions_offset));

    // accounts array
    out.push_str(&format!("{:064x}", 1u64)); // length
    out.push_str(&format!("{:0>64}", owner_clean)); // owner
    out.push_str(&format!("{:064x}", 0u64)); // number = 0

    // actions array
    out.push_str(&format!("{:064x}", 1u64)); // length
    out.push_str(&format!("{:064x}", 32u64)); // offset to element 0 = 0x20 (relative to after length word)

    // ActionArgs element
    out.push_str(&format!("{:064x}", action_type as u64)); // actionType
    out.push_str(&format!("{:064x}", 0u64)); // accountId = 0
    out.push_str(&format!("{:064x}", sign_val as u64)); // amount.sign
    out.push_str(&format!("{:064x}", asset_denom as u64)); // amount.denomination
    out.push_str(&format!("{:064x}", asset_ref as u64)); // amount.ref
    out.push_str(&format!("{:064x}", value)); // amount.value
    out.push_str(&format!("{:064x}", market_id)); // primaryMarketId
    out.push_str(&format!("{:064x}", 0u64)); // secondaryMarketId
    out.push_str(&format!("{:0>64}", other_clean)); // otherAddress
    out.push_str(&format!("{:064x}", 0u64)); // otherAccountId
    out.push_str(&format!("{:064x}", data_offset_in_elem)); // data offset (relative to element start)
    out.push_str(&format!("{:064x}", 0u64)); // data length = 0

    format!("0x{}", out)
}

/// Find market ID for a given token address on DolomiteMargin.
pub async fn find_market_id(
    dolomite_margin: &str,
    token_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<u64> {
    let num = get_num_markets(dolomite_margin, rpc_url).await?;
    let token_lower = token_addr.to_lowercase();
    for i in 0..num {
        let addr = get_market_token_address(dolomite_margin, i, rpc_url).await?;
        if addr.to_lowercase() == token_lower {
            return Ok(i);
        }
    }
    anyhow::bail!("Token {} not found in DolomiteMargin markets", token_addr)
}
