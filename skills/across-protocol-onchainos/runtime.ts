// runtime.ts — runtime backing for across-protocol-onchainos tools.
//
// Form: B-REST (v1.14 taxonomy). Wraps the Across Swap API at app.across.to/api.
// Each transaction tool in index.ts calls a REST endpoint via the apiClient
// below, validates the response shape, and reshapes the returned
// {swapTx: {to, data, value}} (or equivalent) into a pending_sign envelope.
// Each read-only tool returns the API response as {status:'ok', data}.
//
// Modes:
//   • MOCK   (default for offline smoke testing): canned fixtures, no network
//   • REAL   (live): real fetch() calls against app.across.to/api
//
// Switch with:  ACROSS_RUNTIME=real
//
// Helpers exported below (parseUnits/formatUnits/lc/toBigInt/validateDecimals/
// nativeBalanceCheck/erc20Approve) are the standard v1.14 scaffold set —
// imported by tools in index.ts.

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
//   Real-mode helpers — P1-10 (lc) and P1-11 (toBigInt)
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
//   Across API client — starter
// ═══════════════════════════════════════════════════════════════════════════════
//
// The Across API is REST-based. Endpoints are documented in:
//   - swap/SKILL.md (Swap API: GET/POST /swap/approval)
//   - bridge/SKILL.md (Legacy: GET /suggested-fees)
//   - tracking-transactions/SKILL.md (GET /deposit/status)
//   - fetch-chains-tokens/SKILL.md (GET /available-routes, /limits)
//
// Base URLs:
//   - mainnet: https://app.across.to/api
//   - testnet: https://testnet.across.to/api
//
// The starter client below is a thin fetch wrapper. Extend with typed methods
// per endpoint as you build out index.ts tools.

const ACROSS_BASE_URL =
  process.env.ACROSS_API_BASE_URL ?? 'https://app.across.to/api';

export const acrossApi = {
  async get<T = unknown>(path: string, params?: Record<string, string | number | undefined>): Promise<T> {
    const qs = params
      ? '?' + new URLSearchParams(
          Object.fromEntries(
            Object.entries(params).filter(([_, v]) => v !== undefined).map(([k, v]) => [k, String(v)])
          )
        ).toString()
      : '';
    const r = await fetch(`${ACROSS_BASE_URL}${path}${qs}`);
    if (!r.ok) {
      const body = await r.text().catch(() => '');
      throw new Error(`Across API ${r.status} on GET ${path}: ${body.slice(0, 200)}`);
    }
    return r.json() as Promise<T>;
  },

  async post<T = unknown>(path: string, body: unknown): Promise<T> {
    const r = await fetch(`${ACROSS_BASE_URL}${path}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!r.ok) {
      const text = await r.text().catch(() => '');
      throw new Error(`Across API ${r.status} on POST ${path}: ${text.slice(0, 200)}`);
    }
    return r.json() as Promise<T>;
  },
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Chain registry — Across supports 20+ chains
// ═══════════════════════════════════════════════════════════════════════════════
//
// Populate from GET /available-routes if you need the full list, or hand-curate
// the ones you support in index.ts.

export const CHAINS: Record<string, { id: number; caip2: string }> = {
  ethereum:   { id: 1,     caip2: 'eip155:1' },
  arbitrum:   { id: 42161, caip2: 'eip155:42161' },
  base:       { id: 8453,  caip2: 'eip155:8453' },
  optimism:   { id: 10,    caip2: 'eip155:10' },
  polygon:    { id: 137,   caip2: 'eip155:137' },
  bsc:        { id: 56,    caip2: 'eip155:56' },
  // solana intentionally omitted from default chain map — Across supports it
  // but it uses a different (non-EIP155) call shape; handle separately when needed.
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
