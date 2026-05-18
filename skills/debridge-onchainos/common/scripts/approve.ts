#!/usr/bin/env npx tsx
//
// Approve an ERC-20 token for a spender address. Sends the approval
// transaction on-chain using ethers.js or viem (whichever is available).
//
// Usage:
//   npx tsx approve.ts <token_address> <spender> <chain_id> [--amount <human_amount>] [--rpc <url>]
//
// Options:
//   --amount <n>    Human-readable amount to approve (default: unlimited / MaxUint256)
//   --rpc <url>     Override RPC endpoint
//   --json          Output as JSON
//   --dry-run       Build tx but don't send — print calldata and exit
//
// Environment:
//   PRIVATE_KEY or DEBRIDGE_PRIVATE_KEY must be set.
//
// Examples:
//   # Unlimited approval for USDC on Ethereum to a deBridge contract
//   npx tsx approve.ts 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 0xSpenderAddr 1
//   → Approved unlimited USDC for 0xSpender... (tx: 0xabc...)
//
//   # Approve exactly 1000 USDC
//   npx tsx approve.ts 0xA0b8...eB48 0xSpender 1 --amount 1000
//
//   # Dry run — show calldata without sending
//   npx tsx approve.ts 0xA0b8...eB48 0xSpender 1 --amount 1000 --dry-run

import {
  getDecimals,
  getSymbol,
  toRawUnits,
  toHumanUnits,
  buildApproveCalldata,
  resolveRpc,
  MAX_UINT256,
  isNativeToken,
} from "./erc20.js";

async function sendWithEthers(
  rpcUrl: string,
  privateKey: string,
  to: string,
  data: string
): Promise<{ hash: string; wait: () => Promise<any> }> {
  const { ethers } = await import("ethers");
  const provider = new ethers.JsonRpcProvider(rpcUrl);
  const wallet = new ethers.Wallet(privateKey, provider);
  const tx = await wallet.sendTransaction({ to, data });
  return { hash: tx.hash, wait: () => tx.wait() };
}

async function sendWithViem(
  rpcUrl: string,
  privateKey: string,
  to: string,
  data: string,
  chainId: number
): Promise<{ hash: string; wait: () => Promise<any> }> {
  const { createWalletClient, createPublicClient, http } = await import("viem");
  const { privateKeyToAccount } = await import("viem/accounts");
  const account = privateKeyToAccount(privateKey as `0x${string}`);
  const transport = http(rpcUrl);
  const walletClient = createWalletClient({ account, transport });
  const publicClient = createPublicClient({ transport });

  const hash = await walletClient.sendTransaction({
    to: to as `0x${string}`,
    data: data as `0x${string}`,
    chain: { id: chainId } as any,
  });

  return {
    hash,
    wait: () => publicClient.waitForTransactionReceipt({ hash }),
  };
}

async function main() {
  const args = process.argv.slice(2);
  const asJson = args.includes("--json");
  const dryRun = args.includes("--dry-run");
  const amountIdx = args.indexOf("--amount");
  const rpcIdx = args.indexOf("--rpc");
  const humanAmount = amountIdx !== -1 ? args[amountIdx + 1] : undefined;
  const explicitRpc = rpcIdx !== -1 ? args[rpcIdx + 1] : undefined;

  const positional = args.filter(
    (a, i) =>
      !a.startsWith("--") &&
      (amountIdx === -1 || i !== amountIdx + 1) &&
      (rpcIdx === -1 || i !== rpcIdx + 1)
  );

  if (positional.length < 3) {
    console.error(
      "Usage: npx tsx approve.ts <token_address> <spender> <chain_id> [--amount <n>] [--rpc <url>] [--dry-run]"
    );
    process.exit(1);
  }

  const tokenAddress = positional[0];
  const spender = positional[1];
  const chainId = parseInt(positional[2], 10);

  if (isNativeToken(tokenAddress)) {
    console.error("Cannot approve native token — approvals are ERC-20 only");
    process.exit(1);
  }

  const pk = process.env.PRIVATE_KEY || process.env.DEBRIDGE_PRIVATE_KEY;
  if (!pk && !dryRun) {
    console.error("Set PRIVATE_KEY or DEBRIDGE_PRIVATE_KEY environment variable");
    process.exit(1);
  }

  const rpcUrl = await resolveRpc(chainId, explicitRpc);
  const decimals = await getDecimals(tokenAddress, rpcUrl);
  const symbol = await getSymbol(tokenAddress, rpcUrl);

  let approveAmount: bigint;
  let approveHuman: string;
  if (humanAmount) {
    approveAmount = BigInt(toRawUnits(humanAmount, decimals));
    approveHuman = humanAmount;
  } else {
    approveAmount = MAX_UINT256;
    approveHuman = "unlimited";
  }

  const calldata = buildApproveCalldata(spender, approveAmount);

  if (dryRun) {
    const result = {
      to: tokenAddress,
      data: calldata,
      chainId,
      spender,
      amount: approveHuman,
      rawAmount: approveAmount.toString(),
      symbol,
      decimals,
    };
    if (asJson) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(`Dry run — approve ${approveHuman} ${symbol} for ${spender}`);
      console.log(`To: ${tokenAddress}`);
      console.log(`Data: ${calldata}`);
    }
    process.exit(0);
  }

  // Send the transaction
  let hash: string;
  try {
    const tx = await sendWithEthers(rpcUrl, pk!, tokenAddress, calldata);
    hash = tx.hash;
    if (!asJson) console.log(`Tx sent: ${hash}`);
    await tx.wait();
  } catch (ethersErr: any) {
    if (
      ethersErr.code === "MODULE_NOT_FOUND" ||
      ethersErr.message?.includes("Cannot find module")
    ) {
      try {
        const tx = await sendWithViem(rpcUrl, pk!, tokenAddress, calldata, chainId);
        hash = tx.hash;
        if (!asJson) console.log(`Tx sent: ${hash}`);
        await tx.wait();
      } catch (viemErr: any) {
        if (
          viemErr.code === "MODULE_NOT_FOUND" ||
          viemErr.message?.includes("Cannot find module")
        ) {
          console.error(
            "Install ethers or viem to send transactions: npm install ethers"
          );
          process.exit(1);
        }
        throw viemErr;
      }
    } else {
      throw ethersErr;
    }
  }

  if (asJson) {
    console.log(
      JSON.stringify({
        hash,
        token: tokenAddress,
        spender,
        amount: approveHuman,
        rawAmount: approveAmount.toString(),
        symbol,
        decimals,
        chainId,
      })
    );
  } else {
    console.log(
      `Approved ${approveHuman} ${symbol} for ${spender} (tx: ${hash})`
    );
  }
}

main().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
