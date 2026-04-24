#!/usr/bin/env node
/**
 * risk-manager.mjs — KAIROS Floor Position Risk Manager
 * Manages open positions: trailing stops, step-lock, RSI exit, time stop, circuit breakers.
 * Run every 30 seconds during active trading.
 *
 * Usage:
 *   node scripts/risk-manager.mjs check-positions
 *   node scripts/risk-manager.mjs status
 *   node scripts/risk-manager.mjs reset-cooldown
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { hlCmd, hlPost as safeHlPost, validateSymbol, atomicWriteJson } from './lib/safety.mjs';

// ══════════════════════════════════════════════════════
// CONFIG (mirrors kairos-engine config)
// ══════════════════════════════════════════════════════

const RISK_CONFIG = {
  breakevenR:           0.7,
  trailingActivationR:  1.2,
  stepLockAmount:       2,        // $2 step-lock increments
  atrTrailingMult:      1.5,      // trailing SL = price - ATR×1.5

  rsiOverboughtExit:    69,
  rsiOversoldExit:      31,
  rsiExtremeOverbought: 78,
  rsiExtremeOversold:   22,

  maxTradeDurationMs:   120 * 60 * 1000, // 120 minutes time stop

  maxDrawdownPct:       15,
  dailyLossLimitPct:    5,
  maxConsecutiveLosses: 5,
};

const STATE_DIR  = join(process.cwd(), '.kairos-data');
const STATE_FILE = join(STATE_DIR, 'state.json');
const HL_API     = 'https://api.hyperliquid.xyz/info';

// ══════════════════════════════════════════════════════
// STATE
// ══════════════════════════════════════════════════════

function loadState() {
  if (!existsSync(STATE_FILE)) return { positions: {}, mode: 'dry-run', dailyLoss: 0, consecutiveLosses: 0, tradeHistory: [] };
  return JSON.parse(readFileSync(STATE_FILE, 'utf8'));
}

function saveState(state) {
  atomicWriteJson(STATE_FILE, state);
}

// ══════════════════════════════════════════════════════
// HELPERS
// ══════════════════════════════════════════════════════

async function hlPost(body) {
  return safeHlPost(HL_API, body, { timeoutMs: 10000 });
}

async function fetchCandles(symbol, interval, lookbackMs) {
  if (!validateSymbol(symbol)) throw new Error(`invalid symbol: ${symbol}`);
  const endTime   = Date.now();
  const startTime = endTime - lookbackMs;
  const raw = await hlPost({ type: 'candleSnapshot', req: { coin: symbol, interval, startTime, endTime } });
  if (!Array.isArray(raw)) return [];
  return raw.map(c => ({
    high:  parseFloat(c.h),
    low:   parseFloat(c.l),
    close: parseFloat(c.c),
  }));
}

function rsi(closes, period = 14) {
  if (closes.length < period + 1) return 50;
  let avgGain = 0, avgLoss = 0;
  for (let i = 1; i <= period; i++) {
    const d = closes[i] - closes[i - 1];
    if (d > 0) avgGain += d; else avgLoss -= d;
  }
  avgGain /= period;
  avgLoss /= period;
  for (let i = period + 1; i < closes.length; i++) {
    const d = closes[i] - closes[i - 1];
    avgGain = (avgGain * (period - 1) + Math.max(d, 0)) / period;
    avgLoss = (avgLoss * (period - 1) + Math.max(-d, 0)) / period;
  }
  if (avgLoss === 0) return 100;
  return 100 - 100 / (1 + avgGain / avgLoss);
}

function atr(candles, period = 14) {
  if (candles.length < period + 1) return null;
  const trs = [];
  for (let i = 1; i < candles.length; i++) {
    trs.push(Math.max(
      candles[i].high - candles[i].low,
      Math.abs(candles[i].high - candles[i - 1].close),
      Math.abs(candles[i].low  - candles[i - 1].close)
    ));
  }
  let val = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < trs.length; i++) val = (val * (period - 1) + trs[i]) / period;
  return val;
}

function getCurrentPrice(symbol) {
  if (!validateSymbol(symbol)) return null;
  try {
    const out  = hlCmd('price', `${symbol}-PERP`, '--json');
    const data = JSON.parse(out);
    const v = parseFloat(data.markPrice ?? data.price ?? 0);
    return Number.isFinite(v) && v > 0 ? v : null;
  } catch {
    return null;
  }
}

function updateSL(symbol, side, size, newSLPrice, mode) {
  if (!validateSymbol(symbol)) { console.error(`    ✗ Invalid symbol: ${symbol}`); return; }
  if (mode === 'dry-run') {
    console.log(`    [DRY-RUN] Update SL for ${symbol} → ${newSLPrice}`);
    return;
  }
  try {
    const closeSide = side === 'buy' ? 'sell' : 'buy';
    hlCmd('perp', 'order', `${symbol}-PERP`,
      '--side', closeSide,
      '--size', String(size),
      '--reduce-only',
      '--order-type', 'stop',
      '--trigger-price', String(newSLPrice),
      '--strategy-id', 'kairos-floor-strategy');
    console.log(`    ✓ SL updated to ${newSLPrice}`);
  } catch (e) {
    console.error(`    ✗ SL update failed: ${e.message}`);
  }
}

function closePosition(symbol, size, mode, reason) {
  if (!validateSymbol(symbol)) { console.error(`    ✗ Invalid symbol: ${symbol}`); return false; }
  console.log(`    Closing ${symbol}: ${reason}`);
  if (mode === 'dry-run') {
    console.log(`    [DRY-RUN] Would close ${symbol}`);
    return true;
  }
  try {
    hlCmd('perp', 'close', `${symbol}-PERP`, '--strategy-id', 'kairos-floor-strategy');
    console.log(`    ✓ Position closed`);
    return true;
  } catch (e) {
    console.error(`    ✗ Close failed: ${e.message}`);
    return false;
  }
}

// ══════════════════════════════════════════════════════
// STEP-LOCK CALCULATION
// ══════════════════════════════════════════════════════

function calcStepLock(currentPnl, stepLockAmount) {
  // Lock in profit at every $stepLockAmount increment
  return Math.floor(currentPnl / stepLockAmount) * stepLockAmount;
}

// ══════════════════════════════════════════════════════
// TRAILING STOP — 3 Phases
// ══════════════════════════════════════════════════════

function calcTrailingStop(pos, currentPrice, atrVal) {
  const isLong    = pos.direction === 'LONG';
  const slDist    = pos.slDistance;
  const entryPrice = pos.entryPrice;
  const R = slDist; // 1R = 1 ATR × 2 distance

  const priceMoveR = isLong
    ? (currentPrice - entryPrice) / R
    : (entryPrice - currentPrice) / R;

  // Phase INITIAL: SL stays at entry - ATR×2
  if (priceMoveR < RISK_CONFIG.breakevenR) {
    return { phase: 'INITIAL', newSL: pos.slPrice };
  }

  // Phase BREAKEVEN: Move SL to entry + small buffer
  if (priceMoveR >= RISK_CONFIG.breakevenR && priceMoveR < RISK_CONFIG.trailingActivationR) {
    const buffer = entryPrice * 0.0015;
    const newSL  = isLong ? entryPrice + buffer : entryPrice - buffer;
    return { phase: 'BREAKEVEN', newSL: +newSL.toFixed(6) };
  }

  // Phase TRAILING: ATR-adaptive trailing
  const trailDist = (atrVal ?? slDist) * RISK_CONFIG.atrTrailingMult;
  const newSL     = isLong
    ? currentPrice - trailDist
    : currentPrice + trailDist;
  return { phase: 'TRAILING', newSL: +newSL.toFixed(6) };
}

// ══════════════════════════════════════════════════════
// MAIN: CHECK ALL POSITIONS
// ══════════════════════════════════════════════════════

async function checkPositions() {
  const state = loadState();
  const posEntries = Object.entries(state.positions ?? {});

  if (posEntries.length === 0) {
    console.log('[RISK] No open positions to manage.');
    return;
  }

  console.log(`[RISK] Managing ${posEntries.length} open position(s)...`);

  for (const [symbol, pos] of posEntries) {
    // Backfill missing openedAt so an age-less position doesn't live forever.
    if (!Number.isFinite(pos.openedAt)) {
      pos.openedAt = Date.now();
      state.positions[symbol].openedAt = pos.openedAt;
      saveState(state);
    }

    // Skip positions that executor flagged as orphaned — require human intervention.
    if (pos.status === 'ORPHAN_NO_SL') {
      console.log(`\n  ── ${pos.direction} ${symbol} ──`);
      console.error(`    ⚠ ORPHAN_NO_SL flagged — skipping automated management. ${pos.alert ?? ''}`);
      continue;
    }

    console.log(`\n  ── ${pos.direction} ${symbol} ──`);

    // Fetch current price and market data
    let currentPrice = getCurrentPrice(symbol);
    if (!currentPrice) {
      // Fallback: fetch from API
      try {
        const candles = await fetchCandles(symbol, '1m', 5 * 60 * 1000);
        if (candles.length > 0) currentPrice = candles[candles.length - 1].close;
      } catch {
        console.warn(`    Cannot get price for ${symbol} — skipping`);
        continue;
      }
    }

    // Fetch ATR for trailing
    let atrVal = pos.atr ?? pos.slDistance / 2;
    try {
      const candles = await fetchCandles(symbol, '5m', 4 * 60 * 60 * 1000);
      atrVal = atr(candles) ?? atrVal;
    } catch {
      // Use stored ATR
    }

    // Fetch RSI
    let rsiVal = 50;
    try {
      const candles5m = await fetchCandles(symbol, '5m', 6 * 60 * 60 * 1000);
      const closes    = candles5m.map(c => c.close);
      rsiVal = rsi(closes) ?? 50;
    } catch {
      // Use neutral
    }

    const isLong     = pos.direction === 'LONG';
    const priceDiff  = isLong ? currentPrice - pos.entryPrice : pos.entryPrice - currentPrice;
    const pnlPct     = (priceDiff / pos.entryPrice) * 100;
    // PnL in USD = price_diff × quantity. Leverage affects margin, NOT absolute PnL.
    const pnlUsd     = priceDiff * pos.size;
    const ageMs      = Date.now() - pos.openedAt;
    const ageMin     = Math.round(ageMs / 60_000);
    const R          = pos.slDistance;
    const priceMoveR = priceDiff / R;

    console.log(`    Price: ${currentPrice} | Entry: ${pos.entryPrice} | PnL: ${pnlUsd >= 0 ? '+' : ''}${pnlUsd.toFixed(2)} USDC (${pnlPct.toFixed(2)}%)`);
    console.log(`    RSI: ${rsiVal.toFixed(1)} | ATR: ${atrVal.toFixed(4)} | Phase: ${pos.phase} | Age: ${ageMin}min`);

    // ── 1. Time Stop ──
    if (ageMs > RISK_CONFIG.maxTradeDurationMs) {
      const closed = closePosition(symbol, pos.size, state.mode, `Time stop (${ageMin}min > 120min)`);
      if (closed) {
        const pnl = pnlUsd;
        state.tradeHistory.push({ symbol, direction: pos.direction, pnl, reason: 'time_stop', timestamp: Date.now() });
        if (pnl < 0) state.consecutiveLosses++;
        else state.consecutiveLosses = 0;
        delete state.positions[symbol];
        saveState(state);
      }
      continue;
    }

    // ── 2. RSI Extreme Exit (when in profit) ──
    if (priceMoveR > 0.3) {
      const rsiExtremeExit = isLong
        ? rsiVal > RISK_CONFIG.rsiExtremeOverbought
        : rsiVal < RISK_CONFIG.rsiExtremeOversold;

      if (rsiExtremeExit) {
        const closed = closePosition(symbol, pos.size, state.mode, `RSI extreme (${rsiVal.toFixed(1)}) — take profit`);
        if (closed) {
          state.tradeHistory.push({ symbol, direction: pos.direction, pnl: pnlUsd, reason: 'rsi_exit', timestamp: Date.now() });
          if (pnlUsd < 0) state.consecutiveLosses++; else state.consecutiveLosses = 0;
          delete state.positions[symbol];
          saveState(state);
        }
        continue;
      }
    }

    // ── 3. Trailing Stop ──
    const trailing = calcTrailingStop(pos, currentPrice, atrVal);
    const side = isLong ? 'buy' : 'sell';

    if (trailing.phase !== pos.phase) {
      console.log(`    Phase transition: ${pos.phase} → ${trailing.phase}`);
      state.positions[symbol].phase = trailing.phase;
    }

    // SL can only move in favor (never against the position)
    const shouldMoveSL = isLong
      ? trailing.newSL > pos.slPrice
      : trailing.newSL < pos.slPrice;

    if (shouldMoveSL) {
      console.log(`    Trailing SL: ${pos.slPrice} → ${trailing.newSL}`);
      updateSL(symbol, side, pos.size, trailing.newSL, state.mode);
      state.positions[symbol].slPrice = trailing.newSL;
    }

    // ── 4. Step-Lock ──
    if (pnlUsd > 0) {
      const newStepLock = calcStepLock(pnlUsd, RISK_CONFIG.stepLockAmount);
      if (newStepLock > (pos.stepLocked ?? 0)) {
        console.log(`    Step-lock: $${(pos.stepLocked ?? 0).toFixed(2)} → $${newStepLock.toFixed(2)}`);
        state.positions[symbol].stepLocked = newStepLock;
        // Ensure SL locks in the step-locked profit (convert to price)
        // Price diff needed = USD to lock / quantity (leverage doesn't scale PnL).
        const lockedPriceDiff = (newStepLock - RISK_CONFIG.stepLockAmount) / pos.size;
        const lockedSL = isLong
          ? pos.entryPrice + lockedPriceDiff
          : pos.entryPrice - lockedPriceDiff;
        const lockedSLFinal = +lockedSL.toFixed(6);
        const currentSL = state.positions[symbol].slPrice;
        if ((isLong && lockedSLFinal > currentSL) || (!isLong && lockedSLFinal < currentSL)) {
          updateSL(symbol, side, pos.size, lockedSLFinal, state.mode);
          state.positions[symbol].slPrice = lockedSLFinal;
        }
      }
    }

    // ── 5. RSI Tighten (not extreme, but elevated) ──
    if (priceMoveR > 0.3) {
      const rsiTighten = isLong
        ? rsiVal > RISK_CONFIG.rsiOverboughtExit
        : rsiVal < RISK_CONFIG.rsiOversoldExit;

      if (rsiTighten && trailing.phase === 'TRAILING') {
        const tightDist = atrVal * 0.8;
        const tightSL   = isLong ? currentPrice - tightDist : currentPrice + tightDist;
        const tightSLR  = +tightSL.toFixed(6);
        const currentSL = state.positions[symbol].slPrice;
        if ((isLong && tightSLR > currentSL) || (!isLong && tightSLR < currentSL)) {
          console.log(`    RSI tighten (${rsiVal.toFixed(1)}): SL → ${tightSLR}`);
          updateSL(symbol, side, pos.size, tightSLR, state.mode);
          state.positions[symbol].slPrice = tightSLR;
        }
      }
    }

    saveState(state);
  }
}

// ══════════════════════════════════════════════════════
// CIRCUIT BREAKERS CHECK
// ══════════════════════════════════════════════════════

function checkCircuitBreakers() {
  const state = loadState();
  const alerts = [];

  if (state.dailyLoss >= RISK_CONFIG.dailyLossLimitPct) {
    alerts.push(`⚠ DAILY LOSS LIMIT: ${state.dailyLoss.toFixed(2)}% >= ${RISK_CONFIG.dailyLossLimitPct}%`);
  }
  if (state.consecutiveLosses >= RISK_CONFIG.maxConsecutiveLosses) {
    alerts.push(`⚠ CONSECUTIVE LOSSES: ${state.consecutiveLosses} — cooldown active`);
  }

  if (alerts.length > 0) {
    console.log('[CIRCUIT BREAKER] ALERTS:');
    for (const a of alerts) console.log(`  ${a}`);
    return false;
  }
  return true;
}

// ══════════════════════════════════════════════════════
// STATUS
// ══════════════════════════════════════════════════════

function showStatus() {
  const state = loadState();
  const ok    = checkCircuitBreakers();
  console.log('\n═══ RISK MANAGER STATUS ═══');
  console.log(`Trading allowed: ${ok ? '✓ YES' : '✗ NO (circuit breaker)'}`);
  console.log(`Mode:            ${state.mode}`);
  console.log(`Daily loss:      ${state.dailyLoss.toFixed(2)}%`);
  console.log(`Consec. losses:  ${state.consecutiveLosses}`);
  console.log(`Open positions:  ${Object.keys(state.positions).length}`);
  for (const [sym, pos] of Object.entries(state.positions)) {
    const openedAt = Number.isFinite(pos.openedAt) ? pos.openedAt : Date.now();
    const ageMin = Math.round((Date.now() - openedAt) / 60_000);
    const flag = pos.status === 'ORPHAN_NO_SL' ? ' ⚠ ORPHAN' : '';
    console.log(`  ${pos.direction} ${sym} | SL: ${pos.slPrice} | Phase: ${pos.phase} | ${ageMin}min${flag}`);
  }
  console.log('════════════════════════════');
}

// ══════════════════════════════════════════════════════
// CLI
// ══════════════════════════════════════════════════════

const cmd = process.argv[2];
switch (cmd) {
  case 'check-positions':
    checkPositions().catch(e => { console.error(e.message); process.exit(1); });
    break;
  case 'status':
    showStatus();
    break;
  case 'circuit-breakers':
    checkCircuitBreakers();
    break;
  case 'reset-cooldown': {
    const state = loadState();
    state.consecutiveLosses = 0;
    saveState(state);
    console.log('[RISK] Consecutive loss counter reset.');
    break;
  }
  default:
    console.log(`
KAIROS Floor Risk Manager

Usage: node scripts/risk-manager.mjs <command>

Commands:
  check-positions    Manage all open positions (trailing stops, step-lock, exits)
  status             Show current risk status
  circuit-breakers   Check if circuit breakers are active
  reset-cooldown     Reset the consecutive loss counter

Run check-positions every 30 seconds during active trading.
`);
}
