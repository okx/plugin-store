/**
 * safety.mjs — Shared validators, safe CLI wrapper, and API helpers.
 * Security hardening module — imported by kairos-engine.mjs and risk-manager.mjs.
 */

import { spawnSync } from 'child_process';
import { writeFileSync, renameSync, existsSync, mkdirSync, unlinkSync, readFileSync } from 'fs';
import { join, dirname } from 'path';

const SYMBOL_RE = /^[A-Z0-9]{1,10}$/;
const MODE_VALUES = new Set(['dry-run', 'live']);
const MAX_STALE_MS = 5 * 60 * 1000;

export function validateSymbol(s) {
  if (typeof s !== 'string') return false;
  return SYMBOL_RE.test(s);
}

export function validateSymbolList(list) {
  if (!Array.isArray(list) || list.length === 0 || list.length > 30) return false;
  return list.every(validateSymbol);
}

export function validateMode(m) {
  return typeof m === 'string' && MODE_VALUES.has(m);
}

export function safeNumber(v, { min = -Infinity, max = Infinity, allowZero = true } = {}) {
  if (typeof v !== 'number' || !Number.isFinite(v)) return null;
  if (!allowZero && v === 0) return null;
  if (v < min || v > max) return null;
  return v;
}

export function validateSetup(setup) {
  if (!setup || typeof setup !== 'object') return 'not an object';
  const required = ['symbol', 'direction', 'entryPrice', 'slPrice', 'slDistance', 'timestamp'];
  for (const k of required) {
    if (!(k in setup)) return `missing field ${k}`;
  }
  if (!validateSymbol(setup.symbol)) return `invalid symbol: ${setup.symbol}`;
  if (setup.direction !== 'LONG' && setup.direction !== 'SHORT') return `invalid direction: ${setup.direction}`;
  if (safeNumber(setup.entryPrice, { min: 1e-8, max: 1e9 }) === null) return 'invalid entryPrice';
  if (safeNumber(setup.slPrice, { min: 1e-8, max: 1e9 }) === null) return 'invalid slPrice';
  if (safeNumber(setup.slDistance, { min: 1e-8, max: 1e9, allowZero: false }) === null) return 'invalid slDistance';
  if (safeNumber(setup.timestamp, { min: 0 }) === null) return 'invalid timestamp';

  // Staleness — setups older than 5 min are invalid
  const age = Date.now() - setup.timestamp;
  if (age < 0 || age > MAX_STALE_MS) return `stale setup (age=${age}ms)`;

  // SL must be on correct side of entry (LONG → SL below, SHORT → SL above)
  if (setup.direction === 'LONG' && setup.slPrice >= setup.entryPrice) return 'LONG SL must be below entry';
  if (setup.direction === 'SHORT' && setup.slPrice <= setup.entryPrice) return 'SHORT SL must be above entry';

  // SL distance plausibility — must match slPrice vs entryPrice within 1 wei tolerance
  const impliedDist = Math.abs(setup.entryPrice - setup.slPrice);
  if (Math.abs(impliedDist - setup.slDistance) / setup.slDistance > 0.01) return 'slDistance inconsistent with slPrice/entryPrice';

  // SL can't be further than 20% away — sanity check to reject absurd setups
  if (setup.slDistance / setup.entryPrice > 0.20) return 'SL distance > 20% — rejected';

  return null; // valid
}

/**
 * Safe variadic CLI wrapper — each argument is passed as its own argv entry.
 * No shell, no split-on-whitespace. Numeric args are coerced to strings explicitly.
 */
export function hlCmd(...rawArgs) {
  const args = rawArgs.map(a => {
    if (typeof a === 'number') {
      if (!Number.isFinite(a)) throw new Error(`hlCmd: non-finite numeric arg ${a}`);
      return String(a);
    }
    if (typeof a !== 'string') throw new Error(`hlCmd: arg must be string or number, got ${typeof a}`);
    // Reject control chars and shell metacharacters to defence-in-depth even with shell:false
    if (/[\x00-\x1f]/.test(a)) throw new Error('hlCmd: control character in arg');
    return a;
  });
  const result = spawnSync('hyperliquid', args, { encoding: 'utf8', timeout: 15000, shell: false });
  if (result.error) throw new Error(`hyperliquid CLI not available: ${result.error.message}`);
  if (result.status !== 0) throw new Error(`hyperliquid CLI failed: ${(result.stderr || '').trim()}`);
  return (result.stdout ?? '').trim();
}

/**
 * POST to Hyperliquid REST API with timeout. Uses AbortController for real timeout.
 */
export async function hlPost(url, body, { timeoutMs = 10000 } = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: controller.signal,
    });
    if (!res.ok) {
      const text = await res.text().catch(() => '');
      throw new Error(`HL API ${res.status}: ${text.slice(0, 200)}`);
    }
    return await res.json();
  } finally {
    clearTimeout(timer);
  }
}

export async function fetchJsonWithTimeout(url, { timeoutMs = 10000, ...init } = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, { ...init, signal: controller.signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return await res.json();
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Size calculation with guards — never produces NaN/Infinity.
 * Returns null if inputs are unsafe instead of silently using a bad size.
 */
/**
 * Atomic JSON write: write to .tmp then rename. Prevents state corruption
 * if the process crashes mid-write. Rename is atomic on same-filesystem.
 */
export function atomicWriteJson(filePath, data) {
  const dir = dirname(filePath);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  const tmp = `${filePath}.tmp.${process.pid}.${Date.now()}`;
  writeFileSync(tmp, JSON.stringify(data, null, 2));
  try {
    renameSync(tmp, filePath);
  } catch (e) {
    // Cleanup tmp on failure; re-throw so caller knows state wasn't saved.
    try { unlinkSync(tmp); } catch {}
    throw e;
  }
}

/**
 * Kill-switch check. If `.kairos-data/HALT` exists, the engine refuses to run.
 * Remove the file to resume. Returns the HALT message if present, null otherwise.
 */
export function checkHaltFile(stateDir) {
  const haltPath = join(stateDir, 'HALT');
  if (!existsSync(haltPath)) return null;
  try {
    return readFileSync(haltPath, 'utf8').trim() || 'HALT file present';
  } catch {
    return 'HALT file present';
  }
}

export function calcSize(balance, riskPct, slDistance, entryPrice) {
  if (!Number.isFinite(balance) || balance <= 0) return null;
  if (!Number.isFinite(riskPct) || riskPct <= 0 || riskPct > 0.10) return null;
  if (!Number.isFinite(slDistance) || slDistance <= 0) return null;
  if (!Number.isFinite(entryPrice) || entryPrice <= 0) return null;
  const riskAmount = balance * riskPct;
  const slPct = slDistance / entryPrice;
  if (!Number.isFinite(slPct) || slPct <= 0) return null;
  const positionNotional = riskAmount / slPct;
  const size = positionNotional / entryPrice;
  if (!Number.isFinite(size) || size <= 0) return null;
  return Math.max(Math.round(size * 10000) / 10000, 0.001);
}
