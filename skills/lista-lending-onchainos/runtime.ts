// runtime.ts — REAL runtime backing for lista-lending-onchainos tools.
//
// v0.1.4 B-real:
//   - listaSdk → real @lista-dao/moolah-lending-sdk (MoolahSDK class)
//   - erc20    → real viem readContract against BSC / Ethereum public RPC
//   - listaCli → delegates to the same MoolahSDK methods for list / holdings /
//                simulate operations (the source CLI is just a thin SDK wrapper)
//   - parseUnits / formatUnits → hand-rolled (unchanged)
//
// Mock toggle:
//   Set LISTA_RUNTIME=mock to re-export from runtime-mock.ts instead.
//   Useful for offline testing without RPC traffic.

import { MoolahSDK } from '@lista-dao/moolah-lending-sdk';
import { createPublicClient, http, type PublicClient } from 'viem';
import { bsc, mainnet } from 'viem/chains';

// ── Mock toggle: re-export everything from runtime-mock.ts when requested ────

const USE_MOCK = process.env.LISTA_RUNTIME === 'mock';

if (USE_MOCK) {
  // Re-exporting via dynamic re-export isn't ergonomic in ESM, so the toggle
  // is documented but defers to a Node import-side switch — see cli.ts where
  // we honor LISTA_RUNTIME at startup. For now, this file is the real impl;
  // runtime-mock.ts is the mock-only file.
  console.warn('[runtime] LISTA_RUNTIME=mock set but real runtime is loaded. Use runtime-mock.ts directly (e.g. edit index.ts import) for mock mode.');
}

// ── Types (re-exported so index.ts can import) ───────────────────────────────

export type ToolError = {
  status: 'error';
  error_code: string;
  message: string;
  suggestion?: string;
  details?: unknown;
};

export type StepResult = Array<{
  step:
    | 'approve' | 'supply' | 'borrow' | 'repay'
    | 'deposit' | 'withdraw' | 'redeem' | 'withdrawCollateral'
    | string;
  params: { to: string; data: string; value?: bigint };
}>;

// ── Numeric helpers (hand-rolled, same as mock) ──────────────────────────────

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

// ── RPC configuration — mirrors upstream lista-lending/src/config.ts ─────────

const RPC_URLS: Record<number, string[]> = {
  56: [
    'https://bsc-dataseed.binance.org',
    'https://bsc-dataseed1.bnbchain.org',
    'https://bsc-rpc.publicnode.com',
  ],
  1: [
    'https://eth.drpc.org',
    'https://ethereum-rpc.publicnode.com',
    'https://cloudflare-eth.com',
  ],
};

// ── MoolahSDK instance ───────────────────────────────────────────────────────

const _sdk = new MoolahSDK({
  rpcUrls: {
    '56': RPC_URLS[56],
    '1':  RPC_URLS[1],
  },
});

// ── viem public clients (for ERC-20 reads) ───────────────────────────────────

const _publicClients = new Map<number, PublicClient>();
function getPublicClient(chainId: number): PublicClient {
  if (!_publicClients.has(chainId)) {
    const chain = chainId === 1 ? mainnet : bsc;
    const transport = http(RPC_URLS[chainId]?.[0]);
    _publicClients.set(chainId, createPublicClient({ chain, transport }));
  }
  return _publicClients.get(chainId)!;
}

// Minimal ERC-20 allowance ABI fragment.
const ERC20_ALLOWANCE_ABI = [
  {
    name: 'allowance',
    type: 'function' as const,
    stateMutability: 'view' as const,
    inputs: [
      { name: 'owner',   type: 'address' as const },
      { name: 'spender', type: 'address' as const },
    ],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
];

// ERC-4626 read-only ABI fragments — used for direct vault state queries
// that bypass the Lista API indexer (which can lag 30-60s after a deposit).
const ERC4626_READ_ABI = [
  {
    name: 'balanceOf',
    type: 'function' as const,
    stateMutability: 'view' as const,
    inputs: [{ name: 'account', type: 'address' as const }],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
  {
    name: 'convertToAssets',
    type: 'function' as const,
    stateMutability: 'view' as const,
    inputs: [{ name: 'shares', type: 'uint256' as const }],
    outputs: [{ name: '', type: 'uint256' as const }],
  },
  {
    name: 'symbol',
    type: 'function' as const,
    stateMutability: 'view' as const,
    inputs: [],
    outputs: [{ name: '', type: 'string' as const }],
  },
];

// ── listaSdk — thin wrappers around the real MoolahSDK instance ──────────────

export const listaSdk = {
  // Reads
  async getWriteConfig(chainId: number, marketId: string) {
    const cfg = await _sdk.getWriteConfig(chainId, lc(marketId));
    // SDK's getWriteConfig doesn't include the moolah market contract address
    // at the top level — derive it from sdk.getContracts(). index.ts uses
    // marketInfo.marketContract as the spender for ERC-20 approve.
    const moolah = _sdk.getContracts(chainId).moolah;
    return Object.assign({}, cfg, { marketContract: moolah });
  },

  async getMarketUserData(chainId: number, marketId: string, walletAddress: string) {
    const ud = await _sdk.getMarketUserData(chainId, lc(marketId), lc(walletAddress));
    // Normalize Decimal-shaped fields to bigint so index.ts's bigint math works.
    return Object.assign({}, ud, {
      loanable:     toBigInt((ud as any).loanable),
      borrowed:     toBigInt((ud as any).borrowed),
      collateral:   toBigInt((ud as any).collateral),
      withdrawable: toBigInt((ud as any).withdrawable),
    });
  },

  async getVaultInfo(chainId: number, vaultAddress: string) {
    return _sdk.getVaultInfo(chainId, lc(vaultAddress));
  },

  async getVaultUserData(
    chainId: number, vaultAddress: string, walletAddress: string, vaultInfo: unknown,
  ) {
    // upstream's getVaultUserData ignores the precomputed vaultInfo and re-reads;
    // we follow the SDK's 3-arg signature.
    const ud = await _sdk.getVaultUserData(chainId, lc(vaultAddress), lc(walletAddress));
    // Normalize Decimal-shaped fields to bigint.
    return Object.assign({}, ud, {
      shares: toBigInt((ud as any).shares),
      assets: toBigInt((ud as any).assets),
    });
  },

  // Market write builders
  async buildSupplyParams(args: {
    chainId: number; marketId: string; walletAddress: string; assets: bigint;
  }) {
    return _sdk.buildSupplyParams({
      chainId: args.chainId,
      marketId: lc(args.marketId),
      walletAddress: lc(args.walletAddress),
      assets: args.assets,
    } as any);
  },

  async buildBorrowParams(args: {
    chainId: number; marketId: string; assets: bigint; walletAddress: string; marketInfo: unknown;
  }) {
    return _sdk.buildBorrowParams({
      chainId: args.chainId,
      marketId: lc(args.marketId),
      assets: args.assets,
      walletAddress: lc(args.walletAddress),
      marketInfo: args.marketInfo,
    } as any);
  },

  async buildRepayParams(args: {
    chainId: number; marketId: string; assets?: bigint; repayAll: boolean;
    walletAddress: string; marketInfo: unknown; userData: unknown;
  }) {
    return _sdk.buildRepayParams({
      chainId: args.chainId,
      marketId: lc(args.marketId),
      assets: args.assets,
      repayAll: args.repayAll,
      walletAddress: lc(args.walletAddress),
      marketInfo: args.marketInfo,
      userData: args.userData,
    } as any);
  },

  async buildMarketWithdrawParams(args: {
    chainId: number; marketId: string; assets: bigint;
    walletAddress: string; marketInfo: unknown; userData: unknown;
  }) {
    // Upstream uses buildWithdrawParams for market-side withdraw (collateral).
    return _sdk.buildWithdrawParams({
      chainId: args.chainId,
      marketId: lc(args.marketId),
      assets: args.assets,
      walletAddress: lc(args.walletAddress),
      marketInfo: args.marketInfo,
      userData: args.userData,
    } as any);
  },

  // Vault write builders
  async buildDepositParams(args: {
    chainId: number; vaultAddress: string; walletAddress: string; assets: bigint;
  }) {
    return _sdk.buildVaultDepositParams({
      chainId: args.chainId,
      vaultAddress: lc(args.vaultAddress),
      walletAddress: lc(args.walletAddress),
      assets: args.assets,
    } as any);
  },

  async buildWithdrawParams(args: {
    chainId: number; vaultAddress: string; walletAddress: string;
    assets?: bigint; shares?: bigint;
  }) {
    return _sdk.buildVaultWithdrawParams({
      chainId: args.chainId,
      vaultAddress: lc(args.vaultAddress),
      walletAddress: lc(args.walletAddress),
      assets: args.assets,
      shares: args.shares,
    } as any);
  },
};

// ── erc20 — real viem readContract for allowance ─────────────────────────────
//
// Signature: erc20.allowance(token, owner, spender, chainId?) — chainId
// defaults to 56 (BSC) for backwards compatibility with v0.1.3 mock signature.

export const erc20 = {
  async allowance(
    token: string, owner: string, spender: string, chainId: number = 56,
  ): Promise<bigint> {
    const client = getPublicClient(chainId);
    const result = await client.readContract({
      address: lc(token),
      abi: ERC20_ALLOWANCE_ABI,
      functionName: 'allowance',
      args: [lc(owner), lc(spender)],
    });
    return result as bigint;
  },
};

// ── erc4626 — direct vault reads for fresh on-chain state ────────────────────
// Bypasses Lista API indexer lag. Use for position verification after a deposit
// confirms but before the indexer catches up.
export const erc4626 = {
  async balanceOf(vault: string, owner: string, chainId: number = 56): Promise<bigint> {
    const client = getPublicClient(chainId);
    return (await client.readContract({
      address: lc(vault),
      abi: ERC4626_READ_ABI,
      functionName: 'balanceOf',
      args: [lc(owner)],
    })) as bigint;
  },
  async convertToAssets(vault: string, shares: bigint, chainId: number = 56): Promise<bigint> {
    const client = getPublicClient(chainId);
    return (await client.readContract({
      address: lc(vault),
      abi: ERC4626_READ_ABI,
      functionName: 'convertToAssets',
      args: [shares],
    })) as bigint;
  },
  async symbol(vault: string, chainId: number = 56): Promise<string> {
    const client = getPublicClient(chainId);
    return (await client.readContract({
      address: lc(vault),
      abi: ERC4626_READ_ABI,
      functionName: 'symbol',
    })) as string;
  },
};

// ── listaCli — delegates to MoolahSDK for list / holdings / simulate ────────

function caip2ToChainId(caip2: string | unknown): number {
  if (typeof caip2 === 'string') {
    if (caip2.startsWith('eip155:')) return parseInt(caip2.split(':')[1], 10);
    if (caip2 === 'bsc') return 56;
    if (caip2 === 'ethereum') return 1;
  }
  return 56;
}

// viem strictly enforces EIP-55 checksums for mixed-case addresses. We
// normalize to lowercase before forwarding to SDK / viem to accept any
// reasonable user input (lowercase, mixed-case, or proper EIP-55).
function lc(addr: string): `0x${string}` {
  return addr.toLowerCase() as `0x${string}`;
}

// SDK returns Decimal objects ({numerator, denominator, decimal}) for numeric
// position fields. index.ts treats them as bigint. Normalize at the boundary
// by extracting .numerator so downstream code can do plain bigint math.
function toBigInt(v: unknown): bigint {
  if (typeof v === 'bigint') return v;
  if (v === null || v === undefined) return 0n;
  if (typeof v === 'object' && v !== null && 'numerator' in (v as object)) {
    return BigInt((v as { numerator: string | bigint }).numerator);
  }
  if (typeof v === 'string' || typeof v === 'number') return BigInt(v);
  return 0n;
}

export const listaCli = {
  async run(command: string, options: Record<string, unknown>) {
    const chainId = caip2ToChainId(options.chain);

    if (command === 'markets') {
      const list = await _sdk.getMarketList({
        chainId,
        page: (options.page as number) ?? 1,
        pageSize: (options.pageSize as number) ?? 20,
        sort: options.sort as any,
        order: options.order as any,
        keyword: options.keyword as string | undefined,
        // collateralSymbols / loanSymbols filters omitted — SDK API may differ
      } as any);
      // Apply SmartLending (zone=3) + fixed-term (termType=1) filter — mirrors
      // upstream behavior in src/commands/markets.ts. SDK returns { total, list }.
      const items = (list as any)?.list ?? (list as any)?.items ?? [];
      const filtered = Array.isArray(items)
        ? items.filter((m: any) => m.zone !== 3 && m.termType !== 1)
        : items;
      return { total: filtered.length, list: filtered };
    }

    if (command === 'vaults') {
      return _sdk.getVaultList({
        chainId,
        page: (options.page as number) ?? 1,
        pageSize: (options.pageSize as number) ?? 20,
        sort: options.sort as any,
        order: options.order as any,
        keyword: options.keyword as string | undefined,
      } as any);
    }

    if (command === 'holdings') {
      // SDK getHoldings takes { userAddress, type: 'vault' | 'market' }.
      // Our scope param maps as: 'vault' / 'market' → direct, 'all' / 'selected' → fetch both and merge.
      const userAddress = lc(options.address as string);
      const scope = (options.scope as string) ?? 'all';
      if (scope === 'vault') {
        return { vaultPositions: await _sdk.getHoldings({ userAddress, type: 'vault' } as any) };
      }
      if (scope === 'market') {
        return { marketPositions: await _sdk.getHoldings({ userAddress, type: 'market' } as any) };
      }
      const [vaultPositions, marketPositions] = await Promise.all([
        _sdk.getHoldings({ userAddress, type: 'vault' } as any).catch(() => []),
        _sdk.getHoldings({ userAddress, type: 'market' } as any).catch(() => []),
      ]);
      return { vaultPositions, marketPositions, wallet: userAddress };
    }

    if (command === 'borrow' && options.simulate) {
      const marketInfo = await _sdk.getWriteConfig(chainId, lc(options.market as string));
      const userData = await _sdk.getMarketUserData(
        chainId, lc(options.market as string), lc(options.walletAddress as string),
      );
      return _sdk.simulateBorrowPosition({
        chainId,
        marketId: lc(options.market as string),
        walletAddress: lc(options.walletAddress as string),
        marketInfo,
        userData,
        ...(options.simulateSupply ? {
          supplyAssets: parseUnits(String(options.simulateSupply), (marketInfo as any).collateralInfo.decimals),
        } : {}),
      } as any);
    }

    if (command === 'repay' && options.simulate) {
      const marketInfo = await _sdk.getWriteConfig(chainId, lc(options.market as string));
      const userData = await _sdk.getMarketUserData(
        chainId, lc(options.market as string), lc(options.walletAddress as string),
      );
      let repayAssets: bigint = 0n;
      if (options.repayAll) {
        repayAssets = (userData as any).borrowed?.numerator ?? (userData as any).borrowed;
      } else if (options.amount) {
        repayAssets = parseUnits(String(options.amount), (marketInfo as any).loanInfo.decimals);
      }
      return _sdk.simulateRepayPosition({
        chainId,
        marketId: lc(options.market as string),
        walletAddress: lc(options.walletAddress as string),
        repayAssets,
        repayAll: Boolean(options.repayAll),
        marketInfo,
        userData,
      } as any);
    }

    throw new Error(`[listaCli] unsupported command: ${command}`);
  },
};
