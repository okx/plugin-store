#!/usr/bin/env python3
"""Live trading engine for multi-chain spot system via OKX DEX swap.

Usage:
    python3 live.py --pair SOL
    python3 live.py --pair ETH --port 3251

Single-threaded loop:
1. Wait for 15m bar boundary + 30s settling
2. Fetch 299 bars base + BTC via CLI
3. Run strategy.on_bar()
4. If position change: execute OKX DEX swap
5. Update live_state_{pair}.json
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys
import time
import traceback

import config
import okx
import strategy

STATE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "state")

PAPER = getattr(config, "PAPER_TRADE", False)


def _state_file() -> str:
    symbol = config.pair()["base_symbol"].lower()
    return os.path.join(STATE_DIR, f"live_state_{symbol}.json")


def _load_state() -> dict:
    """Load live state from disk."""
    os.makedirs(STATE_DIR, exist_ok=True)
    sf = _state_file()
    if os.path.exists(sf):
        with open(sf) as f:
            return json.load(f)
    return {
        "position": 0.0,      # 0.0 = flat, 1.0 = long
        "strategy_state": strategy.init_state(),
        "daily_pnl": 0.0,
        "daily_start_equity": None,
        "last_trade_ts": 0,
        "trades": [],
        # Paper trade tracking
        "paper_usdc": config.INITIAL_USDC,
        "paper_base": 0.0,
        "paper_entry_price": 0.0,
    }


def _save_state(state: dict):
    """Save live state to disk."""
    os.makedirs(STATE_DIR, exist_ok=True)
    with open(_state_file(), "w") as f:
        json.dump(state, f, indent=2)


def _current_equity() -> float:
    """Current equity in USDC terms."""
    p = config.pair()
    usdc = okx.quote_balance() or 0.0
    base = okx.base_balance() or 0.0
    # Get base price
    candles = okx.kline(p["base_mint"], config.BAR_SIZE, limit=1)
    if candles:
        c = candles[0]
        base_price = float(c.get("c", c.get("close", 0))) if isinstance(c, dict) else float(c[4])
    else:
        base_price = 0.0
    return usdc + base * base_price


def _wait_for_bar():
    """Sleep until next 15m bar boundary + 30s settling."""
    now = time.time()
    next_bar = (int(now) // config.BAR_SECONDS + 1) * config.BAR_SECONDS
    sleep_secs = (next_bar - now) + 30
    if sleep_secs > 0:
        print(f"Sleeping {sleep_secs:.0f}s until next bar...")
        time.sleep(sleep_secs)


def _execute_buy(usdc_amount: float) -> dict | None:
    """Buy base token with USDC. Returns trade info or None."""
    p = config.pair()
    quote_dec = p["quote_decimals"]
    amount_raw = str(int(usdc_amount * 10**quote_dec))
    symbol = p["base_symbol"]
    print(f"  BUY: {usdc_amount:.2f} USDC -> {symbol}")

    if p["chain_family"] == "solana":
        # Solana: 2-step (swap_execute -> sign_and_broadcast)
        try:
            swap_data = okx.swap_execute(
                p["quote_mint"], p["base_mint"], amount_raw,
                slippage=str(config.SLIPPAGE_PCT))
        except Exception as e:
            print(f"  Swap execute failed: {e}")
            return None

        unsigned_tx = swap_data.get("tx", swap_data.get("callData", ""))
        to_addr = swap_data.get("to", swap_data.get("routerAddress", ""))
        if not unsigned_tx:
            print(f"  No unsigned tx in swap response: {list(swap_data.keys())}")
            return None

        try:
            tx_hash = okx.sign_and_broadcast(unsigned_tx, to_addr)
        except Exception as e:
            print(f"  Sign/broadcast failed: {e}")
            return None

        if not tx_hash:
            print("  Empty tx hash")
            return None

        print(f"  TX: {tx_hash}")
        status = okx.tx_status(tx_hash)
        print(f"  Status: {status}")
        return {"side": "BUY", "usdc": usdc_amount, "tx": tx_hash, "status": status,
                "ts": int(time.time())}

    else:
        # EVM: 1-step (handles approval + swap)
        try:
            result = okx.swap_onestep(
                p["quote_mint"], p["base_mint"], amount_raw,
                slippage=str(config.SLIPPAGE_PCT))
        except Exception as e:
            print(f"  Swap onestep failed: {e}")
            return None

        tx_hash = result.get("txHash", "")
        if not tx_hash:
            print(f"  No tx hash: {result}")
            return None

        print(f"  TX: {tx_hash}")
        status = okx.tx_status(tx_hash)
        print(f"  Status: {status}")
        return {"side": "BUY", "usdc": usdc_amount, "tx": tx_hash, "status": status,
                "ts": int(time.time())}


def _execute_sell() -> dict | None:
    """Sell all base token for USDC. Returns trade info or None."""
    p = config.pair()
    base_bal = okx.base_balance()
    base_dec = p["base_decimals"]
    gas_reserve = p["gas_reserve"]
    native_for_sell = p["native_for_sell"]
    symbol = p["base_symbol"]

    if not base_bal or base_bal < gas_reserve * 2:
        print(f"  No {symbol} to sell")
        return None

    # Keep gas_reserve for fees (only for native tokens)
    if native_for_sell.lower() in ("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                                    "11111111111111111111111111111111"):
        sell_amount = base_bal - gas_reserve
    else:
        # ERC-20 tokens — sell all, gas is paid in native token
        sell_amount = base_bal

    if sell_amount <= 0:
        print(f"  {symbol} balance too low to sell")
        return None

    amount_raw = str(int(sell_amount * 10**base_dec))
    print(f"  SELL: {sell_amount:.6f} {symbol} -> USDC")

    if p["chain_family"] == "solana":
        # Solana: 2-step
        try:
            swap_data = okx.swap_execute(
                native_for_sell, p["quote_mint"], amount_raw,
                slippage=str(config.SLIPPAGE_PCT))
        except Exception as e:
            print(f"  Swap execute failed: {e}")
            return None

        unsigned_tx = swap_data.get("tx", swap_data.get("callData", ""))
        to_addr = swap_data.get("to", swap_data.get("routerAddress", ""))
        if not unsigned_tx:
            print(f"  No unsigned tx in swap response: {list(swap_data.keys())}")
            return None

        try:
            tx_hash = okx.sign_and_broadcast(unsigned_tx, to_addr)
        except Exception as e:
            print(f"  Sign/broadcast failed: {e}")
            return None

        if not tx_hash:
            print("  Empty tx hash")
            return None

        print(f"  TX: {tx_hash}")
        status = okx.tx_status(tx_hash)
        print(f"  Status: {status}")
        return {"side": "SELL", "base": sell_amount, "tx": tx_hash, "status": status,
                "ts": int(time.time())}

    else:
        # EVM: 1-step
        try:
            result = okx.swap_onestep(
                native_for_sell, p["quote_mint"], amount_raw,
                slippage=str(config.SLIPPAGE_PCT))
        except Exception as e:
            print(f"  Swap onestep failed: {e}")
            return None

        tx_hash = result.get("txHash", "")
        if not tx_hash:
            print(f"  No tx hash: {result}")
            return None

        print(f"  TX: {tx_hash}")
        status = okx.tx_status(tx_hash)
        print(f"  Status: {status}")
        return {"side": "SELL", "base": sell_amount, "tx": tx_hash, "status": status,
                "ts": int(time.time())}


def _paper_equity(state: dict, base_price: float) -> float:
    """Calculate simulated equity for paper trading."""
    return state.get("paper_usdc", 0) + state.get("paper_base", 0) * base_price


def _paper_buy(state: dict, price: float, target_size: float) -> dict:
    """Simulate buying base token with USDC. Returns trade record."""
    p = config.pair()
    symbol = p["base_symbol"]
    usdc = state["paper_usdc"]
    spend = usdc * config.LIVE_USDC_PCT * target_size
    cost = spend * config.COST_PER_LEG
    slip = spend * config.SLIPPAGE_PCT
    effective = spend - cost - slip
    base_bought = effective / price if price > 0 else 0
    state["paper_usdc"] -= spend
    state["paper_base"] += base_bought
    state["paper_entry_price"] = price
    print(f"  [PAPER] BUY {base_bought:.6f} {symbol} @ ${price:.2f} (spent ${spend:.2f} USDC)")
    return {"side": "BUY", "price": price, "usdc": spend, "base": base_bought,
            "paper": True, "status": "SUCCESS", "ts": int(time.time())}


def _paper_sell(state: dict, price: float) -> dict:
    """Simulate selling all base token for USDC. Returns trade record."""
    p = config.pair()
    symbol = p["base_symbol"]
    base_amt = state["paper_base"]
    gross = base_amt * price
    cost = gross * config.COST_PER_LEG
    slip = gross * config.SLIPPAGE_PCT
    net = gross - cost - slip
    entry = state.get("paper_entry_price", price)
    pnl_pct = (price - entry) / entry if entry > 0 else 0
    state["paper_usdc"] += net
    state["paper_base"] = 0.0
    state["paper_entry_price"] = 0.0
    print(f"  [PAPER] SELL {base_amt:.6f} {symbol} @ ${price:.2f} (received ${net:.2f} USDC, pnl={pnl_pct:+.2%})")
    return {"side": "SELL", "price": price, "usdc": net, "base": base_amt,
            "pnl_pct": round(pnl_pct, 4), "paper": True, "status": "SUCCESS",
            "ts": int(time.time())}


def _fetch_bars():
    """Fetch and parse base + BTC candle bars. Returns (base_bars, btc_bars) or (None, None)."""
    p = config.pair()
    base_candles = okx.kline(p["base_mint"], config.BAR_SIZE, limit=299)

    # BTC overlay
    if config.ACTIVE_PAIR == "BTC":
        btc_candles = base_candles  # BTC is its own overlay
    elif p["chain_family"] == "solana":
        btc_candles = okx.kline(config.WBTC_MINT, config.BAR_SIZE, limit=299)
    elif p["chain_index"] == config.BTC_OVERLAY_CHAIN:
        btc_candles = okx.kline(config.BTC_OVERLAY_MINT, config.BAR_SIZE, limit=299)
    else:
        btc_candles = okx.kline(config.BTC_OVERLAY_MINT, config.BAR_SIZE, limit=299,
                                chain_index=config.BTC_OVERLAY_CHAIN)

    if not base_candles or not btc_candles:
        return None, None

    base_bars = []
    for c in base_candles:
        if isinstance(c, dict):
            base_bars.append({
                "ts": int(c.get("ts", 0)),
                "o": float(c.get("o", 0)), "h": float(c.get("h", 0)),
                "l": float(c.get("l", 0)), "c": float(c.get("c", 0)),
                "vol": float(c.get("vol", c.get("baseVolume", 0))),
            })
    base_bars.sort(key=lambda x: x["ts"])

    btc_bars = []
    for c in btc_candles:
        if isinstance(c, dict):
            btc_bars.append({
                "ts": int(c.get("ts", 0)),
                "o": float(c.get("o", 0)), "h": float(c.get("h", 0)),
                "l": float(c.get("l", 0)), "c": float(c.get("c", 0)),
                "vol": float(c.get("vol", c.get("baseVolume", 0))),
            })
    btc_bars.sort(key=lambda x: x["ts"])
    return base_bars, btc_bars


def main():
    parser = argparse.ArgumentParser(description="Live trading engine")
    parser.add_argument("--pair", default="SOL",
                        help="Trading pair (default: SOL). See: python3 config.py --list")
    parser.add_argument("--port", type=int, default=0,
                        help="Dashboard port override (default: auto from config)")
    args = parser.parse_args()

    config.ACTIVE_PAIR = args.pair.upper()
    p = config.pair()
    symbol = p["base_symbol"]

    if args.port:
        config.DASHBOARD_PORT = args.port

    mode_label = "PAPER" if PAPER else "LIVE"
    print(f"=== {p['label']} Spot Engine [{mode_label}] ===")

    if not PAPER:
        addr = okx.wallet_preflight()
        print(f"Wallet: {addr}")

    state = _load_state()
    strat_state = state["strategy_state"]

    if PAPER:
        equity = _paper_equity(state, 0)
        print(f"Paper balance: ${state['paper_usdc']:.2f} USDC, {state['paper_base']:.6f} {symbol}")
    else:
        equity = _current_equity()
        print(f"Starting equity: {equity:.2f} USDC")

    if state["daily_start_equity"] is None:
        state["daily_start_equity"] = equity

    print(f"Position: {'LONG' if state['position'] > 0.5 else 'FLAT'}")
    print("Starting main loop... (Ctrl+C to stop)\n")

    while True:
        try:
            _wait_for_bar()

            base_bars, btc_bars = _fetch_bars()
            if base_bars is None:
                print("Failed to fetch candles, skipping bar")
                continue
            if len(base_bars) < 50:
                print(f"Not enough {symbol} bars ({len(base_bars)}), skipping")
                continue

            # Run strategy
            signal = strategy.on_bar(strat_state, base_bars, btc_bars, len(base_bars) - 1)
            current_pos = state["position"]
            target_pos = signal.target_position

            ts_str = time.strftime("%H:%M", time.localtime())
            price = base_bars[-1]["c"]

            # Equity display
            if PAPER:
                equity = _paper_equity(state, price)
                eq_str = f"eq=${equity:.2f}"
            else:
                eq_str = ""

            print(f"[{ts_str}] {symbol}=${price:.2f} signal={signal.reason} "
                  f"pos={current_pos:.0f}->{target_pos:.0f} {eq_str}")

            # Execute if position changed
            trade = None
            if target_pos > 0.5 and current_pos < 0.5:
                if PAPER:
                    trade = _paper_buy(state, price, target_pos)
                    state["position"] = target_pos
                else:
                    usdc = okx.quote_balance() or 0.0
                    trade_amount = usdc * config.LIVE_USDC_PCT
                    if trade_amount > 1.0:
                        trade = _execute_buy(trade_amount)
                        if trade and trade["status"] == "SUCCESS":
                            state["position"] = 1.0
                        else:
                            print("  Trade failed, staying flat")

            elif target_pos < 0.5 and current_pos > 0.5:
                if PAPER:
                    trade = _paper_sell(state, price)
                    state["position"] = 0.0
                else:
                    trade = _execute_sell()
                    if trade and trade["status"] == "SUCCESS":
                        state["position"] = 0.0
                    else:
                        print("  Trade failed, staying long")

            # Daily loss check
            if PAPER:
                equity = _paper_equity(state, price)
            else:
                equity = _current_equity()
            daily_start = state["daily_start_equity"] or equity
            daily_pnl = (equity - daily_start) / daily_start if daily_start > 0 else 0
            state["daily_pnl"] = daily_pnl

            if daily_pnl < -config.MAX_DAILY_LOSS:
                print(f"DAILY LOSS LIMIT HIT: {daily_pnl:.2%}. Stopping.")
                if state["position"] > 0.5:
                    if PAPER:
                        _paper_sell(state, price)
                    else:
                        _execute_sell()
                    state["position"] = 0.0
                _save_state(state)
                break

            # Reset daily at midnight UTC
            utc_hour = time.gmtime().tm_hour
            if utc_hour == 0 and state.get("last_reset_hour") != 0:
                state["daily_start_equity"] = equity
                state["daily_pnl"] = 0.0
                state["last_reset_hour"] = 0
            elif utc_hour != 0:
                state["last_reset_hour"] = utc_hour

            # Save state
            if trade:
                state["trades"].append(trade)
                state["last_trade_ts"] = int(time.time())
            state["strategy_state"] = strat_state
            _save_state(state)

        except KeyboardInterrupt:
            print("\nStopping...")
            _save_state(state)
            break
        except Exception as e:
            print(f"Error: {e}")
            traceback.print_exc()
            time.sleep(60)


if __name__ == "__main__":
    main()
