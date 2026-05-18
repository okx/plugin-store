#!/usr/bin/env node
//
// ERC-20 Approve — check allowance and approve if needed via OWS
//
// Usage:
//   node erc20-approve.mjs <wallet_name> --chain <chainId> --token <tokenAddr> --spender <spenderAddr> [--amount <raw>] [--rpc <url>]
//
// If --amount is omitted, approves MaxUint256 (unlimited).
// If current allowance >= amount, skips approval and exits 0.
//
// Requires:
//   npm install ethers
//   ows CLI on PATH (or set OWS_CLI_PATH env var)

import { ethers } from "ethers";
import { execFileSync } from "child_process";
import { getRpc } from "../../common/scripts/rpc.mjs";

// ---------------------------------------------------------------------------
// Parse CLI
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const flags = {};
const positional = [];

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--chain" && args[i + 1]) flags.chain = args[++i];
  else if (args[i] === "--token" && args[i + 1]) flags.token = args[++i];
  else if (args[i] === "--spender" && args[i + 1]) flags.spender = args[++i];
  else if (args[i] === "--amount" && args[i + 1]) flags.amount = args[++i];
  else if (args[i] === "--rpc" && args[i + 1]) flags.rpc = args[++i];
  else if (!args[i].startsWith("--")) positional.push(args[i]);
  else { console.error(`Unknown flag: ${args[i]}`); process.exit(1); }
}

if (positional.length < 1 || !flags.chain || !flags.token || !flags.spender) {
  console.error("Usage: node erc20-approve.mjs <wallet_name> --chain <chainId> --token <addr> --spender <addr> [--amount <raw>] [--rpc <url>]");
  process.exit(1);
}

const walletName = positional[0];
const chainId = Number(flags.chain);
const tokenAddr = flags.token;
const spenderAddr = flags.spender;
const approveAmount = flags.amount ? BigInt(flags.amount) : ethers.MaxUint256;

// ---------------------------------------------------------------------------
// Resolve wallet address
// ---------------------------------------------------------------------------
const owsCmd = process.env.OWS_CLI_PATH || "ows";
let walletInfo;
try {
  walletInfo = execFileSync(owsCmd, ["wallet", "list"], { encoding: "utf8" });
} catch (err) {
  console.error("Failed to list OWS wallets:", err.message || err);
  process.exit(1);
}

const escapedName = walletName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const walletSection = walletInfo.split(/\n(?=ID:)/).find((s) => new RegExp(`Name:\\s+${escapedName}\\b`).test(s));
if (!walletSection) {
  console.error(`Wallet "${walletName}" not found`);
  process.exit(1);
}
const evmMatch = walletSection.match(/eip155:\d+\s+→\s+(0x[0-9a-fA-F]{40})/);
if (!evmMatch) {
  console.error(`No EVM address found for wallet "${walletName}"`);
  process.exit(1);
}
const srcAddress = evmMatch[1];

// ---------------------------------------------------------------------------
// Check allowance
// ---------------------------------------------------------------------------
const rpcUrl = flags.rpc || await getRpc(chainId);
const provider = new ethers.JsonRpcProvider(rpcUrl);

const erc20Iface = new ethers.Interface([
  "function allowance(address owner, address spender) view returns (uint256)",
  "function approve(address spender, uint256 amount) returns (bool)",
]);
const tokenContract = new ethers.Contract(tokenAddr, erc20Iface, provider);
const currentAllowance = await tokenContract.allowance(srcAddress, spenderAddr);

console.log(`Owner:     ${srcAddress}`);
console.log(`Token:     ${tokenAddr}`);
console.log(`Spender:   ${spenderAddr}`);
console.log(`Allowance: ${currentAllowance}`);

if (currentAllowance >= approveAmount) {
  console.log("Allowance sufficient — no approval needed.");
  process.exit(0);
}

// ---------------------------------------------------------------------------
// Send approve transaction
// ---------------------------------------------------------------------------
console.log(`Approving ${approveAmount === ethers.MaxUint256 ? "unlimited" : approveAmount.toString()}...`);

const [nonce, feeData] = await Promise.all([
  provider.getTransactionCount(srcAddress, "pending"),
  provider.getFeeData(),
]);

const approveData = erc20Iface.encodeFunctionData("approve", [spenderAddr, approveAmount]);

const supportsEip1559 = feeData.maxFeePerGas != null;
const txFields = {
  chainId,
  to: tokenAddr,
  data: approveData,
  value: 0n,
  nonce,
  gasLimit: 100000n,
};

if (supportsEip1559) {
  txFields.type = 2;
  txFields.maxFeePerGas = feeData.maxFeePerGas;
  txFields.maxPriorityFeePerGas = feeData.maxPriorityFeePerGas;
} else {
  txFields.type = 0;
  txFields.gasPrice = feeData.gasPrice;
}

const unsignedTx = ethers.Transaction.from(txFields);

const unsignedHex = unsignedTx.unsignedSerialized.replace(/^0x/, "");

console.log("Signing with OWS...");
const signRaw = execFileSync(owsCmd, [
  "sign", "tx",
  "--chain", `eip155:${chainId}`,
  "--wallet", walletName,
  "--tx", unsignedHex,
  "--json",
], { encoding: "utf8" });
const signResult = JSON.parse(signRaw);

const sigHex = signResult.signature;
const r = "0x" + sigHex.slice(0, 64);
const s = "0x" + sigHex.slice(64, 128);
const v = signResult.recovery_id;
unsignedTx.signature = ethers.Signature.from({ r, s, v });

console.log("Broadcasting...");
const txResp = await provider.broadcastTransaction(unsignedTx.serialized);
console.log("Transaction hash:", txResp.hash);

console.log("Waiting for confirmation...");
const receipt = await txResp.wait(1);
console.log("Confirmed in block:", receipt.blockNumber);
console.log("Status:", receipt.status === 1 ? "success" : "reverted");

if (receipt.status !== 1) {
  console.error("Approve transaction reverted.");
  process.exit(1);
}

console.log("Approval confirmed.");
