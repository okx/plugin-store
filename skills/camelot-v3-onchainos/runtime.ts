// runtime.ts — runtime backing for camelot-v3-onchainos tools.
//
// Direct viem integration with the Algebra SwapRouter + Quoter contracts deployed
// by Camelot Labs on Arbitrum. The official camelot-sdk repo (v0.0.8) ships only
// a Logger and Redis client; all swap logic is encoded against the on-chain ABI.

import { createPublicClient, http, type PublicClient } from 'viem';
import { arbitrum } from 'viem/chains';

// ═══════════════════════════════════════════════════════════════════════════════
//   Types
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
  unsigned_tx: { to: string; data: string; value: string; chain: string };
  description: string;
  next_action: { tool: string; reason?: string };
  step_info?: { current: number; total: number; name?: string };
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Helpers
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
  return BigInt(padded.replace(/^0+(?=\d)/, '') || '0');
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

// ═══════════════════════════════════════════════════════════════════════════════
//   Chain registry + RPC (v1.7 P2-15: pinned to publicnode)
// ═══════════════════════════════════════════════════════════════════════════════

export const VIEM_CHAINS = { arbitrum } as const;
export type ChainName = keyof typeof VIEM_CHAINS;

export const CHAINS: Record<ChainName, { id: number; caip2: string }> = {
  arbitrum: { id: 42161, caip2: 'eip155:42161' },
};

const PUBLIC_RPCS: Record<ChainName, string> = {
  arbitrum: process.env.CAMELOT_RPC_ARBITRUM ?? 'https://arbitrum-one.publicnode.com',
};

export function isSupportedChain(name: string): name is ChainName {
  return name in VIEM_CHAINS;
}

export function getPublicClient(chain: ChainName): PublicClient {
  return createPublicClient({ chain: VIEM_CHAINS[chain], transport: http(PUBLIC_RPCS[chain]) }) as PublicClient;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Camelot V3 (Algebra) contracts — Arbitrum
// ═══════════════════════════════════════════════════════════════════════════════

export const CAMELOT_V3 = {
  arbitrum: {
    swapRouter: '0x1F721E2E82F6676FCE4eA07A5958cF098D339e18' as `0x${string}`,
    quoter:     '0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E' as `0x${string}`,
    nfpm:       '0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15' as `0x${string}`,
  },
};

// ═══════════════════════════════════════════════════════════════════════════════
//   ABI fragments
// ═══════════════════════════════════════════════════════════════════════════════

export const ERC20_ABI = [
  { type: 'function', name: 'allowance', stateMutability: 'view',
    inputs: [{ name: 'owner', type: 'address' }, { name: 'spender', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }] },
  { type: 'function', name: 'balanceOf', stateMutability: 'view',
    inputs: [{ name: 'account', type: 'address' }],
    outputs: [{ name: '', type: 'uint256' }] },
  { type: 'function', name: 'symbol', stateMutability: 'view',
    inputs: [], outputs: [{ name: '', type: 'string' }] },
  { type: 'function', name: 'decimals', stateMutability: 'view',
    inputs: [], outputs: [{ name: '', type: 'uint8' }] },
] as const;

// Algebra SwapRouter — exactInputSingle takes a struct with 7 fields (NOTE: no
// `fee` — Algebra uses dynamic fees per pool, looked up from the pool itself).
export const SWAP_ROUTER_ABI = [
  {
    type: 'function',
    name: 'exactInputSingle',
    stateMutability: 'payable',
    inputs: [{
      name: 'params',
      type: 'tuple',
      components: [
        { name: 'tokenIn',          type: 'address' },
        { name: 'tokenOut',         type: 'address' },
        { name: 'recipient',        type: 'address' },
        { name: 'deadline',         type: 'uint256' },
        { name: 'amountIn',         type: 'uint256' },
        { name: 'amountOutMinimum', type: 'uint256' },
        { name: 'limitSqrtPrice',   type: 'uint160' },
      ],
    }],
    outputs: [{ name: 'amountOut', type: 'uint256' }],
  },
] as const;

// Algebra Quoter — non-view (revert-encoded result) but eth_call works.
// Returns (uint256 amountOut, uint16 fee).
export const QUOTER_ABI = [
  {
    type: 'function',
    name: 'quoteExactInputSingle',
    stateMutability: 'nonpayable',
    inputs: [
      { name: 'tokenIn',        type: 'address' },
      { name: 'tokenOut',       type: 'address' },
      { name: 'amountIn',       type: 'uint256' },
      { name: 'limitSqrtPrice', type: 'uint160' },
    ],
    outputs: [
      { name: 'amountOut', type: 'uint256' },
      { name: 'fee',       type: 'uint16' },
    ],
  },
] as const;

// ═══════════════════════════════════════════════════════════════════════════════
//   Pre-broadcast native-balance gate — P1-14
// ═══════════════════════════════════════════════════════════════════════════════

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
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_NATIVE_BALANCE',
      message: `Need ${formatUnits(required, 18)} ETH for tx.value, but wallet has only ${formatUnits(balance, 18)} ETH on ${chainName}.`,
      suggestion: `Top up ETH on ${chainName}.`,
    };
  }
  return null;
}
