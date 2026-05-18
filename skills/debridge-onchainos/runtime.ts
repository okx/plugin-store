// runtime.ts — runtime backing for debridge-onchainos tools.
//
// Direct REST integration with the deBridge DLN (DeBridge Liquidity Network)
// public API at https://dln.debridge.finance/v1.0/. No MCP intermediary; no
// local signing; pending_sign envelopes route signing to onchainos.

// ═══════════════════════════════════════════════════════════════════════════════
//   Types (re-exported for index.ts)
// ═══════════════════════════════════════════════════════════════════════════════

export type ToolError = {
  status: 'error';
  error_code: string;
  message: string;
  suggestion?: string;
  details?: unknown;
};

export type PendingSign = {
  status: 'pending_sign';
  unsigned_tx: {
    to: string;
    data: string;
    value: string;
    chain: string;
  };
  description: string;
  next_action: { tool: string; reason?: string };
  step_info?: { current: number; total: number; name?: string };
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Numeric helpers (hand-rolled, no deps)
// ═══════════════════════════════════════════════════════════════════════════════

export function parseUnits(amount: string, decimals: number): bigint {
  if (!/^\d+(\.\d+)?$/.test(amount)) {
    throw new Error(`[parseUnits] '${amount}' is not a non-negative decimal`);
  }
  const [whole, frac = ''] = amount.split('.');
  if (frac.length > decimals) {
    throw new Error(`[parseUnits] '${amount}' has more than ${decimals} fractional digits`);
  }
  const padded = whole + frac.padEnd(decimals, '0');
  const trimmed = padded.replace(/^0+(?=\d)/, '');
  return BigInt(trimmed || '0');
}

export function formatUnits(value: bigint, decimals: number): string {
  const s = value.toString().padStart(decimals + 1, '0');
  const whole = s.slice(0, -decimals) || '0';
  const frac = s.slice(-decimals).replace(/0+$/, '');
  return frac ? `${whole}.${frac}` : whole;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Boundary normalizers
// ═══════════════════════════════════════════════════════════════════════════════

export function lc(addr: string): `0x${string}` {
  return addr.toLowerCase() as `0x${string}`;
}

export function toBigInt(v: unknown): bigint {
  if (typeof v === 'bigint') return v;
  if (v === null || v === undefined) return 0n;
  if (typeof v === 'object' && v !== null && 'numerator' in (v as object)) {
    return BigInt((v as { numerator: string | bigint }).numerator);
  }
  if (typeof v === 'string' || typeof v === 'number') return BigInt(v);
  return 0n;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   deBridge DLN REST client
// ═══════════════════════════════════════════════════════════════════════════════
//
// Public, unauthenticated. Endpoint catalog (see SKILL.md Command Index):
//   GET /v1.0/supported-chains-info
//   GET /v1.0/dln/order/quote
//   GET /v1.0/dln/order/create-tx
//   GET /v1.0/dln/order/{orderId}
//   GET /v1.0/chain/transaction        (same-chain swap aggregator)
//
// All inputs are passed as query-string parameters; all responses are JSON.

const DEBRIDGE_BASE_URL =
  process.env.DEBRIDGE_API_BASE_URL ?? 'https://dln.debridge.finance/v1.0';

type Primitive = string | number | boolean | undefined | null;

function buildQS(params?: Record<string, Primitive>): string {
  if (!params) return '';
  const entries = Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== '')
    .map(([k, v]) => [k, String(v)] as [string, string]);
  if (entries.length === 0) return '';
  return '?' + new URLSearchParams(entries).toString();
}

export const debridgeApi = {
  async get<T = unknown>(path: string, params?: Record<string, Primitive>): Promise<T> {
    const url = `${DEBRIDGE_BASE_URL}${path}${buildQS(params)}`;
    const r = await fetch(url);
    if (!r.ok) {
      const body = await r.text().catch(() => '');
      throw new Error(`deBridge DLN ${r.status} on GET ${path}: ${body.slice(0, 300)}`);
    }
    return r.json() as Promise<T>;
  },
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Chain registry — DLN uses internal chainIds that mostly match EIP-155
//   (Solana uses a custom id: 7565164)
// ═══════════════════════════════════════════════════════════════════════════════

export const CHAINS: Record<string, { id: number; caip2: string }> = {
  ethereum:  { id: 1,       caip2: 'eip155:1' },
  arbitrum:  { id: 42161,   caip2: 'eip155:42161' },
  base:      { id: 8453,    caip2: 'eip155:8453' },
  optimism:  { id: 10,      caip2: 'eip155:10' },
  polygon:   { id: 137,     caip2: 'eip155:137' },
  bsc:       { id: 56,      caip2: 'eip155:56' },
  avalanche: { id: 43114,   caip2: 'eip155:43114' },
  linea:     { id: 59144,   caip2: 'eip155:59144' },
  // DLN's Solana chainId is non-EIP155; treat as opaque numeric id.
  solana:    { id: 7565164, caip2: 'solana:mainnet' },
};

export function resolveChain(name: string | undefined): { name: string; id: number; caip2: string } | ToolError {
  const normalized = (name ?? 'ethereum').toLowerCase();
  const entry = CHAINS[normalized];
  if (!entry) {
    return {
      status: 'error',
      error_code: 'UNSUPPORTED_CHAIN',
      message: `Chain '${name}' is not in this skill's allowlist.`,
      suggestion: `Use one of: ${Object.keys(CHAINS).join(', ')}.`,
    };
  }
  return { name: normalized, ...entry };
}

export function chainIdToName(id: number): string {
  for (const [name, info] of Object.entries(CHAINS)) {
    if (info.id === id) return name;
  }
  return `eip155:${id}`;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Pre-broadcast native-balance gate — P1-14
// ═══════════════════════════════════════════════════════════════════════════════
//
// DLN's protocol fee is paid in native gas (e.g. 0.005 BNB on BSC, 0.001 ETH on
// Base). The DLN order tx WILL revert on-chain if the EOA holds less native than
// tx.value. Wire this into every transaction tool that returns non-zero value.

const NATIVE_SYMBOL: Record<string, string> = {
  ethereum: 'ETH',
  arbitrum: 'ETH',
  base:     'ETH',
  optimism: 'ETH',
  linea:    'ETH',
  polygon:  'POL',
  bsc:      'BNB',
  avalanche:'AVAX',
};

export async function nativeBalanceCheck(
  client: { getBalance: (args: { address: `0x${string}` }) => Promise<bigint> },
  chainName: string,
  wallet: string,
  required: bigint,
): Promise<ToolError | null> {
  if (required === 0n) return null;
  let balance: bigint;
  try {
    balance = await client.getBalance({ address: lc(wallet) });
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Could not read native balance for ${wallet} on ${chainName}: ${(e as Error).message}`,
    };
  }
  if (balance < required) {
    const symbol = NATIVE_SYMBOL[chainName] ?? 'native';
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_NATIVE_BALANCE',
      message:
        `Need ${formatUnits(required, 18)} ${symbol} for DLN protocol fee + tx.value, ` +
        `but wallet has only ${formatUnits(balance, 18)} ${symbol} on ${chainName}.`,
      suggestion:
        `Top up ${symbol} on ${chainName}, or use a different source chain where you hold native gas.`,
    };
  }
  return null;
}
