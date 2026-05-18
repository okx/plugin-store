#!/usr/bin/env npx tsx
//
// Check ERC-20 token allowance granted by an owner to a spender.
// Used before bridge/swap to verify if approval is needed.
//
// Usage:
//   npx tsx allowance.ts <token_address> <owner> <spender> <chain_id> [--rpc <url>]
//   npx tsx allowance.ts <token_address> <spender> <chain_id> --derive [--rpc <url>]
//
// Options:
//   --derive       Use $PRIVATE_KEY to derive owner address
//   --rpc <url>    Override RPC endpoint
//   --json         Output as JSON
//   --check <amt>  Check if allowance >= amount (human-readable). Exits 0 if sufficient, 1 if not.
//
// Examples:
//   npx tsx allowance.ts 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
//     0xOwnerAddr 0xSpenderAddr 1
//   → Allowance: 1000000.00 USDC
//
//   npx tsx allowance.ts 0xA0b8...eB48 0xOwner 0xSpender 1 --check 100 --json
//   → {"token":"0xA0b8...","owner":"0x...","spender":"0x...","allowance":"1000000","sufficient":true}

import {
  getAllowance,
  getDecimals,
  getSymbol,
  toHumanUnits,
  toRawUnits,
  isNativeToken,
  resolveRpc,
} from "./erc20.js";

async function deriveAddress(): Promise<string> {
  const pk = process.env.PRIVATE_KEY || process.env.DEBRIDGE_PRIVATE_KEY;
  if (!pk) throw new Error("No PRIVATE_KEY env var set");
  try {
    const { ethers } = await import("ethers");
    return new ethers.Wallet(pk).address;
  } catch {
    try {
      const { privateKeyToAccount } = await import("viem/accounts");
      return privateKeyToAccount(pk as `0x${string}`).address;
    } catch {
      throw new Error("Install ethers or viem to derive address: npm install ethers");
    }
  }
}

async function main() {
  const args = process.argv.slice(2);
  const derive = args.includes("--derive");
  const asJson = args.includes("--json");
  const rpcIdx = args.indexOf("--rpc");
  const checkIdx = args.indexOf("--check");
  const explicitRpc = rpcIdx !== -1 ? args[rpcIdx + 1] : undefined;
  const checkAmount = checkIdx !== -1 ? args[checkIdx + 1] : undefined;

  const positional = args.filter(
    (a, i) =>
      !a.startsWith("--") &&
      (rpcIdx === -1 || i !== rpcIdx + 1) &&
      (checkIdx === -1 || i !== checkIdx + 1)
  );

  let tokenAddress: string;
  let owner: string;
  let spender: string;
  let chainId: number;

  if (derive) {
    if (positional.length < 3) {
      console.error(
        "Usage: npx tsx allowance.ts <token> <spender> <chain_id> --derive"
      );
      process.exit(1);
    }
    tokenAddress = positional[0];
    spender = positional[1];
    chainId = parseInt(positional[2], 10);
    owner = await deriveAddress();
  } else {
    if (positional.length < 4) {
      console.error(
        "Usage: npx tsx allowance.ts <token> <owner> <spender> <chain_id> [--rpc <url>] [--check <amount>]"
      );
      process.exit(1);
    }
    tokenAddress = positional[0];
    owner = positional[1];
    spender = positional[2];
    chainId = parseInt(positional[3], 10);
  }

  if (isNativeToken(tokenAddress)) {
    const result = { token: tokenAddress, owner, spender, allowance: "unlimited", sufficient: true };
    if (asJson) {
      console.log(JSON.stringify(result));
    } else {
      console.log("Native token — no approval needed (unlimited)");
    }
    process.exit(0);
  }

  const rpcUrl = await resolveRpc(chainId, explicitRpc);
  const rawAllowance = await getAllowance(tokenAddress, owner, spender, rpcUrl);
  const decimals = await getDecimals(tokenAddress, rpcUrl);
  const symbol = await getSymbol(tokenAddress, rpcUrl);
  const humanAllowance = toHumanUnits(rawAllowance, decimals);

  let sufficient: boolean | undefined;
  if (checkAmount) {
    const requiredRaw = BigInt(toRawUnits(checkAmount, decimals));
    sufficient = rawAllowance >= requiredRaw;
  }

  if (asJson) {
    const result: any = {
      token: tokenAddress,
      owner,
      spender,
      chainId,
      allowance: humanAllowance,
      rawAllowance: rawAllowance.toString(),
      decimals,
      symbol,
    };
    if (sufficient !== undefined) result.sufficient = sufficient;
    console.log(JSON.stringify(result));
  } else {
    console.log(`Allowance: ${humanAllowance} ${symbol}`);
    if (sufficient !== undefined) {
      console.log(
        sufficient
          ? `✓ Sufficient for ${checkAmount} ${symbol}`
          : `✗ Insufficient — need ${checkAmount} ${symbol}, have ${humanAllowance} ${symbol} approved`
      );
    }
  }

  if (sufficient === false) process.exit(1);
}

main().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
