// runtime.ts — runtime backing for renzo-onchainos tools.
//
// Form C (primitive source) — no upstream JS/TS SDK with calldata builders
// exists. This file holds:
//   - Minimal ABIs (verified from Renzo-Protocol/contracts-public@master)
//   - viem public client (Ethereum mainnet) with publicnode default + env override
//   - A thin REST client for app.renzoprotocol.com/api/{apr,stats}
//   - Numeric + address helpers shared with the scaffold pattern
//
// Modes:
//   • MOCK   (default for B-mock smoke testing): canned data, no RPC traffic
//   • REAL   (B-real / live): viem RPC reads + real REST calls
//
// Switch with:  RENZO_RUNTIME=real

import { createPublicClient, http, type PublicClient } from 'viem';
import { mainnet } from 'viem/chains';

// ═══════════════════════════════════════════════════════════════════════════════
//   Types — re-exported so index.ts can import
// ═══════════════════════════════════════════════════════════════════════════════

export type ToolError = {
  status: 'error';
  error_code: string;
  message: string;
  suggestion?: string;
  details?: unknown;
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Numeric helpers — hand-rolled, no deps
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
//   Address helpers
// ═══════════════════════════════════════════════════════════════════════════════

export function lc(addr: string): `0x${string}` {
  return addr.toLowerCase() as `0x${string}`;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Mode toggle + RPC config
// ═══════════════════════════════════════════════════════════════════════════════

const USE_REAL = process.env.RENZO_RUNTIME === 'real';

const PUBLIC_RPCS: Record<string, string> = {
  ethereum: process.env.RENZO_RPC_ETHEREUM ?? 'https://ethereum.publicnode.com',
};

const _publicClients = new Map<string, PublicClient>();
function getPublicClient(chain: string = 'ethereum'): PublicClient {
  if (chain !== 'ethereum') {
    throw new Error(`[runtime] Unsupported chain '${chain}' — v0.1.0 supports ethereum only`);
  }
  if (!_publicClients.has(chain)) {
    _publicClients.set(
      chain,
      createPublicClient({ chain: mainnet, transport: http(PUBLIC_RPCS[chain]) }) as PublicClient,
    );
  }
  return _publicClients.get(chain)!;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Deployed contract addresses (Ethereum mainnet — verified 2026-05-12)
// ═══════════════════════════════════════════════════════════════════════════════
//   Source: https://docs.renzoprotocol.com/docs/contracts/ethereum-mainnet

export const RENZO_ADDRESSES = {
  ethereum: {
    restakeManager: lc('0x74a09653A083691711cF8215a6ab074BB4e99ef5'),
    withdrawQueue:  lc('0x5efc9D10E42FB517456f4ac41EB5e2eBe42C8918'),
    ezETH:          lc('0xbf5495Efe5DB9ce00f80364C8B423567e58d2110'),
  },
} as const;

// ═══════════════════════════════════════════════════════════════════════════════
//   ABIs — minimal slices, verified from Renzo-Protocol/contracts-public@master
// ═══════════════════════════════════════════════════════════════════════════════
//   Sources:
//     - contracts/IRestakeManager.sol
//     - contracts/Withdraw/IWithdrawQueue.sol

export const RESTAKE_MANAGER_ABI = [
  {
    type: 'function' as const,
    name: 'depositETH',
    stateMutability: 'payable' as const,
    inputs: [],
    outputs: [],
  },
  {
    type: 'function' as const,
    name: 'deposit',
    stateMutability: 'nonpayable' as const,
    inputs: [
      { name: '_collateralToken', type: 'address' as const },
      { name: '_amount',          type: 'uint256' as const },
    ],
    outputs: [],
  },
  {
    type: 'function' as const,
    name: 'calculateTVLs',
    stateMutability: 'view' as const,
    inputs: [],
    outputs: [
      { name: '',                type: 'uint256[][]' as const },
      { name: '',                type: 'uint256[]'   as const },
      { name: 'totalTVLInETH',   type: 'uint256'     as const },
    ],
  },
];

export const WITHDRAW_QUEUE_ABI = [
  {
    type: 'function' as const,
    name: 'withdraw',
    stateMutability: 'nonpayable' as const,
    inputs: [
      { name: '_amount',   type: 'uint256' as const },
      { name: '_assetOut', type: 'address' as const },
    ],
    outputs: [],
  },
  {
    type: 'function' as const,
    name: 'claim',
    stateMutability: 'nonpayable' as const,
    inputs: [
      { name: 'withdrawRequestIndex', type: 'uint256' as const },
      { name: 'user',                 type: 'address' as const },
    ],
    outputs: [],
  },
  {
    type: 'function' as const,
    name: 'withdrawRequests',
    stateMutability: 'view' as const,
    inputs: [{ name: '_user', type: 'address' as const }],
    outputs: [{ name: '', type: 'uint256[]' as const }],
  },
  {
    type: 'function' as const,
    name: 'getOutstandingWithdrawRequests',
    stateMutability: 'view' as const,
    inputs: [{ name: 'user', type: 'address' as const }],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
  {
    type: 'function' as const,
    name: 'calculateAmountToRedeem',
    stateMutability: 'view' as const,
    inputs: [
      { name: '_amount',   type: 'uint256' as const },
      { name: '_assetOut', type: 'address' as const },
    ],
    outputs: [
      { name: 'operatorDelegatorTokenTVLs', type: 'uint256[][]' as const },
      { name: '_amountToRedeem',            type: 'uint256'     as const },
    ],
  },
];

export const ERC20_ABI = [
  {
    type: 'function' as const,
    name: 'allowance',
    stateMutability: 'view' as const,
    inputs: [
      { name: 'owner',   type: 'address' as const },
      { name: 'spender', type: 'address' as const },
    ],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
  {
    type: 'function' as const,
    name: 'balanceOf',
    stateMutability: 'view' as const,
    inputs: [{ name: 'account', type: 'address' as const }],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
  {
    type: 'function' as const,
    name: 'totalSupply',
    stateMutability: 'view' as const,
    inputs: [],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
  {
    type: 'function' as const,
    name: 'decimals',
    stateMutability: 'view' as const,
    inputs: [],
    outputs: [{ name: '', type: 'uint8' as const }],
  },
];

// ═══════════════════════════════════════════════════════════════════════════════
//   erc20 — generic ERC-20 reads
// ═══════════════════════════════════════════════════════════════════════════════

export const erc20 = {
  async allowance(token: string, owner: string, spender: string, chain: string = 'ethereum'): Promise<bigint> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      return (await client.readContract({
        address: lc(token),
        abi: ERC20_ABI,
        functionName: 'allowance',
        args: [lc(owner), lc(spender)],
      })) as bigint;
    }
    // MOCK: env-driven, mirrors the scaffold pattern
    return process.env.RENZO_MOCK_ALLOWANCE === 'sufficient' ? (1n << 256n) - 1n : 0n;
  },
  async balanceOf(token: string, owner: string, chain: string = 'ethereum'): Promise<bigint> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      return (await client.readContract({
        address: lc(token),
        abi: ERC20_ABI,
        functionName: 'balanceOf',
        args: [lc(owner)],
      })) as bigint;
    }
    return 0n;
  },
  async totalSupply(token: string, chain: string = 'ethereum'): Promise<bigint> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      return (await client.readContract({
        address: lc(token),
        abi: ERC20_ABI,
        functionName: 'totalSupply',
      })) as bigint;
    }
    // Mock totalSupply ~ 83,906 ezETH (matches live snapshot)
    return parseUnits('83906.690188599870000000', 18);
  },
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Renzo-protocol-specific chain readers (RestakeManager + WithdrawQueue)
// ═══════════════════════════════════════════════════════════════════════════════

export const renzoChain = {
  /** Total TVL across the protocol denominated in ETH (wei). */
  async totalTVLInETH(chain: string = 'ethereum'): Promise<bigint> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      const res = (await client.readContract({
        address: RENZO_ADDRESSES.ethereum.restakeManager,
        abi: RESTAKE_MANAGER_ABI,
        functionName: 'calculateTVLs',
      })) as [bigint[][], bigint[], bigint];
      return res[2];
    }
    // Mock: ~95,367 ETH TVL (matches live snapshot)
    return parseUnits('95367.240504269910000000', 18);
  },

  async calculateAmountToRedeem(amount: bigint, assetOut: string, chain: string = 'ethereum'): Promise<bigint> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      const res = (await client.readContract({
        address: RENZO_ADDRESSES.ethereum.withdrawQueue,
        abi: WITHDRAW_QUEUE_ABI,
        functionName: 'calculateAmountToRedeem',
        args: [amount, lc(assetOut)],
      })) as [bigint[][], bigint];
      return res[1];
    }
    // Mock: 1:1 with input amount
    return amount;
  },

  async withdrawRequests(walletAddress: string, chain: string = 'ethereum'): Promise<bigint[]> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      return (await client.readContract({
        address: RENZO_ADDRESSES.ethereum.withdrawQueue,
        abi: WITHDRAW_QUEUE_ABI,
        functionName: 'withdrawRequests',
        args: [lc(walletAddress)],
      })) as bigint[];
    }
    return [];
  },

  async getOutstandingWithdrawRequests(walletAddress: string, chain: string = 'ethereum'): Promise<bigint> {
    if (USE_REAL) {
      const client = getPublicClient(chain);
      return (await client.readContract({
        address: RENZO_ADDRESSES.ethereum.withdrawQueue,
        abi: WITHDRAW_QUEUE_ABI,
        functionName: 'getOutstandingWithdrawRequests',
        args: [lc(walletAddress)],
      })) as bigint;
    }
    return 0n;
  },
};

// ═══════════════════════════════════════════════════════════════════════════════
//   Pre-broadcast native-balance gate (P1-14)
// ═══════════════════════════════════════════════════════════════════════════════

export async function nativeBalanceCheck(
  wallet: string,
  required: bigint,
  chain: string = 'ethereum',
  nativeSymbol: string = 'ETH',
): Promise<ToolError | null> {
  if (required === 0n) return null;
  if (!USE_REAL) return null; // mock mode: skip gate so smoke tests work without RPC
  let balance: bigint;
  try {
    const client = getPublicClient(chain);
    balance = await client.getBalance({ address: lc(wallet) });
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Could not read native balance for ${wallet}: ${(e as Error).message}`,
    };
  }
  if (balance < required) {
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_NATIVE_BALANCE',
      message:
        `Need ${formatUnits(required, 18)} ${nativeSymbol} (~${(Number(required) / 1e18).toFixed(6)}) ` +
        `but wallet has only ${formatUnits(balance, 18)} ${nativeSymbol}.`,
      suggestion:
        `Top up ${nativeSymbol} on ${chain} before broadcasting.`,
    };
  }
  return null;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   Renzo app API client — read-only enrichment (APR, stats)
// ═══════════════════════════════════════════════════════════════════════════════
//   Base: https://app.renzoprotocol.com/api
//   Verified endpoints:
//     GET /apr   → { apr: number }
//     GET /stats → { success: bool, data: { restakedTVL, ezETH: {price, eth, usd, ...}, ... } }
//
//   NOTE: there is no api.renzoprotocol.com host. Treat any prompt that
//   names that host as a typo for app.renzoprotocol.com/api.

const RENZO_API_BASE = process.env.RENZO_API_BASE ?? 'https://app.renzoprotocol.com/api';

export const renzoApi = {
  async getApr(): Promise<{ apr: number }> {
    if (!USE_REAL) {
      return { apr: 1.5895805126210139 }; // mock snapshot
    }
    const res = await fetch(`${RENZO_API_BASE}/apr`);
    if (!res.ok) {
      throw new Error(`[renzoApi.getApr] HTTP ${res.status} ${res.statusText}`);
    }
    return (await res.json()) as { apr: number };
  },

  async getStats(): Promise<{ success: boolean; data: Record<string, unknown> }> {
    if (!USE_REAL) {
      // Mock mirrors the real response shape verified 2026-05-12. Vault details
      // (ezETH, pzETH, ezREZ, …) live UNDER data.restakedTVL — NOT at data.ezETH.
      return {
        success: true,
        data: {
          restakedTVL: {
            success: true,
            data: { usd: 231_093_046.35, eth: 95_349.3458, ethUSD: 212_937_820.37, solUSD: 6_956_877.09 },
            ezETH: {
              eth: 83_906.690188,
              usd: 193_505_640.74,
              price: 2484.6958998355863,
              underlyingTotalSupply: 83_906.690188,
              lpTotalSupply: 77_879.00352,
            },
            pzETH: {
              pzEth: 4_861.026354,
              usd: 13_838_229.48,
              price: 2846.7711290018815,
              underlyingTotalSupply: 4_861.789770,
              lpTotalSupply: 4_861.026354,
            },
          },
          apr: {
            success: true,
            data: { rate: 1.5895805126210139, avgPeriodDays: 30 },
          },
        },
      };
    }
    const res = await fetch(`${RENZO_API_BASE}/stats`);
    if (!res.ok) {
      throw new Error(`[renzoApi.getStats] HTTP ${res.status} ${res.statusText}`);
    }
    return (await res.json()) as { success: boolean; data: Record<string, unknown> };
  },
};
