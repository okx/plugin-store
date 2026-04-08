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
/// Selector: 0x97aedce2
///
/// CreateOrderParams:
///   Addresses: (account, receiver, cancellationReceiver, callbackContract, uiFeeReceiver, market, initialCollateralToken, swapPath[])
///   Numbers:   (orderType, decreasePositionSwapType, sizeDeltaUsd, initialCollateralDeltaAmount, triggerPrice, acceptablePrice, executionFee, callbackGasLimit, minOutputAmount, updatedAtTime, validFromTime, srcChainId)
///   Flags:     (isLong, shouldUnwrapNativeToken, isFrozen, autoCancel)
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
    src_chain_id: u64,
) -> String {
    // The createOrder function takes a single struct param.
    // ABI-encode as tuple:
    // - Addresses tuple (8 slots: 8 addresses, last is dynamic array swapPath=[])
    // - Numbers tuple (12 slots)
    // - Flags tuple (4 bools)
    //
    // The full ABI for the struct has dynamic elements (swapPath array in Addresses).
    // We encode the struct as a tuple with dynamic tail.
    //
    // Selector: 0x97aedce2
    // ABI: createOrder((address,address,address,address,address,address,address,address[]),(uint8,uint8,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256),(bool,bool,bool,bool))
    //
    // The outer tuple is the single function argument.
    // Top-level: 3 tuple components -> head has 3 offsets (each 32 bytes), then data.
    // But since the top-level is a single struct, the function arg IS the struct.
    // Let's use manual ABI encoding:

    // The createOrder function takes one argument of struct type.
    // We encode it as a tuple (addresses_tuple, numbers_tuple, flags_tuple).
    // Since addresses_tuple contains a dynamic array (swapPath), addresses_tuple is dynamic.
    // numbers_tuple and flags_tuple are static.
    //
    // Function encoding:
    // [selector][offset_to_struct]
    // [struct encoding] = [offset_addr_tuple][offset_num_tuple][offset_flags_tuple][addr_tuple data][num_tuple data][flags_tuple data]
    //
    // Wait — the struct is passed as a single tuple argument.
    // For ABI: function takes (Addresses, Numbers, Flags) as a single tuple param.
    // The outer encoding is: offset to the tuple (32 bytes = 0x20), then the tuple contents.
    //
    // Actually for a struct argument in Solidity ABI:
    // The encoding is: head = [offset_to_tuple_data], then the tuple encoded.
    // Since the struct contains dynamic data (swapPath array), the struct itself is dynamic.
    //
    // Let's build it step by step:

    let mut addresses_encoded = String::new();
    // Addresses tuple:
    // (account, receiver, cancellationReceiver=account, callbackContract=0x0, uiFeeReceiver=0x0, market, initialCollateralToken, swapPath=[])
    // Static slots for 7 addresses + 1 offset for swapPath dynamic array
    // Layout: 7 addresses (7*32) + offset to swapPath (1*32) + swapPath data (1*32 length + 0 elements)
    // The tuple itself is dynamic because of swapPath.

    // Head of addresses tuple: 8 slots
    // First 7 are addresses (static), 8th is offset to swapPath
    let swap_path_offset = 8 * 32usize; // offset within the tuple to the swapPath data
    addresses_encoded.push_str(&encode_address(account));          // account
    addresses_encoded.push_str(&encode_address(receiver));         // receiver
    addresses_encoded.push_str(&encode_address(account));          // cancellationReceiver = account
    addresses_encoded.push_str(&zero_address());                   // callbackContract = 0x0
    addresses_encoded.push_str(&zero_address());                   // uiFeeReceiver = 0x0
    addresses_encoded.push_str(&encode_address(market));           // market
    addresses_encoded.push_str(&encode_address(collateral_token)); // initialCollateralToken
    addresses_encoded.push_str(&encode_u256(swap_path_offset as u128)); // offset to swapPath
    // swapPath data: length=0, no elements
    addresses_encoded.push_str(&encode_u256(0)); // swapPath length = 0

    // Numbers tuple (12 static slots, all uint types):
    let mut numbers_encoded = String::new();
    numbers_encoded.push_str(&encode_u256(order_type as u128));          // orderType
    numbers_encoded.push_str(&encode_u256(0));                           // decreasePositionSwapType = 0
    numbers_encoded.push_str(&encode_u256(size_delta_usd));              // sizeDeltaUsd
    numbers_encoded.push_str(&encode_u256(collateral_delta_amount));     // initialCollateralDeltaAmount
    numbers_encoded.push_str(&encode_u256(trigger_price));               // triggerPrice
    numbers_encoded.push_str(&encode_u256(acceptable_price));            // acceptablePrice
    numbers_encoded.push_str(&encode_u256(execution_fee));       // executionFee
    numbers_encoded.push_str(&encode_u256(0));                           // callbackGasLimit = 0
    numbers_encoded.push_str(&encode_u256(0));                           // minOutputAmount = 0
    numbers_encoded.push_str(&encode_u256(0));                           // updatedAtTime = 0
    numbers_encoded.push_str(&encode_u256(0));                           // validFromTime = 0
    numbers_encoded.push_str(&encode_u256(src_chain_id as u128));        // srcChainId

    // Flags tuple (4 static bools):
    let mut flags_encoded = String::new();
    flags_encoded.push_str(&encode_bool(is_long));   // isLong
    flags_encoded.push_str(&encode_bool(false));     // shouldUnwrapNativeToken = false
    flags_encoded.push_str(&encode_bool(false));     // isFrozen = false
    flags_encoded.push_str(&encode_bool(false));     // autoCancel = false

    // The createOrder function takes a single struct param (dynamic because Addresses is dynamic).
    // Encode the struct as an ABI tuple:
    // Head: offset_to_addresses (32), offset_to_numbers (32), offset_to_flags (32)
    // Addresses is dynamic (contains swapPath), Numbers and Flags are static.
    //
    // Actually in Solidity ABI, a tuple containing a dynamic element is itself dynamic.
    // The top-level argument encoding:
    // [head: 3 offsets][addresses_encoded][numbers_encoded][flags_encoded]
    //
    // Offsets are relative to the start of the tuple data area.
    // Head = 3 * 32 = 96 bytes
    // offset_addr = 96 (0x60) bytes from start of tuple
    // Numbers is static (12 * 32 = 384 bytes), but since we place it after addresses, offset_num depends on addresses size
    // Flags is static (4 * 32 = 128 bytes)
    //
    // Since addresses_encoded is dynamic, we compute its byte length:
    let addr_bytes = addresses_encoded.len() / 2; // hex string → bytes
    let num_bytes = numbers_encoded.len() / 2;

    let offset_addr = 3 * 32usize; // 96 bytes = 0x60
    let offset_num = offset_addr + addr_bytes;
    let offset_flags = offset_num + num_bytes;

    let mut struct_encoding = String::new();
    struct_encoding.push_str(&encode_u256(offset_addr as u128));
    struct_encoding.push_str(&encode_u256(offset_num as u128));
    struct_encoding.push_str(&encode_u256(offset_flags as u128));
    struct_encoding.push_str(&addresses_encoded);
    struct_encoding.push_str(&numbers_encoded);
    struct_encoding.push_str(&flags_encoded);

    // The function takes the struct as a direct argument (not wrapped in another offset)
    // because the function signature already specifies the struct type.
    // However, since the struct is dynamic, the function's ABI encoding wraps it in an offset:
    // [selector][offset_to_struct=0x20][struct_data]
    let struct_bytes = struct_encoding.len() / 2;
    let _ = struct_bytes; // compiler warning suppression

    format!("97aedce2{}{}", encode_u256(0x20), struct_encoding)
}

/// Encode `createDeposit(CreateDepositParams)` calldata
/// Selector: 0xadc567e6 (createDeposit((address,address,address,address,address,address[],address[],uint256,uint256,uint256,uint256,uint256)))
/// We use manual ABI encoding for the struct.
#[allow(clippy::too_many_arguments)]
pub fn encode_create_deposit(
    receiver: &str,
    callback_contract: &str,
    ui_fee_receiver: &str,
    market: &str,
    initial_long_token: &str,
    initial_short_token: &str,
    min_market_tokens: u128,
    execution_fee: u128,
    src_chain_id: u64,
) -> String {
    // createDeposit((Addresses, Numbers, Flags))
    // Addresses: (receiver, callbackContract, uiFeeReceiver, market, initialLongToken, initialShortToken, longTokenSwapPath[], shortTokenSwapPath[])
    // Numbers: (minMarketTokens, executionFee, callbackGasLimit, srcChainId)
    // Flags: (shouldUnwrapNativeToken)
    //
    // Selector: let's use the verified one from design
    // The function signature is complex, so we'll build it piece by piece.

    // Addresses tuple (static head + 2 dynamic arrays):
    // 6 static address slots + offset to longSwapPath + offset to shortSwapPath + 2 empty arrays
    let addr_head_slots = 8usize; // 6 addresses + 2 offsets
    let long_swap_offset = addr_head_slots * 32; // offset to longSwapPath within addr tuple
    let short_swap_offset = long_swap_offset + 32; // 32 bytes for length=0 array

    let mut addr_encoded = String::new();
    addr_encoded.push_str(&encode_address(receiver));
    addr_encoded.push_str(&encode_address(callback_contract));
    addr_encoded.push_str(&encode_address(ui_fee_receiver));
    addr_encoded.push_str(&encode_address(market));
    addr_encoded.push_str(&encode_address(initial_long_token));
    addr_encoded.push_str(&encode_address(initial_short_token));
    addr_encoded.push_str(&encode_u256(long_swap_offset as u128));
    addr_encoded.push_str(&encode_u256(short_swap_offset as u128));
    addr_encoded.push_str(&encode_u256(0)); // longSwapPath length=0
    addr_encoded.push_str(&encode_u256(0)); // shortSwapPath length=0

    // Numbers tuple (4 static slots):
    let mut num_encoded = String::new();
    num_encoded.push_str(&encode_u256(min_market_tokens));
    num_encoded.push_str(&encode_u256(execution_fee));
    num_encoded.push_str(&encode_u256(0)); // callbackGasLimit
    num_encoded.push_str(&encode_u256(src_chain_id as u128));

    // Flags tuple (1 bool):
    let mut flags_encoded = String::new();
    flags_encoded.push_str(&encode_bool(false)); // shouldUnwrapNativeToken

    // Build struct encoding
    let addr_bytes = addr_encoded.len() / 2;
    let num_bytes = num_encoded.len() / 2;
    let offset_addr = 3 * 32usize;
    let offset_num = offset_addr + addr_bytes;
    let offset_flags = offset_num + num_bytes;

    let mut struct_encoding = String::new();
    struct_encoding.push_str(&encode_u256(offset_addr as u128));
    struct_encoding.push_str(&encode_u256(offset_num as u128));
    struct_encoding.push_str(&encode_u256(offset_flags as u128));
    struct_encoding.push_str(&addr_encoded);
    struct_encoding.push_str(&num_encoded);
    struct_encoding.push_str(&flags_encoded);

    // Selector for createDeposit
    // createDeposit((address,address,address,address,address,address,address[],address[],uint256,uint256,uint256,uint256,bool))
    // We use: 0xadc567e6
    format!("adc567e6{}{}", encode_u256(0x20), struct_encoding)
}

/// Encode `createWithdrawal(CreateWithdrawalParams)` calldata
/// Selector: 0x9b8eb9e7
#[allow(clippy::too_many_arguments)]
pub fn encode_create_withdrawal(
    receiver: &str,
    callback_contract: &str,
    ui_fee_receiver: &str,
    market: &str,
    min_long_token_amount: u128,
    min_short_token_amount: u128,
    execution_fee: u128,
    src_chain_id: u64,
) -> String {
    // CreateWithdrawalParams: (receiver, callbackContract, uiFeeReceiver, market, longTokenSwapPath[], shortTokenSwapPath[])
    // Numbers: (minLongTokenAmount, minShortTokenAmount, executionFee, callbackGasLimit, srcChainId)
    // Flags: (shouldUnwrapNativeToken)

    // Addresses tuple
    let addr_head_slots = 6usize; // 4 addresses + 2 offsets for swap paths
    let long_swap_offset = addr_head_slots * 32;
    let short_swap_offset = long_swap_offset + 32;

    let mut addr_encoded = String::new();
    addr_encoded.push_str(&encode_address(receiver));
    addr_encoded.push_str(&encode_address(callback_contract));
    addr_encoded.push_str(&encode_address(ui_fee_receiver));
    addr_encoded.push_str(&encode_address(market));
    addr_encoded.push_str(&encode_u256(long_swap_offset as u128));
    addr_encoded.push_str(&encode_u256(short_swap_offset as u128));
    addr_encoded.push_str(&encode_u256(0)); // longSwapPath length=0
    addr_encoded.push_str(&encode_u256(0)); // shortSwapPath length=0

    // Numbers tuple
    let mut num_encoded = String::new();
    num_encoded.push_str(&encode_u256(min_long_token_amount));
    num_encoded.push_str(&encode_u256(min_short_token_amount));
    num_encoded.push_str(&encode_u256(execution_fee));
    num_encoded.push_str(&encode_u256(0)); // callbackGasLimit
    num_encoded.push_str(&encode_u256(src_chain_id as u128));

    // Flags
    let mut flags_encoded = String::new();
    flags_encoded.push_str(&encode_bool(false)); // shouldUnwrapNativeToken

    let addr_bytes = addr_encoded.len() / 2;
    let num_bytes = num_encoded.len() / 2;
    let offset_addr = 3 * 32usize;
    let offset_num = offset_addr + addr_bytes;
    let offset_flags = offset_num + num_bytes;

    let mut struct_encoding = String::new();
    struct_encoding.push_str(&encode_u256(offset_addr as u128));
    struct_encoding.push_str(&encode_u256(offset_num as u128));
    struct_encoding.push_str(&encode_u256(offset_flags as u128));
    struct_encoding.push_str(&addr_encoded);
    struct_encoding.push_str(&num_encoded);
    struct_encoding.push_str(&flags_encoded);

    // Selector for createWithdrawal: 0x9b8eb9e7
    format!("9b8eb9e7{}{}", encode_u256(0x20), struct_encoding)
}

/// Encode the outer `multicall(bytes[])` calldata
/// Selector: 0xac9650d8
pub fn encode_multicall(inner_calls: &[String]) -> String {
    // multicall(bytes[]) — single dynamic array argument
    // Encoding:
    // [selector][offset_to_array=0x20][array_length][offsets_to_each_element][element_data]

    let n = inner_calls.len();

    // Calculate offsets for each bytes element relative to start of array data
    // Array data starts after: length word (32) + n offset words (n*32)
    // Each bytes element is 32-byte-aligned: 32 (length) + ceil(data_len/32)*32
    let array_head_size = (1 + n) * 32; // length word + n offset words

    let mut element_offsets: Vec<usize> = Vec::with_capacity(n);
    let mut element_data: Vec<String> = Vec::with_capacity(n);
    let mut current_offset = array_head_size;

    for call_hex in inner_calls {
        element_offsets.push(current_offset);
        let data_bytes = call_hex.len() / 2; // hex string → byte length
        // Encode: length (32 bytes) + data (padded to 32-byte boundary)
        let padded_len = (data_bytes + 31) / 32 * 32;
        let padded_hex_len = padded_len * 2;
        let data_padded = format!("{:<0width$}", call_hex, width = padded_hex_len);
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
