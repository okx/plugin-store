#!/usr/bin/env npx tsx
//
// Convert between human-readable and raw token amounts.
// Reads decimals on-chain from the token contract if needed.
//
// Usage:
//   npx tsx convert-amount.ts <amount> <decimals>
//   npx tsx convert-amount.ts <amount> <token_address> <chain_id> [--rpc <url>]
//   npx tsx convert-amount.ts <raw_amount> <decimals> --reverse
//   npx tsx convert-amount.ts <raw_amount> <token_address> <chain_id> --reverse [--rpc <url>]
//
// Examples:
//   npx tsx convert-amount.ts 100 6
//   → {"raw":"100000000","decimals":6,"human":"100"}
//
//   npx tsx convert-amount.ts 0.5 18
//   → {"raw":"500000000000000000","decimals":18,"human":"0.5"}
//
//   npx tsx convert-amount.ts 100 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 1
//   → {"raw":"100000000","decimals":6,"human":"100","symbol":"USDC"}
//
//   npx tsx convert-amount.ts 100000000 6 --reverse
//   → {"raw":"100000000","decimals":6,"human":"100"}

import { getDecimals, getSymbol, toRawUnits, toHumanUnits, resolveRpc } from "./erc20.js";

async function main() {
  const args = process.argv.slice(2);
  const reverse = args.includes("--reverse");
  const rpcIdx = args.indexOf("--rpc");
  const explicitRpc = rpcIdx !== -1 ? args[rpcIdx + 1] : undefined;
  const positional = args.filter(
    (a, i) => !a.startsWith("--") && (rpcIdx === -1 || i !== rpcIdx + 1)
  );

  if (positional.length < 2) {
    console.error(
      "Usage:\n" +
        "  npx tsx convert-amount.ts <amount> <decimals>\n" +
        "  npx tsx convert-amount.ts <amount> <token_address> <chain_id> [--rpc <url>]\n" +
        "  Add --reverse to convert raw → human"
    );
    process.exit(1);
  }

  const amount = positional[0];
  let decimals: number;
  let symbol: string | undefined;

  if (positional.length === 2 && /^\d+$/.test(positional[1])) {
    decimals = parseInt(positional[1], 10);
  } else if (positional.length >= 3) {
    const tokenAddress = positional[1];
    const chainId = parseInt(positional[2], 10);
    const rpcUrl = await resolveRpc(chainId, explicitRpc);
    decimals = await getDecimals(tokenAddress, rpcUrl);
    symbol = await getSymbol(tokenAddress, rpcUrl);
  } else {
    console.error("Provide either <decimals> or <token_address> <chain_id>");
    process.exit(1);
  }

  if (reverse) {
    const human = toHumanUnits(BigInt(amount), decimals);
    const result: any = { raw: amount, decimals, human };
    if (symbol) result.symbol = symbol;
    console.log(JSON.stringify(result));
  } else {
    const raw = toRawUnits(amount, decimals);
    const result: any = { raw, decimals, human: amount };
    if (symbol) result.symbol = symbol;
    console.log(JSON.stringify(result));
  }
}

main().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
