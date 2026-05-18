#!/usr/bin/env npx tsx
//
// Shared ERC-20 utilities used by balance, allowance, approve, and
// convert-amount scripts. Provides on-chain reads via raw eth_call
// (no ethers/viem dependency for read-only ops) and ethers/viem
// helpers for write operations.
//
// This module is imported by sibling scripts — not meant to run standalone.

import { getRpc } from "./rpc.js";

// ERC-20 function selectors
const SEL = {
  decimals: "0x313ce567",
  symbol: "0x95d89b41",
  balanceOf: "0x70a08231",
  allowance: "0xdd62ed3e",
  approve: "0x095ea7b3",
  name: "0x06fdde03",
};

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";

// ── Raw RPC call (no library needed) ──────────────────────────────

async function ethCall(
  rpcUrl: string,
  to: string,
  data: string
): Promise<string> {
  const resp = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "eth_call",
      params: [{ to, data }, "latest"],
    }),
  });
  const json = (await resp.json()) as any;
  if (json.error) throw new Error(`RPC error: ${json.error.message}`);
  return json.result;
}

async function ethGetBalance(
  rpcUrl: string,
  address: string
): Promise<bigint> {
  const resp = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "eth_getBalance",
      params: [address, "latest"],
    }),
  });
  const json = (await resp.json()) as any;
  if (json.error) throw new Error(`RPC error: ${json.error.message}`);
  return BigInt(json.result);
}

// ── Padding helpers ───────────────────────────────────────────────

function padAddress(address: string): string {
  return address.toLowerCase().replace("0x", "").padStart(64, "0");
}

function padUint256(value: bigint): string {
  return value.toString(16).padStart(64, "0");
}

// ── Public API ────────────────────────────────────────────────────

export function isNativeToken(address: string): boolean {
  return (
    address === ZERO_ADDRESS ||
    address === "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE" ||
    address.toLowerCase() === "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
  );
}

export async function getDecimals(
  tokenAddress: string,
  rpcUrl: string
): Promise<number> {
  if (isNativeToken(tokenAddress)) return 18;
  const result = await ethCall(rpcUrl, tokenAddress, SEL.decimals);
  return parseInt(result, 16);
}

export async function getSymbol(
  tokenAddress: string,
  rpcUrl: string
): Promise<string> {
  if (isNativeToken(tokenAddress)) return "ETH";
  try {
    const result = await ethCall(rpcUrl, tokenAddress, SEL.symbol);
    // ABI-decode string: offset at 0x20, length at 0x40, data after
    const len = parseInt(result.slice(66, 130), 16);
    const hex = result.slice(130, 130 + len * 2);
    return Buffer.from(hex, "hex").toString("utf-8").replace(/\0/g, "");
  } catch {
    return "UNKNOWN";
  }
}

export async function getNativeBalance(
  address: string,
  rpcUrl: string
): Promise<bigint> {
  return ethGetBalance(rpcUrl, address);
}

export async function getTokenBalance(
  tokenAddress: string,
  owner: string,
  rpcUrl: string
): Promise<bigint> {
  if (isNativeToken(tokenAddress)) {
    return getNativeBalance(owner, rpcUrl);
  }
  const data = SEL.balanceOf + padAddress(owner);
  const result = await ethCall(rpcUrl, tokenAddress, data);
  return BigInt(result);
}

export async function getAllowance(
  tokenAddress: string,
  owner: string,
  spender: string,
  rpcUrl: string
): Promise<bigint> {
  if (isNativeToken(tokenAddress)) {
    // Native tokens don't need allowance
    return BigInt("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
  }
  const data = SEL.allowance + padAddress(owner) + padAddress(spender);
  const result = await ethCall(rpcUrl, tokenAddress, data);
  return BigInt(result);
}

export function toRawUnits(humanAmount: string, decimals: number): string {
  const [whole, frac = ""] = humanAmount.split(".");
  const paddedFrac = frac.padEnd(decimals, "0").slice(0, decimals);
  const raw = whole + paddedFrac;
  return raw.replace(/^0+/, "") || "0";
}

export function toHumanUnits(rawAmount: bigint, decimals: number): string {
  const str = rawAmount.toString().padStart(decimals + 1, "0");
  const whole = str.slice(0, str.length - decimals) || "0";
  const frac = str.slice(str.length - decimals).replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : whole;
}

// ── Resolve RPC for a chain ───────────────────────────────────────

export async function resolveRpc(
  chainId: number,
  explicitRpc?: string
): Promise<string> {
  if (explicitRpc) return explicitRpc;
  return getRpc(chainId);
}

// ── Build approve calldata ────────────────────────────────────────

export function buildApproveCalldata(
  spender: string,
  amount: bigint
): string {
  return SEL.approve + padAddress(spender) + padUint256(amount);
}

export const MAX_UINT256 = BigInt(
  "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
);
