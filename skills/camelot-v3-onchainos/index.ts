// camelot-v3-onchainos — index.ts (Form C v0.1.0)
//
// Form C (primitive source — viem-direct calldata against verified Algebra ABIs;
// no upstream SDK or skill, the published camelot-sdk v0.0.8 is stubs only).
// 4 tools wrapping Camelot V3 (Algebra) exact-input single-pool swaps on Arbitrum.
// buildSwap returns pending_sign | ToolError.
// Read-only tools return {status:'ok', data} | ToolError.

import { encodeFunctionData, type Address } from 'viem';
import {
  CAMELOT_V3,
  CHAINS,
  ERC20_ABI,
  QUOTER_ABI,
  SWAP_ROUTER_ABI,
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

// P1-15: explicit decimals validator
function validateDecimals(d: number | undefined, fieldName: string): ToolError | null {
  if (d === undefined || d === null) {
    return { status: 'error', error_code: 'MISSING_PARAM',
             message: `\`${fieldName}\` is required (e.g. 6 for USDC, 18 for WETH).` };
  }
  if (typeof d !== 'number' || !Number.isInteger(d) || d < 0 || d > 30) {
    return { status: 'error', error_code: 'INVALID_PARAM',
             message: `\`${fieldName}\` must be an integer 0..30 (got ${JSON.stringify(d)}).` };
  }
  return null;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   READ-ONLY TOOLS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * List the chains this skill supports for Camelot V3 swaps.
 */
export async function listSupportedChains(_params: Record<string, unknown> = {}) {
  return {
    status: 'ok' as const,
    data: Object.entries(CHAINS).map(([name, info]) => ({
      name,
      chainId: info.id,
      swapRouter: CAMELOT_V3[name as ChainName]?.swapRouter,
      quoter: CAMELOT_V3[name as ChainName]?.quoter,
    })),
    note: 'Camelot V3 is currently live on Arbitrum. Other deployments are roadmap.',
  };
}

/**
 * Read symbol/decimals/balanceOf for any ERC-20 token. Helper for users who
 * don't know the decimals to pass into getQuote/buildSwap.
 */
export async function getTokenInfo(params: {
  chain: string;
  tokenAddress: string;
  walletAddress?: string;
}) {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.tokenAddress, 'tokenAddress');
  if (validationError) return validationError;
  if (params.walletAddress) {
    const e = validateEvmAddress(params.walletAddress, 'walletAddress');
    if (e) return e;
  }

  const chain = params.chain as ChainName;
  const client = getPublicClient(chain);
  const token = lc(params.tokenAddress) as Address;

  try {
    const [symbol, decimals] = await Promise.all([
      client.readContract({ address: token, abi: ERC20_ABI, functionName: 'symbol' }) as Promise<string>,
      client.readContract({ address: token, abi: ERC20_ABI, functionName: 'decimals' }) as Promise<number>,
    ]);
    let balance = 0n;
    let balanceUI = '0';
    if (params.walletAddress) {
      balance = (await client.readContract({
        address: token,
        abi: ERC20_ABI,
        functionName: 'balanceOf',
        args: [lc(params.walletAddress)],
      })) as bigint;
      balanceUI = formatUnits(balance, decimals);
    }
    return {
      status: 'ok' as const,
      data: {
        chain,
        tokenAddress: token,
        symbol,
        decimals,
        balanceRaw: balance.toString(),
        balance: balanceUI,
      },
    };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'RPC_ERROR',
      message: `Failed to read ERC-20 ${token} on ${chain}: ${(e as Error).message}`,
    };
  }
}

/**
 * Quote an exact-input single-pool swap on Camelot V3.
 * Calls the Algebra Quoter (state-mutating, revert-encoded result — viem's
 * simulateContract handles this transparently via eth_call).
 */
export async function getQuote(params: {
  chain: string;
  tokenIn: string;
  tokenOut: string;
  amountIn: string;
  amountInDecimals: number;
}) {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.tokenIn, 'tokenIn') ??
    validateEvmAddress(params.tokenOut, 'tokenOut') ??
    validatePositiveAmount(params.amountIn, 'amountIn') ??
    validateDecimals(params.amountInDecimals, 'amountInDecimals');
  if (validationError) return validationError;

  const chain = params.chain as ChainName;
  const amountInWei = parseUnits(params.amountIn, params.amountInDecimals);

  const client = getPublicClient(chain);
  const quoter = CAMELOT_V3[chain].quoter;

  try {
    const { result } = await client.simulateContract({
      address: quoter,
      abi: QUOTER_ABI,
      functionName: 'quoteExactInputSingle',
      args: [lc(params.tokenIn), lc(params.tokenOut), amountInWei, 0n],
    });
    const [amountOut, fee] = result as readonly [bigint, number];
    return {
      status: 'ok' as const,
      data: {
        chain,
        tokenIn: lc(params.tokenIn),
        tokenOut: lc(params.tokenOut),
        amountInRaw: amountInWei.toString(),
        amountIn: params.amountIn,
        amountOutRaw: amountOut.toString(),
        dynamicFee: fee,
        dynamicFeePercent: (fee / 10000).toFixed(4) + '%',
        note: 'Algebra uses dynamic fees per pool. `dynamicFee` is in hundredths of basis points (millionths of unity, matching Uniswap V3 convention): 100 = 0.01%, 3000 = 0.30%. `dynamicFeePercent` is the same value pre-formatted.',
      },
    };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'QUOTE_FAILED',
      message: `Camelot V3 quote failed: ${(e as Error).message}`,
      suggestion: 'Verify both tokens have a Camelot V3 pool. Check the Algebra pool factory or try a different token pair.',
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
//   TRANSACTION TOOL
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Build an exact-input single-pool swap on Camelot V3 (Algebra).
 *
 * Multi-step stateful single-step emission:
 *  1. If allowance(walletAddress → SwapRouter) < amountIn, emits ERC-20 approve
 *     pending_sign (Step 1 of 2).
 *  2. Otherwise emits exactInputSingle pending_sign.
 *
 * Defaults:
 *  - recipient = walletAddress
 *  - deadline = now + 20 minutes
 *  - amountOutMinimum = quote × (1 - slippageBps/10000)
 *  - limitSqrtPrice = 0 (no price limit; relies on amountOutMinimum for safety)
 */
export async function buildSwap(params: {
  chain: string;
  tokenIn: string;
  tokenOut: string;
  amountIn: string;
  amountInDecimals: number;
  walletAddress: string;
  recipient?: string;
  slippageBps?: number;     // default 50 (0.50%)
  deadlineSeconds?: number; // default 1200 (20 min from now)
}): Promise<PendingSign | ToolError> {
  const validationError =
    validateChainName(params.chain) ??
    validateEvmAddress(params.tokenIn, 'tokenIn') ??
    validateEvmAddress(params.tokenOut, 'tokenOut') ??
    validateEvmAddress(params.walletAddress, 'walletAddress') ??
    validatePositiveAmount(params.amountIn, 'amountIn') ??
    validateDecimals(params.amountInDecimals, 'amountInDecimals');
  if (validationError) return validationError;
  if (params.recipient) {
    const e = validateEvmAddress(params.recipient, 'recipient');
    if (e) return e;
  }
  if (params.slippageBps !== undefined) {
    if (typeof params.slippageBps !== 'number' || params.slippageBps < 0 || params.slippageBps > 10000) {
      return { status: 'error', error_code: 'INVALID_PARAM',
               message: `\`slippageBps\` must be 0..10000 (got ${params.slippageBps}).` };
    }
  }
  const deadlineSeconds = params.deadlineSeconds ?? 1200;
  if (deadlineSeconds <= 0) {
    return { status: 'error', error_code: 'INVALID_DEADLINE',
             message: `\`deadlineSeconds\` must be positive (got ${deadlineSeconds}).` };
  }

  const chain = params.chain as ChainName;
  const router = CAMELOT_V3[chain].swapRouter;
  const amountInWei = parseUnits(params.amountIn, params.amountInDecimals);
  const slippageBps = params.slippageBps ?? 50;

  const client = getPublicClient(chain);
  const tokenIn = lc(params.tokenIn) as Address;
  const tokenOut = lc(params.tokenOut) as Address;
  const owner = lc(params.walletAddress) as Address;
  const recipient = lc(params.recipient ?? params.walletAddress) as Address;

  // Early quote — fail-fast on bad pairs BEFORE emitting an approve that would
  // waste the user's gas. Cached for reuse in the swap step below.
  let amountOut: bigint;
  try {
    const sim = await client.simulateContract({
      address: CAMELOT_V3[chain].quoter,
      abi: QUOTER_ABI,
      functionName: 'quoteExactInputSingle',
      args: [tokenIn, tokenOut, amountInWei, 0n],
    });
    amountOut = (sim.result as readonly [bigint, number])[0];
  } catch (e) {
    return {
      status: 'error',
      error_code: 'QUOTE_FAILED',
      message: `Camelot V3 quote failed for ${tokenIn} → ${tokenOut} on ${chain}: ${(e as Error).message}`,
      suggestion: 'Verify both tokens have a Camelot V3 pool (e.g. via getQuote first). No on-chain transaction was emitted.',
    };
  }
  if (amountOut === 0n) {
    return {
      status: 'error',
      error_code: 'QUOTE_FAILED',
      message: 'Quote returned 0 amountOut — pool may be empty or pair unsupported.',
      suggestion: 'Try a different pair or chain. No on-chain transaction was emitted.',
    };
  }

  // Allowance check (only AFTER quote sanity-checks the pair)
  let allowance: bigint;
  try {
    allowance = (await client.readContract({
      address: tokenIn,
      abi: ERC20_ABI,
      functionName: 'allowance',
      args: [owner, router],
    })) as bigint;
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Failed to read allowance for ${tokenIn} → SwapRouter on ${chain}: ${(e as Error).message}`,
    };
  }

  if (allowance < amountInWei) {
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
      args: [router, amountInWei],
    });
    return {
      status: 'pending_sign',
      unsigned_tx: { to: lc(tokenIn), data: approveData, value: '0', chain },
      description:
        `Step 1 of 2: Approve ${params.amountIn} tokenIn (${tokenIn}) for the Camelot V3 SwapRouter (${router}) on ${chain}. ` +
        `After this transaction lands, re-invoke buildSwap with the same params to receive the swap transaction.`,
      next_action: { tool: 'onchainos wallet contract-call' },
      step_info: { current: 1, total: 2, name: 'approve' },
    };
  }

  // tokenIn balance pre-flight
  let inBalance: bigint;
  try {
    inBalance = (await client.readContract({
      address: tokenIn,
      abi: ERC20_ABI,
      functionName: 'balanceOf',
      args: [owner],
    })) as bigint;
  } catch (e) {
    return {
      status: 'error',
      error_code: 'RPC_ERROR',
      message: `Failed to read tokenIn balance: ${(e as Error).message}`,
    };
  }
  if (inBalance < amountInWei) {
    return {
      status: 'error',
      error_code: 'INSUFFICIENT_BALANCE',
      message: `Need ${params.amountIn} tokenIn but wallet only holds ${formatUnits(inBalance, params.amountInDecimals)} (decimals=${params.amountInDecimals}) on ${chain}.`,
      suggestion: 'Reduce amountIn or top up.',
    };
  }

  // Reuse the quote we fetched earlier (pre-approval gas-waste guard).
  const amountOutMinimum = (amountOut * BigInt(10000 - slippageBps)) / 10000n;

  const deadline = BigInt(Math.floor(Date.now() / 1000) + deadlineSeconds);

  const swapData = encodeFunctionData({
    abi: SWAP_ROUTER_ABI,
    functionName: 'exactInputSingle',
    args: [{
      tokenIn,
      tokenOut,
      recipient,
      deadline,
      amountIn: amountInWei,
      amountOutMinimum,
      limitSqrtPrice: 0n,
    }],
  });

  // Native-balance gate (value=0 for ERC-20 swap; helper is wired uniformly per P1-14)
  const balanceErr = await nativeBalanceCheck(client, chain, params.walletAddress, 0n);
  if (balanceErr) return balanceErr;

  return {
    status: 'pending_sign',
    unsigned_tx: { to: lc(router), data: swapData, value: '0', chain },
    description:
      `Swap ${params.amountIn} of ${tokenIn} → ${tokenOut} on Camelot V3 (${chain}). ` +
      `Quoted amountOut: ${amountOut.toString()} (raw); amountOutMinimum: ${amountOutMinimum.toString()} (slippage ${slippageBps} bps). ` +
      `Deadline: ${deadlineSeconds}s from now.`,
    next_action: { tool: 'onchainos wallet contract-call' },
  };
}
