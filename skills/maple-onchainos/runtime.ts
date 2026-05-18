// runtime.ts — runtime backing for maple-onchainos tools.
//
// Reads pool state via viem against a curated list of Maple/Syrup pool
// addresses (canonical user-facing pools, not the SDK's infrastructure addresses).
// Returns pending_sign envelopes for all transactions — signing routes through
// `onchainos wallet contract-call`.

import { createPublicClient, http, type PublicClient } from 'viem';
import { base, mainnet } from 'viem/chains';

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
//   Numeric helpers — hand-rolled, no deps.
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
//   Chain registry
// ═══════════════════════════════════════════════════════════════════════════════

export const VIEM_CHAINS = {
  ethereum: mainnet,
  base,
} as const;

export type ChainName = keyof typeof VIEM_CHAINS;

export const CHAINS: Record<ChainName, { id: number; caip2: string }> = {
  ethereum: { id: 1,    caip2: 'eip155:1' },
  base:     { id: 8453, caip2: 'eip155:8453' },
};

export function isSupportedChain(name: string): name is ChainName {
  return name in VIEM_CHAINS;
}

// Public RPC fallbacks — viem's defaults (e.g. eth.merkle.io) can be slow / rate-limited,
// llamarpc sometimes returns "header not found", cloudflare-eth has been flaky too.
// publicnode and 1rpc.io are most reliable in our smoke testing.
// Override via MAPLE_RPC_<CHAIN> env vars if needed.
const PUBLIC_RPCS: Record<ChainName, string> = {
  ethereum: process.env.MAPLE_RPC_ETHEREUM ?? 'https://ethereum.publicnode.com',
  base:     process.env.MAPLE_RPC_BASE     ?? 'https://mainnet.base.org',
};

export function getPublicClient(chain: ChainName): PublicClient {
  return createPublicClient({
    chain: VIEM_CHAINS[chain],
    transport: http(PUBLIC_RPCS[chain]),
  }) as PublicClient;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Curated pool registry — canonical Maple/Syrup pools (ERC-4626)
// ═══════════════════════════════════════════════════════════════════════════════
//
// These are the user-facing pool addresses, NOT the infrastructure addresses
// from maple-labs/maple-js/src/addresses/<chain>.ts (which are mostly Globals,
// Factories, etc.). The SDK doesn't ship a registry of deployed pool instances;
// these are sourced from the Syrup app and verified via Etherscan/Basescan.

export interface PoolInfo {
  chain: ChainName;
  address: `0x${string}`;
  symbol: string;
  underlyingSymbol: string;
  underlyingDecimals: number;
}

export const POOLS: PoolInfo[] = [
  // Ethereum mainnet
  {
    chain: 'ethereum',
    address: '0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b',
    symbol: 'syrupUSDC',
    underlyingSymbol: 'USDC',
    underlyingDecimals: 6,
  },
  {
    chain: 'ethereum',
    address: '0x356b8d89c1e1239cbbb9de4815c39a1474d5ba7d',
    symbol: 'syrupUSDT',
    underlyingSymbol: 'USDT',
    underlyingDecimals: 6,
  },
];

export function findPool(chain: ChainName, poolAddress: string): PoolInfo | undefined {
  const a = lc(poolAddress);
  return POOLS.find((p) => p.chain === chain && lc(p.address) === a);
}

// ═══════════════════════════════════════════════════════════════════════════════
//   ERC-20 / ERC-4626 ABI fragments
// ═══════════════════════════════════════════════════════════════════════════════

export const ERC20_ABI = [
  {
    type: 'function',
    name: 'allowance',
    stateMutability: 'view',
    inputs: [
      { name: 'owner', type: 'address' },
      { name: 'spender', type: 'address' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
  },
  {
    type: 'function',
    name: 'balanceOf',
    stateMutability: 'view',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }],
  },
  {
    type: 'function',
    name: 'symbol',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ name: '', type: 'string' }],
  },
  {
    type: 'function',
    name: 'decimals',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ name: '', type: 'uint8' }],
  },
] as const;

export const ERC4626_ABI = [
  {
    type: 'function',
    name: 'asset',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ name: '', type: 'address' }],
  },
  {
    type: 'function',
    name: 'totalAssets',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
  },
  {
    type: 'function',
    name: 'totalSupply',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ name: '', type: 'uint256' }],
  },
  {
    type: 'function',
    name: 'convertToAssets',
    stateMutability: 'view',
    inputs: [{ name: 'shares', type: 'uint256' }],
    outputs: [{ name: '', type: 'uint256' }],
  },
  {
    type: 'function',
    name: 'previewDeposit',
    stateMutability: 'view',
    inputs: [{ name: 'assets', type: 'uint256' }],
    outputs: [{ name: '', type: 'uint256' }],
  },
] as const;

// Maple pool entrypoints — same selectors as the SDK's
// generateUnsignedTransactionData uses (poolDeposit → deposit, poolQueueWithdrawal → requestRedeem).
export const POOL_ABI = [
  {
    type: 'function',
    name: 'deposit',
    stateMutability: 'nonpayable',
    inputs: [
      { name: 'assets', type: 'uint256' },
      { name: 'receiver', type: 'address' },
    ],
    outputs: [{ name: 'shares', type: 'uint256' }],
  },
  {
    type: 'function',
    name: 'requestRedeem',
    stateMutability: 'nonpayable',
    inputs: [
      { name: 'shares', type: 'uint256' },
      { name: 'owner', type: 'address' },
    ],
    outputs: [{ name: 'escrowed', type: 'uint256' }],
  },
] as const;

// ═══════════════════════════════════════════════════════════════════════════════
//   Pre-broadcast native-balance gate — P1-14
// ═══════════════════════════════════════════════════════════════════════════════

const NATIVE_SYMBOL: Record<ChainName, string> = {
  ethereum: 'ETH',
  base: 'ETH',
};

export async function nativeBalanceCheck(
  client: PublicClient,
  chainName: ChainName,
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
    const symbol = NATIVE_SYMBOL[chainName];
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_NATIVE_BALANCE',
      message:
        `Need ${formatUnits(required, 18)} ${symbol} for tx.value, ` +
        `but wallet has only ${formatUnits(balance, 18)} ${symbol} on ${chainName}.`,
      suggestion: `Top up ${symbol} on ${chainName}.`,
    };
  }
  return null;
}
