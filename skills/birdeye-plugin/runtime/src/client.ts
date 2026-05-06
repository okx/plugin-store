import { createKeyPairSignerFromBytes } from '@solana/kit';
import { ExactSvmScheme, toClientSvmSigner } from '@x402/svm';
import { wrapFetchWithPayment, x402Client } from '@x402/fetch';
import { base58 } from '@scure/base';
import { getApiKey, getMode, getSolanaPrivateKey } from './config.js';

const BASE = 'https://public-api.birdeye.so';

type Resolved = {
  mode: 'apikey' | 'x402';
  baseUrl: string;
  fetcher: typeof fetch;
  headers: Record<string, string>;
};

function generatePaymentId() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let id = 'pay_';
  for (let i = 0; i < 20; i++) id += chars[Math.floor(Math.random() * chars.length)];
  return id;
}

function withPaymentIdentifier(baseFetch: typeof fetch): typeof fetch {
  return (async (input: RequestInfo | URL, init?: RequestInit) => {
    const req = new Request(input, init);
    const sig = req.headers.get('PAYMENT-SIGNATURE');
    if (sig) {
      try {
        const decoded = JSON.parse(Buffer.from(sig, 'base64').toString('utf-8'));
        decoded.extensions = {
          ...(decoded.extensions || {}),
          'payment-identifier': {
            info: { id: generatePaymentId() }
          }

        };
        req.headers.set('PAYMENT-SIGNATURE', Buffer.from(JSON.stringify(decoded)).toString('base64'));
      } catch {}
    }
    return baseFetch(req);
  }) as typeof fetch;
}

export function resolveMode(): 'apikey' | 'x402' {
  const mode = getMode();
  const apiKey = getApiKey();
  const solPk = getSolanaPrivateKey();

  if (mode === 'apikey') {
    if (!apiKey) throw new Error('BIRDEYE_API_KEY is required in apikey mode');
    return 'apikey';
  }
  if (mode === 'x402') {
    if (!solPk) throw new Error('SOLANA_PRIVATE_KEY (base58) is required in x402 mode');
    return 'x402';
  }
  if (apiKey) return 'apikey';
  if (solPk) return 'x402';
  throw new Error('auto mode failed: set BIRDEYE_API_KEY or SOLANA_PRIVATE_KEY');
}

async function createX402Fetch(): Promise<typeof fetch> {
  const pk = getSolanaPrivateKey();
  if (!pk) throw new Error('SOLANA_PRIVATE_KEY (base58) is required in x402 mode');

  const keyBytes = base58.decode(pk);
  const keypair = await createKeyPairSignerFromBytes(keyBytes);
  const signer = toClientSvmSigner(keypair);
  const client = new x402Client().register('solana:*', new ExactSvmScheme(signer));

  return wrapFetchWithPayment(withPaymentIdentifier(fetch), client);
}

export async function createClient(chain = 'solana'): Promise<Resolved> {
  const mode = resolveMode();

  if (mode === 'apikey') {
    return {
      mode,
      baseUrl: BASE,
      fetcher: fetch,
      headers: { 'X-API-KEY': getApiKey() as string, 'x-chain': chain, accept: 'application/json' }
    };
  }

  return {
    mode,
    baseUrl: `${BASE}/x402`,
    fetcher: await createX402Fetch(),
    headers: { 'x-chain': chain, accept: 'application/json' }
  };
}

export async function birdeyeGet(path: string, params: Record<string, string>, chain = 'solana') {
  const client = await createClient(chain);
  const url = new URL(`${client.baseUrl}${path}`);
  for (const [k, v] of Object.entries(params)) if (v) url.searchParams.set(k, v);

  const res = await client.fetcher(url.toString(), { headers: client.headers });
  const text = await res.text();
  if (!res.ok) throw new Error(`Birdeye request failed (${res.status}): ${text}`);
  return JSON.parse(text);
}
