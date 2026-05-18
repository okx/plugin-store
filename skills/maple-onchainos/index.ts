// maple-onchainos — index.ts (Form A v0.1.0 · maple-js SDK with calldata builders)
//
// 5 tools wrapping Maple/Syrup ERC-4626 pool operations.
// Transaction tools (buildDeposit, buildQueueWithdrawal) return pending_sign | ToolError.
// Read-only tools return {status:'ok', data} | ToolError.

import { encodeFunctionData, type Address } from 'viem';
import {
  CHAINS,
  ERC20_ABI,
  ERC4626_ABI,
  POOL_ABI,
  POOLS,
  findPool,
  formatUnits,
  getPublicClient,
  isSupportedChain,
  lc,
  nativeBalanceCheck,
  parseUnits,
  type ChainName,
  type PendingSign,
  type ToolError,
} from './runtime.ts';

// ── Validators ────────────────────────────────────────────────────────────────

function validatePositiveAmount(amount: string | undefined, fieldName = 'amount'): ToolError | null {
  if (amount === undefined || amount === null || amount === '') {
    return { status: 'error', error_code: 'MISSING_PARAM', message: `\`${fieldName}\` is required.` };
  }
  if (!/^\d+(\.\d+)?$/.test(amount)) {
    return { status: 'error', error_code: 'INVALID_PARAM',
             message: `\`${fieldName}\` must be a non-negative decimal string (got '${amount}').` };
  }
  if (Number(amount) === 0) {
    return { status: 'error', error_code: 'INVALID_PARAM',
             message: `\`${fieldName}\` must be greater than zero.` };
  }
  return null;
}

function validateEvmAddress(addr: string | undefined, fieldName: string): ToolError | null {
  if (!addr) {
    return { status: 'error', error_code: 'MISSING_PARAM', message: `\`${fieldName}\` is required.` };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(addr)) {
    return {
      status: 'error',
      error_code: 'INVALID_PARAM',
      message: `\`${fieldName}\` must be a 0x-prefixed 40-char hex EVM address (got '${addr}').`,
    };
  }
  return null;
}

function validateChainName(name: string | undefined, fieldName = 'chain'): ToolError | null {
  if (!name) {
    return { status: 'error', error_code: 'MISSING_PARAM', message: `\`${fieldName}\` is required.` };
  }
  if (!isSupportedChain(name)) {
    return {
      status: 'error',
      error_code: 'UNSUPPORTED_CHAIN',
      message: `\`${fieldName}\` '${name}' is not supported.`,
      suggestion: `Use one of: ${Object.keys(CHAINS).join(', ')}.`,
    };
  }
  return null;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   READ-ONLY TOOLS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * List the curated set of Maple/Syrup pools supported by this skill.
 */
export async function listSupportedPools(params: { chain?: string } = {}) {
  if (params.chain) {
    const err = validateChainName(params.chain, 'chain');
    if (err) return err;
  }
  const pools = params.chain
    ? POOLS.filter((p) => p.chain === params.chain)
    : POOLS;
  return {
    status: 'ok' as const,
    data: pools,
    note: 'Each pool is an ERC-4626 vault. Deposit underlying asset to receive shares (e.g. USDC → syrupUSDC).',
  };
}

/**
 * Read live pool state via viem: symbol, underlying asset, totalAssets, totalSupply,
 * and a 1-unit `previewDeposit` to surface the current exchange rate.
 */
export async function getPool(params: { chain: string; poolAddress: string }) {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.poolAddress, 'poolAddress');
  if (validationError) return validationError;

  const chain = params.chain as ChainName;
  const known = findPool(chain, params.poolAddress);
  if (!known) {
    return {
      status: 'error' as const,
      error_code: 'UNKNOWN_POOL',
      message: `Pool ${params.poolAddress} is not in the curated registry for ${chain}.`,
      suggestion: `Use listSupportedPools to see the known pools, or open a PR to add this one.`,
    };
  }

  const client = getPublicClient(chain);
  const pool = lc(params.poolAddress) as Address;

  try {
    const oneAsset = parseUnits('1', known.underlyingDecimals);
    const [symbol, asset, totalAssets, totalSupply, previewShares] = await Promise.all([
      client.readContract({ address: pool, abi: ERC20_ABI, functionName: 'symbol' }) as Promise<string>,
      client.readContract({ address: pool, abi: ERC4626_ABI, functionName: 'asset' }) as Promise<Address>,
      client.readContract({ address: pool, abi: ERC4626_ABI, functionName: 'totalAssets' }) as Promise<bigint>,
      client.readContract({ address: pool, abi: ERC4626_ABI, functionName: 'totalSupply' }) as Promise<bigint>,
      client.readContract({ address: pool, abi: ERC4626_ABI, functionName: 'previewDeposit', args: [oneAsset] }) as Promise<bigint>,
    ]);

    return {
      status: 'ok' as const,
      data: {
        chain,
        poolAddress: pool,
        symbol,
        asset,
        underlyingSymbol: known.underlyingSymbol,
        underlyingDecimals: known.underlyingDecimals,
        totalAssetsRaw: totalAssets.toString(),
        totalAssets: formatUnits(totalAssets, known.underlyingDecimals),
        totalSupplyRaw: totalSupply.toString(),
        previewDepositOneAsset: {
          assets: '1 ' + known.underlyingSymbol,
          shares: previewShares.toString(),
          note: 'Number of pool shares minted for a single unit of the underlying asset (in share-token wei).',
        },
      },
    };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'RPC_ERROR',
      message: `Failed to read pool state for ${pool} on ${chain}: ${(e as Error).message}`,
    };
  }
}

/**
 * Read a user's position in a specific pool: share balance and assets-equivalent.
 */
export async function getPosition(params: {
  chain: string;
  poolAddress: string;
  walletAddress: string;
}) {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.poolAddress, 'poolAddress') ??
    validateEvmAddress(params.walletAddress, 'walletAddress');
  if (validationError) return validationError;

  const chain = params.chain as ChainName;
  const known = findPool(chain, params.poolAddress);
  if (!known) {
    return {
      status: 'error' as const,
      error_code: 'UNKNOWN_POOL',
      message: `Pool ${params.poolAddress} is not in the curated registry for ${chain}.`,
    };
  }

  const client = getPublicClient(chain);
  const pool = lc(params.poolAddress) as Address;
  const owner = lc(params.walletAddress) as Address;

  try {
    const shares = (await client.readContract({
      address: pool,
      abi: ERC20_ABI,
      functionName: 'balanceOf',
      args: [owner],
    })) as bigint;

    let assets = 0n;
    if (shares > 0n) {
      assets = (await client.readContract({
        address: pool,
        abi: ERC4626_ABI,
        functionName: 'convertToAssets',
        args: [shares],
      })) as bigint;
    }

    return {
      status: 'ok' as const,
      data: {
        chain,
        poolAddress: pool,
        walletAddress: owner,
        sharesRaw: shares.toString(),
        symbol: known.symbol,
        underlyingSymbol: known.underlyingSymbol,
        assetsRaw: assets.toString(),
        assets: formatUnits(assets, known.underlyingDecimals),
        note: shares === 0n
          ? `No position. Use buildDeposit to lend ${known.underlyingSymbol} into this pool.`
          : `${formatUnits(shares, known.underlyingDecimals)} ${known.symbol} ≈ ${formatUnits(assets, known.underlyingDecimals)} ${known.underlyingSymbol}.`,
      },
    };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'RPC_ERROR',
      message: `Failed to read position for ${owner} in pool ${pool} on ${chain}: ${(e as Error).message}`,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
//   TRANSACTION TOOLS — return pending_sign | ToolError
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Deposit underlying asset into a Maple/Syrup pool (ERC-4626 `deposit`).
 *
 * Multi-step stateful single-step emission:
 *  1. If allowance(walletAddress → poolAddress) < amount, emits the ERC-20
 *     approve `pending_sign` (Step 1 of 2).
 *  2. Otherwise emits the `deposit(amount, walletAddress)` `pending_sign`.
 */
export async function buildDeposit(params: {
  chain: string;
  poolAddress: string;
  amount: string;          // UI units of underlying (e.g. "100" for 100 USDC)
  walletAddress: string;
  recipient?: string;      // defaults to walletAddress
}): Promise<PendingSign | ToolError> {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.poolAddress, 'poolAddress') ??
    validateEvmAddress(params.walletAddress, 'walletAddress') ??
    validatePositiveAmount(params.amount, 'amount');
  if (validationError) return validationError;
  if (params.recipient) {
    const rErr = validateEvmAddress(params.recipient, 'recipient');
    if (rErr) return rErr;
  }

  const chain = params.chain as ChainName;
  const known = findPool(chain, params.poolAddress);
  if (!known) {
    return {
      status: 'error',
      error_code: 'UNKNOWN_POOL',
      message: `Pool ${params.poolAddress} is not in the curated registry for ${chain}.`,
      suggestion: 'Use listSupportedPools.',
    };
  }

  const amountWei = parseUnits(params.amount, known.underlyingDecimals);

  const client = getPublicClient(chain);
  const pool = lc(params.poolAddress) as Address;
  const owner = lc(params.walletAddress) as Address;
  const receiver = lc(params.recipient ?? params.walletAddress) as Address;

  // Read the underlying asset address from the pool (so we don't trust client input alone)
  let underlying: Address;
  try {
    underlying = (await client.readContract({
      address: pool,
      abi: ERC4626_ABI,
      functionName: 'asset',
    })) as Address;
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Failed to read pool.asset() on ${chain}: ${(e as Error).message}`,
    };
  }

  // Allowance check
  let allowance: bigint;
  try {
    allowance = (await client.readContract({
      address: underlying,
      abi: ERC20_ABI,
      functionName: 'allowance',
      args: [owner, pool],
    })) as bigint;
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Failed to read allowance on ${chain}: ${(e as Error).message}`,
    };
  }

  if (allowance < amountWei) {
    const approveData = encodeFunctionData({
      abi: [
        {
          type: 'function',
          name: 'approve',
          stateMutability: 'nonpayable',
          inputs: [
            { name: 'spender', type: 'address' },
            { name: 'amount', type: 'uint256' },
          ],
          outputs: [{ name: '', type: 'bool' }],
        },
      ],
      functionName: 'approve',
      args: [pool, amountWei],
    });

    return {
      status: 'pending_sign',
      unsigned_tx: {
        to: lc(underlying),
        data: approveData,
        value: '0',
        chain,
      },
      description:
        `Step 1 of 2: Approve ${params.amount} ${known.underlyingSymbol} for the Maple/Syrup pool (${known.symbol}) on ${chain}. ` +
        `After this transaction lands, re-invoke buildDeposit with the same params to receive the deposit transaction.`,
      next_action: { tool: 'onchainos wallet contract-call' },
      step_info: { current: 1, total: 2, name: 'approve' },
    };
  }

  // Balance pre-flight on the underlying (helps catch INSUFFICIENT_BALANCE before broadcast)
  let underlyingBalance: bigint;
  try {
    underlyingBalance = (await client.readContract({
      address: underlying,
      abi: ERC20_ABI,
      functionName: 'balanceOf',
      args: [owner],
    })) as bigint;
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Failed to read ${known.underlyingSymbol} balance on ${chain}: ${(e as Error).message}`,
    };
  }
  if (underlyingBalance < amountWei) {
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_BALANCE',
      message: `Need ${params.amount} ${known.underlyingSymbol} but wallet only has ${formatUnits(underlyingBalance, known.underlyingDecimals)} ${known.underlyingSymbol} on ${chain}.`,
      suggestion: 'Reduce amount or top up.',
    };
  }

  // Native-balance gate — P1-14. Deposit tx itself has value=0, but we still
  // need gas. We don't know the exact gas cost ahead of time, so this is a
  // soft sanity check: at least 0 wei (effectively a no-op for value=0 txs).
  const balanceErr = await nativeBalanceCheck(client, chain, params.walletAddress, 0n);
  if (balanceErr) return balanceErr;

  const depositData = encodeFunctionData({
    abi: POOL_ABI,
    functionName: 'deposit',
    args: [amountWei, receiver],
  });

  return {
    status: 'pending_sign',
    unsigned_tx: {
      to: lc(pool),
      data: depositData,
      value: '0',
      chain,
    },
    description:
      `Deposit ${params.amount} ${known.underlyingSymbol} into ${known.symbol} on ${chain}` +
      (receiver !== owner ? ` (shares to ${receiver})` : '') +
      '. ERC-4626 deposit; shares minted to the receiver.',
    next_action: { tool: 'onchainos wallet contract-call' },
  };
}

/**
 * Queue a withdrawal from a Maple/Syrup pool — calls `requestRedeem(shares, owner)`.
 * Withdrawals are processed by the withdrawal manager in cycles; this does NOT
 * settle immediately.
 */
export async function buildQueueWithdrawal(params: {
  chain: string;
  poolAddress: string;
  shares: string;          // UI units (e.g. "100" for 100 syrupUSDC shares)
  walletAddress: string;
}): Promise<PendingSign | ToolError> {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.poolAddress, 'poolAddress') ??
    validateEvmAddress(params.walletAddress, 'walletAddress') ??
    validatePositiveAmount(params.shares, 'shares');
  if (validationError) return validationError;

  const chain = params.chain as ChainName;
  const known = findPool(chain, params.poolAddress);
  if (!known) {
    return {
      status: 'error',
      error_code: 'UNKNOWN_POOL',
      message: `Pool ${params.poolAddress} is not in the curated registry for ${chain}.`,
    };
  }

  // Share token has the SAME decimals as the underlying for Maple pools
  // (ERC-4626 default — confirmed via decimals() == underlying.decimals()).
  const sharesWei = parseUnits(params.shares, known.underlyingDecimals);
  const client = getPublicClient(chain);
  const pool = lc(params.poolAddress) as Address;
  const owner = lc(params.walletAddress) as Address;

  // Check the user actually holds enough shares
  let shareBalance: bigint;
  try {
    shareBalance = (await client.readContract({
      address: pool,
      abi: ERC20_ABI,
      functionName: 'balanceOf',
      args: [owner],
    })) as bigint;
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Failed to read ${known.symbol} balance on ${chain}: ${(e as Error).message}`,
    };
  }
  if (shareBalance === 0n) {
    return {
      status: 'error',
      error_code: 'NO_POSITION',
      message: `Wallet ${owner} has no ${known.symbol} shares to redeem.`,
      suggestion: 'Use buildDeposit first.',
    };
  }
  if (shareBalance < sharesWei) {
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_BALANCE',
      message: `Requested ${params.shares} ${known.symbol} but wallet only holds ${formatUnits(shareBalance, known.underlyingDecimals)} ${known.symbol}.`,
    };
  }

  const requestRedeemData = encodeFunctionData({
    abi: POOL_ABI,
    functionName: 'requestRedeem',
    args: [sharesWei, owner],
  });

  return {
    status: 'pending_sign',
    unsigned_tx: {
      to: lc(pool),
      data: requestRedeemData,
      value: '0',
      chain,
    },
    description:
      `Queue redemption of ${params.shares} ${known.symbol} on ${chain}. ` +
      `Withdrawals are processed by Maple's withdrawal manager in cycles — this is NOT instant. ` +
      `Underlying ${known.underlyingSymbol} becomes claimable after the next cycle settles.`,
    next_action: { tool: 'onchainos wallet contract-call' },
  };
}
