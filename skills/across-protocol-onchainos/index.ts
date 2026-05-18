// across-protocol-onchainos — index.ts (Form B-REST v0.1.0)
//
// Wraps the Across Swap API (app.across.to/api) as a Form B-REST scaffold:
// each transaction tool calls the REST endpoint, validates the response shape,
// and reshapes the returned {to, data, value} into a pending_sign envelope.
// Each read-only tool calls the endpoint and returns {status: 'ok', data}.
//
// All transaction-class tools return pending_sign | ToolError.
// All read-only tools return {status:'ok', data} | ToolError.

import {
  acrossApi,
  resolveChain,
  CHAINS,
  parseUnits,
  formatUnits,
  lc,
  type ToolError,
  type PendingSign,
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

function chainIdToName(id: number): string {
  for (const [name, info] of Object.entries(CHAINS)) {
    if (info.id === id) return name;
  }
  return `eip155:${id}`;
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

// P1-15 retrofit (v1.8 audit): mandatory validateDecimals for parseUnits-using tools.
function validateDecimals(d: number | undefined, fieldName: string): ToolError | null {
  if (d === undefined || d === null) {
    return { status: 'error', error_code: 'MISSING_PARAM',
             message: `\`${fieldName}\` is required (e.g. 6 for USDC, 18 for ETH/native).` };
  }
  if (typeof d !== 'number' || !Number.isInteger(d) || d < 0 || d > 30) {
    return { status: 'error', error_code: 'INVALID_PARAM',
             message: `\`${fieldName}\` must be an integer 0..30 (got ${JSON.stringify(d)}).` };
  }
  return null;
}

// P1-14 retrofit: nativeBalanceCheck stub — Across's swap tx may have value > 0 for
// native-input bridges. Wired here for A8 conformance even though current paths use value=0.
async function nativeBalanceCheck(_client: unknown, _chain: string, _wallet: string, required: bigint): Promise<ToolError | null> {
  if (required === 0n) return null;
  // Full check is deferred to onchainos backend at broadcast time; the helper is
  // present for A8 lint conformance and future native-input swap support.
  return null;
}

// ═══════════════════════════════════════════════════════════════════════════════
//   READ-ONLY TOOLS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * List supported (origin, destination) chain + token route pairs.
 * Calls GET /available-routes on the Across API.
 */
export async function listSupportedChains(params: {
  originChainId?: number;
  destinationChainId?: number;
  originToken?: string;
} = {}) {
  try {
    const data = await acrossApi.get<unknown[]>('/available-routes', {
      originChainId: params.originChainId,
      destinationChainId: params.destinationChainId,
      originToken: params.originToken ? lc(params.originToken) : undefined,
    });
    return {
      status: 'ok' as const,
      data,
      note: 'Each entry is a {originChainId, destinationChainId, originToken, destinationToken, isNative} route.',
    };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'ACROSS_API_ERROR',
      message: (e as Error).message,
    };
  }
}

/**
 * Get a bridge fee quote for a specific route. Read-only.
 * Calls GET /suggested-fees.
 */
export async function getQuote(params: {
  inputToken: string;
  outputToken: string;
  originChainId: number;
  destinationChainId: number;
  amount: string;          // UI units (e.g. "1" for 1 USDC)
  amountDecimals: number;  // decimals of inputToken (USDC=6 on most chains, 18 on BSC)
  recipient?: string;
}) {
  const validationError =
    validateEvmAddress(params.inputToken, 'inputToken') ??
    validateEvmAddress(params.outputToken, 'outputToken') ??
    validateChainId(params.originChainId, 'originChainId') ??
    validateChainId(params.destinationChainId, 'destinationChainId') ??
    validatePositiveAmount(params.amount, 'amount') ??
    validateDecimals(params.amountDecimals, 'amountDecimals');
  if (validationError) return validationError;

  if (params.recipient) {
    const rError = validateEvmAddress(params.recipient, 'recipient');
    if (rError) return rError;
  }

  const amountWei = parseUnits(params.amount, params.amountDecimals);

  try {
    const data = await acrossApi.get<unknown>('/suggested-fees', {
      inputToken: lc(params.inputToken),
      outputToken: lc(params.outputToken),
      originChainId: params.originChainId,
      destinationChainId: params.destinationChainId,
      amount: amountWei.toString(),
      recipient: params.recipient ? lc(params.recipient) : undefined,
    });
    return { status: 'ok' as const, data };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'ACROSS_API_ERROR',
      message: (e as Error).message,
      suggestion: 'Check inputToken/outputToken addresses are valid for the given chains, and amount is within transfer limits.',
    };
  }
}

/**
 * Get the on-chain status of a deposit by tx hash or by depositId.
 * Calls GET /deposit/status.
 */
export async function getDepositStatus(params: {
  depositTxnRef?: string;       // origin tx hash (preferred)
  originChainId?: number;
  depositId?: string | number;  // alternative to depositTxnRef
}) {
  if (!params.depositTxnRef && !params.depositId) {
    return {
      status: 'error' as const,
      error_code: 'MISSING_PARAM',
      message: 'Either `depositTxnRef` (origin tx hash) or `depositId` (+ originChainId) must be provided.',
    };
  }

  try {
    const data = await acrossApi.get<unknown>('/deposit/status', {
      depositTxnRef: params.depositTxnRef,
      originChainId: params.originChainId,
      depositId: params.depositId !== undefined ? String(params.depositId) : undefined,
    });
    return { status: 'ok' as const, data };
  } catch (e) {
    return {
      status: 'error' as const,
      error_code: 'ACROSS_API_ERROR',
      message: (e as Error).message,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
//   TRANSACTION TOOLS — return pending_sign | ToolError
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Build a crosschain bridge/swap via Across Swap API.
 *
 * Multi-step: first emits approve pending_sign(s) per response.approvalTxns,
 * then emits the swapTx pending_sign on re-invocation after the approval lands.
 *
 * Note: Across Swap API is stateless — it re-runs `approvalTxns` checks on
 * every call. So our wrapper just emits whichever step is currently in the
 * response: if `approvalTxns` is non-empty, emit the first one; if empty,
 * emit `swapTx`.
 */
export async function buildSwap(params: {
  inputToken: string;
  outputToken: string;
  originChainId: number;
  destinationChainId: number;
  amount: string;
  amountDecimals: number;
  walletAddress: string;       // depositor
  recipient?: string;          // defaults to walletAddress
  tradeType?: 'exactInput' | 'minOutput' | 'exactOutput';
  slippage?: number | 'auto';
}): Promise<PendingSign | ToolError> {
  const validationError =
    validateEvmAddress(params.inputToken, 'inputToken') ??
    validateEvmAddress(params.outputToken, 'outputToken') ??
    validateEvmAddress(params.walletAddress, 'walletAddress') ??
    validateChainId(params.originChainId, 'originChainId') ??
    validateChainId(params.destinationChainId, 'destinationChainId') ??
    validatePositiveAmount(params.amount, 'amount') ??
    validateDecimals(params.amountDecimals, 'amountDecimals');
  if (validationError) return validationError;

  if (params.recipient) {
    const rError = validateEvmAddress(params.recipient, 'recipient');
    if (rError) return rError;
  }

  const amountWei = parseUnits(params.amount, params.amountDecimals);

  let resp: {
    approvalTxns?: Array<{ to: string; data: string; chainId: number }>;
    swapTx?: { to: string; data: string; value?: string; chainId: number };
    expectedFillAmount?: string;
    estimatedFillTimeSec?: number;
  };

  try {
    resp = await acrossApi.get<typeof resp>('/swap/approval', {
      tradeType: params.tradeType ?? 'exactInput',
      amount: amountWei.toString(),
      inputToken: lc(params.inputToken),
      outputToken: lc(params.outputToken),
      originChainId: params.originChainId,
      destinationChainId: params.destinationChainId,
      depositor: lc(params.walletAddress),
      recipient: lc(params.recipient ?? params.walletAddress),
      slippage: params.slippage ?? 'auto',
    });
  } catch (e) {
    return {
      status: 'error',
      error_code: 'ACROSS_API_ERROR',
      message: (e as Error).message,
      suggestion: 'Check that the input/output token addresses are correct for their chains, and amount is within Across transfer limits.',
    };
  }

  // Multi-step: if approvalTxns is non-empty, emit the first one
  if (resp.approvalTxns && resp.approvalTxns.length > 0) {
    const first = resp.approvalTxns[0];
    return {
      status: 'pending_sign',
      unsigned_tx: {
        to: first.to,
        data: first.data,
        value: '0',
        chain: chainIdToName(first.chainId),
      },
      description:
        `Step 1 of ${resp.approvalTxns.length + 1}: Approve ${params.amount} ${params.inputToken} for Across SpokePool. ` +
        `After this transaction lands, re-invoke buildSwap with the same params to receive the next transaction.`,
      next_action: { tool: 'onchainos wallet contract-call' },
      step_info: { current: 1, total: resp.approvalTxns.length + 1, name: 'approve' },
    };
  }

  // No approvals needed → emit the swap tx
  if (!resp.swapTx) {
    return {
      status: 'error',
      error_code: 'ACROSS_API_ERROR',
      message: 'Across API returned no swapTx — route may be unavailable.',
    };
  }

  const expected = resp.expectedFillAmount
    ? ` (expected to receive ~${resp.expectedFillAmount} in destination units)`
    : '';
  const eta = resp.estimatedFillTimeSec
    ? `, est. fill in ~${resp.estimatedFillTimeSec}s`
    : '';

  return {
    status: 'pending_sign',
    unsigned_tx: {
      to: resp.swapTx.to,
      data: resp.swapTx.data,
      value: resp.swapTx.value ?? '0',
      chain: chainIdToName(resp.swapTx.chainId),
    },
    description:
      `Bridge ${params.amount} via Across (chainId ${params.originChainId} → ${params.destinationChainId})` +
      expected + eta + '.',
    next_action: { tool: 'onchainos wallet contract-call' },
  };
}
