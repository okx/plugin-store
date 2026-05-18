#!/usr/bin/env npx tsx
//
// Query native or ERC-20 token balances. Supports single-chain and
// multi-chain scans. RPC endpoints are auto-discovered from Chainlist.
//
// Usage:
//   npx tsx balance.ts <address> <chain_id>                         # native balance
//   npx tsx balance.ts <address> <chain_id> --token <token_address> # ERC-20 balance
//   npx tsx balance.ts <address> <chain_ids...>                     # multi-chain native
//   npx tsx balance.ts <address> <chain_ids...> --token <token>     # multi-chain ERC-20
//   npx tsx balance.ts --derive <chain_id>                          # derive address from PRIVATE_KEY, then query
//
// Options:
//   --token <addr>   ERC-20 token address (omit for native balance)
//   --rpc <url>      Override RPC endpoint (single-chain only)
//   --derive         Derive address from $PRIVATE_KEY env var
//   --json           Output as JSON
//   --raw            Output raw units instead of human-readable
//
// Examples:
//   npx tsx balance.ts 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045 1
//   → 1523.456 ETH
//
//   npx tsx balance.ts 0xd8dA... 42161 --token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831
//   → 10000.00 USDC
//
//   npx tsx balance.ts 0xd8dA... 1 42161 8453 --json
//   → [{"chainId":1,"balance":"1523.456","symbol":"ETH"}, ...]

import {
  getTokenBalance,
  getDecimals,
  getSymbol,
  toHumanUnits,
  isNativeToken,
  resolveRpc,
} from "./erc20.js";
import { getChainInfo } from "./rpc.js";

async function deriveAddress(): Promise<string> {
  const pk = process.env.PRIVATE_KEY || process.env.DEBRIDGE_PRIVATE_KEY;
  if (!pk) throw new Error("No PRIVATE_KEY env var set");
  // Minimal address derivation without ethers — use eth_accounts or compute
  // For simplicity, try dynamic import of ethers or viem
  try {
    const { ethers } = await import("ethers");
    return new ethers.Wallet(pk).address;
  } catch {
    try {
      const { privateKeyToAccount } = await import("viem/accounts");
      return privateKeyToAccount(pk as `0x${string}`).address;
    } catch {
      throw new Error(
        "Install ethers or viem to derive address from private key: npm install ethers"
      );
    }
  }
}

interface BalanceResult {
  chainId: number;
  chainName: string;
  token: string;
  symbol: string;
  balance: string;
  rawBalance: string;
  decimals: number;
}

async function queryBalance(
  address: string,
  chainId: number,
  tokenAddress: string | null,
  explicitRpc?: string
): Promise<BalanceResult> {
  const rpcUrl = await resolveRpc(chainId, explicitRpc);
  const chainInfo = await getChainInfo(chainId);

  const token = tokenAddress ?? "0x0000000000000000000000000000000000000000";
  const rawBalance = await getTokenBalance(token, address, rpcUrl);
  const decimals = isNativeToken(token)
    ? (chainInfo?.nativeCurrency.decimals ?? 18)
    : await getDecimals(token, rpcUrl);
  const symbol = isNativeToken(token)
    ? (chainInfo?.nativeCurrency.symbol ?? "ETH")
    : await getSymbol(token, rpcUrl);
  const balance = toHumanUnits(rawBalance, decimals);

  return {
    chainId,
    chainName: chainInfo?.name ?? `Chain ${chainId}`,
    token,
    symbol,
    balance,
    rawBalance: rawBalance.toString(),
    decimals,
  };
}

async function main() {
  const args = process.argv.slice(2);
  const derive = args.includes("--derive");
  const asJson = args.includes("--json");
  const showRaw = args.includes("--raw");
  const tokenIdx = args.indexOf("--token");
  const rpcIdx = args.indexOf("--rpc");
  const tokenAddress = tokenIdx !== -1 ? args[tokenIdx + 1] : null;
  const explicitRpc = rpcIdx !== -1 ? args[rpcIdx + 1] : undefined;

  const positional = args.filter(
    (a, i) =>
      !a.startsWith("--") &&
      (tokenIdx === -1 || i !== tokenIdx + 1) &&
      (rpcIdx === -1 || i !== rpcIdx + 1)
  );

  let address: string;
  let chainIds: number[];

  if (derive) {
    address = await deriveAddress();
    chainIds = positional.map((a) => parseInt(a, 10)).filter((n) => !isNaN(n));
  } else {
    if (positional.length < 2) {
      console.error(
        "Usage: npx tsx balance.ts <address> <chain_id> [chain_id...] [--token <addr>] [--rpc <url>] [--json]"
      );
      process.exit(1);
    }
    address = positional[0];
    chainIds = positional.slice(1).map((a) => parseInt(a, 10));
  }

  if (chainIds.length === 0) {
    console.error("Provide at least one chain ID");
    process.exit(1);
  }

  const results = await Promise.allSettled(
    chainIds.map((cid) =>
      queryBalance(address, cid, tokenAddress, explicitRpc)
    )
  );

  const output: BalanceResult[] = [];
  for (const r of results) {
    if (r.status === "fulfilled") {
      output.push(r.value);
    } else {
      console.error(`Error: ${r.reason?.message ?? r.reason}`);
    }
  }

  if (asJson) {
    console.log(JSON.stringify(output, null, 2));
  } else {
    for (const r of output) {
      const val = showRaw ? r.rawBalance : r.balance;
      console.log(`${r.chainName} (${r.chainId}): ${val} ${r.symbol}`);
    }
  }
}

main().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
