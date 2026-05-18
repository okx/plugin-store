#!/usr/bin/env node
//
// deBridge EVM Bridge — sign and broadcast a create_tx response via OWS
//
// Usage:
//   echo '<create_tx JSON>' | node debridge-evm-bridge.mjs <wallet_name> [--src-chain <id>] [--rpc <url>] [--json]
//
// The script reads a deBridge create_tx JSON response from stdin.
// The agent calls MCP to get the quote, then pipes it here for signing.
//
// Example:
//   echo '<create_tx JSON from mcp__debridge__create_tx>' | node debridge-evm-bridge.mjs default --src-chain 137
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
  if (args[i] === "--src-chain" && args[i + 1]) flags.srcChain = args[++i];
  else if (args[i] === "--rpc" && args[i + 1]) flags.rpc = args[++i];
  else if (args[i] === "--json") flags.json = true;
  else if (!args[i].startsWith("--")) positional.push(args[i]);
  else { console.error(`Unknown flag: ${args[i]}`); process.exit(1); }
}

if (positional.length < 1) {
  console.error("Usage: echo '<create_tx JSON>' | node debridge-evm-bridge.mjs <wallet_name> [--src-chain <id>] [--rpc <url>] [--json]");
  process.exit(1);
}

const walletName = positional[0];
const srcChain = flags.srcChain || "1";

// ---------------------------------------------------------------------------
// Read create_tx response from stdin
// ---------------------------------------------------------------------------
let stdinData = "";
const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(chunk);
}
stdinData = Buffer.concat(chunks).toString("utf8");

// Expect plain create_tx JSON on stdin (agent parses MCP response before piping)
const quote = JSON.parse(stdinData.trim());

if (quote.message || quote.code) {
  console.error("deBridge error:", quote.message || JSON.stringify(quote));
  process.exit(1);
}

const est = quote.estimation.dstChainTokenOut;
const estAmount = Number(est.amount) / Math.pow(10, est.decimals);
if (!flags.json) {
  console.log(`Estimated output: ${estAmount.toFixed(4)} ${est.symbol} (~$${est.approximateUsdValue.toFixed(2)})`);
  console.log("Order ID:", quote.orderId);
}

// ---------------------------------------------------------------------------
// Resolve source address from OWS wallet
// ---------------------------------------------------------------------------
const owsCmd = process.env.OWS_CLI_PATH || "ows";
let walletInfo;
try {
  walletInfo = execFileSync(owsCmd, ["wallet", "list"], { encoding: "utf8" });
} catch (err) {
  if (err && err.code === "ENOENT") {
    console.error("ows CLI not found. Please ensure 'ows' is on your PATH or set OWS_CLI_PATH to its full path.");
  } else {
    console.error("Failed to execute 'ows wallet list':", err.stderr?.toString() || err.message || err);
  }
  process.exit(1);
}

// Extract EVM address scoped to the requested wallet name and any eip155 chain
const escapedName = walletName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const walletSection = walletInfo.split(/\n(?=ID:)/).find((s) => new RegExp(`Name:\\s+${escapedName}\\b`).test(s));
if (!walletSection) {
  console.error(`Wallet "${walletName}" not found in OWS wallet list`);
  process.exit(1);
}
const evmMatch = walletSection.match(/eip155:\d+\s+→\s+(0x[0-9a-fA-F]{40})/);
if (!evmMatch) {
  console.error(`No EVM address found for wallet "${walletName}"`);
  process.exit(1);
}
const srcAddress = evmMatch[1];
if (!flags.json) {
  console.log("Source address:", srcAddress);
}

// ---------------------------------------------------------------------------
// Helpers: sign and broadcast a single EVM transaction
// ---------------------------------------------------------------------------
const rpcUrl = flags.rpc || await getRpc(Number(srcChain));
const provider = new ethers.JsonRpcProvider(rpcUrl);
const chainId = Number(srcChain);

async function signAndBroadcast(txData, label) {
  console.log(`\n--- ${label} ---`);

  // Fetch nonce and fee data
  const [nonce, feeData] = await Promise.all([
    provider.getTransactionCount(srcAddress, "pending"),
    provider.getFeeData(),
  ]);

  // Build unsigned transaction — use EIP-1559 if supported, legacy otherwise
  const supportsEip1559 = feeData.maxFeePerGas != null;
  const txFields = {
    chainId,
    to: txData.to,
    data: txData.data,
    value: BigInt(txData.value || "0"),
    nonce,
    gasLimit: 900000n,
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

  // Serialize unsigned tx, strip 0x prefix
  const unsignedHex = unsignedTx.unsignedSerialized.replace(/^0x/, "");

  // Sign with OWS CLI
  console.log("Signing with OWS...");
  const signRaw = execFileSync(owsCmd, [
    "sign", "tx",
    "--chain", `eip155:${chainId}`,
    "--wallet", walletName,
    "--tx", unsignedHex,
    "--json",
  ], { encoding: "utf8" });
  const signResult = JSON.parse(signRaw);

  // Extract r, s, v from OWS signature
  // OWS returns {signature: "<128-char hex>", recovery_id: 0|1}
  const sigHex = signResult.signature;
  const r = "0x" + sigHex.slice(0, 64);
  const s = "0x" + sigHex.slice(64, 128);
  const v = signResult.recovery_id;

  // Attach signature to transaction
  unsignedTx.signature = ethers.Signature.from({ r, s, v });

  // Broadcast
  console.log("Broadcasting...");
  const txResp = await provider.broadcastTransaction(unsignedTx.serialized);
  console.log("Transaction hash:", txResp.hash);

  // Wait for 1 confirmation
  console.log("Waiting for confirmation...");
  const receipt = await txResp.wait(1);
  console.log("Confirmed in block:", receipt.blockNumber);
  console.log("Status:", receipt.status === 1 ? "success" : "reverted");

  return receipt;
}

// ---------------------------------------------------------------------------
// If there is an approveTx, sign and send it first
// ---------------------------------------------------------------------------
if (quote.approveTx) {
  console.log("\nToken approval required — sending approve transaction first.");
  const approveReceipt = await signAndBroadcast(quote.approveTx, "Approve");
  if (approveReceipt.status !== 1) {
    console.error("Approve transaction reverted. Aborting.");
    process.exit(1);
  }
  console.log("Approval confirmed. Proceeding to bridge transaction.");
}

// ---------------------------------------------------------------------------
// Sign and broadcast the bridge transaction
// ---------------------------------------------------------------------------
const bridgeReceipt = await signAndBroadcast(quote.tx, "Bridge");

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------
if (flags.json) {
  console.log(JSON.stringify({
    hash: bridgeReceipt.hash,
    blockNumber: bridgeReceipt.blockNumber,
    status: bridgeReceipt.status === 1 ? "success" : "reverted",
    orderId: quote.orderId,
  }, null, 2));
} else {
  console.log("\nBridge transaction complete.");
  console.log("Order ID:", quote.orderId);
}
