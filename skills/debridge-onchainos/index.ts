// debridge-onchainos — index.ts (Form B-REST v0.1.0)
//
// Wraps the deBridge DLN public REST API (dln.debridge.finance/v1.0/).
// Five-tool surface:
// Transaction-class tools (buildBridge, buildSameChainSwap) return
// pending_sign | ToolError.
// Read-only tools (listSupportedChains, getQuote, getOrderStatus) return
// {status:'ok', data} | ToolError.
//
// All signing routes through `onchainos wallet contract-call`.

import {
  createPublicClient,
  http,
  type Address,
  type Hex,
} from 'viem';
import {
  arbitrum, avalanche, base, bsc, linea, mainnet, optimism, polygon,
} from 'viem/chains';
import {
  debridgeApi,
  CHAINS,
  parseUnits,
  lc,
  nativeBalanceCheck,
  type ToolError,
  type PendingSign,
} from './runtime.ts';

// DLN's native-asset sentinel address.
const NATIVE_SENTINEL = '0x0000000000000000000000000000000000000000';

// viem chain registry — keyed by our internal chain name.
const VIEM_CHAINS = {
  ethereum: mainnet,
  arbitrum,
  base,
  optimism,
  polygon,
  bsc,
  avalanche,
  linea,
} as const;

type EvmChainName = keyof typeof VIEM_CHAINS;

function isEvmChain(name: string): name is EvmChainName {
  return name in VIEM_CHAINS;
}

function chainNameFromId(id: number): string {
  for (const [name, info] of Object.entries(CHAINS)) {
    if (info.id === id) return name;
  }
  return `eip155:${id}`;
}

function isNative(token: string): boolean {
  return token.toLowerCase() === NATIVE_SENTINEL;
}

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

function validateChainId(id: number | undefined, fieldName: string): ToolError | null {
  if (id === undefined || id === null) {
    return { status: 'error', error_code: 'MISSING_PARAM', message: `\`${fieldName}\` is required (numeric chain ID).` };
  }
  if (typeof id !== 'number' || !Number.isInteger(id) || id <= 0) {
    return { status: 'error', error_code: 'INVALID_PARAM',
             message: `\`${fieldName}\` must be a positive integer chain ID (got ${JSON.stringify(id)}).` };
  }
  return null;
}

function validateDecimals(d: number | undefined, fieldName: string): ToolError | null {
  if (d === undefined || d === null) {
    return { status: 'error', error_code: 'MISSING_PARAM',
             message: `\`${fieldName}\` is required (e.g. 6 for USDC on most chains, 18 for ETH/native and BSC USDT/USDC).` };
  }
  if (typeof d !== 'number' || !Number.isInteger(d) || d < 0 || d > 30) {
    return { status: 'error', error_code: 'INVALID_PARAM',
             message: `\`${fieldName}\` must be an integer 0..30 (got ${JSON.stringify(d)}).` };
  }
  return null;
}

// ── Allowance helper (used by buildBridge + buildSameChainSwap) ───────────────

const ERC20_ALLOWANCE_ABI = [
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
] as const;

// ERC-20 `approve(spender, amount)` selector = 0x095ea7b3
function encodeApprove(spender: Address, amount: bigint): Hex {
  const spenderHex = spender.toLowerCase().replace(/^0x/, '').padStart(64, '0');
  const amountHex = amount.toString(16).padStart(64, '0');
  return `0x095ea7b3${spenderHex}${amountHex}` as Hex;
}

async function readAllowance(
  chainName: EvmChainName,
  token: Address,
  owner: Address,
  spender: Address,
): Promise<bigint> {
  const client = createPublicClient({ chain: VIEM_CHAINS[chainName], transport: http() });
  return (await client.readContract({
    address: token,
    abi: ERC20_ALLOWANCE_ABI,
    functionName: 'allowance',
    args: [owner, spender],
  })) as bigint;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   READ-ONLY TOOLS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * List chains supported by deBridge DLN.
 * Calls GET /v1.0/supported-chains-info.
 */
export async function listSupportedChains(_params: Record<string, unknown> = {}) {
  try {
    const data = await debridgeApi.get<{ chains: Array<Record<string, unknown>> }>(
      '/supported-chains-info',
    );
    return {
      status: 'ok' as const,
      data: data.chains,
      note: 'Each entry is {chainId, originalChainId, chainName}. Solana uses a non-EIP155 chainId.',
    };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'DEBRIDGE_API_ERROR',
      message: (e as Error).message,
    };
  }
}

/**
 * Get a cross-chain bridge quote.
 * Calls GET /v1.0/dln/order/quote.
 */
export async function getQuote(params: {
  srcChainId: number;
  srcChainTokenIn: string;
  srcChainTokenInAmount: string;          // UI units, e.g. "1" for 1 USDC
  srcChainTokenInDecimals: number;
  dstChainId: number;
  dstChainTokenOut: string;
  prependOperatingExpenses?: boolean;
}) {
  const validationError =
    validateChainId(params.srcChainId, 'srcChainId') ??
    validateChainId(params.dstChainId, 'dstChainId') ??
    validateEvmAddress(params.srcChainTokenIn, 'srcChainTokenIn') ??
    validateEvmAddress(params.dstChainTokenOut, 'dstChainTokenOut') ??
    validatePositiveAmount(params.srcChainTokenInAmount, 'srcChainTokenInAmount') ??
    validateDecimals(params.srcChainTokenInDecimals, 'srcChainTokenInDecimals');
  if (validationError) return validationError;

  const amountWei = parseUnits(params.srcChainTokenInAmount, params.srcChainTokenInDecimals);

  try {
    const data = await debridgeApi.get<unknown>('/dln/order/quote', {
      srcChainId: params.srcChainId,
      srcChainTokenIn: lc(params.srcChainTokenIn),
      srcChainTokenInAmount: amountWei.toString(),
      dstChainId: params.dstChainId,
      dstChainTokenOut: lc(params.dstChainTokenOut),
      prependOperatingExpenses: params.prependOperatingExpenses ?? true,
    });
    return { status: 'ok' as const, data };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'DEBRIDGE_API_ERROR',
      message: (e as Error).message,
      suggestion:
        'Verify srcChainTokenIn/dstChainTokenOut addresses are valid for their chains, and that the (src,dst,token) route is supported.',
    };
  }
}

/**
 * Get DLN order status by orderId.
 * Calls GET /v1.0/dln/order/{orderId}.
 */
export async function getOrderStatus(params: { orderId: string }) {
  if (!params.orderId) {
    return {
      status: 'error' as const,
      error_code: 'MISSING_PARAM',
      message: '`orderId` is required (returned by buildBridge in the create-tx response).',
    };
  }
  if (!/^0x[a-fA-F0-9]{64}$/.test(params.orderId)) {
    return {
      status: 'error' as const,
      error_code: 'INVALID_PARAM',
      message: `\`orderId\` must be a 0x-prefixed 64-char hex string (got '${params.orderId}').`,
    };
  }

  try {
    const data = await debridgeApi.get<unknown>(`/dln/order/${params.orderId}`);
    return { status: 'ok' as const, data };
  } catch (e) {
    const msg = (e as Error).message;
    if (/UNKNOWN_ORDER|404|400/.test(msg)) {
      return {
        status: 'error' as const,
        error_code: 'ORDER_NOT_FOUND',
        message: `Order ${params.orderId} not found by DLN. Either the origin tx has not been mined/indexed, or the orderId is wrong.`,
        details: msg,
      };
    }
    return {
      status: 'error' as const,
      error_code: 'DEBRIDGE_API_ERROR',
      message: msg,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
//   TRANSACTION TOOLS — return pending_sign | ToolError
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Build a cross-chain bridge/swap via deBridge DLN.
 *
 * Stateful single-step emission:
 *   - If src token is ERC-20 AND allowance to the DLN router is insufficient,
 *     emits the approve pending_sign (Step 1 of 2).
 *   - Otherwise emits the DLN create-tx pending_sign (the actual order).
 *
 * Native-asset bridges (srcChainTokenIn = 0x0000…0000) skip approval and emit
 * the order directly with `value` set.
 */
export async function buildBridge(params: {
  srcChainId: number;
  srcChainTokenIn: string;
  srcChainTokenInAmount: string;          // UI units
  srcChainTokenInDecimals: number;
  dstChainId: number;
  dstChainTokenOut: string;
  dstChainTokenOutAmount?: string;        // "auto" by default
  walletAddress: string;                  // src + dst order authority, also recipient default
  dstChainTokenOutRecipient?: string;     // defaults to walletAddress
  prependOperatingExpenses?: boolean;
}): Promise<PendingSign | ToolError> {
  const validationError =
    validateChainId(params.srcChainId, 'srcChainId') ??
    validateChainId(params.dstChainId, 'dstChainId') ??
    validateEvmAddress(params.srcChainTokenIn, 'srcChainTokenIn') ??
    validateEvmAddress(params.dstChainTokenOut, 'dstChainTokenOut') ??
    validateEvmAddress(params.walletAddress, 'walletAddress') ??
    validatePositiveAmount(params.srcChainTokenInAmount, 'srcChainTokenInAmount') ??
    validateDecimals(params.srcChainTokenInDecimals, 'srcChainTokenInDecimals');
  if (validationError) return validationError;

  if (params.dstChainTokenOutRecipient) {
    const rError = validateEvmAddress(params.dstChainTokenOutRecipient, 'dstChainTokenOutRecipient');
    if (rError) return rError;
  }

  const srcChainName = chainNameFromId(params.srcChainId);
  if (!isEvmChain(srcChainName)) {
    return {
      status: 'error',
      error_code: 'UNSUPPORTED_CHAIN',
      message: `srcChainId ${params.srcChainId} (${srcChainName}) is not in the EVM allowlist. Solana origin is not yet supported by this skill.`,
    };
  }

  const amountWei = parseUnits(params.srcChainTokenInAmount, params.srcChainTokenInDecimals);

  type CreateTxResponse = {
    estimation?: unknown;
    tx?: { to: string; data: string; value?: string };
    orderId?: string;
  };

  let resp: CreateTxResponse;
  try {
    resp = await debridgeApi.get<CreateTxResponse>('/dln/order/create-tx', {
      srcChainId: params.srcChainId,
      srcChainTokenIn: lc(params.srcChainTokenIn),
      srcChainTokenInAmount: amountWei.toString(),
      dstChainId: params.dstChainId,
      dstChainTokenOut: lc(params.dstChainTokenOut),
      dstChainTokenOutAmount: params.dstChainTokenOutAmount ?? 'auto',
      dstChainTokenOutRecipient: lc(params.dstChainTokenOutRecipient ?? params.walletAddress),
      srcChainOrderAuthorityAddress: lc(params.walletAddress),
      dstChainOrderAuthorityAddress: lc(params.walletAddress),
      prependOperatingExpenses: params.prependOperatingExpenses ?? true,
    });
  } catch (e) {
    const msg = (e as Error).message;
    if (/INSUFFICIENT_AMOUNT|MIN_AMOUNT/i.test(msg)) {
      return {
        status: 'error',
        error_code: 'INSUFFICIENT_AMOUNT',
        message: msg,
        suggestion: 'Increase srcChainTokenInAmount above the DLN minimum for this route.',
      };
    }
    if (/no route|ROUTE/i.test(msg)) {
      return {
        status: 'error',
        error_code: 'ROUTE_NOT_FOUND',
        message: msg,
        suggestion: 'Check that the (srcChainId, dstChainId, token) route is supported by DLN.',
      };
    }
    return {
      status: 'error',
      error_code: 'DEBRIDGE_API_ERROR',
      message: msg,
    };
  }

  if (!resp.tx || !resp.tx.to || !resp.tx.data) {
    return {
      status: 'error',
      error_code: 'DEBRIDGE_API_ERROR',
      message: 'DLN create-tx response missing tx.to/tx.data.',
      details: resp,
    };
  }

  const router = lc(resp.tx.to) as Address;
  const value = resp.tx.value ?? '0';

  // ERC-20 path → check allowance and possibly emit approve first.
  if (!isNative(params.srcChainTokenIn)) {
    let allowance: bigint;
    try {
      allowance = await readAllowance(
        srcChainName,
        lc(params.srcChainTokenIn) as Address,
        lc(params.walletAddress) as Address,
        router,
      );
    } catch (e) {
      return {
        status: 'error',
        error_code: 'RPC_ERROR',
        message: `Failed to read ERC-20 allowance on ${srcChainName}: ${(e as Error).message}`,
        suggestion: 'Check RPC health or retry. The fallback is to broadcast an approval manually.',
      };
    }

    if (allowance < amountWei) {
      return {
        status: 'pending_sign',
        unsigned_tx: {
          to: lc(params.srcChainTokenIn),
          data: encodeApprove(router, amountWei),
          value: '0',
          chain: srcChainName,
        },
        description:
          `Step 1 of 2: Approve ${params.srcChainTokenInAmount} of ${params.srcChainTokenIn} ` +
          `for the deBridge DLN router (${router}) on ${srcChainName}. ` +
          `After this transaction lands, re-invoke buildBridge with the same params to receive the DLN order.`,
        next_action: { tool: 'onchainos wallet contract-call' },
        step_info: { current: 1, total: 2, name: 'approve' },
      };
    }
  }

  // Native or allowance sufficient → emit the DLN order tx.
  // P1-14: gate on native balance — DLN's protocol fixFee is paid in native gas.
  // Underfunded broadcast reverts on-chain and wastes gas.
  const client = createPublicClient({ chain: VIEM_CHAINS[srcChainName], transport: http() });
  const balanceErr = await nativeBalanceCheck(client, srcChainName, params.walletAddress, BigInt(value));
  if (balanceErr) return balanceErr;

  const orderRef = resp.orderId ? ` orderId=${resp.orderId}` : '';
  return {
    status: 'pending_sign',
    unsigned_tx: {
      to: lc(resp.tx.to),
      data: resp.tx.data,
      value,
      chain: srcChainName,
    },
    description:
      `Bridge ${params.srcChainTokenInAmount} from chainId ${params.srcChainId} → ${params.dstChainId} ` +
      `via deBridge DLN.${orderRef} After broadcast, poll getOrderStatus with the returned orderId.`,
    next_action: { tool: 'onchainos wallet contract-call' },
  };
}

/**
 * Build a same-chain swap via the deBridge DLN aggregator.
 * Calls GET /v1.0/chain/transaction.
 *
 * Stateful single-step emission: emits approve first if needed (ERC-20),
 * otherwise emits the swap tx directly.
 */
export async function buildSameChainSwap(params: {
  chainId: number;
  tokenIn: string;
  tokenInAmount: string;        // UI units
  tokenInDecimals: number;
  tokenOut: string;
  walletAddress: string;
  tokenOutRecipient?: string;   // defaults to walletAddress
  slippage?: number;            // percent, e.g. 1 = 1%
  affiliateFeePercent?: number; // default 0
}): Promise<PendingSign | ToolError> {
  const validationError =
    validateChainId(params.chainId, 'chainId') ??
    validateEvmAddress(params.tokenIn, 'tokenIn') ??
    validateEvmAddress(params.tokenOut, 'tokenOut') ??
    validateEvmAddress(params.walletAddress, 'walletAddress') ??
    validatePositiveAmount(params.tokenInAmount, 'tokenInAmount') ??
    validateDecimals(params.tokenInDecimals, 'tokenInDecimals');
  if (validationError) return validationError;

  if (params.tokenOutRecipient) {
    const rError = validateEvmAddress(params.tokenOutRecipient, 'tokenOutRecipient');
    if (rError) return rError;
  }

  const chainName = chainNameFromId(params.chainId);
  if (!isEvmChain(chainName)) {
    return {
      status: 'error',
      error_code: 'UNSUPPORTED_CHAIN',
      message: `chainId ${params.chainId} (${chainName}) is not in the EVM allowlist for same-chain swap.`,
    };
  }

  const amountWei = parseUnits(params.tokenInAmount, params.tokenInDecimals);

  type SwapResponse = {
    tx?: { to: string; data: string; value?: string };
    tokenIn?: unknown;
    tokenOut?: unknown;
    slippage?: unknown;
  };

  let resp: SwapResponse;
  try {
    resp = await debridgeApi.get<SwapResponse>('/chain/transaction', {
      chainId: params.chainId,
      tokenIn: lc(params.tokenIn),
      tokenInAmount: amountWei.toString(),
      tokenOut: lc(params.tokenOut),
      tokenOutRecipient: lc(params.tokenOutRecipient ?? params.walletAddress),
      senderAddress: lc(params.walletAddress),
      slippage: params.slippage ?? 1,
      affiliateFeePercent: params.affiliateFeePercent ?? 0,
    });
  } catch (e) {
    return {
      status: 'error',
      error_code: 'DEBRIDGE_API_ERROR',
      message: (e as Error).message,
      suggestion: 'Verify tokenIn/tokenOut are valid addresses on the given chain, and that a route exists.',
    };
  }

  if (!resp.tx || !resp.tx.to || !resp.tx.data) {
    return {
      status: 'error',
      error_code: 'NO_QUOTE',
      message: 'DLN same-chain aggregator returned no tx — route may be unavailable.',
      details: resp,
    };
  }

  const router = lc(resp.tx.to) as Address;
  const value = resp.tx.value ?? '0';

  if (!isNative(params.tokenIn)) {
    let allowance: bigint;
    try {
      allowance = await readAllowance(
        chainName,
        lc(params.tokenIn) as Address,
        lc(params.walletAddress) as Address,
        router,
      );
    } catch (e) {
      return {
        status: 'error',
        error_code: 'RPC_ERROR',
        message: `Failed to read ERC-20 allowance on ${chainName}: ${(e as Error).message}`,
      };
    }

    if (allowance < amountWei) {
      return {
        status: 'pending_sign',
        unsigned_tx: {
          to: lc(params.tokenIn),
          data: encodeApprove(router, amountWei),
          value: '0',
          chain: chainName,
        },
        description:
          `Step 1 of 2: Approve ${params.tokenInAmount} of ${params.tokenIn} ` +
          `for the deBridge swap router (${router}) on ${chainName}. ` +
          `After this transaction lands, re-invoke buildSameChainSwap with the same params.`,
        next_action: { tool: 'onchainos wallet contract-call' },
        step_info: { current: 1, total: 2, name: 'approve' },
      };
    }
  }

  // P1-14: same-chain swap with native input also requires value > 0 from the wallet.
  // Gate on native balance for any non-zero tx.value to avoid revert-on-broadcast.
  if (BigInt(value) > 0n) {
    const client = createPublicClient({ chain: VIEM_CHAINS[chainName], transport: http() });
    const balanceErr = await nativeBalanceCheck(client, chainName, params.walletAddress, BigInt(value));
    if (balanceErr) return balanceErr;
  }

  return {
    status: 'pending_sign',
    unsigned_tx: {
      to: lc(resp.tx.to),
      data: resp.tx.data,
      value,
      chain: chainName,
    },
    description:
      `Same-chain swap ${params.tokenInAmount} ${params.tokenIn} → ${params.tokenOut} ` +
      `on ${chainName} via deBridge DLN aggregator.`,
    next_action: { tool: 'onchainos wallet contract-call' },
  };
}
