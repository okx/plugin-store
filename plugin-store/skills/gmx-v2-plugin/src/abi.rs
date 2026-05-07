/// ABI encoding utilities for GMX V2 multicall construction

/// Encode a single bytes32 value (already 32 bytes as hex string)
pub fn encode_bytes32(val: &str) -> String {
    let v = val.trim_start_matches("0x");
    format!("{:0>64}", v)
}

/// Encode an address (20 bytes) into 32-byte ABI slot (left-zero-padded)
pub fn encode_address(addr: &str) -> String {
    let a = addr.trim_start_matches("0x");
    format!("{:0>64}", a)
}

/// Encode a uint256 value into 32-byte ABI slot
pub fn encode_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode a bool into 32-byte ABI slot
pub fn encode_bool(val: bool) -> String {
    if val {
        "0000000000000000000000000000000000000000000000000000000000000001".to_string()
    } else {
        "0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }
}

/// Zero address (32 bytes)
pub fn zero_address() -> String {
    "0000000000000000000000000000000000000000000000000000000000000000".to_string()
}

/// Max uint256
pub fn max_uint256() -> u128 {
    u128::MAX
}

/// Encode `sendWnt(address receiver, uint256 amount)` calldata
/// Selector: 0x7d39aaf1
pub fn encode_send_wnt(receiver: &str, amount: u128) -> String {
    let receiver_padded = encode_address(receiver);
    let amount_padded = encode_u256(amount);
    format!("7d39aaf1{}{}", receiver_padded, amount_padded)
}

/// Encode `sendTokens(address token, address receiver, uint256 amount)` calldata
/// Selector: 0xe6d66ac8
pub fn encode_send_tokens(token: &str, receiver: &str, amount: u128) -> String {
    let token_padded = encode_address(token);
    let receiver_padded = encode_address(receiver);
    let amount_padded = encode_u256(amount);
    format!("e6d66ac8{}{}{}", token_padded, receiver_padded, amount_padded)
}

/// Encode `cancelOrder(bytes32 key)` calldata
/// Selector: 0x7489ec23
pub fn encode_cancel_order(key: &str) -> String {
    let key_clean = key.trim_start_matches("0x");
    let key_padded = format!("{:0>64}", key_clean);
    format!("7489ec23{}", key_padded)
}

/// Encode `claimFundingFees(address[] markets, address[] tokens, address receiver)` calldata
/// Selector: 0xc41b1ab3
pub fn encode_claim_funding_fees(markets: &[&str], tokens: &[&str], receiver: &str) -> String {
    // ABI encoding for dynamic arrays:
    // selector (4 bytes) + offset(markets) + offset(tokens) + offset(receiver_param -> but receiver is address, not dynamic)
    // Actually: claimFundingFees(address[],address[],address)
    // Head: offset to markets array, offset to tokens array, receiver address (padded)
    // Then arrays inline

    let head_size = 3 * 32; // 3 slots in head
    let markets_array_size = (1 + markets.len()) * 32; // length + elements

    let offset_markets = head_size; // 0x60
    let offset_tokens = head_size + markets_array_size;

    let mut out = String::from("c41b1ab3");
    // Head
    out.push_str(&encode_u256(offset_markets as u128));
    out.push_str(&encode_u256(offset_tokens as u128));
    out.push_str(&encode_address(receiver));
    // markets array
    out.push_str(&encode_u256(markets.len() as u128));
    for m in markets {
        out.push_str(&encode_address(m));
    }
    // tokens array
    out.push_str(&encode_u256(tokens.len() as u128));
    for t in tokens {
        out.push_str(&encode_address(t));
    }
    out
}

/// Encode `createOrder(CreateOrderParams)` calldata for GMX V2
/// Selector: 0xf59c48eb
///
/// CreateOrderParams (actual deployed struct):
///   addresses: (receiver, cancellationReceiver, callbackContract, uiFeeReceiver, market, initialCollateralToken, swapPath[])
///   numbers:   (sizeDeltaUsd, initialCollateralDeltaAmount, triggerPrice, acceptablePrice, executionFee, callbackGasLimit, minOutputAmount, validFromTime)
///   orderType:                  uint8
///   decreasePositionSwapType:   uint8
///   isLong:                     bool
///   shouldUnwrapNativeToken:    bool
///   autoCancel:                 bool
///   referralCode:               bytes32
///   dataList:                   bytes32[]  (empty)
///
/// ABI sig: createOrder(((address,address,address,address,address,address,address[]),(uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256),uint8,uint8,bool,bool,bool,bytes32,bytes32[]))
#[allow(clippy::too_many_arguments)]
pub fn encode_create_order(
    account: &str,
    receiver: &str,
    market: &str,
    collateral_token: &str,
    order_type: u8,
    size_delta_usd: u128,
    collateral_delta_amount: u128,
    trigger_price: u128,
    acceptable_price: u128,
    execution_fee: u128,
    is_long: bool,
    _src_chain_id: u64,
) -> String {
    // ── Addresses tuple ──────────────────────────────────────────────────────
    // (receiver, cancellationReceiver, callbackContract, uiFeeReceiver, market, initialCollateralToken, swapPath[])
    // 6 static addresses + 1 offset for swapPath (dynamic array, length=0)
    // Head: 7 words (6 addrs + swapPath offset), Tail: swapPath length word (=0)
    // swapPath offset = 7*32 = 224 (relative to start of addresses tuple head[0])
    let swap_path_offset: usize = 7 * 32;
    let mut addr_enc = String::new();
    addr_enc.push_str(&encode_address(receiver));         // receiver
    addr_enc.push_str(&encode_address(account));          // cancellationReceiver = account
    addr_enc.push_str(&zero_address());                   // callbackContract = 0x0
    addr_enc.push_str(&zero_address());                   // uiFeeReceiver = 0x0
    addr_enc.push_str(&encode_address(market));           // market
    addr_enc.push_str(&encode_address(collateral_token)); // initialCollateralToken
    addr_enc.push_str(&encode_u256(swap_path_offset as u128)); // offset to swapPath[]
    addr_enc.push_str(&encode_u256(0));                   // swapPath length = 0
    // addr_enc = 8 words = 256 bytes

    // ── Numbers tuple (8 static uint256 fields, encoded inline) ─────────────
    let mut num_enc = String::new();
    num_enc.push_str(&encode_u256(size_delta_usd));           // sizeDeltaUsd
    num_enc.push_str(&encode_u256(collateral_delta_amount));  // initialCollateralDeltaAmount
    num_enc.push_str(&encode_u256(trigger_price));            // triggerPrice
    num_enc.push_str(&encode_u256(acceptable_price));         // acceptablePrice
    num_enc.push_str(&encode_u256(execution_fee));            // executionFee
    num_enc.push_str(&encode_u256(0));                        // callbackGasLimit = 0
    num_enc.push_str(&encode_u256(0));                        // minOutputAmount = 0
    num_enc.push_str(&encode_u256(0));                        // validFromTime = 0
    // num_enc = 8 words = 256 bytes (static tuple, encoded inline in parent head)

    // ── Top-level struct encoding ─────────────────────────────────────────────
    // The outer struct (CreateOrderParams) has 9 top-level components:
    //   0. addresses   — DYNAMIC  → offset in head
    //   1. numbers     — STATIC   → encoded inline (8 words)
    //   2. orderType   — STATIC   → 1 word
    //   3. decreasePositionSwapType — STATIC → 1 word
    //   4. isLong      — STATIC   → 1 word
    //   5. shouldUnwrapNativeToken — STATIC → 1 word
    //   6. autoCancel  — STATIC   → 1 word
    //   7. referralCode — STATIC  → 1 word
    //   8. dataList    — DYNAMIC  → offset in head
    //
    // Head layout (words): addresses_offset(1) + numbers(8) + orderType(1) +
    //                      decreaseType(1) + isLong(1) + shouldUnwrap(1) +
    //                      autoCancel(1) + referralCode(1) + dataList_offset(1) = 16 words
    // Head size = 16 * 32 = 512 bytes
    //
    // Offsets are relative to start of struct encoding (start of head[0]).
    // addresses tail starts at offset 512 (= head end)
    // dataList tail starts at offset 512 + 256 = 768 (= after addresses 8 words)
    let head_size: usize = 16 * 32; // 512 bytes
    let addr_offset = head_size;                      // 512
    let datalist_offset = head_size + addr_enc.len() / 2; // 512 + 256 = 768

    let mut struct_enc = String::new();
    struct_enc.push_str(&encode_u256(addr_offset as u128));     // [0] addresses offset
    struct_enc.push_str(&num_enc);                               // [1-8] numbers inline (8 words)
    struct_enc.push_str(&encode_u256(order_type as u128));       // [9] orderType
    struct_enc.push_str(&encode_u256(0));                        // [10] decreasePositionSwapType = 0
    struct_enc.push_str(&encode_bool(is_long));                  // [11] isLong
    struct_enc.push_str(&encode_bool(false));                    // [12] shouldUnwrapNativeToken = false
    struct_enc.push_str(&encode_bool(false));                    // [13] autoCancel = false
    struct_enc.push_str(&encode_u256(0));                        // [14] referralCode = bytes32(0)
    struct_enc.push_str(&encode_u256(datalist_offset as u128));  // [15] dataList offset
    // Tail:
    struct_enc.push_str(&addr_enc);                              // addresses tail (256 bytes)
    struct_enc.push_str(&encode_u256(0));                        // dataList length = 0

    // Function: createOrder takes 1 dynamic struct arg → wrap in offset 0x20
    format!("f59c48eb{}{}", encode_u256(0x20), struct_enc)
}

/// Encode `createDeposit(CreateDepositParams)` calldata
///
/// Selector: 0xc82aa41b
/// keccak256("createDeposit(((address,address,address,address,address,address,address[],address[]),uint256,bool,uint256,uint256,bytes32[]))")
/// Verified from deployed ExchangeRouter bytecode (PUSH4 scan on Arbitrum mainnet).
///
/// Flat struct layout (T = outer tuple):
///   T HEAD (6 words = 192 bytes):
///     W0: offset_to_addresses = 192
///     W1: minMarketTokens
///     W2: shouldUnwrapNativeToken = false
///     W3: executionFee
///     W4: callbackGasLimit = 0
///     W5: offset_to_dataList = 192 + 320 = 512
///   addresses tuple (10 words = 320 bytes):
///     receiver, callbackContract=0, uiFeeReceiver=0, market,
///     initialLongToken, initialShortToken,
///     offset_longSwapPath=256, offset_shortSwapPath=288,
///     longSwapPath length=0, shortSwapPath length=0
///   dataList (1 word): length = 0
#[allow(clippy::too_many_arguments)]
pub fn encode_create_deposit(
    receiver: &str,
    _callback_contract: &str,
    _ui_fee_receiver: &str,
    market: &str,
    initial_long_token: &str,
    initial_short_token: &str,
    min_market_tokens: u128,
    execution_fee: u128,
    _src_chain_id: u64,
) -> String {
    // --- addresses tuple (10 words = 320 bytes) ---
    let mut addresses = String::new();
    addresses.push_str(&encode_address(receiver));            // receiver
    addresses.push_str(&zero_address());                      // callbackContract = 0
    addresses.push_str(&zero_address());                      // uiFeeReceiver = 0
    addresses.push_str(&encode_address(market));              // market
    addresses.push_str(&encode_address(initial_long_token));  // initialLongToken
    addresses.push_str(&encode_address(initial_short_token)); // initialShortToken
    addresses.push_str(&encode_u256(256));                    // offset to longSwapPath = A_HEAD_SIZE
    addresses.push_str(&encode_u256(288));                    // offset to shortSwapPath = 256 + 32
    addresses.push_str(&encode_u256(0));                      // longSwapPath length = 0
    addresses.push_str(&encode_u256(0));                      // shortSwapPath length = 0

    // --- T HEAD (6 words = 192 bytes) ---
    const T_HEAD_SIZE: usize = 192;
    const A_SIZE: usize = 320;
    const DATALIST_OFFSET: usize = T_HEAD_SIZE + A_SIZE; // = 512

    let mut t = String::new();
    t.push_str(&encode_u256(T_HEAD_SIZE as u128));     // W0: offset to addresses
    t.push_str(&encode_u256(min_market_tokens));        // W1: minMarketTokens
    t.push_str(&encode_bool(false));                    // W2: shouldUnwrapNativeToken
    t.push_str(&encode_u256(execution_fee));            // W3: executionFee
    t.push_str(&encode_u256(0));                        // W4: callbackGasLimit = 0
    t.push_str(&encode_u256(DATALIST_OFFSET as u128)); // W5: offset to dataList
    t.push_str(&addresses);                             // addresses (320 bytes)
    t.push_str(&encode_u256(0));                        // dataList length = 0

    format!("c82aa41b{}{}", encode_u256(0x20), t)
}

/// Encode `createWithdrawal(CreateWithdrawalParams)` calldata
///
/// Selector: 0xe78dc235
/// keccak256("createWithdrawal(((address,address,address,address,address[],address[]),uint256,uint256,bool,uint256,uint256,bytes32[]))")
/// Verified from deployed ExchangeRouter bytecode (PUSH4 scan on Arbitrum mainnet).
///
/// Flat struct layout (T = outer tuple):
///   T HEAD (7 words = 224 bytes):
///     W0: offset_to_addresses = 224
///     W1: minLongTokenAmount
///     W2: minShortTokenAmount
///     W3: shouldUnwrapNativeToken = false
///     W4: executionFee
///     W5: callbackGasLimit = 0
///     W6: offset_to_dataList = 224 + 256 = 480
///   addresses tuple (8 words = 256 bytes):
///     receiver, callbackContract=0, uiFeeReceiver=0, market,
///     offset_longSwapPath=192, offset_shortSwapPath=224,
///     longSwapPath length=0, shortSwapPath length=0
///   dataList (1 word): length = 0
pub fn encode_create_withdrawal(
    receiver: &str,
    market: &str,
    min_long_token_amount: u128,
    min_short_token_amount: u128,
    execution_fee: u128,
) -> String {
    // --- addresses tuple (8 words = 256 bytes) ---
    let mut addresses = String::new();
    addresses.push_str(&encode_address(receiver)); // receiver
    addresses.push_str(&zero_address());            // callbackContract = 0
    addresses.push_str(&zero_address());            // uiFeeReceiver = 0
    addresses.push_str(&encode_address(market));    // market
    addresses.push_str(&encode_u256(192));          // offset to longSwapPath = A_HEAD_SIZE
    addresses.push_str(&encode_u256(224));          // offset to shortSwapPath = 192 + 32
    addresses.push_str(&encode_u256(0));            // longSwapPath length = 0
    addresses.push_str(&encode_u256(0));            // shortSwapPath length = 0

    // --- T HEAD (7 words = 224 bytes) ---
    const T_HEAD_SIZE: usize = 224;
    const A_SIZE: usize = 256;
    const DATALIST_OFFSET: usize = T_HEAD_SIZE + A_SIZE; // = 480

    let mut t = String::new();
    t.push_str(&encode_u256(T_HEAD_SIZE as u128));      // W0: offset to addresses
    t.push_str(&encode_u256(min_long_token_amount));     // W1
    t.push_str(&encode_u256(min_short_token_amount));    // W2
    t.push_str(&encode_bool(false));                     // W3: shouldUnwrapNativeToken
    t.push_str(&encode_u256(execution_fee));             // W4
    t.push_str(&encode_u256(0));                         // W5: callbackGasLimit = 0
    t.push_str(&encode_u256(DATALIST_OFFSET as u128));   // W6: offset to dataList
    t.push_str(&addresses);                              // addresses (256 bytes)
    t.push_str(&encode_u256(0));                         // dataList length = 0

    format!("e78dc235{}{}", encode_u256(0x20), t)
}

/// Encode the outer `multicall(bytes[])` calldata
/// Selector: 0xac9650d8
pub fn encode_multicall(inner_calls: &[String]) -> String {
    // multicall(bytes[]) — single dynamic array argument
    // Encoding:
    // [selector][offset_to_array=0x20][array_length][offsets_to_each_element][element_data]

    let n = inner_calls.len();

    // Calculate offsets for each bytes element.
    // Per ABI spec: offsets in bytes[] are relative to the start of the FIRST offset word
    // (immediately after the length word), NOT relative to the length word itself.
    // So the first element's offset = n * 32 (just the n offset words).
    // Each bytes element is: 32 (length word) + ceil(data_len/32)*32 (padded data)
    let array_head_size = n * 32; // n offset words only (length word excluded from offset base)

    let mut element_offsets: Vec<usize> = Vec::with_capacity(n);
    let mut element_data: Vec<String> = Vec::with_capacity(n);
    let mut current_offset = array_head_size;

    for call_hex in inner_calls {
        element_offsets.push(current_offset);
        let data_bytes = call_hex.len() / 2; // hex string → byte length
        // Encode: length (32 bytes) + data (padded to 32-byte boundary)
        let padded_len = (data_bytes + 31) / 32 * 32;
        let padded_hex_len = padded_len * 2;
        let padding_chars = padded_hex_len - call_hex.len();
        let data_padded = format!("{}{}", call_hex, "0".repeat(padding_chars));
        let encoded_element = format!("{}{}", encode_u256(data_bytes as u128), data_padded);
        current_offset += encoded_element.len() / 2;
        element_data.push(encoded_element);
    }

    let mut result = String::from("ac9650d8");
    // Outer offset: points to start of bytes[] data = 0x20
    result.push_str(&encode_u256(0x20));
    // Array length
    result.push_str(&encode_u256(n as u128));
    // Offsets to each element (relative to start of array = after length word)
    for &off in &element_offsets {
        // Offset is relative to the start of the array data area (after length word)
        // The array data area starts at offset 0x20 + 0x20 = 0x40 from calldata start (after selector+outer_offset+length)
        // But ABI spec: offsets within the array are relative to the start of the array encoding
        // (which includes the length word itself)
        result.push_str(&encode_u256(off as u128));
    }
    // Element data
    for ed in &element_data {
        result.push_str(ed);
    }

    result
}

/// Convert a U256 price in 30-decimal GMX precision to a human-readable USD string
pub fn price_from_gmx(price_str: &str) -> f64 {
    let price_u128 = if let Ok(v) = price_str.parse::<u128>() {
        v
    } else {
        return 0.0;
    };
    // Price is in 10^30 units; divide by 10^30
    price_u128 as f64 / 1e30
}

/// Compute acceptable price with slippage
/// long: minPrice * (1 - slippage_bps/10000)
/// short: maxPrice * (1 + slippage_bps/10000)
pub fn compute_acceptable_price(price: u128, is_long: bool, slippage_bps: u32) -> u128 {
    let bps = slippage_bps as u128;
    if is_long {
        price.saturating_sub(price * bps / 10_000)
    } else {
        price + price * bps / 10_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_address() {
        let addr = "0x1C3fa76e6E1088bCE750f23a5BFcffa1efEF6A41";
        let encoded = encode_address(addr);
        assert_eq!(encoded.len(), 64);
        assert!(encoded.ends_with("1c3fa76e6e1088bce750f23a5bfcffa1efef6a41") || encoded.to_lowercase().ends_with("1c3fa76e6e1088bce750f23a5bfcffa1efef6a41"));
    }

    #[test]
    fn test_encode_u256() {
        let encoded = encode_u256(1000);
        assert_eq!(encoded.len(), 64);
    }

    #[test]
    fn test_price_from_gmx() {
        let price = "1800000000000000000000000000000000"; // 1800 * 10^30
        let usd = price_from_gmx(price);
        assert!((usd - 1800.0).abs() < 1.0);
    }
}
