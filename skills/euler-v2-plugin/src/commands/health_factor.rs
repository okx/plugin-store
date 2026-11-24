/// `euler-v2-plugin health-factor` — true liquidation buffer using oracle pricing.
///
/// **v0.2 implementation**: computes the real health factor by querying:
///   1. EVC for the user's enabled collaterals + controllers
///   2. Each controller for asset, unitOfAccount, oracle, debtOf(user), LTVBorrow per collateral
///   3. Each collateral for balanceOf(user), previewRedeem(shares), asset
///   4. The controller's oracle for `getQuote(amount, asset, unitOfAccount)` per position
///   5. Computes `HF = sum(collateral_quote × LTVBorrow / 10000) / debt_quote`
///
/// Two on-chain Multicall3 round-trips (one for metadata + balances, one for oracle quotes).
///
/// Status output:
///   - `no_position`     — no enabled collateral/controller, no debt
///   - `no_borrow`       — has supply position but no controller
///   - `safe`            — HF >= 1.5
///   - `at_risk`         — 1.0 <= HF < 1.5
///   - `liquidatable`    — HF < 1.0
///   - `multiple_controllers` — Euler v2 normally allows just 1 active controller;
///                              if EVC reports >1, we surface them rather than try to
///                              cross-aggregate (deferred to v0.3)

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};
use crate::multicall::{aggregate3, Call3};
use crate::rpc::{build_address_call, build_preview_redeem,
    SELECTOR_BALANCE_OF, SELECTOR_DEBT_OF};

/// EVC `getCollaterals(address)` selector
const SEL_GET_COLLATERALS: &str = "a4d25d1e";
/// EVC `getControllers(address)` selector
const SEL_GET_CONTROLLERS: &str = "fd6046d7";
/// EVK `asset()` selector
const SEL_ASSET: &str = "38d52e0f";
/// EVK `unitOfAccount()` selector
const SEL_UNIT_OF_ACCOUNT: &str = "3e833364";
/// EVK `oracle()` selector
const SEL_ORACLE: &str = "7dc0d1d0";
/// EVK `LTVBorrow(address collateral)` selector — returns uint16 LTV in basis-points-ish
const SEL_LTV_BORROW: &str = "bf58094d";
/// IPriceOracle `getQuote(uint256 amount, address base, address quote)` selector
const SEL_GET_QUOTE: &str = "ae68676c";

#[derive(Args)]
pub struct HealthFactorArgs {
    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: HealthFactorArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("health-factor"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: HealthFactorArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1.", args.chain);
    }
    let wallet = match args.address {
        Some(a) => a.to_lowercase(),
        None    => crate::onchainos::get_wallet_address(args.chain).await?.to_lowercase(),
    };

    // Look up the EVC address from /api/euler-chains.
    let chain_info = crate::api::get_chain(args.chain).await?;
    let evc = chain_info.addresses.core_addrs.evc.clone()
        .ok_or_else(|| anyhow::anyhow!("EVC address missing for chain {}", args.chain))?
        .to_lowercase();

    // ── Round 1: get the user's enabled collaterals + controllers from EVC ──
    let pad = crate::rpc::pad_address(&wallet);
    let r1 = aggregate3(args.chain, &[
        Call3 { target: evc.clone(), allow_failure: false,
                calldata: format!("0x{}{}", SEL_GET_COLLATERALS, pad) },
        Call3 { target: evc.clone(), allow_failure: false,
                calldata: format!("0x{}{}", SEL_GET_CONTROLLERS, pad) },
    ]).await?;
    let collaterals = parse_address_array(&r1[0].return_data);
    let controllers = parse_address_array(&r1[1].return_data);

    // No controller → no debt → trivially safe (or no position at all).
    if controllers.is_empty() {
        let status = if collaterals.is_empty() { "no_position" } else { "no_borrow" };
        let tip = if status == "no_position" {
            "No Euler positions on this chain. Run `list-vaults` to browse.".to_string()
        } else {
            format!("{} collateral position(s) enabled, no active borrow. Health factor is effectively infinity.", collaterals.len())
        };
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet, "chain": chain_name(args.chain), "chain_id": args.chain,
                "status": status,
                "collaterals_enabled": collaterals,
                "controllers_enabled": controllers,
                "health_factor": null,
                "tip": tip,
            }
        }))?);
        return Ok(());
    }

    // Multiple controllers is unusual in Euler v2; surface them instead of trying to
    // cross-aggregate HF (deferred to v0.3).
    if controllers.len() > 1 {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet, "chain": chain_name(args.chain), "chain_id": args.chain,
                "status": "multiple_controllers",
                "controllers_enabled": controllers,
                "health_factor": null,
                "tip": "Multiple active controllers detected. v0.2 single-controller HF computation \
                        does not aggregate across them; check each via Euler app or repay one to simplify.",
            }
        }))?);
        return Ok(());
    }
    let controller = &controllers[0];

    // ── Round 2: gather all read data needed for HF in a single multicall ──
    //
    // Per controller (1):    asset, unitOfAccount, oracle, debtOf(user)
    // Per collateral (N):    asset, controller.LTVBorrow(this), balanceOf(user)
    //
    // shares→assets (previewRedeem) needs the share count first, so we defer it
    // to a follow-up multicall once we know what shares the user has.
    let mut calls: Vec<Call3> = Vec::new();
    // Controller metadata + debt (4 calls)
    calls.push(Call3 { target: controller.clone(), allow_failure: false,
                      calldata: format!("0x{}", SEL_ASSET) });
    calls.push(Call3 { target: controller.clone(), allow_failure: false,
                      calldata: format!("0x{}", SEL_UNIT_OF_ACCOUNT) });
    calls.push(Call3 { target: controller.clone(), allow_failure: false,
                      calldata: format!("0x{}", SEL_ORACLE) });
    calls.push(Call3 { target: controller.clone(), allow_failure: false,
                      calldata: build_address_call(SELECTOR_DEBT_OF, &wallet) });
    // Per collateral: asset + LTVBorrow on controller + balanceOf (3N calls)
    for c in &collaterals {
        calls.push(Call3 { target: c.clone(), allow_failure: true,
                          calldata: format!("0x{}", SEL_ASSET) });
        calls.push(Call3 { target: controller.clone(), allow_failure: true,
                          calldata: format!("0x{}{}", SEL_LTV_BORROW,
                                            crate::rpc::pad_address(c)) });
        calls.push(Call3 { target: c.clone(), allow_failure: true,
                          calldata: build_address_call(SELECTOR_BALANCE_OF, &wallet) });
    }
    let r2 = aggregate3(args.chain, &calls).await?;

    let controller_asset      = r2[0].as_address().unwrap_or_default();
    let unit_of_account       = r2[1].as_address().unwrap_or_default();
    let oracle                = r2[2].as_address().unwrap_or_default();
    // EVM-012: debtOf is the canonical debt read for HF. If the sub-call
    // reverts (RPC issue / controller misbehavior), silent fallback to 0
    // would render below as `debt_in_uoa == 0 → HF = INFINITY` — misleading
    // users into thinking they're safe when their debt couldn't be read.
    // Fail closed: propagate as RPC_ERROR (the run() wrapper classifies it
    // via classify_error in commands/mod.rs).
    let debt_amount = match r2[3].as_u128() {
        Some(v) => v,
        None => anyhow::bail!(
            "RPC request failed: controller {} debtOf({}) returned no data on chain {} \
             (multicall sub-call reverted). Health factor cannot be reported without \
             an authoritative debt read.",
            controller, wallet, args.chain
        ),
    };

    // Per-collateral data
    struct CollInfo {
        addr:        String,
        asset:       String,
        ltv_borrow:  u128, // basis points × 1, 10000 = 100% (sometimes scaled up to 4e4 in EVK; 8400=84%)
        shares:      u128,
    }
    let mut coll_infos: Vec<CollInfo> = Vec::new();
    for (i, c) in collaterals.iter().enumerate() {
        let base = 4 + i * 3;
        let asset = r2[base].as_address().unwrap_or_default();
        let ltv   = r2[base + 1].as_u128().unwrap_or(0);
        let shares = r2[base + 2].as_u128().unwrap_or(0);
        // Skip collaterals that aren't accepted by the controller (LTV = 0) or have no shares
        if ltv == 0 || shares == 0 { continue; }
        coll_infos.push(CollInfo { addr: c.clone(), asset, ltv_borrow: ltv, shares });
    }

    if coll_infos.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet, "chain": chain_name(args.chain), "chain_id": args.chain,
                "status": "uncollateralized_borrow",
                "controller": controller, "controller_debt_raw": debt_amount.to_string(),
                "collaterals_enabled": collaterals,
                "health_factor": 0.0,
                "tip": "You have an active borrow but no eligible collateral with non-zero LTV. \
                        Position is undercollateralized — repay or add accepted collateral.",
            }
        }))?);
        return Ok(());
    }

    // ── Round 3: previewRedeem per collateral (need each one's share count) +
    //              oracle.getQuote per (collateral_asset → unitOfAccount) +
    //              oracle.getQuote(debt → unitOfAccount)                    ──
    let mut calls: Vec<Call3> = Vec::new();
    for ci in &coll_infos {
        calls.push(Call3 { target: ci.addr.clone(), allow_failure: true,
                          calldata: build_preview_redeem(ci.shares) });
    }
    // After previewRedeem, we still need to encode getQuote calls — but those depend
    // on previewRedeem results. So do this in one more pass.
    let preview_rs = aggregate3(args.chain, &calls).await?;

    let mut quote_calls: Vec<Call3> = Vec::with_capacity(coll_infos.len() + 1);
    let mut underlying_amounts: Vec<u128> = Vec::with_capacity(coll_infos.len());
    for (i, ci) in coll_infos.iter().enumerate() {
        let assets_amount = preview_rs[i].as_u128().unwrap_or(0);
        underlying_amounts.push(assets_amount);
        if assets_amount == 0 { continue; }
        quote_calls.push(Call3 {
            target: oracle.clone(),
            allow_failure: true,
            calldata: format!("0x{}{}{}{}",
                SEL_GET_QUOTE,
                format!("{:064x}", assets_amount),
                crate::rpc::pad_address(&ci.asset),
                crate::rpc::pad_address(&unit_of_account)),
        });
    }
    // Quote the debt as well
    if debt_amount > 0 {
        quote_calls.push(Call3 {
            target: oracle.clone(),
            allow_failure: true,
            calldata: format!("0x{}{}{}{}",
                SEL_GET_QUOTE,
                format!("{:064x}", debt_amount),
                crate::rpc::pad_address(&controller_asset),
                crate::rpc::pad_address(&unit_of_account)),
        });
    }
    let quote_rs = aggregate3(args.chain, &quote_calls).await?;

    // Compute HF
    let mut total_collateral_credit_uoa: u128 = 0;
    let mut quote_iter = quote_rs.into_iter();
    let mut coll_breakdown = Vec::with_capacity(coll_infos.len());
    for (i, ci) in coll_infos.iter().enumerate() {
        let assets_amount = underlying_amounts[i];
        if assets_amount == 0 {
            coll_breakdown.push(serde_json::json!({
                "collateral":         ci.addr,
                "shares_raw":         ci.shares.to_string(),
                "underlying_assets_raw": "0",
                "ltv_borrow_bps":     ci.ltv_borrow,
                "value_in_uoa_raw":   "0",
                "credit_in_uoa_raw":  "0",
            }));
            continue;
        }
        let q = quote_iter.next().and_then(|r| r.as_u128()).unwrap_or(0);
        // Credit = quote × LTV / 10000 (LTV is in bps; 10000 = 100%)
        let credit = q.saturating_mul(ci.ltv_borrow) / 10_000;
        total_collateral_credit_uoa = total_collateral_credit_uoa.saturating_add(credit);
        coll_breakdown.push(serde_json::json!({
            "collateral":           ci.addr,
            "shares_raw":           ci.shares.to_string(),
            "underlying_assets_raw": assets_amount.to_string(),
            "ltv_borrow_bps":       ci.ltv_borrow,
            "value_in_uoa_raw":     q.to_string(),
            "credit_in_uoa_raw":    credit.to_string(),
        }));
    }
    // EVM-012: debt oracle quote is critical for HF correctness. Silent
    // fallback to 0 here would render HF = collateral / 0 → INFINITY, telling
    // users they're safe when in fact the oracle just couldn't price their debt.
    // Fail closed: propagate as RPC_ERROR.
    let debt_in_uoa = if debt_amount > 0 {
        match quote_iter.next().and_then(|r| r.as_u128()) {
            Some(v) => v,
            None => anyhow::bail!(
                "RPC request failed: oracle quote for debt asset {} → unitOfAccount {} \
                 returned no data on chain {}. Health factor cannot be computed without \
                 an authoritative debt-asset price.",
                controller_asset, unit_of_account, args.chain
            ),
        }
    } else { 0 };

    // HF = collateral_credit / debt_value (ratio in unit-of-account terms)
    let hf: f64 = if debt_in_uoa == 0 {
        f64::INFINITY
    } else {
        total_collateral_credit_uoa as f64 / debt_in_uoa as f64
    };

    let (status, tip) = if hf == f64::INFINITY {
        ("safe", "No active debt — health factor is infinity.".to_string())
    } else if hf < 1.0 {
        ("liquidatable",
         format!("⚠ Health factor {:.4} — liquidation imminent. Repay debt or add collateral immediately.", hf))
    } else if hf < 1.5 {
        ("at_risk",
         format!("Health factor {:.4} — close to liquidation threshold. Consider repaying or topping up.", hf))
    } else {
        ("safe", format!("Health factor {:.4} — safely above the liquidation threshold (1.0).", hf))
    };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "wallet": wallet, "chain": chain_name(args.chain), "chain_id": args.chain,
            "status": status,
            "controller": controller,
            "controller_asset": controller_asset,
            "unit_of_account": unit_of_account,
            "oracle": oracle,
            "debt_raw": debt_amount.to_string(),
            "debt_in_uoa_raw": debt_in_uoa.to_string(),
            "collateral_credit_uoa_raw": total_collateral_credit_uoa.to_string(),
            "collaterals": coll_breakdown,
            "health_factor": if hf == f64::INFINITY { serde_json::Value::Null }
                             else { serde_json::Value::from(hf) },
            "tip": tip,
            "rpc_calls": 3,  // EVC list + metadata-bundle + previewRedeem-bundle (+ quote-bundle merged into the third)
        }
    }))?);
    Ok(())
}

/// Decode an ABI-encoded `address[]` result.
fn parse_address_array(data: &[u8]) -> Vec<String> {
    if data.len() < 64 { return Vec::new(); }
    // [0..32]  offset (always 0x20)
    // [32..64] length
    let n = u64::from_be_bytes(data[56..64].try_into().unwrap()) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let start = 64 + i * 32 + 12; // address is right-aligned in 32-byte word
        if start + 20 > data.len() { break; }
        out.push(format!("0x{}", hex::encode(&data[start..start + 20])));
    }
    out
}
