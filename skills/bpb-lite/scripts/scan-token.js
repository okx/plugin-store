#!/usr/bin/env node

/**
 * BPB Lite public scanner.
 *
 * This version is intentionally lightweight and public-safe. It uses broadly
 * accessible market data plus optional Printr metadata when a bearer token is
 * available in the environment.
 */

import fs from 'fs';
import os from 'os';
import path from 'path';

const GLOBAL_ENV_PATH = path.join(os.homedir(), '.config', 'env', 'global.env');

function loadEnvFile(filePath) {
  if (!fs.existsSync(filePath)) return;
  const raw = fs.readFileSync(filePath, 'utf8');
  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const idx = trimmed.indexOf('=');
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim();
    let value = trimmed.slice(idx + 1).trim();
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(GLOBAL_ENV_PATH);

function isLikelySolanaAddress(value) {
  return typeof value === 'string' && /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value.trim());
}

function fmtUsd(n) {
  const value = Number(n ?? 0);
  if (!Number.isFinite(value) || value <= 0) return '$0';
  if (value >= 1_000_000_000) return `$${(value / 1_000_000_000).toFixed(2)}B`;
  if (value >= 1_000_000) return `$${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `$${(value / 1_000).toFixed(1)}K`;
  return `$${value.toFixed(0)}`;
}

function fmtPct(n, digits = 1) {
  const value = Number(n);
  if (!Number.isFinite(value)) return '—';
  return `${value.toFixed(digits)}%`;
}

function fmtAge(ms) {
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) return '—';
  const totalSeconds = Math.floor(value / 1000);
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function chooseBestPair(pairs = []) {
  return [...pairs].sort((a, b) => {
    const liqA = Number(a?.liquidity?.usd ?? 0);
    const liqB = Number(b?.liquidity?.usd ?? 0);
    const volA = Number(a?.volume?.h24 ?? 0);
    const volB = Number(b?.volume?.h24 ?? 0);
    return (liqB - liqA) || (volB - volA);
  })[0] ?? null;
}

async function fetchDexScreener(ca) {
  const res = await fetch(`https://api.dexscreener.com/latest/dex/tokens/${ca}`);
  if (!res.ok) throw new Error(`DexScreener request failed: ${res.status}`);
  const json = await res.json();
  const pairs = Array.isArray(json?.pairs) ? json.pairs : [];
  return { pairs, bestPair: chooseBestPair(pairs) };
}

function toPrintrCaip10(ca) {
  return `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp:${ca}`;
}

async function fetchPrintrMetadata(ca) {
  const bearer = process.env.PRINTR_BEARER_TOKEN || process.env.BPB_PRINTR_BEARER_TOKEN;
  if (!bearer) return null;
  const url = `https://api-preview.printr.money/v0/tokens/${encodeURIComponent(toPrintrCaip10(ca))}`;
  const res = await fetch(url, {
    headers: {
      Authorization: `Bearer ${bearer}`,
      Accept: 'application/json',
    },
  });
  if (!res.ok) return null;
  return res.json();
}

function normalizeTextParts(parts = []) {
  return parts
    .flatMap((part) => Array.isArray(part) ? part : [part])
    .map((part) => String(part ?? '').toLowerCase())
    .filter(Boolean)
    .join(' ');
}

export function inferPlatform({ ca, bestPair, printr }) {
  if (printr?.id || printr?.tokenId) {
    return {
      platform: 'Printr',
      note: 'Direct token metadata points to Printr as the launch source.',
      confidence: 'High',
    };
  }

  const baseText = normalizeTextParts([
    bestPair?.dexId,
    bestPair?.labels,
    bestPair?.url,
    bestPair?.baseToken?.name,
    bestPair?.baseToken?.symbol,
    bestPair?.quoteToken?.symbol,
    ca,
  ]);

  if (baseText.includes('pumpfun') || ca.toLowerCase().endsWith('pump')) {
    return { platform: 'Pumpfun', note: 'Public pair metadata suggests a Pumpfun-style route.', confidence: 'Medium' };
  }
  if (baseText.includes('launchlab')) {
    return { platform: 'LaunchLab', note: 'Public pair metadata suggests LaunchLab involvement.', confidence: 'Medium' };
  }
  if (baseText.includes('bonk')) {
    return { platform: 'Bonk', note: 'Public pair metadata suggests Bonk ecosystem routing.', confidence: 'Medium' };
  }
  if (baseText.includes('rise')) {
    return { platform: 'RISE', note: 'Public market metadata suggests RISE platform context.', confidence: 'Medium' };
  }
  if (baseText.includes('raydium')) {
    return { platform: 'Raydium', note: 'Public pair metadata suggests Raydium liquidity or routing.', confidence: 'Medium' };
  }
  if (baseText.includes('meteora') || baseText.includes('damm')) {
    return {
      platform: 'Meteora',
      note: 'Public pair metadata points to Meteora as the current liquidity venue.',
      confidence: 'Medium',
    };
  }

  return { platform: 'Unresolved', note: 'Current public evidence does not yet cleanly resolve the platform context.', confidence: 'Low' };
}

export function inferSignal(snapshot = {}) {
  const mc = Number(snapshot.marketCapUsd ?? 0);
  const vol1h = Number(snapshot.volume1hUsd ?? 0);
  const liq = Number(snapshot.liquidityUsd ?? 0);
  const holders = Number(snapshot.holders ?? 0);

  if (mc > 0 && vol1h >= mc && liq >= 10000 && holders >= 150) return 'EARLY';
  if (mc > 0 && vol1h >= mc * 0.6 && liq >= 8000) return 'MOMENTUM';
  return 'CONFIRMATION';
}

export function inferConfidence(snapshot = {}) {
  if (snapshot.attributionConfidence === 'High') return 'High';
  const mc = Number(snapshot.marketCapUsd ?? 0);
  const vol1h = Number(snapshot.volume1hUsd ?? 0);
  const liq = Number(snapshot.liquidityUsd ?? 0);
  const holders = Number(snapshot.holders ?? 0);
  if (mc > 0 && vol1h > 0 && liq >= 10000 && holders >= 150) return 'High';
  if (mc > 0 && (vol1h > 0 || liq > 0)) return 'Medium';
  return 'Low';
}

function buildHeadline(signal) {
  if (signal === 'EARLY') return '🟣 EARLY';
  if (signal === 'MOMENTUM') return '🟢 MOMENTUM';
  return '🔵 CONFIRMATION';
}

function buildEvidence(snapshot) {
  const points = [];
  if (snapshot.attributionConfidence === 'High' && snapshot.platform === 'Printr') points.push('Direct Printr attribution resolved');
  if (Number(snapshot.volume1hUsd) > Number(snapshot.marketCapUsd) && Number(snapshot.marketCapUsd) > 0) points.push('Volume is healthy relative to market cap');
  if (Number(snapshot.liquidityUsd) >= 10000) points.push('Liquidity is established enough for continued monitoring');
  if (Number(snapshot.holders) > 0 && Number(snapshot.holders) < 100) points.push('Holder count is still thin, so structure may be fragile');
  if (snapshot.platform === 'RISE') points.push('Platform evidence suggests RISE context for this move');
  if (points.length === 0) points.push('Current public evidence is still incomplete, but the market structure is readable enough for a first-pass signal view');
  return [...new Set(points)];
}

function buildVerdict(snapshot) {
  const { signal, platform, marketCapUsd, volume1hUsd, liquidityUsd, holders } = snapshot;
  const lines = [];

  if (signal === 'EARLY') lines.push('Early traction is live and the public structure still looks reasonably intact.');
  else if (signal === 'MOMENTUM') lines.push('This looks like an active continuation phase rather than a cold starter.');
  else if (signal === 'CONFIRMATION') lines.push('Structure looks more established here, with less early asymmetry but cleaner confirmation.');

  if (platform === 'Printr') lines.push('Direct attribution supports treating Printr as the launch source in this read.');
  if (platform === 'RISE') lines.push('Platform context points to RISE, which matters for how continuation and momentum are interpreted.');
  if (Number(volume1hUsd) > Number(marketCapUsd) && Number(liquidityUsd) >= 10000) lines.push('Volume is healthy relative to market cap and liquidity is non-trivial.');
  if (Number(holders) > 0 && Number(holders) < 100) lines.push('Holder count is still thin, so structure should be treated carefully.');

  return lines.join(' ');
}

function buildLinks(ca, bestPair) {
  const pairUrl = bestPair?.url || null;
  const lines = [];
  if (pairUrl) lines.push(`DEX: ${pairUrl}`);
  lines.push(`Explorer: https://solscan.io/token/${ca}`);
  lines.push(`X Search: https://x.com/search?q=${encodeURIComponent(ca)}`);
  return lines.join('\n');
}

async function buildScan(ca) {
  const { pairs, bestPair } = await fetchDexScreener(ca);
  const printr = await fetchPrintrMetadata(ca).catch(() => null);
  const platformHint = inferPlatform({ ca, bestPair, printr });

  const pairCreatedAt = Number(bestPair?.pairCreatedAt ?? 0);
  const ageMs = pairCreatedAt > 0 ? Date.now() - pairCreatedAt : null;
  const snapshot = {
    token: bestPair?.baseToken?.name ?? printr?.name ?? 'UNKNOWN',
    symbol: bestPair?.baseToken?.symbol ?? printr?.symbol ?? 'UNKNOWN',
    contractAddress: ca,
    platform: platformHint.platform,
    platformNote: platformHint.note,
    attributionConfidence: platformHint.confidence,
    marketCapUsd: Number(bestPair?.marketCap ?? bestPair?.fdv ?? 0),
    volume1hUsd: Number(bestPair?.volume?.h1 ?? 0),
    liquidityUsd: Number(bestPair?.liquidity?.usd ?? 0),
    holders: null,
    top10Pct: null,
    smartHolders: null,
    ageMs,
    pairCount: pairs.length,
    dexId: bestPair?.dexId ?? null,
  };

  snapshot.signal = inferSignal(snapshot);
  snapshot.confidence = inferConfidence(snapshot);
  snapshot.evidence = buildEvidence(snapshot);
  snapshot.verdict = buildVerdict(snapshot);
  snapshot.links = buildLinks(ca, bestPair);

  return `${buildHeadline(snapshot.signal)} · BPB Lite\n${snapshot.symbol} (${snapshot.token})\nCA: ${snapshot.contractAddress}\n\n📍 Platform: ${snapshot.platform}\n🎯 Confidence: ${snapshot.confidence}\n💰 MC: ${fmtUsd(snapshot.marketCapUsd)}\n📊 Vol 1h: ${fmtUsd(snapshot.volume1hUsd)}\n💧 Liq: ${fmtUsd(snapshot.liquidityUsd)}\n⏳ Age: ${fmtAge(snapshot.ageMs)}\n🧩 Pairs seen: ${snapshot.pairCount}\n👥 Holders: ${snapshot.holders ?? '—'}\n🏦 Top10: ${snapshot.top10Pct != null ? fmtPct(snapshot.top10Pct) : '—'}\n👀 Smart holders: ${snapshot.smartHolders ?? '—'}\n\nEvidence\n${snapshot.evidence.map((line) => `• ${line}`).join('\n')}\n\nVerdict\n${snapshot.verdict}\n\nPlatform note\n${snapshot.platformNote}\n\nLinks\n${snapshot.links}`;
}

const isMainModule = process.argv[1] && path.resolve(process.argv[1]) === path.resolve(new URL(import.meta.url).pathname);

if (isMainModule) {
  const ca = process.argv[2];

  if (!isLikelySolanaAddress(ca)) {
    console.error('Usage: node scripts/scan-token.js <solana-contract-address>');
    process.exit(1);
  }

  buildScan(ca)
    .then((output) => console.log(output))
    .catch((error) => {
      console.error(`BPB Lite scan failed: ${error.message}`);
      process.exit(1);
    });
}
