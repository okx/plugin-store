#!/usr/bin/env node
/**
 * market-data.mjs — KAIROS Floor Market Data Layer
 * Wraps Hyperliquid REST API, CoinGecko, and Fear & Greed Index.
 * All fetches include retry logic and graceful fallbacks.
 *
 * Usage:
 *   node scripts/market-data.mjs ping
 *   node scripts/market-data.mjs candles BTC 5m
 *   node scripts/market-data.mjs price BTC
 *   node scripts/market-data.mjs funding BTC
 *   node scripts/market-data.mjs fear-greed
 *   node scripts/market-data.mjs market-overview
 */

const HL_API  = 'https://api.hyperliquid.xyz/info';
const CG_API  = 'https://api.coingecko.com/api/v3';
const FNG_API = 'https://api.alternative.me/fng/';

// ══════════════════════════════════════════════════════
// FETCH WITH RETRY
// ══════════════════════════════════════════════════════

async function fetchWithRetry(url, options = {}, maxRetries = 3, timeoutMs = 10000) {
  let lastError;
  for (let i = 0; i < maxRetries; i++) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const res = await fetch(url, { ...options, signal: controller.signal });
      if (res.status === 429) {
        const wait = (i + 1) * 2000;
        console.warn(`[MARKET-DATA] Rate limited. Waiting ${wait}ms...`);
        await new Promise(r => setTimeout(r, wait));
        continue;
      }
      if (!res.ok) throw new Error(`HTTP ${res.status}: ${url}`);
      return await res.json();
    } catch (e) {
      lastError = e;
      if (i < maxRetries - 1) {
        await new Promise(r => setTimeout(r, (i + 1) * 1000));
      }
    } finally {
      clearTimeout(timer);
    }
  }
  throw lastError;
}

async function hlPost(body, maxRetries = 3) {
  return fetchWithRetry(HL_API, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  }, maxRetries);
}

// ══════════════════════════════════════════════════════
// HYPERLIQUID: CANDLES
// ══════════════════════════════════════════════════════

/**
 * Fetch OHLCV candles from Hyperliquid.
 * @param {string} symbol — e.g. 'BTC', 'ETH', 'SOL'
 * @param {string} interval — '1m'|'5m'|'15m'|'30m'|'1h'|'4h'|'1d'
 * @param {number} lookbackMs — milliseconds to look back
 * @returns {Array<{timestamp, open, high, low, close, volume}>}
 */
export async function fetchCandles(symbol, interval, lookbackMs) {
  const endTime   = Date.now();
  const startTime = endTime - lookbackMs;
  const raw = await hlPost({
    type: 'candleSnapshot',
    req: { coin: symbol, interval, startTime, endTime },
  });
  if (!Array.isArray(raw)) throw new Error(`Unexpected candle format for ${symbol}`);
  return raw.map(c => ({
    timestamp: c.t,
    open:      parseFloat(c.o),
    high:      parseFloat(c.h),
    low:       parseFloat(c.l),
    close:     parseFloat(c.c),
    volume:    parseFloat(c.v),
    numTrades: c.n ?? 0,
  }));
}

// ══════════════════════════════════════════════════════
// HYPERLIQUID: PRICE & MARK
// ══════════════════════════════════════════════════════

/**
 * Fetch current mark price and mid price for a symbol.
 */
export async function fetchPrice(symbol) {
  const [meta, assetCtxs] = await hlPost({ type: 'metaAndAssetCtxs' });
  const idx = meta.universe.findIndex(a => a.name === symbol);
  if (idx === -1) throw new Error(`Symbol ${symbol} not found in Hyperliquid universe`);
  return {
    markPrice: parseFloat(assetCtxs[idx].markPx ?? 0),
    midPrice:  parseFloat(assetCtxs[idx].midPx  ?? 0),
    funding:   parseFloat(assetCtxs[idx].funding ?? 0),
    openInterest: parseFloat(assetCtxs[idx].openInterest ?? 0),
  };
}

// ══════════════════════════════════════════════════════
// HYPERLIQUID: FUNDING RATES
// ══════════════════════════════════════════════════════

/**
 * Fetch funding rate for all or a specific symbol.
 */
export async function fetchFundingRates(symbol = null) {
  const [meta, assetCtxs] = await hlPost({ type: 'metaAndAssetCtxs' });
  const rates = {};
  for (let i = 0; i < meta.universe.length; i++) {
    const name = meta.universe[i].name;
    if (symbol && name !== symbol) continue;
    rates[name] = parseFloat(assetCtxs[i]?.funding ?? 0);
  }
  return symbol ? (rates[symbol] ?? 0) : rates;
}

// ══════════════════════════════════════════════════════
// HYPERLIQUID: ORDER BOOK SPREAD
// ══════════════════════════════════════════════════════

export async function fetchOrderBookSpread(symbol) {
  try {
    const data = await hlPost({ type: 'l2Book', coin: symbol });
    const asks  = data?.levels?.[1];
    const bids  = data?.levels?.[0];
    if (!asks || !bids || asks.length === 0 || bids.length === 0) return null;
    const bestAsk = parseFloat(asks[0]?.[0]?.px ?? 0);
    const bestBid = parseFloat(bids[0]?.[0]?.px ?? 0);
    const mid     = (bestAsk + bestBid) / 2;
    return { bestAsk, bestBid, spread: bestAsk - bestBid, spreadPct: ((bestAsk - bestBid) / mid) * 100 };
  } catch {
    return null;
  }
}

// ══════════════════════════════════════════════════════
// FEAR & GREED INDEX
// ══════════════════════════════════════════════════════

/**
 * Fetch the Crypto Fear & Greed Index (0=Extreme Fear, 100=Extreme Greed).
 */
export async function fetchFearGreed() {
  try {
    const data = await fetchWithRetry(`${FNG_API}?limit=1`);
    const entry = data.data?.[0];
    return {
      value:          parseInt(entry.value, 10),
      classification: entry.value_classification,
      timestamp:      entry.timestamp,
    };
  } catch {
    return { value: 50, classification: 'Neutral', timestamp: Date.now() };
  }
}

// ══════════════════════════════════════════════════════
// COINGECKO: MARKET OVERVIEW
// ══════════════════════════════════════════════════════

/**
 * Fetch global market data: BTC dominance, total market cap, volume.
 */
export async function fetchGlobalMarket() {
  try {
    const data = await fetchWithRetry(`${CG_API}/global`);
    const g = data.data;
    return {
      btcDominance:       parseFloat(g.market_cap_percentage?.btc ?? 50),
      totalMarketCap:     g.total_market_cap?.usd ?? 0,
      total24hVolume:     g.total_volume?.usd ?? 0,
      activeCryptocurrencies: g.active_cryptocurrencies ?? 0,
    };
  } catch {
    return { btcDominance: 50, totalMarketCap: 0, total24hVolume: 0 };
  }
}

/**
 * Fetch 24h price change % for a CoinGecko coin id.
 */
export async function fetchCoinData(cgId = 'bitcoin') {
  try {
    const data = await fetchWithRetry(`${CG_API}/coins/${cgId}?localization=false&tickers=false&community_data=false&developer_data=false`);
    return {
      price:        data.market_data?.current_price?.usd ?? 0,
      change24h:    data.market_data?.price_change_percentage_24h ?? 0,
      change7d:     data.market_data?.price_change_percentage_7d ?? 0,
      volume24h:    data.market_data?.total_volume?.usd ?? 0,
      marketCap:    data.market_data?.market_cap?.usd ?? 0,
    };
  } catch {
    return { price: 0, change24h: 0, change7d: 0, volume24h: 0, marketCap: 0 };
  }
}

// ══════════════════════════════════════════════════════
// CONNECTIVITY CHECK
// ══════════════════════════════════════════════════════

export async function ping() {
  const results = {};
  const checks  = [
    { name: 'Hyperliquid API', fn: () => hlPost({ type: 'meta' }) },
    { name: 'Fear & Greed',    fn: () => fetchWithRetry(`${FNG_API}?limit=1`) },
    { name: 'CoinGecko',       fn: () => fetchWithRetry(`${CG_API}/ping`) },
  ];
  for (const { name, fn } of checks) {
    const t0 = Date.now();
    try {
      await fn();
      results[name] = { ok: true, latencyMs: Date.now() - t0 };
    } catch (e) {
      results[name] = { ok: false, error: e.message };
    }
  }
  return results;
}

// ══════════════════════════════════════════════════════
// FULL MARKET SNAPSHOT (used by Sentinel)
// ══════════════════════════════════════════════════════

export async function fetchMarketSnapshot(symbols = ['BTC', 'ETH', 'SOL']) {
  const [fg, global, ...prices] = await Promise.all([
    fetchFearGreed(),
    fetchGlobalMarket(),
    ...symbols.map(s => fetchPrice(s).catch(() => null)),
  ]);

  return {
    fearGreed:   fg,
    global,
    prices:      Object.fromEntries(symbols.map((s, i) => [s, prices[i]])),
    timestamp:   Date.now(),
  };
}

// ══════════════════════════════════════════════════════
// CLI
// ══════════════════════════════════════════════════════

const args = process.argv.slice(2);
const cmd  = args[0];

switch (cmd) {
  case 'ping':
    ping().then(r => {
      for (const [name, res] of Object.entries(r)) {
        const status = res.ok ? `✓ ${res.latencyMs}ms` : `✗ ${res.error}`;
        console.log(`  ${name.padEnd(20)} ${status}`);
      }
    }).catch(console.error);
    break;

  case 'candles':
    (async () => {
      const symbol   = args[1] ?? 'BTC';
      const interval = args[2] ?? '5m';
      const lookback = parseInt(args[3] ?? '3600000');
      const candles  = await fetchCandles(symbol, interval, lookback);
      console.log(`Fetched ${candles.length} ${interval} candles for ${symbol}`);
      console.log('Latest:', candles[candles.length - 1]);
    })().catch(console.error);
    break;

  case 'price':
    (async () => {
      const symbol = args[1] ?? 'BTC';
      const data   = await fetchPrice(symbol);
      console.log(JSON.stringify({ symbol, ...data }, null, 2));
    })().catch(console.error);
    break;

  case 'funding':
    (async () => {
      const symbol = args[1] ?? null;
      const rates  = await fetchFundingRates(symbol);
      console.log(JSON.stringify(rates, null, 2));
    })().catch(console.error);
    break;

  case 'fear-greed':
    fetchFearGreed().then(r => console.log(JSON.stringify(r, null, 2))).catch(console.error);
    break;

  case 'market-overview':
    fetchMarketSnapshot().then(r => console.log(JSON.stringify(r, null, 2))).catch(console.error);
    break;

  default:
    console.log(`
KAIROS Floor Market Data

Usage: node scripts/market-data.mjs <command> [args]

Commands:
  ping                     Test API connectivity
  candles <SYM> <TF> [ms]  Fetch OHLCV candles
  price <SYM>              Get mark price and funding
  funding [SYM]            Get funding rates (all or specific)
  fear-greed               Get Fear & Greed Index
  market-overview          Full market snapshot

Examples:
  node scripts/market-data.mjs ping
  node scripts/market-data.mjs candles BTC 1h 86400000
  node scripts/market-data.mjs price ETH
  node scripts/market-data.mjs fear-greed
`);
}
