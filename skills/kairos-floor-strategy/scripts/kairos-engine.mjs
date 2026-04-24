#!/usr/bin/env node
/**
 * kairos-engine.mjs — KAIROS Floor Strategy Engine for Hyperliquid
 * Pipeline: Sentinel → Scanner → Guardian → Executor
 * Default mode: dry-run (paper trading)
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import {
  validateSymbol,
  validateSymbolList,
  validateSetup,
  hlCmd,
  hlPost as safeHlPost,
  fetchJsonWithTimeout,
  calcSize as safeCalcSize,
  atomicWriteJson,
  checkHaltFile,
} from './lib/safety.mjs';

// ══════════════════════════════════════════════════════
// CONFIG
// ══════════════════════════════════════════════════════

const CONFIG = {
  symbols: (process.env.KAIROS_SYMBOLS || 'BTC,ETH,SOL').split(','),
  mode: process.env.KAIROS_MODE || 'dry-run',

  // Confluence gates (relaxed for challenge: volume + tx count)
  minConfluence: 2,        // 2/9 (prod KAIROS = 3/9)
  minConfidence: 40,       // 40% (prod = 50%)
  minRR: 1.5,

  // Position limits
  maxPositions: 6,
  riskPerTrade: 0.01,      // 1% of capital per trade
  maxLeverage: 25,

  // Timing
  scanIntervalMs: 120_000, // 2 minutes
  staggerMs: 30_000,       // 30s between trades
  maxTradesPerDay: 100,

  // Circuit breakers
  maxDrawdownPct: 15,
  dailyLossLimitPct: 5,
  maxConsecutiveLosses: 5,

  // Trailing stop phases
  breakevenR: 0.7,
  trailingActivationR: 1.2,
  stepLockAmount: 2,        // $2 step lock

  // RSI exit levels
  rsiOverboughtExit: 69,
  rsiOversoldExit: 31,
  rsiExtremeOverbought: 78,
  rsiExtremeOversold: 22,

  maxTradeDurationMs: 120 * 60 * 1000, // 120 minutes

  convictionTiers: [
    { name: 'SCOUT',      minScore: 0,  maxScore: 30,  leverage: 5,  riskPct: 0.005 },
    { name: 'STANDARD',   minScore: 30, maxScore: 50,  leverage: 10, riskPct: 0.010 },
    { name: 'CONFIDENT',  minScore: 50, maxScore: 70,  leverage: 15, riskPct: 0.015 },
    { name: 'CONVICTION', minScore: 70, maxScore: 85,  leverage: 20, riskPct: 0.020 },
    { name: 'APOCALYPSE', minScore: 85, maxScore: 101, leverage: 25, riskPct: 0.020 },
  ],
};

// ══════════════════════════════════════════════════════
// STATE MANAGEMENT
// ══════════════════════════════════════════════════════

const STATE_DIR = join(process.cwd(), '.kairos-data');
const STATE_FILE = join(STATE_DIR, 'state.json');

function loadState() {
  if (!existsSync(STATE_DIR)) mkdirSync(STATE_DIR, { recursive: true });
  if (!existsSync(STATE_FILE)) {
    const initial = {
      mode: 'dry-run',
      dailyLoss: 0,
      dailyTrades: 0,
      consecutiveLosses: 0,
      lastReset: new Date().toDateString(),
      positions: {},
      peakBalance: 0,
      tradeHistory: [],
    };
    atomicWriteJson(STATE_FILE, initial);
    return initial;
  }
  const state = JSON.parse(readFileSync(STATE_FILE, 'utf8'));
  if (state.lastReset !== new Date().toDateString()) {
    state.dailyLoss = 0;
    state.dailyTrades = 0;
    state.lastReset = new Date().toDateString();
    saveState(state);
  }
  return state;
}

function saveState(state) {
  atomicWriteJson(STATE_FILE, state);
}

// ══════════════════════════════════════════════════════
// TECHNICAL INDICATORS — Pure JS, zero dependencies
// ══════════════════════════════════════════════════════

function emaArray(data, period) {
  if (data.length < period) return [];
  const k = 2 / (period + 1);
  const result = new Array(data.length).fill(null);
  let sum = 0;
  for (let i = 0; i < period; i++) sum += data[i];
  result[period - 1] = sum / period;
  for (let i = period; i < data.length; i++) {
    result[i] = data[i] * k + result[i - 1] * (1 - k);
  }
  return result;
}

function ema(data, period) {
  const arr = emaArray(data, period);
  return arr.length > 0 ? arr[arr.length - 1] : null;
}

function sma(data, period) {
  if (data.length < period) return null;
  return data.slice(-period).reduce((a, b) => a + b, 0) / period;
}

function rsi(closes, period = 14) {
  if (closes.length < period + 1) return null;
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

function macd(closes, fast = 12, slow = 26, signal = 9) {
  if (closes.length < slow + signal) return null;
  const fastArr = emaArray(closes, fast);
  const slowArr = emaArray(closes, slow);
  const macdLine = closes
    .map((_, i) => (fastArr[i] !== null && slowArr[i] !== null ? fastArr[i] - slowArr[i] : null))
    .filter(v => v !== null);
  if (macdLine.length < signal) return null;
  const sigArr = emaArray(macdLine, signal);
  const latestMacd = macdLine[macdLine.length - 1];
  const latestSig = sigArr[sigArr.length - 1];
  return { macd: latestMacd, signal: latestSig, histogram: latestMacd - latestSig };
}

function atr(highs, lows, closes, period = 14) {
  if (closes.length < period + 1) return null;
  const trs = [];
  for (let i = 1; i < closes.length; i++) {
    trs.push(Math.max(
      highs[i] - lows[i],
      Math.abs(highs[i] - closes[i - 1]),
      Math.abs(lows[i] - closes[i - 1])
    ));
  }
  if (trs.length < period) return null;
  let val = trs.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < trs.length; i++) val = (val * (period - 1) + trs[i]) / period;
  return val;
}

function adx(highs, lows, closes, period = 14) {
  const n = closes.length;
  if (n < period * 2 + 1) return null;
  const pDMs = [], mDMs = [], trs = [];
  for (let i = 1; i < n; i++) {
    const tr = Math.max(
      highs[i] - lows[i],
      Math.abs(highs[i] - closes[i - 1]),
      Math.abs(lows[i] - closes[i - 1])
    );
    const up = highs[i] - highs[i - 1];
    const down = lows[i - 1] - lows[i];
    trs.push(tr);
    pDMs.push(up > down && up > 0 ? up : 0);
    mDMs.push(down > up && down > 0 ? down : 0);
  }
  // Wilder smoothing: sum-based (not divided)
  const wilderSmooth = (arr) => {
    let v = arr.slice(0, period).reduce((a, b) => a + b, 0);
    const out = [v];
    for (let i = period; i < arr.length; i++) {
      v = v - v / period + arr[i];
      out.push(v);
    }
    return out;
  };
  const sTR = wilderSmooth(trs);
  const sPDM = wilderSmooth(pDMs);
  const sMDM = wilderSmooth(mDMs);
  const pDI = sPDM.map((v, i) => sTR[i] === 0 ? 0 : 100 * v / sTR[i]);
  const mDI = sMDM.map((v, i) => sTR[i] === 0 ? 0 : 100 * v / sTR[i]);
  const dxArr = pDI.map((p, i) => {
    const sum = p + mDI[i];
    return sum === 0 ? 0 : 100 * Math.abs(p - mDI[i]) / sum;
  });
  if (dxArr.length < period) return null;
  let adxVal = dxArr.slice(0, period).reduce((a, b) => a + b, 0) / period;
  for (let i = period; i < dxArr.length; i++) adxVal = (adxVal * (period - 1) + dxArr[i]) / period;
  return { adx: adxVal, plusDI: pDI[pDI.length - 1], minusDI: mDI[mDI.length - 1] };
}

function bollingerBands(closes, period = 20, mult = 2) {
  if (closes.length < period) return null;
  const slice = closes.slice(-period);
  const mean = slice.reduce((a, b) => a + b, 0) / period;
  const variance = slice.reduce((s, v) => s + (v - mean) ** 2, 0) / period;
  const std = Math.sqrt(variance);
  return {
    upper: mean + mult * std,
    middle: mean,
    lower: mean - mult * std,
    width: mean > 0 ? (4 * std) / mean : 0,
  };
}

function bollingerWidthHistory(closes, period = 20) {
  const widths = [];
  for (let i = period; i <= closes.length; i++) {
    const bb = bollingerBands(closes.slice(0, i), period);
    if (bb) widths.push(bb.width);
  }
  return widths;
}

// ══════════════════════════════════════════════════════
// HYPERLIQUID API
// ══════════════════════════════════════════════════════

const HL_API = 'https://api.hyperliquid.xyz/info';

async function hlPost(body) {
  return safeHlPost(HL_API, body, { timeoutMs: 10000 });
}

async function fetchCandles(symbol, interval, lookbackMs) {
  const endTime = Date.now();
  const startTime = endTime - lookbackMs;
  const raw = await hlPost({ type: 'candleSnapshot', req: { coin: symbol, interval, startTime, endTime } });
  if (!Array.isArray(raw)) throw new Error(`Invalid candle response for ${symbol} ${interval}`);
  return raw.map(c => ({
    timestamp: c.t,
    open: parseFloat(c.o),
    high: parseFloat(c.h),
    low: parseFloat(c.l),
    close: parseFloat(c.c),
    volume: parseFloat(c.v),
  }));
}

async function fetchFundingRate(symbol) {
  try {
    const [meta, assetCtxs] = await hlPost({ type: 'metaAndAssetCtxs' });
    const idx = meta.universe.findIndex(a => a.name === symbol);
    if (idx === -1) return 0;
    return parseFloat(assetCtxs[idx]?.funding ?? 0);
  } catch {
    return 0;
  }
}

async function fetchFearGreed() {
  try {
    const data = await fetchJsonWithTimeout('https://api.alternative.me/fng/?limit=1', { timeoutMs: 8000 });
    const v = parseInt(data?.data?.[0]?.value, 10);
    return Number.isFinite(v) ? v : 50;
  } catch {
    return 50; // neutral fallback
  }
}

// ══════════════════════════════════════════════════════
// HYPERLIQUID CLI WRAPPER — uses safe variadic hlCmd from safety lib
// ══════════════════════════════════════════════════════

function getBalance(state) {
  try {
    const out = hlCmd('balance', '--json');
    const data = JSON.parse(out);
    const raw = data.total ?? data.balance ?? data.equity;
    const val = parseFloat(raw);
    if (!Number.isFinite(val) || val <= 0) {
      if (CONFIG.mode === 'live') throw new Error(`Unrecognized balance response: ${out.slice(0, 120)}`);
      return 1000;
    }
    return val;
  } catch (e) {
    if (CONFIG.mode === 'live') throw new Error(`Cannot fetch balance for live trading: ${e.message}`);
    return 1000; // dry-run mock only
  }
}

function getOpenPositions(state) {
  try {
    const out = hlCmd('perp', 'positions', '--json');
    const parsed = JSON.parse(out);
    return Array.isArray(parsed) ? parsed : Object.values(state.positions || {});
  } catch {
    return Object.values(state.positions || {});
  }
}

// ══════════════════════════════════════════════════════
// PHASE 1: SENTINEL — Market Regime
// ══════════════════════════════════════════════════════

async function runSentinel(symbols) {
  const [c15m, c1h] = await Promise.all([
    fetchCandles('BTC', '15m', 48 * 60 * 60 * 1000),
    fetchCandles('BTC', '1h',   7 * 24 * 60 * 60 * 1000),
  ]);

  const closes15m = c15m.map(c => c.close);
  const highs15m  = c15m.map(c => c.high);
  const lows15m   = c15m.map(c => c.low);
  const vols15m   = c15m.map(c => c.volume);
  const closes1h  = c1h.map(c => c.close);

  const adxResult = adx(highs15m, lows15m, closes15m);
  const adxVal    = adxResult?.adx ?? 15;
  const ema9      = ema(closes15m, 9);
  const ema21     = ema(closes15m, 21);
  const ema50     = ema(closes15m, 50);
  const rsiVal    = rsi(closes15m, 14);
  const avgVol20  = vols15m.slice(-20).reduce((a, b) => a + b, 0) / 20;
  const lastVol   = vols15m[vols15m.length - 1];
  const fg        = await fetchFearGreed();

  let regime    = 'ranging';
  let direction = 'BOTH';
  let strength  = 1;
  let tradable  = true;

  if (ema9 && ema21 && ema50) {
    if (ema9 > ema21 && ema21 > ema50 && adxVal > 20) {
      regime    = 'trending_up';
      direction = 'LONG';
      strength  = Math.min(5, Math.ceil(adxVal / 10));
    } else if (ema9 < ema21 && ema21 < ema50 && adxVal > 20) {
      regime    = 'trending_down';
      direction = 'SHORT';
      strength  = Math.min(5, Math.ceil(adxVal / 10));
    } else if (adxVal > 40 || (rsiVal && (rsiVal > 82 || rsiVal < 18))) {
      regime   = 'volatile';
      strength = 2;
      tradable = adxVal < 55; // very high ADX = strong trend, still ok
    }
  }

  // Fear & Greed directional bias
  if (fg < 15 && direction === 'BOTH') direction = 'LONG';
  if (fg > 85 && direction === 'BOTH') direction = 'SHORT';
  if (fg < 10 || fg > 90) strength = Math.max(1, strength - 1);

  return { regime, tradable, direction, strength, adx: adxVal, ema9, ema21, ema50, rsi: rsiVal, fearGreed: fg };
}

// ══════════════════════════════════════════════════════
// PHASE 2: SCANNER — Setup Detection (9-factor confluence)
// ══════════════════════════════════════════════════════

async function analyzeSymbol(symbol, bulletin) {
  const [c5m, c15m, c1h] = await Promise.all([
    fetchCandles(symbol, '5m',  12 * 60 * 60 * 1000),
    fetchCandles(symbol, '15m', 48 * 60 * 60 * 1000),
    fetchCandles(symbol, '1h',   7 * 24 * 60 * 60 * 1000),
  ]);

  if (c5m.length < 30 || c15m.length < 30 || c1h.length < 30) return [];

  const closes5m = c5m.map(c => c.close);
  const highs5m  = c5m.map(c => c.high);
  const lows5m   = c5m.map(c => c.low);
  const vols5m   = c5m.map(c => c.volume);

  const closes15m = c15m.map(c => c.close);
  const closes1h  = c1h.map(c => c.close);

  // Indicators on primary TF (5m)
  const ema9_5m  = ema(closes5m, 9);
  const ema21_5m = ema(closes5m, 21);
  const adx5m    = adx(highs5m, lows5m, closes5m);
  const rsi5m    = rsi(closes5m, 14);
  const macd5m   = macd(closes5m);
  const atr5m    = atr(highs5m, lows5m, closes5m, 14);
  const bb5m     = bollingerBands(closes5m, 20);

  // Multi-TF alignment
  const ema9_15m  = ema(closes15m, 9);
  const ema21_15m = ema(closes15m, 21);
  const ema9_1h   = ema(closes1h, 9);
  const ema21_1h  = ema(closes1h, 21);

  // Volume
  const avgVol20 = vols5m.slice(-20).reduce((a, b) => a + b, 0) / 20;
  const lastVol  = vols5m[vols5m.length - 1];

  // BB width percentile (last 50 bars vs current)
  const bbWidths    = bollingerWidthHistory(closes5m.slice(-70), 20);
  const sortedW     = [...bbWidths].sort((a, b) => a - b);
  const currentW    = bb5m?.width ?? Infinity;
  const bbRank      = sortedW.findIndex(w => w >= currentW);
  const bbPercentile = bbWidths.length > 0 ? (bbRank / sortedW.length) * 100 : 100;

  const entryPrice = closes5m[closes5m.length - 1];
  const directions = bulletin.direction === 'BOTH' ? ['LONG', 'SHORT'] : [bulletin.direction];
  const setups = [];

  for (const direction of directions) {
    const isLong = direction === 'LONG';
    let confluenceCount = 0;
    const signals = [];

    // Factor 1: EMA 9/21 alignment
    if (ema9_5m && ema21_5m) {
      if ((isLong && ema9_5m > ema21_5m) || (!isLong && ema9_5m < ema21_5m)) {
        confluenceCount++; signals.push('EMA_ALIGNED');
      }
    }

    // Factor 2: ADX > 20
    if (adx5m && adx5m.adx > 20) {
      confluenceCount++; signals.push('ADX_TRENDING');
    }

    // Factor 3: RSI sweet spot 25-75
    if (rsi5m !== null) {
      const ok = isLong ? (rsi5m > 25 && rsi5m < 70) : (rsi5m > 30 && rsi5m < 75);
      if (ok) { confluenceCount++; signals.push('RSI_SWEET_SPOT'); }
    }

    // Factor 4: MACD histogram confirms
    if (macd5m) {
      const ok = isLong ? macd5m.histogram > 0 : macd5m.histogram < 0;
      if (ok) { confluenceCount++; signals.push('MACD_CONFIRMS'); }
    }

    // Factor 5: Volume above average
    if (lastVol >= avgVol20 * 0.9) { confluenceCount++; signals.push('VOLUME_OK'); }

    // Factor 6: 15m TF alignment
    if (ema9_15m && ema21_15m) {
      if ((isLong && ema9_15m > ema21_15m) || (!isLong && ema9_15m < ema21_15m)) {
        confluenceCount++; signals.push('TF_15M_ALIGNED');
      }
    }

    // Factor 7: 1h TF alignment
    if (ema9_1h && ema21_1h) {
      if ((isLong && ema9_1h > ema21_1h) || (!isLong && ema9_1h < ema21_1h)) {
        confluenceCount++; signals.push('TF_1H_ALIGNED');
      }
    }

    // Factor 8: ATR breakout — last candle move > 1.5x ATR
    if (atr5m && closes5m.length >= 2) {
      const lastMove = Math.abs(closes5m[closes5m.length - 1] - closes5m[closes5m.length - 2]);
      if (lastMove > atr5m * 1.5) { confluenceCount++; signals.push('ATR_BREAKOUT'); }
    }

    // Factor 9: Bollinger Band compression < 40th percentile
    if (bbPercentile < 40) { confluenceCount++; signals.push('BB_COMPRESSION'); }

    if (confluenceCount < CONFIG.minConfluence) continue;

    const confidence = Math.min(100, Math.round((confluenceCount / 9) * 80 + bulletin.strength * 4));
    if (confidence < CONFIG.minConfidence) continue;

    const slDistance = (atr5m ?? entryPrice * 0.002) * 2.0;
    const tpDistance = slDistance * 2.5;
    const rr = tpDistance / slDistance;
    if (rr < CONFIG.minRR) continue;

    const slPrice = isLong ? entryPrice - slDistance : entryPrice + slDistance;
    const tpPrice = isLong ? entryPrice + tpDistance : entryPrice - tpDistance;

    setups.push({
      symbol,
      direction,
      entryPrice,
      slPrice: +slPrice.toFixed(6),
      tpPrice: +tpPrice.toFixed(6),
      slDistance: +slDistance.toFixed(6),
      atr: +(atr5m ?? 0).toFixed(6),
      confluenceCount,
      signals,
      confidence,
      rr: +rr.toFixed(2),
      timestamp: Date.now(),
    });
  }

  return setups;
}

async function runScanner(symbols, bulletin) {
  if (!bulletin.tradable) return [];
  const allSetups = [];
  for (const symbol of symbols) {
    try {
      const setups = await analyzeSymbol(symbol, bulletin);
      allSetups.push(...setups);
    } catch (e) {
      console.error(`[SCANNER] ${symbol}: ${e.message}`);
    }
  }
  return allSetups.sort((a, b) => b.confidence - a.confidence).slice(0, 2);
}

// ══════════════════════════════════════════════════════
// PHASE 3: GUARDIAN — Risk Validation
// ══════════════════════════════════════════════════════

async function runGuardian(setup, state, openPositions) {
  if (openPositions.length >= CONFIG.maxPositions)
    return { verdict: 'VETO', reasoning: `Max positions (${CONFIG.maxPositions}) reached` };

  const alreadyOpen = openPositions.some(p => (p.symbol ?? p.coin) === setup.symbol);
  if (alreadyOpen)
    return { verdict: 'VETO', reasoning: `Already have open position in ${setup.symbol}` };

  if (state.dailyLoss >= CONFIG.dailyLossLimitPct)
    return { verdict: 'VETO', reasoning: `Daily loss limit ${CONFIG.dailyLossLimitPct}% reached` };

  if (state.consecutiveLosses >= CONFIG.maxConsecutiveLosses)
    return { verdict: 'VETO', reasoning: `${state.consecutiveLosses} consecutive losses — cooldown active` };

  if (state.dailyTrades >= CONFIG.maxTradesPerDay)
    return { verdict: 'VETO', reasoning: `Daily trade limit (${CONFIG.maxTradesPerDay}) reached` };

  // Direction lock: if 3+ positions all same side, block opposing
  if (openPositions.length >= 3) {
    const longs  = openPositions.filter(p => (p.direction ?? p.side) === 'LONG').length;
    const shorts = openPositions.filter(p => (p.direction ?? p.side) === 'SHORT').length;
    if (longs > 0 && shorts > 0) {
      const dominant = longs > shorts ? 'LONG' : 'SHORT';
      if (setup.direction !== dominant && openPositions.length >= 4)
        return { verdict: 'VETO', reasoning: 'Net direction lock — opposing direction blocked' };
    }
  }

  // Funding rate check
  try {
    const funding = await fetchFundingRate(setup.symbol);
    const payingFunding = setup.direction === 'LONG' ? funding > 0 : funding < 0;
    if (payingFunding && Math.abs(funding) > 0.0008)
      return { verdict: 'VETO', reasoning: `Funding rate ${(funding * 100).toFixed(4)}% too high against position` };
  } catch {
    // Non-fatal: proceed without funding check
  }

  return {
    verdict: 'GO',
    reasoning: `${setup.confluenceCount}/9 confluence · ${setup.confidence}% confidence · R:R ${setup.rr}`,
  };
}

// ══════════════════════════════════════════════════════
// PHASE 4: EXECUTOR — Order Execution
// ══════════════════════════════════════════════════════

function getConvictionTier(score) {
  return CONFIG.convictionTiers.find(t => score >= t.minScore && score < t.maxScore)
    ?? CONFIG.convictionTiers[0];
}

async function runExecutor(setup, state) {
  // SECURITY: Validate setup before touching the wallet.
  const validationError = validateSetup(setup);
  if (validationError) {
    console.error(`  [EXECUTOR] ✗ Rejected invalid setup: ${validationError}`);
    return { executed: false, error: `invalid setup: ${validationError}` };
  }

  // SECURITY: authoritative mode is the persisted state — never CLI arg.
  const activeMode = state.mode === 'live' ? 'live' : 'dry-run';

  const convictionScore = Math.min(100, setup.confidence + (setup.confluenceCount - CONFIG.minConfluence) * 8);
  const tier    = getConvictionTier(convictionScore);
  const balance = getBalance(state);
  const leverage = Math.min(tier.leverage, CONFIG.maxLeverage);
  const size     = safeCalcSize(balance, tier.riskPct, setup.slDistance, setup.entryPrice);
  if (size === null) {
    console.error('  [EXECUTOR] ✗ Unsafe position size calculation — aborting');
    return { executed: false, error: 'calcSize produced unsafe result' };
  }
  const side     = setup.direction === 'LONG' ? 'buy' : 'sell';
  const closeSide = side === 'buy' ? 'sell' : 'buy';
  const symbolPerp = `${setup.symbol}-PERP`;

  const orderInfo = {
    symbol:         setup.symbol,
    direction:      setup.direction,
    entryPrice:     setup.entryPrice,
    slPrice:        setup.slPrice,
    tpPrice:        setup.tpPrice,
    slDistance:     setup.slDistance,
    atr:            setup.atr,
    size,
    leverage,
    tier:           tier.name,
    convictionScore,
    rr:             setup.rr,
    signals:        setup.signals,
    openedAt:       Date.now(),
    stepLocked:     0,
    phase:          'INITIAL',
    status:         'pending',
  };

  if (activeMode === 'dry-run') {
    const notional = +(size * setup.entryPrice * leverage).toFixed(2);
    console.log(`  ┌─ [DRY-RUN] ${tier.name} — ${setup.direction} ${setup.symbol}`);
    console.log(`  │  Entry: ${setup.entryPrice} | SL: ${setup.slPrice} | TP: ${setup.tpPrice}`);
    console.log(`  │  Size: ${size} | Leverage: ${leverage}x | Notional: $${notional}`);
    console.log(`  │  Signals: ${setup.signals.join(', ')}`);
    console.log(`  └─ Confluence: ${setup.confluenceCount}/9 | Confidence: ${setup.confidence}%`);

    state.dailyTrades++;
    orderInfo.status = 'open';
    state.positions[setup.symbol] = orderInfo;
    saveState(state);
    return { executed: false, dryRun: true, orderInfo };
  }

  // ── LIVE EXECUTION ──
  console.log(`  [LIVE] ${tier.name} — Placing ${setup.direction} ${setup.symbol}...`);

  // Step 1: Persist intent BEFORE the order so a crash between order + SL
  // still leaves a breadcrumb for the risk-manager to discover and close.
  state.positions[setup.symbol] = orderInfo;
  saveState(state);

  try {
    hlCmd('perp', 'order', symbolPerp, '--side', side, '--size', String(size), '--leverage', String(leverage), '--strategy-id', 'kairos-floor-strategy');
    console.log(`  [EXECUTOR] ✓ Position opened`);
    orderInfo.status = 'open_no_sl';
    state.positions[setup.symbol] = orderInfo;
    saveState(state);
  } catch (e) {
    // Entry failed — remove the pending breadcrumb and abort.
    delete state.positions[setup.symbol];
    saveState(state);
    console.error(`  [EXECUTOR] ✗ Entry order failed: ${e.message}`);
    return { executed: false, error: `entry: ${e.message}` };
  }

  // Step 2: Place SL. If it fails, emergency-close the position — a naked
  // position is unacceptable. Retry SL once, then close if both attempts fail.
  let slPlaced = false;
  let slError = null;
  for (let attempt = 1; attempt <= 2 && !slPlaced; attempt++) {
    try {
      hlCmd('perp', 'order', symbolPerp,
        '--side', closeSide,
        '--size', String(size),
        '--reduce-only',
        '--order-type', 'stop',
        '--trigger-price', String(setup.slPrice),
        '--strategy-id', 'kairos-floor-strategy');
      slPlaced = true;
      console.log(`  [EXECUTOR] ✓ Stop-loss set at ${setup.slPrice}`);
    } catch (e) {
      slError = e.message;
      console.error(`  [EXECUTOR] ✗ SL attempt ${attempt} failed: ${e.message}`);
    }
  }

  if (!slPlaced) {
    console.error('  [EXECUTOR] ⚠ SL placement failed — emergency closing naked position');
    try {
      hlCmd('perp', 'close', symbolPerp, '--strategy-id', 'kairos-floor-strategy');
      console.error('  [EXECUTOR] ✓ Emergency close succeeded');
    } catch (closeErr) {
      console.error(`  [EXECUTOR] ✗✗ EMERGENCY CLOSE FAILED — MANUAL INTERVENTION REQUIRED: ${closeErr.message}`);
      orderInfo.status = 'ORPHAN_NO_SL';
      orderInfo.alert = 'Naked position — manual close required';
      state.positions[setup.symbol] = orderInfo;
      saveState(state);
    }
    if (orderInfo.status !== 'ORPHAN_NO_SL') {
      delete state.positions[setup.symbol];
      saveState(state);
    }
    return { executed: false, error: `sl: ${slError}` };
  }

  state.dailyTrades++;
  orderInfo.status = 'open';
  state.positions[setup.symbol] = orderInfo;
  saveState(state);
  return { executed: true, dryRun: false, orderInfo };
}

// ══════════════════════════════════════════════════════
// MAIN CYCLE
// ══════════════════════════════════════════════════════

async function runCycle(symbols) {
  // Kill switch — if `.kairos-data/HALT` exists, refuse to trade.
  const haltMsg = checkHaltFile(STATE_DIR);
  if (haltMsg) {
    console.error(`[KAIROS] ⛔ HALT active: ${haltMsg}`);
    console.error(`[KAIROS] Remove ${join(STATE_DIR, 'HALT')} to resume trading.`);
    return;
  }

  const state = loadState();
  CONFIG.mode = state.mode;

  const ts = new Date().toISOString();
  console.log(`\n╔══════════════════════════════════════════╗`);
  console.log(`║  KAIROS FLOOR CYCLE  ${ts.slice(11, 19)} UTC  ║`);
  console.log(`║  Mode: ${state.mode.padEnd(8)} | Trades: ${String(state.dailyTrades).padStart(3)}/${CONFIG.maxTradesPerDay}  ║`);
  console.log(`╚══════════════════════════════════════════╝`);

  // ── Phase 1: Sentinel ──
  console.log('\n[1/4] SENTINEL — Analyzing market regime...');
  let bulletin;
  try {
    bulletin = await runSentinel(symbols);
  } catch (e) {
    console.error(`      ✗ Sentinel failed: ${e.message}`);
    return;
  }
  console.log(`      Regime: ${bulletin.regime} | Direction: ${bulletin.direction} | Strength: ${bulletin.strength}/5`);
  console.log(`      ADX: ${bulletin.adx?.toFixed(1)} | RSI: ${bulletin.rsi?.toFixed(1)} | Fear&Greed: ${bulletin.fearGreed}`);

  if (!bulletin.tradable) {
    console.log('      Market NOT tradable — sitting out this cycle.');
    return;
  }

  // ── Phase 2: Scanner ──
  console.log('\n[2/4] SCANNER — Scanning for setups...');
  let setups;
  try {
    setups = await runScanner(symbols, bulletin);
  } catch (e) {
    console.error(`      ✗ Scanner failed: ${e.message}`);
    return;
  }
  console.log(`      Found ${setups.length} setup(s)`);
  for (const s of setups) {
    console.log(`      · ${s.direction} ${s.symbol} | Confluence: ${s.confluenceCount}/9 | Confidence: ${s.confidence}%`);
  }

  if (setups.length === 0) {
    console.log('      No setups meet minimum criteria.');
    return;
  }

  // ── Phases 3+4: Guardian + Executor per setup ──
  const openPositions = getOpenPositions(state);

  for (let i = 0; i < setups.length; i++) {
    const setup = setups[i];
    console.log(`\n[3/4] GUARDIAN — Validating ${setup.direction} ${setup.symbol}...`);
    const verdict = await runGuardian(setup, state, openPositions);
    console.log(`      Verdict: ${verdict.verdict} — ${verdict.reasoning}`);

    if (verdict.verdict === 'GO') {
      console.log(`\n[4/4] EXECUTOR — Executing ${setup.direction} ${setup.symbol}...`);
      const result = await runExecutor(setup, state);
      if (result.executed || result.dryRun) {
        console.log(`      ✓ ${result.dryRun ? 'Dry-run recorded' : 'Trade executed!'}`);
        openPositions.push({ symbol: setup.symbol, direction: setup.direction });
      }
      if (i < setups.length - 1) {
        console.log(`\n      Staggering ${CONFIG.staggerMs / 1000}s before next setup...`);
        await new Promise(r => setTimeout(r, CONFIG.staggerMs));
      }
    }
  }

  console.log('\n╔══ CYCLE COMPLETE ══╝');
}

// ══════════════════════════════════════════════════════
// AUTO MODE — Continuous Trading
// ══════════════════════════════════════════════════════

async function runAuto(symbols, durationMin = 60) {
  const endTime = Date.now() + durationMin * 60_000;
  console.log(`[AUTO] Starting autonomous trading for ${durationMin} minutes...`);
  console.log(`[AUTO] Emergency stop: create ${join(STATE_DIR, 'HALT')} to halt at next boundary.`);
  while (Date.now() < endTime) {
    const haltMsg = checkHaltFile(STATE_DIR);
    if (haltMsg) {
      console.error(`[AUTO] ⛔ HALT active: ${haltMsg} — stopping.`);
      return;
    }
    await runCycle(symbols);
    const remaining = Math.ceil((endTime - Date.now()) / 60_000);
    if (remaining <= 0) break;
    console.log(`\n[AUTO] Next cycle in 2min. ~${remaining}min remaining.\n`);
    await new Promise(r => setTimeout(r, CONFIG.scanIntervalMs));
  }
  console.log('[AUTO] Duration elapsed — autonomous trading stopped.');
}

// ══════════════════════════════════════════════════════
// STATS
// ══════════════════════════════════════════════════════

function showStats() {
  const state = loadState();
  console.log('\n═══ KAIROS FLOOR — PERFORMANCE STATS ═══');
  console.log(`Mode:              ${state.mode}`);
  console.log(`Today's trades:    ${state.dailyTrades} / ${CONFIG.maxTradesPerDay}`);
  console.log(`Daily loss:        ${state.dailyLoss.toFixed(2)}%`);
  console.log(`Consecutive losses:${state.consecutiveLosses}`);
  console.log(`Open positions:    ${Object.keys(state.positions).length}`);
  if (Object.keys(state.positions).length > 0) {
    console.log('\nOpen Positions:');
    for (const [sym, pos] of Object.entries(state.positions)) {
      const age = Math.round((Date.now() - pos.openedAt) / 60_000);
      console.log(`  ${pos.direction} ${sym} | Entry: ${pos.entryPrice} | SL: ${pos.slPrice} | ${age}min open`);
    }
  }
  if (state.tradeHistory.length > 0) {
    const wins  = state.tradeHistory.filter(t => t.pnl > 0).length;
    const total = state.tradeHistory.length;
    const totalPnl = state.tradeHistory.reduce((s, t) => s + (t.pnl ?? 0), 0);
    console.log(`\nHistory (${total} trades):`);
    console.log(`  Win rate:  ${((wins / total) * 100).toFixed(1)}% (${wins}W / ${total - wins}L)`);
    console.log(`  Total PnL: ${totalPnl >= 0 ? '+' : ''}${totalPnl.toFixed(2)} USDC`);
  } else {
    console.log('\nNo completed trades yet.');
  }
  console.log('═══════════════════════════════════════════');
}

// ══════════════════════════════════════════════════════
// CLI INTERFACE
// ══════════════════════════════════════════════════════

const args = process.argv.slice(2);
const command = args[0];

// Parse flags
const symIdx  = args.indexOf('--symbols');
const durIdx  = args.indexOf('--duration');
const setupIdx = args.indexOf('--setup');

// SECURITY: validate symbols — only uppercase alphanumerics, length 1-10
const rawSymbols = symIdx !== -1 ? (args[symIdx + 1] ?? '').split(',').map(s => s.trim().toUpperCase()).filter(Boolean) : CONFIG.symbols;
if (!validateSymbolList(rawSymbols)) {
  console.error(`[KAIROS] Invalid --symbols: each must match /^[A-Z0-9]{1,10}$/ and list length 1-30.`);
  process.exit(1);
}
const symbols = rawSymbols;

// SECURITY: cap --duration to 480 minutes (8h) to prevent runaway auto sessions
const parsedDuration = durIdx !== -1 ? parseInt(args[durIdx + 1], 10) : 60;
if (durIdx !== -1 && (!Number.isFinite(parsedDuration) || parsedDuration <= 0 || parsedDuration > 480)) {
  console.error('[KAIROS] --duration must be a positive integer ≤ 480 minutes.');
  process.exit(1);
}
const duration = Math.min(Math.max(parsedDuration || 60, 1), 480);

// SECURITY: --mode CLI flag is intentionally removed. Mode is read from persisted
// state only (via `config.mjs set-mode live --confirm`). This prevents dry-run bypass.
if (args.indexOf('--mode') !== -1) {
  console.error('[KAIROS] --mode flag is disabled. Use `node scripts/config.mjs set-mode <dry-run|live> --confirm` instead.');
  process.exit(1);
}

switch (command) {
  case 'sentinel':
    runSentinel(symbols)
      .then(r => console.log(JSON.stringify(r, null, 2)))
      .catch(e => { console.error(e.message); process.exit(1); });
    break;

  case 'scan':
    (async () => {
      const bulletin = await runSentinel(symbols);
      const setups   = await runScanner(symbols, bulletin);
      console.log(JSON.stringify({ bulletin, setups }, null, 2));
    })().catch(e => { console.error(e.message); process.exit(1); });
    break;

  case 'guardian': {
    if (setupIdx === -1) { console.error('Usage: kairos-engine.mjs guardian --setup <JSON>'); process.exit(1); }
    let setup;
    try { setup = JSON.parse(args[setupIdx + 1]); }
    catch (e) { console.error(`Invalid --setup JSON: ${e.message}`); process.exit(1); }
    const verr = validateSetup(setup);
    if (verr) { console.error(`Invalid setup: ${verr}`); process.exit(1); }
    const state = loadState();
    runGuardian(setup, state, getOpenPositions(state))
      .then(r => console.log(JSON.stringify(r, null, 2)))
      .catch(e => { console.error(e.message); process.exit(1); });
    break;
  }

  case 'execute': {
    if (setupIdx === -1) { console.error('Usage: kairos-engine.mjs execute --setup <JSON>'); process.exit(1); }
    let setup;
    try { setup = JSON.parse(args[setupIdx + 1]); }
    catch (e) { console.error(`Invalid --setup JSON: ${e.message}`); process.exit(1); }
    const verr = validateSetup(setup);
    if (verr) { console.error(`Invalid setup: ${verr}`); process.exit(1); }
    const state = loadState();
    CONFIG.mode = state.mode === 'live' ? 'live' : 'dry-run';
    runExecutor(setup, state)
      .then(r => console.log(JSON.stringify(r, null, 2)))
      .catch(e => { console.error(e.message); process.exit(1); });
    break;
  }

  case 'cycle':
    runCycle(symbols).catch(e => { console.error(e.message); process.exit(1); });
    break;

  case 'auto':
    runAuto(symbols, duration).catch(e => { console.error(e.message); process.exit(1); });
    break;

  case 'stats':
    showStats();
    break;

  default:
    console.log(`
KAIROS Floor Strategy Engine — OKX Plugin Store Challenge

Usage: node scripts/kairos-engine.mjs <command> [options]

Commands:
  sentinel              Run Phase 1: Market regime analysis
  scan                  Run Phase 1+2: Regime + setup detection
  guardian --setup <J>  Run Phase 3: Validate a setup (JSON string)
  execute  --setup <J>  Run Phase 4: Execute a setup (JSON string)
  cycle                 Run full 4-phase pipeline (single cycle)
  auto                  Run continuously [--duration <minutes>]
  stats                 Show performance statistics

Options:
  --symbols BTC,ETH,SOL   Symbols to trade (default: BTC,ETH,SOL, max 30)
  --duration <min>        Auto mode duration (1-480 min, default: 60)

Mode control:
  Mode is read from persisted state only. To switch:
    node scripts/config.mjs set-mode live --confirm
    node scripts/config.mjs set-mode dry-run

Examples:
  node scripts/kairos-engine.mjs cycle --symbols BTC,ETH
  node scripts/kairos-engine.mjs auto --duration 120
  node scripts/kairos-engine.mjs sentinel
`);
}
