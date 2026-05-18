#!/usr/bin/env tsx
// cli.ts — dispatcher CLI for lista-lending-onchainos.
//
// Invocation:
//   tsx cli.ts <toolName> '<jsonParams>'
//   tsx cli.ts --version
//   tsx cli.ts --help
//   tsx cli.ts <toolName> --help
//
// Examples:
//   tsx cli.ts --version
//   tsx cli.ts --help
//   tsx cli.ts listMarkets '{"chain":"bsc"}'
//   tsx cli.ts buildSupply '{"marketId":"0xd3845...","amount":"0.5","walletAddress":"0xabc..."}'
//
// Env vars (forwarded to runtime.ts mock state):
//   LISTA_MOCK_ALLOWANCE = insufficient | sufficient
//   LISTA_MOCK_POSITION  = fresh | supplied | borrowed | indebted
//   LISTA_MOCK_VAULT_SHARES = zero | some

import * as tools from './index.ts';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

const VERSION = (() => {
  try {
    const pkg = JSON.parse(readFileSync(join(__dirname, 'package.json'), 'utf8'));
    return pkg.version ?? 'unknown';
  } catch {
    return 'unknown';
  }
})();

const TOOL_NAMES = Object.keys(tools).filter(
  k => typeof (tools as Record<string, unknown>)[k] === 'function'
);

function printHelp(): void {
  console.log(`lista-lending-onchainos v${VERSION}

Usage:
  tsx cli.ts <toolName> '<jsonParams>'
  tsx cli.ts --version
  tsx cli.ts --help
  tsx cli.ts <toolName> --help

Available tools (${TOOL_NAMES.length}):

  Transaction tools (return pending_sign | ToolError):
    buildDeposit         { vaultAddress, amount, walletAddress, chain? }
    buildWithdraw        { vaultAddress, walletAddress, amount? | withdrawAll, chain? }
    buildSupply          { marketId, amount, walletAddress, chain? }
    buildBorrow          { marketId, amount, walletAddress, chain? }
    buildRepay           { marketId, walletAddress, amount? | repayAll, chain? }
    buildMarketWithdraw  { marketId, walletAddress, amount? | withdrawAll, chain? }

  Read-only tools (return ok | error):
    listMarkets          { chain?, page?, pageSize?, sort?, order?, loans?, collaterals?, keyword? }
    listVaults           { chain?, page?, pageSize?, sort?, order?, assets?, curators?, zone?, keyword? }
    getHoldings          { address, chain?, scope? }                  (Lista API, may lag 30-60s)
    getVaultPosition     { vaultAddress, walletAddress, chain? }      (direct RPC, real-time)
    simulateBorrow       { marketId, walletAddress, chain?, simulateSupply? }
    simulateRepay        { marketId, walletAddress, amount? | repayAll, chain? }

Supported chains: bsc, ethereum
Output: pretty-printed JSON to stdout, errors to stderr.

Mock state env vars (B-mock only):
  LISTA_MOCK_ALLOWANCE = insufficient | sufficient   (default: insufficient)
  LISTA_MOCK_POSITION  = fresh | supplied | borrowed | indebted  (default: supplied)
  LISTA_MOCK_VAULT_SHARES = zero | some              (default: some)
`);
}

function bigintReplacer(_key: string, value: unknown): unknown {
  return typeof value === 'bigint' ? value.toString() : value;
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);

  if (argv.length === 0 || argv[0] === '--help' || argv[0] === '-h') {
    printHelp();
    return;
  }

  if (argv[0] === '--version' || argv[0] === '-v') {
    console.log(VERSION);
    return;
  }

  const toolName = argv[0];
  if (!TOOL_NAMES.includes(toolName)) {
    console.error(`Unknown tool: ${toolName}`);
    console.error(`Available: ${TOOL_NAMES.join(', ')}`);
    console.error(`Run with --help for full usage.`);
    process.exit(2);
  }

  if (argv[1] === '--help' || argv[1] === '-h') {
    console.log(`Run \`tsx cli.ts --help\` for the full tool parameter reference.`);
    return;
  }

  let params: Record<string, unknown> = {};
  if (argv[1]) {
    try {
      params = JSON.parse(argv[1]);
    } catch (e) {
      console.error(`Failed to parse params JSON: ${(e as Error).message}`);
      console.error(`Usage: tsx cli.ts ${toolName} '<jsonParams>'`);
      process.exit(2);
    }
  }

  const fn = (tools as Record<string, (p: unknown) => Promise<unknown>>)[toolName];
  try {
    const result = await fn(params);
    console.log(JSON.stringify(result, bigintReplacer, 2));

    // Exit code: 0 for ok/pending_sign, 1 for error variant.
    const status = (result as { status?: string } | null | undefined)?.status;
    if (status === 'error') {
      process.exit(1);
    }
  } catch (e) {
    console.error(`Runtime error in ${toolName}:`);
    console.error((e as Error).stack ?? (e as Error).message);
    process.exit(3);
  }
}

main();
