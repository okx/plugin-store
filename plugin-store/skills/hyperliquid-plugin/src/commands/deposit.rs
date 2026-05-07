use clap::Args;
use sha3::{Digest, Keccak256};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::config::{ARBITRUM_CHAIN_ID, HL_BRIDGE_ARBITRUM, USDC_ARBITRUM};
use crate::onchainos::{resolve_wallet, wallet_contract_call, onchainos_sign_eip712};
use crate::rpc::{ARBITRUM_RPC, erc20_balance, usdc_permit_nonce, pad_address, pad_u256};

#[derive(Args)]
pub struct DepositArgs {
    /// USDC amount to deposit (e.g. 100 for $100 USDC)
    #[arg(long)]
    pub amount: f64,

    /// Dry run — show calldata without submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and broadcast (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

/// Build ABI-encoded calldata for batchedDepositWithPermit([(user, usdc_units, deadline, (r,s,v))])
///
/// ABI encoding for batchedDepositWithPermit((address,uint64,uint64,(uint256,uint256,uint8))[]):
///   - array offset (0x20)
///   - array length (1)
///   - tuple data:
///     - address (32 bytes)
///     - uint64 amount (32 bytes)
///     - uint64 deadline (32 bytes)
///     - offset to sig tuple (0x80 = 128 bytes from start of tuple)
///     - r (32 bytes)
///     - s (32 bytes)
///     - v (32 bytes, uint8 padded)
fn build_batched_deposit_calldata(
    user: &str,
    usdc_units: u64,
    deadline: u64,
    r: &str,   // 32-byte hex, no 0x
    s: &str,   // 32-byte hex, no 0x
    v: u8,
) -> String {
    let mut h = Keccak256::new();
    h.update(b"batchedDepositWithPermit((address,uint64,uint64,(uint256,uint256,uint8))[])");
    let selector = hex::encode(&h.finalize()[..4]);

    // Tuple has 5 static slots + 3 sig slots = 8 slots
    // But the sig is a dynamic-within-static sub-tuple (fixed size), encoded inline
    let addr_padded = pad_address(user);
    let amount_padded = pad_u256(usdc_units as u128);
    let deadline_padded = pad_u256(deadline as u128);
    let r_padded = format!("{:0>64}", r.trim_start_matches("0x"));
    let s_padded = format!("{:0>64}", s.trim_start_matches("0x"));
    let v_padded = pad_u256(v as u128);

    // Array wrapper: offset=0x20, length=1
    let arr_offset = pad_u256(0x20_u128);
    let arr_len = pad_u256(1_u128);

    // Static tuple: all fields encoded inline — no sig_offset pointer needed
    // Layout: selector | arr_offset | arr_len | addr | amount | deadline | r | s | v
    format!(
        "0x{}{}{}{}{}{}{}{}{}",
        selector,
        arr_offset,
        arr_len,
        addr_padded,
        amount_padded,
        deadline_padded,
        r_padded,
        s_padded,
        v_padded,
    )
}

pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    if args.amount <= 0.0 {
        println!("{}", super::error_response("Amount must be greater than 0", "INVALID_ARGUMENT", "Provide a positive USDC amount with --amount."));
        return Ok(());
    }
    if args.amount < 5.0 {
        eprintln!("WARNING: Minimum recommended deposit is $5 USDC. Amounts below $5 may not arrive.");
    }

    // USDC has 6 decimals
    let usdc_units = (args.amount * 1_000_000.0).round() as u64;
    let usdc_u128 = usdc_units as u128;

    let wallet = match resolve_wallet(ARBITRUM_CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };

    // Permit deadline: now + 1 hour
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let deadline = now + 3600;

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "wallet": wallet,
            "amount_usd": args.amount,
            "usdc_units": usdc_units,
            "bridge": HL_BRIDGE_ARBITRUM,
            "usdc_token": USDC_ARBITRUM,
            "chain": ARBITRUM_CHAIN_ID,
            "mechanism": "batchedDepositWithPermit with EIP-2612 permit",
            "note": "Dry run — verify parameters before executing with --confirm"
        }));
        return Ok(());
    }

    // Check USDC balance on Arbitrum
    let balance = match erc20_balance(USDC_ARBITRUM, &wallet, ARBITRUM_RPC).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let balance_usd = balance as f64 / 1_000_000.0;
    if balance < usdc_u128 {
        println!("{}", super::error_response(
            &format!("Insufficient USDC on Arbitrum: have {:.6} USDC, need {:.6} USDC", balance_usd, args.amount),
            "INSUFFICIENT_BALANCE",
            "Add USDC to your Arbitrum wallet before depositing."
        ));
        return Ok(());
    }

    // Get USDC permit nonce
    let permit_nonce = match usdc_permit_nonce(USDC_ARBITRUM, &wallet, ARBITRUM_RPC).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    if !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": true,
            "wallet": wallet,
            "amount_usd": args.amount,
            "usdc_units": usdc_units,
            "usdc_balance": format!("{:.6}", balance_usd),
            "permit_nonce": permit_nonce,
            "deadline": deadline,
            "bridge": HL_BRIDGE_ARBITRUM,
            "chain": "arbitrum",
            "note": "Add --confirm to sign permit and execute deposit"
        }));
        return Ok(());
    }

    // Step 1: Build EIP-2612 permit typed data for USDC on Arbitrum
    let permit_typed_data = serde_json::json!({
        "domain": {
            "name": "USD Coin",
            "version": "2",
            "chainId": ARBITRUM_CHAIN_ID,
            "verifyingContract": USDC_ARBITRUM
        },
        "types": {
            "EIP712Domain": [
                { "name": "name",              "type": "string"  },
                { "name": "version",           "type": "string"  },
                { "name": "chainId",           "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ],
            "Permit": [
                { "name": "owner",    "type": "address" },
                { "name": "spender",  "type": "address" },
                { "name": "value",    "type": "uint256" },
                { "name": "nonce",    "type": "uint256" },
                { "name": "deadline", "type": "uint256" }
            ]
        },
        "primaryType": "Permit",
        "message": {
            "owner":    wallet.clone(),
            "spender":  HL_BRIDGE_ARBITRUM,
            "value":    usdc_units,
            "nonce":    permit_nonce,
            "deadline": deadline
        }
    });

    // Step 2: Sign the permit via onchainos
    eprintln!("Signing USDC permit for {} USDC...", args.amount);
    let sig_hex = match onchainos_sign_eip712(&permit_typed_data, &wallet) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

    // Parse r, s, v from the 65-byte hex signature
    let sig_hex = sig_hex.trim_start_matches("0x");
    if sig_hex.len() != 130 {
        println!("{}", super::error_response(
            &format!("Expected 130-char hex signature, got {}", sig_hex.len()),
            "SIGNING_FAILED",
            "Retry the command. If the issue persists, check onchainos status."
        ));
        return Ok(());
    }
    let r = &sig_hex[0..64];
    let s = &sig_hex[64..128];
    let v = match u8::from_str_radix(&sig_hex[128..130], 16) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("Failed to parse signature v byte: {:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

    // Step 3: Build batchedDepositWithPermit calldata
    let calldata = build_batched_deposit_calldata(&wallet, usdc_units, deadline, r, s, v);

    // Step 4: Submit the transaction
    eprintln!("Depositing {:.6} USDC to Hyperliquid via Arbitrum bridge...", args.amount);
    let deposit_result = match wallet_contract_call(
        ARBITRUM_CHAIN_ID,
        HL_BRIDGE_ARBITRUM,
        &calldata,
        None,
        false,
    ) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

    let deposit_tx_hash = deposit_result["data"]["txHash"]
        .as_str()
        .unwrap_or("pending");

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "deposit",
        "wallet": wallet,
        "amount_usd": args.amount,
        "usdc_units": usdc_units,
        "bridge": HL_BRIDGE_ARBITRUM,
        "depositTxHash": deposit_tx_hash,
        "note": "USDC bridging from Arbitrum to Hyperliquid typically takes 2-5 minutes."
    }));

    Ok(())
}
