use serde_json::json;

use crate::config::{format_units, CHAIN_ID};
use crate::onchainos::{gas_limit, gas_price_wei, wallet_balance};

pub mod claim_withdraw;
pub mod instant_withdraw;
pub mod positions;
pub mod quickstart;
pub mod rate;
pub mod request_withdraw;
pub mod stake;
pub mod withdraw_options;
pub mod withdraw_status;

/// Outcome of the gas pre-flight check, attached to each write command's preview/success JSON.
#[derive(Debug)]
pub struct GasEstimate {
    pub gas_units: u128,
    pub gas_price_wei: u128,
    pub estimated_fee_wei: u128,
    pub wallet_eth_balance_wei: u128,
    pub required_eth_wei: u128,
}

impl GasEstimate {
    /// Produce a stable JSON shape so external agents can read `gas_check.required_eth` etc.
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "gas_units": self.gas_units,
            "gas_price_gwei": format_units(self.gas_price_wei, 9),
            "estimated_fee_eth": format_units(self.estimated_fee_wei, 18),
            "estimated_fee_wei": self.estimated_fee_wei.to_string(),
            "wallet_eth_balance": format_units(self.wallet_eth_balance_wei, 18),
            "wallet_eth_balance_raw": self.wallet_eth_balance_wei.to_string(),
            "required_eth": format_units(self.required_eth_wei, 18),
            "required_eth_raw": self.required_eth_wei.to_string(),
        })
    }
}

/// Pre-flight check: estimate gas, compare wallet ETH balance to `value_wei + estimated_fee`.
/// Bails with an actionable error if the wallet can't afford the combined cost.
///
/// `value_wei` is the ETH sent as msg.value (0 for non-payable calls).
///
/// The check multiplies estimated gas by a small 1.2× safety buffer since gas price can
/// move between now and broadcast, and estimateGas itself can undershoot by a few percent.
pub async fn check_gas_budget(
    from: &str,
    to: &str,
    data: &str,
    value_wei: u128,
    _rpc_url: &str, // retained for signature compatibility; onchainos does not need it
) -> anyhow::Result<GasEstimate> {
    // gas limit + gas price + wallet ETH balance all via onchainos (uses OKX backend,
    // stays consistent with the broadcast path).
    let gas_units = gas_limit("ethereum", from, to, value_wei, data).await?;
    let gas_price = gas_price_wei("ethereum").await?;
    let fee_wei = gas_units
        .checked_mul(gas_price)
        .ok_or_else(|| anyhow::anyhow!("overflow computing gas fee: gas={}, price={}", gas_units, gas_price))?
        .checked_mul(120)
        .ok_or_else(|| anyhow::anyhow!("overflow applying gas buffer"))?
        / 100;
    // Native ETH balance of the connected wallet — pass None to match native token.
    let wallet_eth = wallet_balance(CHAIN_ID, None, false).await?;
    let required = value_wei
        .checked_add(fee_wei)
        .ok_or_else(|| anyhow::anyhow!("overflow computing required ETH"))?;
    if wallet_eth < required {
        let shortfall = required - wallet_eth;
        anyhow::bail!(
            "INSUFFICIENT_GAS: wallet ETH {} < required {} (value {} + gas {} at {} gwei × {} units). Short by {} ETH.",
            format_units(wallet_eth, 18),
            format_units(required, 18),
            format_units(value_wei, 18),
            format_units(fee_wei, 18),
            format_units(gas_price, 9),
            gas_units,
            format_units(shortfall, 18),
        );
    }
    Ok(GasEstimate {
        gas_units,
        gas_price_wei: gas_price,
        estimated_fee_wei: fee_wei,
        wallet_eth_balance_wei: wallet_eth,
        required_eth_wei: required,
    })
}

/// Static-cap variant: when the main call depends on state (e.g. allowance) that hasn't been
/// established yet, `eth_estimateGas` would revert. Callers pass a conservative cap instead.
pub async fn check_gas_budget_cap(
    _from: &str,
    gas_cap_units: u128,
    value_wei: u128,
    _rpc_url: &str,
) -> anyhow::Result<GasEstimate> {
    let gas_price = gas_price_wei("ethereum").await?;
    let fee_wei = gas_cap_units
        .checked_mul(gas_price)
        .ok_or_else(|| anyhow::anyhow!("overflow computing gas fee: gas={}, price={}", gas_cap_units, gas_price))?
        .checked_mul(120)
        .ok_or_else(|| anyhow::anyhow!("overflow applying gas buffer"))?
        / 100;
    let wallet_eth = wallet_balance(CHAIN_ID, None, false).await?;
    let required = value_wei
        .checked_add(fee_wei)
        .ok_or_else(|| anyhow::anyhow!("overflow computing required ETH"))?;
    if wallet_eth < required {
        let shortfall = required - wallet_eth;
        anyhow::bail!(
            "INSUFFICIENT_GAS: wallet ETH {} < required {} (value {} + gas cap {} at {} gwei × {} units). Short by {} ETH.",
            format_units(wallet_eth, 18),
            format_units(required, 18),
            format_units(value_wei, 18),
            format_units(fee_wei, 18),
            format_units(gas_price, 9),
            gas_cap_units,
            format_units(shortfall, 18),
        );
    }
    Ok(GasEstimate {
        gas_units: gas_cap_units,
        gas_price_wei: gas_price,
        estimated_fee_wei: fee_wei,
        wallet_eth_balance_wei: wallet_eth,
        required_eth_wei: required,
    })
}

/// Structured error response (→ GEN-001).
/// Every command prints this to **stdout** (not stderr) and returns Ok(()) so an
/// external agent can parse the JSON and decide the next step instead of seeing a
/// generic exit-code-1 failure.
pub fn error_response(err: &anyhow::Error, context: Option<&str>) -> String {
    let msg = format!("{:#}", err);
    let (error_code, suggestion) = classify_error(&msg, context);
    serde_json::to_string_pretty(&json!({
        "ok": false,
        "error": msg,
        "error_code": error_code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

fn classify_error(msg: &str, ctx: Option<&str>) -> (&'static str, String) {
    let lower = msg.to_lowercase();
    if lower.contains("insufficient") && lower.contains("balance") {
        return (
            "INSUFFICIENT_BALANCE",
            "Wallet balance is below the requested amount. Top up or reduce the amount.".into(),
        );
    }
    if lower.contains("withdrawalamounttoolow") || lower.contains("min_withdrawal_amount") {
        return (
            "WITHDRAWAL_AMOUNT_TOO_LOW",
            "2-step withdrawals require at least 0.01 pufETH. Increase the amount or use instant-withdraw.".into(),
        );
    }
    if lower.contains("withdrawalamounttoohigh") || lower.contains("max_withdrawal_amount") || lower.contains("exceeds max withdrawal") {
        return (
            "WITHDRAWAL_AMOUNT_TOO_HIGH",
            "Requested amount exceeds the 2-step per-request maximum. Split into smaller requests or use instant-withdraw.".into(),
        );
    }
    if lower.contains("insufficient_gas")
        || lower.contains("insufficient eth")
        || lower.contains("insufficient funds")
    {
        return (
            "INSUFFICIENT_GAS",
            "Wallet does not hold enough ETH to cover gas (plus any value sent). Top up ETH on Ethereum mainnet.".into(),
        );
    }
    if lower.contains("eth_estimategas revert") {
        return (
            "TX_WILL_REVERT",
            "The transaction would revert on-chain (gas estimation failed). See `error` for the revert reason; re-check amount, allowance, or state.".into(),
        );
    }
    if lower.contains("notyetfinalized") || lower.contains("not yet finalized") || lower.contains("batchnotfinalized") {
        return (
            "WITHDRAWAL_NOT_FINALIZED",
            "The batch is not yet finalized (~14 days). Run withdraw-status --id <idx> to poll.".into(),
        );
    }
    if lower.contains("alreadyclaimed")
        || lower.contains("already claimed")
        || lower.contains("alreadycompleted")
        || lower.contains("already been claimed")
    {
        return (
            "WITHDRAWAL_ALREADY_CLAIMED",
            "This withdrawal has already been claimed.".into(),
        );
    }
    if lower.contains("does not exist") || lower.contains("out of range") {
        return (
            "WITHDRAWAL_OUT_OF_RANGE",
            "No withdrawal exists at this index. Check the id returned by request-withdraw.".into(),
        );
    }
    if lower.contains("timeout") {
        return (
            "TX_CONFIRMATION_TIMEOUT",
            "Transaction did not confirm in time. Check onchainos wallet history manually.".into(),
        );
    }
    if lower.contains("rpc") || lower.contains("eth_call") {
        return (
            "RPC_ERROR",
            "RPC node returned an error. Retry after a few seconds.".into(),
        );
    }
    let suggestion = match ctx {
        Some(c) => format!("See error field; context: {}", c),
        None => "See error field for details.".into(),
    };
    ("UNKNOWN_ERROR", suggestion)
}
