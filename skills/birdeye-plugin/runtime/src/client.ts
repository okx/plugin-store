import { getApiKey, getMode, getSolanaPrivateKey } from './config.js';

const BASE = 'https://public-api.birdeye.so';

type Resolved = {
  mode: 'apikey' | 'x402';
  baseUrl: string;
  fetcher: typeof fetch;
  headers: Record<string, string>;
};

const X402_SUPPORTED_PATHS = new Set([
  '/defi/price',
  '/defi/multi_price',
  '/defi/v3/ohlcv',
  '/defi/history_price',
  '/defi/historical_price_unix',
  '/defi/v3/price/stats/single',
  '/defi/token_overview',
  '/defi/token_security',
  '/defi/v3/token/meta-data/single',
  '/defi/v3/token/market-data',
  '/defi/v3/token/list',
  '/defi/token_trending',
  '/defi/v2/tokens/new_listing',
  '/defi/v3/token/meme/list',
  '/defi/v3/search',
  '/defi/v2/markets',
  '/defi/v3/pair/overview/single',
  '/defi/v3/token/txs',
  '/defi/v2/tokens/top_traders',
  '/trader/gainers-losers',
  '/defi/v3/token/holder',
  '/holder/v1/distribution',
  '/smart-money/v1/token/list'
]);

export function resolveMode(): 'apikey' | 'x402' {
  const mode = getMode();
  const apiKey = getApiKey();
  const solPk = getSolanaPrivateKey();

  if (mode === 'apikey') {
    if (!apiKey) throw new Error('BIRDEYE_API_KEY is required in apikey mode');
    return 'apikey';
  }
  if (mode === 'x402') {
    if (!solPk) throw new Error('SOLANA_PRIVATE_KEY is required in x402 mode');
    return 'x402';
  }

  if (apiKey) return 'apikey';
  if (solPk) return 'x402';
  throw new Error('auto mode failed: set BIRDEYE_API_KEY or SOLANA_PRIVATE_KEY');
}

export function createClient(chain = 'solana'): Resolved {
  const mode = resolveMode();

  if (mode === 'apikey') {
    return {
      mode,
      baseUrl: BASE,
      fetcher: fetch,
      headers: {
        'X-API-KEY': getApiKey() as string,
        'x-chain': chain,
        accept: 'application/json'
      }
    };
  }

  throw new Error(
    'x402 mode is scaffolded but not enabled yet: missing payment client implementation. Use BIRDEYE_MODE=apikey for now.'
  );
}

function ensureX402Support(path: string): void {
  if (!X402_SUPPORTED_PATHS.has(path)) {
    throw new Error(`ENDPOINT_NOT_SUPPORTED_IN_X402: ${path}`);
  }
}

export async function birdeyeGet(path: string, params: Record<string, string>, chain = 'solana') {
  const mode = resolveMode();
  if (mode === 'x402') {
    ensureX402Support(path);
  }

  const client = createClient(chain);
  const url = new URL(`${client.baseUrl}${path}`);
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== '') url.searchParams.set(k, v);
  }

  const res = await client.fetcher(url.toString(), { headers: client.headers });
  const text = await res.text();

  if (!res.ok) {
    throw new Error(`Birdeye request failed (${res.status}): ${text}`);
  }

  return JSON.parse(text);
}
