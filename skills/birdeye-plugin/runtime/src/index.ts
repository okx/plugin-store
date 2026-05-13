import { birdeyeGet, resolveMode } from './client.js';
import { APIKEY_ENDPOINTS, type EndpointDef } from './endpoints-apikey.js';
import { filterX402 } from './endpoints-x402.js';

const SAFE_FIELDS: Record<string, string[]> = {
  price: ['address', 'value', 'updateUnixTime'],
  token_overview: ['address', 'symbol', 'name', 'price', 'liquidity', 'marketCap'],
  token_security: ['address', 'top10HolderPercent', 'totalSupply', 'isOnAllowList'],
  token_trending: ['address', 'symbol', 'name', 'price', 'liquidity', 'marketCap', 'rank'],
  price_volume_single: ['address', 'price', 'volume24h'],
  historical_price_unix: ['address', 'value', 'updateUnixTime'],
  history_price: ['items'],
  search_v3: ['items'],
  token_list_v3: ['items'],
  token_meme_list_v3: ['items'],
  token_meta_data_single_v3: ['address', 'symbol', 'name', 'decimals', 'logoURI'],
  token_market_data_v3: ['address', 'price', 'liquidity', 'marketCap'],
  token_holder_v3: ['items'],
  token_txs_v3: ['items'],
  ohlcv_v3: ['items'],
  ohlcv_pair_v3: ['items'],
  price_stats_single_v3: ['address', 'priceChangePercent', 'volumeChangePercent'],
  new_listing_v2: ['items'],
  top_traders_v2: ['items'],
  markets_v2: ['items'],
  trader_gainers_losers: ['items'],
  smart_money_list: ['items'],
  holder_distribution: ['items'],
};

function pickFields(value: unknown, fields: string[]): unknown {
  if (!value || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value;
  const out: Record<string, unknown> = {};
  const obj = value as Record<string, unknown>;
  for (const k of fields) if (k in obj) out[k] = obj[k];
  return out;
}

function sanitizeResponse(endpointKey: string, data: unknown): unknown {
  const safe = SAFE_FIELDS[endpointKey];
  if (!safe) {
    throw new Error(`No safe output whitelist for endpoint: ${endpointKey}`);
  }
  if (typeof data !== 'object' || data === null) return data;
  const root = data as Record<string, unknown>;
  if ('data' in root) {
    return { success: root.success, data: pickFields(root.data, safe) };
  }
  return pickFields(root, safe);
}

function arg(name: string, fallback = ''): string {
  const i = process.argv.indexOf(`--${name}`);
  if (i === -1 || i + 1 >= process.argv.length) return fallback;
  return process.argv[i + 1];
}

function collectParams(argv: string[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith('--')) continue;
    const key = a.slice(2);
    const val = argv[i + 1];
    if (!val || val.startsWith('--')) continue;
    out[key] = val;
    i++;
  }
  return out;
}

function getEndpointsForMode(mode: 'apikey' | 'x402'): EndpointDef[] {
  if (mode === 'x402') return filterX402(APIKEY_ENDPOINTS);
  return APIKEY_ENDPOINTS;
}

function findEndpoint(key: string, mode: 'apikey' | 'x402'): EndpointDef | undefined {
  return getEndpointsForMode(mode).find((e) => e.key === key);
}

function ensureRequired(endpoint: EndpointDef, params: Record<string, string>) {
  for (const req of endpoint.required || []) {
    if (!params[req]) throw new Error(`Missing required param --${req} for endpoint ${endpoint.key}`);
  }
}

function assertNodeForX402(mode: 'apikey' | 'x402') {
  if (mode !== 'x402') return;
  const major = Number(process.versions.node.split('.')[0] || '0');
  if (major < 20) {
    throw new Error(`x402 requires Node.js 20+. Current: ${process.versions.node}`);
  }
}

async function runCall(endpointKey: string, chain: string, params: Record<string, string>) {
  const mode = resolveMode();
  assertNodeForX402(mode);
  const ep = findEndpoint(endpointKey, mode);
  if (!ep) throw new Error(`Endpoint not available in mode=${mode}: ${endpointKey}`);
  ensureRequired(ep, params);
  const data = await birdeyeGet(ep.path, params, chain);
  const filtered = sanitizeResponse(endpointKey, data);
  console.log(JSON.stringify(filtered, null, 2));
}

async function main() {
  const cmd = process.argv[2];
  const chain = arg('chain', 'solana');

  if (cmd === 'list') {
    const modeArg = (arg('mode') as 'apikey' | 'x402') || resolveMode();
    const list = getEndpointsForMode(modeArg).map((e) => ({ key: e.key, path: e.path, required: e.required || [] }));
    console.log(JSON.stringify(list, null, 2));
    return;
  }

  if (cmd === 'price') return runCall('price', chain, { address: arg('address') });
  if (cmd === 'trending') return runCall('token_trending', chain, { sort_by: 'rank', sort_type: 'asc', limit: arg('limit', '20') });
  if (cmd === 'overview') return runCall('token_overview', chain, { address: arg('address') });
  if (cmd === 'security') return runCall('token_security', chain, { address: arg('address') });

  if (cmd === 'call') {
    const endpoint = arg('endpoint');
    if (!endpoint) throw new Error('Missing --endpoint <key>');
    const params = collectParams(process.argv.slice(3));
    delete params.endpoint;
    delete params.chain;
    delete params.mode;
    return runCall(endpoint, chain, params);
  }

  throw new Error('Usage: node dist/index.js list [--mode apikey|x402] | call --endpoint <key> [--chain solana] [--param value...] | price|trending|overview|security');
}

main().catch((e) => {
  console.error(e.message || String(e));
  process.exit(1);
});
