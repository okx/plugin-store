pub mod address;
pub mod cancel;
pub mod cancel_batch;
pub mod close;
pub mod deposit;
pub mod dex_list;
pub mod dex_transfer;
pub mod markets;
pub mod outcome_list;
pub mod outcome_positions;
pub mod usdh_fund;
pub mod outcome_buy;
pub mod outcome_sell;
pub mod outcome_cancel;
pub mod abstraction;
pub mod evm_send;
pub mod get_gas;
pub mod order;
pub mod order_batch;
pub mod orders;
pub mod positions;
pub mod prices;
pub mod register;
pub mod spot_balances;
pub mod spot_cancel;
pub mod spot_order;
pub mod spot_prices;
pub mod tpsl;
pub mod transfer;
pub mod withdraw;
pub mod quickstart;
pub mod delegation_history;
pub mod redelegate;
pub mod stake;
pub mod staking_info;
pub mod staking_rewards;
pub mod unbonding;
pub mod unstake;
pub mod validators;

/// Render a structured error JSON string for stdout output.
/// All command failures must use this instead of anyhow::bail! or ?.
pub fn error_response(msg: &str, code: &str, suggestion: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

pub const AUTOTRADE_FAILURE_SUGGESTION: &str =
    "Report the failure through the execution card's failure branch. \
     Do not fund, modify, or retry this card.";

/// Render a command error without leaking interactive remediation into the autotrade path.
///
/// An execution card is immutable: an agent must not respond to a failed card by changing
/// parameters, moving funds, or retrying it. Interactive callers keep the existing,
/// actionable suggestion.
pub fn execution_error_response(
    msg: &str,
    code: &str,
    interactive_suggestion: &str,
    autotrade: bool,
) -> String {
    error_response(
        msg,
        code,
        if autotrade {
            AUTOTRADE_FAILURE_SUGGESTION
        } else {
            interactive_suggestion
        },
    )
}

pub fn execution_failure_suggestion(
    autotrade: bool,
    interactive_suggestion: &str,
) -> &str {
    if autotrade {
        AUTOTRADE_FAILURE_SUGGESTION
    } else {
        interactive_suggestion
    }
}

/// Rejection for a malformed `--autotrade-job` value.
pub fn autotrade_invalid_input_response() -> String {
    error_response(
        "invalid --autotrade-job: expected 1..=128 characters from [A-Za-z0-9_-]",
        "INVALID_INPUT",
        "Execute the autotrade card verbatim; do not edit the job ID.",
    )
}

/// Fail-closed rejection when the onchainos autotrade grant check does not pass.
pub fn autotrade_grant_denied_response(reason: &str) -> String {
    error_response(
        &format!("autotrade grant denied: {}", reason),
        "AUTOTRADE_GRANT_DENIED",
        "Do not retry and do not re-run without --autotrade-job. \
         Report the failure through the execution card's failure branch.",
    )
}

/// Validate an `--autotrade-job` value at command entry, before any network call or
/// subprocess. Pure input validation, so a malformed job ID costs nothing and the
/// caller can assert zero side effects.
pub fn check_autotrade_job_id(job_id: &str) -> Result<(), String> {
    if crate::onchainos::is_valid_autotrade_job_id(job_id) {
        Ok(())
    } else {
        Err(autotrade_invalid_input_response())
    }
}

#[derive(Clone, Copy, Debug)]
struct PositiveDecimal {
    coefficient: u128,
    scale: u32,
}

fn pow10_u128(scale: u32) -> Option<u128> {
    10_u128.checked_pow(scale)
}

/// Parse a positive fixed-point decimal without going through `f64`.
///
/// Scientific notation and signs are deliberately rejected: execution-card values should
/// be canonical decimal quantities, and accepting multiple syntaxes makes exact
/// grid-alignment checks harder to audit.
fn parse_positive_decimal(input: &str) -> Option<PositiveDecimal> {
    let value = input.trim();
    if value.is_empty() || value.starts_with('+') || value.starts_with('-') {
        return None;
    }

    let mut parts = value.split('.');
    let whole = parts.next()?;
    let fractional = parts.next().unwrap_or("");
    if parts.next().is_some()
        || (whole.is_empty() && fractional.is_empty())
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fractional.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }

    let fractional = fractional.trim_end_matches('0');
    let scale = u32::try_from(fractional.len()).ok()?;
    let factor = pow10_u128(scale)?;
    let whole_value = if whole.is_empty() {
        0
    } else {
        whole.parse::<u128>().ok()?
    };
    let fractional_value = if fractional.is_empty() {
        0
    } else {
        fractional.parse::<u128>().ok()?
    };
    let coefficient = whole_value
        .checked_mul(factor)?
        .checked_add(fractional_value)?;
    if coefficient == 0 {
        return None;
    }

    Some(PositiveDecimal { coefficient, scale })
}

fn format_decimal(value: PositiveDecimal) -> Option<String> {
    let factor = pow10_u128(value.scale)?;
    if value.scale == 0 {
        return Some(value.coefficient.to_string());
    }
    let whole = value.coefficient / factor;
    let fractional = value.coefficient % factor;
    Some(format!(
        "{}.{:0width$}",
        whole,
        fractional,
        width = value.scale as usize
    ))
}

/// Validate that an execution-card size is already on the exchange size grid.
///
/// Interactive orders retain their existing rounding behavior. Autotrade orders are
/// immutable, so a value with more significant decimal places than `sz_decimals` is
/// rejected instead of rounded up or down.
pub fn normalize_autotrade_size(input: &str, sz_decimals: u32) -> Option<String> {
    let parsed = parse_positive_decimal(input)?;
    if parsed.scale > sz_decimals {
        return None;
    }
    format_decimal(parsed)
}

/// Compare two positive decimal strings exactly, ignoring representation-only zeroes.
pub fn positive_decimals_equal(left: &str, right: &str) -> bool {
    let Some(left) = parse_positive_decimal(left) else {
        return false;
    };
    let Some(right) = parse_positive_decimal(right) else {
        return false;
    };
    let common_scale = left.scale.max(right.scale);
    let Some(left_factor) = pow10_u128(common_scale - left.scale) else {
        return false;
    };
    let Some(right_factor) = pow10_u128(common_scale - right.scale) else {
        return false;
    };
    let Some(left_value) = left.coefficient.checked_mul(left_factor) else {
        return false;
    };
    let Some(right_value) = right.coefficient.checked_mul(right_factor) else {
        return false;
    };
    left_value == right_value
}

/// Quote-currency notional to submit for authorization: the most this order can consume.
///
/// The written cap is denominated in quote stablecoin, so submitting a base-unit size
/// would compare e.g. 0.01 (BTC) against a 100 (USDT) cap and clear it unconditionally —
/// the authorization would be bypassed by a unit mismatch, not by a lenient limit. Taking
/// the highest of the candidate prices keeps the figure an upper bound for longs and
/// shorts, market and limit alike: over-stating can only cause a refusal near the cap,
/// while under-stating is a bypass. `None` when no price is usable — the caller must then
/// refuse rather than submit a zero. Fixed-point arithmetic avoids an `f64` boundary
/// rounding down by one cent.
pub fn autotrade_quote_amount(size: &str, candidate_prices: &[&str]) -> Option<String> {
    let size = parse_positive_decimal(size)?;
    let mut max_cents = 0_u128;

    for candidate in candidate_prices {
        let Some(price) = parse_positive_decimal(candidate) else {
            continue;
        };
        let scale = size.scale.checked_add(price.scale)?;
        let denominator = pow10_u128(scale)?;
        let numerator = size
            .coefficient
            .checked_mul(price.coefficient)?
            .checked_mul(100)?;
        let cents = numerator
            .checked_add(denominator.checked_sub(1)?)?
            .checked_div(denominator)?;
        max_cents = max_cents.max(cents);
    }

    if max_cents == 0 {
        return None;
    }
    Some(format!("{}.{:02}", max_cents / 100, max_cents % 100))
}

#[cfg(test)]
mod tests {
    use super::{autotrade_quote_amount, normalize_autotrade_size, positive_decimals_equal};

    #[test]
    fn quote_amount_uses_the_highest_price_and_ceils_to_cents() {
        assert_eq!(
            autotrade_quote_amount("0.00001", &["99999.999", "100000.001"]),
            Some("1.01".to_string())
        );
    }

    #[test]
    fn quote_amount_does_not_round_down_on_a_fractional_cent() {
        assert_eq!(
            autotrade_quote_amount("0.01", &["100000.001"]),
            Some("1000.01".to_string())
        );
    }

    #[test]
    fn size_grid_ignores_representation_only_zeroes() {
        assert_eq!(
            normalize_autotrade_size("1.23000", 2),
            Some("1.23".to_string())
        );
        assert_eq!(normalize_autotrade_size("1.23001", 2), None);
    }

    #[test]
    fn decimal_equality_is_exact() {
        assert!(positive_decimals_equal("100.0", "100"));
        assert!(!positive_decimals_equal("100.00001", "100"));
    }
}

/// Fail-closed rejection when the quote-currency notional cannot be established.
pub fn autotrade_amount_unavailable_response() -> String {
    error_response(
        "cannot determine the quote-currency notional for the autotrade authorization check",
        "AUTOTRADE_GRANT_DENIED",
        AUTOTRADE_FAILURE_SUGGESTION,
    )
}

/// Ask onchainos to authorize this order. Skipped under `--dry-run`, which signs and
/// submits nothing; the caller surfaces that the check was skipped so a dry-run
/// preview is never mistaken for an authorized one.
pub async fn autotrade_gate(
    job_id: &str,
    action: &str,
    amount: &str,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }
    crate::onchainos::autotrade_grant_check(job_id, action, amount)
        .await
        .map_err(|reason| autotrade_grant_denied_response(&reason))
}
