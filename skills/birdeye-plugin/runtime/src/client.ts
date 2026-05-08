import { randomBytes } from 'node:crypto';
import { ExactSvmScheme, toClientSvmSigner } from '@x402/svm';
import { wrapFetchWithPayment, x402Client } from '@x402/fetch';
import { getApiKey, getMaxDailySpend, getMode, getSignerKeyFile } from './config.js';
import { createIpcSigner } from './signer-client.js';

const BASE = 'https://public-api.birdeye.so';

type Resolved = {
  mode: 'apikey' | 'x402';
  baseUrl: string;
  fetcher: typeof fetch;
  headers: Record<string, string>;
};

function generatePaymentId(): string {
  return 'pay_' + randomBytes(15).toString('base64url');
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
          'payment-identifier': { info: { id: generatePaymentId() } },
        };
        req.headers.set('PAYMENT-SIGNATURE', Buffer.from(JSON.stringify(decoded)).toString('base64'));
      } catch (e) {
        console.warn(`[birdeye] payment-identifier injection failed: ${(e as Error).message}`);
      }
    }
    return baseFetch(req);
  }) as typeof fetch;
}

import { existsSync } from 'node:fs';
import { homedir } from 'node:os';

const DEFAULT_KEY_FILE = `${homedir()}/.birdeye/key`;

function hasKeyFile(): boolean {
  const p = getSignerKeyFile() || DEFAULT_KEY_FILE;
  return existsSync(p);
}

export function resolveMode(): 'apikey' | 'x402' {
  const mode = getMode();
  const apiKey = getApiKey();

  if (mode === 'apikey') {
    if (!apiKey) throw new Error('BIRDEYE_API_KEY is required in apikey mode');
    return 'apikey';
  }
  if (mode === 'x402') {
    if (!hasKeyFile()) {
      throw new Error(`x402 mode needs a signer key file. Default: ${DEFAULT_KEY_FILE}. Override via BIRDEYE_SIGNER_KEY_FILE.`);
    }
    return 'x402';
  }
  if (apiKey) return 'apikey';
  if (hasKeyFile()) return 'x402';
  throw new Error('No credentials. Set BIRDEYE_API_KEY (apikey mode) or place a base58 key at ~/.birdeye/key (x402 mode).');
}

async function createX402Fetch(): Promise<typeof fetch> {
  void getMaxDailySpend();
  const ipcSigner = await createIpcSigner();
  const signer = toClientSvmSigner(ipcSigner as never);
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
      headers: { 'X-API-KEY': getApiKey() as string, 'x-chain': chain, accept: 'application/json' },
    };
  }

  return {
    mode,
    baseUrl: `${BASE}/x402`,
    fetcher: await createX402Fetch(),
    headers: { 'x-chain': chain, accept: 'application/json' },
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
