#!/usr/bin/env node
/**
 * config.mjs — KAIROS Floor Configuration Manager
 * Manages trading mode (dry-run / live), symbols, and runtime settings.
 *
 * Usage:
 *   node scripts/config.mjs status
 *   node scripts/config.mjs set-mode live
 *   node scripts/config.mjs set-mode dry-run
 *   node scripts/config.mjs set-symbols BTC,ETH,SOL
 *   node scripts/config.mjs reset
 *   node scripts/config.mjs show
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { atomicWriteJson } from './lib/safety.mjs';

const STATE_DIR  = join(process.cwd(), '.kairos-data');
const STATE_FILE = join(STATE_DIR, 'state.json');

const DEFAULTS = {
  mode:               'dry-run',
  symbols:            ['BTC', 'ETH', 'SOL'],
  minConfluence:      2,
  minConfidence:      40,
  maxPositions:       6,
  maxTradesPerDay:    100,
  riskPerTrade:       0.01,
  maxLeverage:        25,
  dailyLossLimitPct:  5,
  maxConsecutiveLosses: 5,
  dailyLoss:          0,
  dailyTrades:        0,
  consecutiveLosses:  0,
  lastReset:          new Date().toDateString(),
  positions:          {},
  peakBalance:        0,
  tradeHistory:       [],
};

function loadState() {
  if (!existsSync(STATE_DIR)) mkdirSync(STATE_DIR, { recursive: true });
  if (!existsSync(STATE_FILE)) {
    atomicWriteJson(STATE_FILE, DEFAULTS);
    return { ...DEFAULTS };
  }
  try {
    return JSON.parse(readFileSync(STATE_FILE, 'utf8'));
  } catch {
    console.error('[CONFIG] State file corrupted — resetting to defaults.');
    atomicWriteJson(STATE_FILE, DEFAULTS);
    return { ...DEFAULTS };
  }
}

function saveState(state) {
  atomicWriteJson(STATE_FILE, state);
}

function showStatus(state) {
  console.log('\n═══ KAIROS FLOOR — CONFIG STATUS ═══');
  console.log(`  Mode:                ${state.mode.toUpperCase()}`);
  console.log(`  Symbols:             ${(state.symbols ?? ['BTC', 'ETH', 'SOL']).join(', ')}`);
  console.log(`  Min confluence:      ${state.minConfluence ?? 2}/9`);
  console.log(`  Min confidence:      ${state.minConfidence ?? 40}%`);
  console.log(`  Max positions:       ${state.maxPositions ?? 6}`);
  console.log(`  Risk per trade:      ${((state.riskPerTrade ?? 0.01) * 100).toFixed(1)}%`);
  console.log(`  Max leverage:        ${state.maxLeverage ?? 25}x`);
  console.log(`  Daily loss limit:    ${state.dailyLossLimitPct ?? 5}%`);
  console.log(`  Max consec. losses:  ${state.maxConsecutiveLosses ?? 5}`);
  console.log(`  Max trades/day:      ${state.maxTradesPerDay ?? 100}`);
  console.log('\n  ── Today ──');
  console.log(`  Trades:              ${state.dailyTrades ?? 0}`);
  console.log(`  Daily loss:          ${(state.dailyLoss ?? 0).toFixed(2)}%`);
  console.log(`  Consecutive losses:  ${state.consecutiveLosses ?? 0}`);
  console.log(`  Open positions:      ${Object.keys(state.positions ?? {}).length}`);
  if (state.mode === 'live') {
    console.log('\n  ⚠ LIVE MODE — Real funds at risk');
  } else {
    console.log('\n  ✓ DRY-RUN MODE — Paper trading only');
  }
  console.log('══════════════════════════════════════');
}

const args = process.argv.slice(2);
const cmd  = args[0];

switch (cmd) {
  case 'status': {
    const state = loadState();
    showStatus(state);
    break;
  }

  case 'show': {
    const state = loadState();
    showStatus(state);
    console.log('\nFull state:');
    console.log(JSON.stringify(state, null, 2));
    break;
  }

  case 'set-mode': {
    const newMode = args[1];
    if (!['dry-run', 'live'].includes(newMode)) {
      console.error('Usage: config.mjs set-mode <dry-run|live>');
      process.exit(1);
    }
    if (newMode === 'live') {
      console.log('');
      console.log('  ⚠⚠⚠  WARNING: SWITCHING TO LIVE MODE  ⚠⚠⚠');
      console.log('');
      console.log('  You are about to trade with REAL money on Hyperliquid.');
      console.log('  This strategy can and WILL lose money.');
      console.log('  Make sure you have read and understood the risk disclaimer in SKILL.md.');
      console.log('');
      console.log('  To confirm: run this command again with --confirm flag:');
      console.log('  node scripts/config.mjs set-mode live --confirm');
      console.log('');
      if (!args.includes('--confirm')) break;
    }
    const state = loadState();
    state.mode  = newMode;
    saveState(state);
    console.log(`[CONFIG] Mode set to: ${newMode.toUpperCase()}`);
    if (newMode === 'live') {
      console.log('[CONFIG] ⚠ LIVE MODE ACTIVE — Real funds at risk!');
    } else {
      console.log('[CONFIG] ✓ DRY-RUN MODE — Paper trading active.');
    }
    break;
  }

  case 'set-symbols': {
    const symStr = args[1];
    if (!symStr) { console.error('Usage: config.mjs set-symbols BTC,ETH,SOL'); process.exit(1); }
    const symbols = symStr.split(',').map(s => s.trim().toUpperCase()).filter(Boolean);
    if (symbols.length === 0 || symbols.length > 30) {
      console.error('Provide 1-30 symbols.'); process.exit(1);
    }
    const SYMBOL_RE = /^[A-Z0-9]{1,10}$/;
    for (const s of symbols) {
      if (!SYMBOL_RE.test(s)) {
        console.error(`Invalid symbol "${s}" — must match /^[A-Z0-9]{1,10}$/`);
        process.exit(1);
      }
    }
    const state   = loadState();
    state.symbols = symbols;
    saveState(state);
    console.log(`[CONFIG] Symbols set to: ${symbols.join(', ')}`);
    break;
  }

  case 'set': {
    const key   = args[1];
    const value = args[2];
    if (!key || value === undefined) { console.error('Usage: config.mjs set <key> <value>'); process.exit(1); }

    // Hard-coded bounds — prevents accidental or malicious setting of absurd values.
    const bounds = {
      minConfluence:        { min: 1,   max: 9,    type: 'int' },
      minConfidence:        { min: 0,   max: 100,  type: 'int' },
      maxPositions:         { min: 1,   max: 20,   type: 'int' },
      riskPerTrade:         { min: 0.001, max: 0.05, type: 'float' },
      maxLeverage:          { min: 1,   max: 25,   type: 'int' },
      dailyLossLimitPct:    { min: 0.5, max: 20,   type: 'float' },
      maxConsecutiveLosses: { min: 1,   max: 20,   type: 'int' },
      maxTradesPerDay:      { min: 1,   max: 500,  type: 'int' },
    };

    const bound = bounds[key];
    if (!bound) {
      console.error(`[CONFIG] Unknown or non-settable key: ${key}`);
      console.error(`Allowed keys: ${Object.keys(bounds).join(', ')}`);
      process.exit(1);
    }
    const parsed = bound.type === 'int' ? parseInt(value, 10) : parseFloat(value);
    if (!Number.isFinite(parsed) || parsed < bound.min || parsed > bound.max) {
      console.error(`[CONFIG] ${key} must be ${bound.type} in [${bound.min}, ${bound.max}], got: ${value}`);
      process.exit(1);
    }
    const state = loadState();
    state[key] = parsed;
    saveState(state);
    console.log(`[CONFIG] ${key} = ${state[key]}`);
    break;
  }

  case 'reset-daily': {
    const state = loadState();
    state.dailyLoss         = 0;
    state.dailyTrades       = 0;
    state.consecutiveLosses = 0;
    state.lastReset         = new Date().toDateString();
    saveState(state);
    console.log('[CONFIG] Daily stats reset.');
    break;
  }

  case 'reset': {
    if (!args.includes('--confirm')) {
      console.log('This will reset ALL state including trade history.');
      console.log('Run with --confirm to proceed: config.mjs reset --confirm');
      break;
    }
    atomicWriteJson(STATE_FILE, DEFAULTS);
    console.log('[CONFIG] State fully reset to defaults.');
    break;
  }

  case 'export': {
    const state = loadState();
    console.log(JSON.stringify(state, null, 2));
    break;
  }

  default:
    console.log(`
KAIROS Floor Config Manager

Usage: node scripts/config.mjs <command> [args]

Commands:
  status                     Show current configuration and daily stats
  show                       Show config + full state JSON
  set-mode <dry-run|live>    Switch trading mode (live requires --confirm)
  set-symbols <BTC,ETH,SOL>  Change trading symbols
  set <key> <value>          Set a specific config value
  reset-daily                Reset daily loss/trade counters
  reset [--confirm]          Full state reset to defaults
  export                     Print full state as JSON

Examples:
  node scripts/config.mjs status
  node scripts/config.mjs set-mode live --confirm
  node scripts/config.mjs set-symbols BTC,ETH,SOL,ARB
  node scripts/config.mjs set maxPositions 4
`);
}
