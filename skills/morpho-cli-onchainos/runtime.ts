// runtime.ts — runtime backing for morpho-cli-onchainos tools.
//
// Two modes:
//   * MOCK   (default for B-mock smoke testing): canned Morpho CLI responses
//   * REAL   (B-real / live): actually invokes `npx @morpho-org/cli@latest <cmd>` and parses JSON output
//
// Switch by setting env var:  MORPHO_RUNTIME=real
// Default is MOCK so `tsx cli.ts --help` and a no-deps smoke don't need network.

import { execSync } from 'node:child_process';

// ============================================================================
//   Types
// ============================================================================

export type ToolError = {
  status: 'error';
  error_code: string;
  message: string;
  suggestion?: string;
  details?: unknown;
};

export type MorphoTransaction = {
  to: string;
  data: string;
  value: string;
  chainId?: string;
  description?: string;
};

export type PreparedOperation = {
  operation:
    | 'deposit'
    | 'withdraw'
    | 'supply'
    | 'borrow'
    | 'repay'
    | 'supply_collateral'
    | 'withdraw_collateral';
  chain: string;
  summary: string;
  requirements?: Array<{ type: string; token?: string; spender?: string; amount?: string; deadline?: string }>;
  transactions: MorphoTransaction[];
  simulated?: boolean;
  simulationOk?: boolean;
  revertReason?: string;
  totalGasUsed?: string;
  outcome?: {
    vault?: Record<string, unknown>;
    market?: Record<string, unknown>;
  };
  warnings?: Array<{ level?: string; message: string; code?: string }>;
};

// ============================================================================
//   Numeric helpers (kept for parity with the scaffold template; the Morpho
//   CLI already does decimal handling, so they're rarely needed here)
// ============================================================================

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

// Lowercase an EVM address for stable comparison. Morpho CLI tolerates both
// lowercase and EIP-55, but downstream onchainos calls expect lowercase.
export function lc(addr: string): `0x${string}` {
  return addr.toLowerCase() as `0x${string}`;
}

// ============================================================================
//   Mode toggle
// ============================================================================

const USE_REAL = process.env.MORPHO_RUNTIME === 'real';

// ============================================================================
//   Mock fixtures
// ============================================================================

const MOCK_VAULT_ADDR = '0x0000000000000000000000000000000000000aaa';
const MOCK_MARKET_ID = '0x' + 'cd'.repeat(32);
const MOCK_USDC_BASE = '0x833589fcd6edb6e08f4c7c32d4f71b54bda02913';
const MOCK_USER_ADDR = '0x1111111111111111111111111111111111111111';

const MOCK_ALLOWANCE = process.env.MORPHO_MOCK_ALLOWANCE ?? 'insufficient';

function mockPreparedDeposit(chain: string, vaultAddress: string, amount: string): PreparedOperation {
  const needsApprove = MOCK_ALLOWANCE !== 'sufficient';
  const transactions: MorphoTransaction[] = [];
  if (needsApprove) {
    transactions.push({
      to: MOCK_USDC_BASE,
      data: '0x095ea7b3' + '00'.repeat(64),
      value: '0',
      chainId: '8453',
      description: `Approve ${amount} USDC to vault`,
    });
  }
  transactions.push({
    to: vaultAddress,
    data: '0x6e553f65' + '00'.repeat(64),
    value: '0',
    chainId: '8453',
    description: `Deposit ${amount} USDC into vault`,
  });
  return {
    operation: 'deposit',
    chain,
    summary: `Deposit ${amount} USDC into Steakhouse USDC`,
    requirements: needsApprove
      ? [{ type: 'approval', token: MOCK_USDC_BASE, spender: vaultAddress, amount: '1000000000' }]
      : [],
    transactions,
    simulated: true,
    simulationOk: true,
    totalGasUsed: '350000',
    outcome: {
      vault: {
        sharesReceived: '987654321',
        assetsReceived: '1000000000',
        positionAssets: '1000000000',
        positionShares: '987654321',
      },
    },
    warnings: [],
  };
}

function mockPreparedSingleAction(
  operation: PreparedOperation['operation'],
  chain: string,
  target: string,
  amount: string,
): PreparedOperation {
  const isMarket = operation !== 'deposit' && operation !== 'withdraw';
  return {
    operation,
    chain,
    summary: `${operation} ${amount} on ${target}`,
    requirements: [],
    transactions: [
      {
        to: '0xbbbb000000000000000000000000000000000bbb',
        data: '0xaaaaaaaa' + '00'.repeat(60),
        value: '0',
        chainId: '8453',
        description: `${operation} ${amount}`,
      },
    ],
    simulated: true,
    simulationOk: true,
    totalGasUsed: '180000',
    outcome: isMarket
      ? {
          market: {
            supplied: '0',
            borrowed: '500000000',
            collateral: '1000000000000000000',
            healthFactor: '1.85',
            isHealthy: true,
            maxBorrowable: '200000000',
            utilizationBeforePct: '75',
            utilizationAfterPct: '76',
            borrowApyBeforePct: '3.12',
            borrowApyAfterPct: '3.20',
          },
        }
      : {
          vault: {
            sharesReceived: '0',
            assetsReceived: '1000000000',
            positionAssets: '0',
            positionShares: '0',
          },
        },
    warnings: [],
  };
}

const MOCK_RESPONSES: Record<string, (args: Record<string, string>) => unknown> = {
  'health-check': () => ({ status: 'healthy' }),
  'get-supported-chains': () => [
    { slug: 'ethereum', name: 'Ethereum', chainId: '1', explorerUrl: 'https://etherscan.io', isTestnet: false },
    { slug: 'base', name: 'Base', chainId: '8453', explorerUrl: 'https://basescan.org', isTestnet: false },
    { slug: 'arbitrum', name: 'Arbitrum One', chainId: '42161', explorerUrl: 'https://arbiscan.io', isTestnet: false },
  ],
  'query-vaults': (args) => ({
    chain: args.chain ?? 'base',
    vaults: [
      {
        address: MOCK_VAULT_ADDR,
        name: 'Steakhouse USDC',
        version: 'v1',
        asset: { address: MOCK_USDC_BASE, symbol: 'USDC' },
        apyPct: '5.34',
        tvl: { symbol: 'USDC', value: '125000000' },
        tvlUsd: '125000000.00',
        feePct: '10',
      },
    ],
  }),
  'get-vault': (args) => ({
    address: args.address ?? MOCK_VAULT_ADDR,
    chain: args.chain ?? 'base',
    name: 'Steakhouse USDC',
    version: 'v1',
    asset: { address: MOCK_USDC_BASE, symbol: 'USDC' },
    curator: 'Steakhouse',
    apyPct: '5.34',
    feePct: '10',
    tvl: { symbol: 'USDC', value: '125000000' },
    tvlUsd: '125000000.00',
  }),
  'query-markets': (args) => ({
    chain: args.chain ?? 'base',
    markets: [
      {
        id: MOCK_MARKET_ID,
        loanAsset: { address: MOCK_USDC_BASE, symbol: 'USDC' },
        collateralAsset: { address: '0x4200000000000000000000000000000000000006', symbol: 'WETH' },
        lltvPct: '86',
        borrowApyPct: '3.12',
        supplyApyPct: '2.10',
        utilizationPct: '75',
        totalSupply: { symbol: 'USDC', value: '5000000' },
        totalBorrow: { symbol: 'USDC', value: '3500000' },
        totalCollateral: { symbol: 'WETH', value: '1500' },
        totalLiquidity: { symbol: 'USDC', value: '1500000' },
      },
    ],
  }),
  'get-market': (args) => ({
    id: args.id ?? MOCK_MARKET_ID,
    chain: args.chain ?? 'base',
    loanAsset: { address: MOCK_USDC_BASE, symbol: 'USDC' },
    collateralAsset: { address: '0x4200000000000000000000000000000000000006', symbol: 'WETH' },
    lltvPct: '86',
    borrowApyPct: '3.12',
    supplyApyPct: '2.10',
    utilizationPct: '75',
  }),
  'get-positions': (args) => ({
    chain: args.chain ?? 'base',
    userAddress: args['user-address'] ?? MOCK_USER_ADDR,
    totals: { vaultCount: 1, marketCount: 0, suppliedUsd: '10005.42', netWorthUsd: '10005.42' },
    vaultPositions: [
      {
        vault: {
          address: MOCK_VAULT_ADDR,
          name: 'Steakhouse USDC',
          asset: { address: MOCK_USDC_BASE, symbol: 'USDC' },
          version: 'v1',
        },
        supplied: { symbol: 'USDC', value: '10000' },
        suppliedUsd: '10005.42',
      },
    ],
    marketPositions: [],
  }),
  'get-token-balance': (args) => ({
    chain: args.chain ?? 'base',
    userAddress: args['user-address'] ?? MOCK_USER_ADDR,
    asset: { address: args['token-address'] ?? MOCK_USDC_BASE, symbol: 'USDC' },
    balance: { symbol: 'USDC', value: '1000.00' },
    morphoAllowance: { symbol: 'USDC', value: '0' },
    bundlerAllowance: { symbol: 'USDC', value: '0' },
    permit2Allowance: { symbol: 'USDC', value: '0' },
    needsApprovalForMorpho: true,
    needsApprovalForBundler: true,
  }),
  'simulate-transactions': (args) => ({
    chain: args.chain ?? 'base',
    allSucceeded: true,
    totalGasUsed: '245000',
    executionResults: [],
    warnings: [],
  }),
  'prepare-deposit': (args) =>
    mockPreparedDeposit(args.chain ?? 'base', args['vault-address'] ?? MOCK_VAULT_ADDR, args.amount ?? '1000'),
  'prepare-withdraw': (args) =>
    mockPreparedSingleAction('withdraw', args.chain ?? 'base', args['vault-address'] ?? MOCK_VAULT_ADDR, args.amount ?? 'max'),
  'prepare-supply': (args) =>
    mockPreparedDeposit(args.chain ?? 'base', args['market-id'] ?? MOCK_MARKET_ID, args.amount ?? '5000'),
  'prepare-borrow': (args) =>
    mockPreparedSingleAction('borrow', args.chain ?? 'base', args['market-id'] ?? MOCK_MARKET_ID, args['borrow-amount'] ?? '1'),
  'prepare-repay': (args) =>
    mockPreparedSingleAction('repay', args.chain ?? 'base', args['market-id'] ?? MOCK_MARKET_ID, args.amount ?? 'max'),
  'prepare-supply-collateral': (args) =>
    mockPreparedDeposit(args.chain ?? 'base', args['market-id'] ?? MOCK_MARKET_ID, args.amount ?? '5000'),
  'prepare-withdraw-collateral': (args) =>
    mockPreparedSingleAction('withdraw_collateral', args.chain ?? 'base', args['market-id'] ?? MOCK_MARKET_ID, args.amount ?? 'max'),
};

// ============================================================================
//   Real-mode invocation
// ============================================================================

const CLI_TIMEOUT_MS = Number(process.env.MORPHO_CLI_TIMEOUT_MS ?? '60000');
const CLI_PACKAGE = process.env.MORPHO_CLI_PACKAGE ?? '@morpho-org/cli@latest';

function flagify(key: string, value: string | undefined): string[] {
  if (value === undefined || value === null || value === '') return [];
  return [`--${key}`, shellQuote(value)];
}

function shellQuote(s: string): string {
  if (/^[A-Za-z0-9_./:=,-]+$/.test(s)) return s;
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

function invokeRealCli(command: string, args: Record<string, string | undefined>): unknown {
  const parts = ['npx', '--yes', CLI_PACKAGE, command];
  for (const [k, v] of Object.entries(args)) {
    parts.push(...flagify(k, v));
  }
  const cmd = parts.join(' ');
  let stdout: Buffer;
  try {
    stdout = execSync(cmd, {
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: CLI_TIMEOUT_MS,
      maxBuffer: 16 * 1024 * 1024,
    });
  } catch (e) {
    const err = e as NodeJS.ErrnoException & { stderr?: Buffer; signal?: string };
    if (err.signal === 'SIGTERM') {
      const wrapped: ToolError = {
        status: 'error',
        error_code: 'CLI_TIMEOUT',
        message: `Morpho CLI command '${command}' timed out after ${CLI_TIMEOUT_MS}ms.`,
        suggestion: 'Retry; if the issue persists, set MORPHO_CLI_TIMEOUT_MS to a larger value.',
      };
      throw wrapped;
    }
    if (err.code === 'ENOENT') {
      const wrapped: ToolError = {
        status: 'error',
        error_code: 'CLI_NOT_FOUND',
        message: 'Could not find `npx`. Install Node.js >= 18 and re-run.',
      };
      throw wrapped;
    }
    const stderr = err.stderr ? err.stderr.toString() : (err as Error).message;
    const wrapped: ToolError = {
      status: 'error',
      error_code: 'SDK_ERROR',
      message: `Morpho CLI '${command}' failed: ${stderr.trim().slice(0, 500)}`,
    };
    throw wrapped;
  }
  const raw = stdout.toString();
  try {
    return JSON.parse(raw);
  } catch (e) {
    const wrapped: ToolError = {
      status: 'error',
      error_code: 'CLI_PARSE_ERROR',
      message: `Morpho CLI '${command}' returned non-JSON output (first 200 chars): ${raw.slice(0, 200)}`,
    };
    throw wrapped;
  }
}

// ============================================================================
//   Public surface — invokeMorphoCli is the one entry point the tools use
// ============================================================================

export function invokeMorphoCli<T = unknown>(command: string, args: Record<string, string | undefined>): T {
  if (!USE_REAL) {
    const mock = MOCK_RESPONSES[command];
    if (!mock) {
      throw {
        status: 'error',
        error_code: 'SDK_ERROR',
        message: `[mock] no mock fixture for Morpho CLI command '${command}'`,
      } satisfies ToolError;
    }
    const stringArgs: Record<string, string> = {};
    for (const [k, v] of Object.entries(args)) {
      if (v !== undefined && v !== null) stringArgs[k] = String(v);
    }
    return mock(stringArgs) as T;
  }
  return invokeRealCli(command, args) as T;
}
