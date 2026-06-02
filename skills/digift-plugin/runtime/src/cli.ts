#!/usr/bin/env bun

import { ethers } from "ethers";
import {
  DigiFTApiClient,
  DigiFTApiError,
  calculateTransactionFee,
  buildTradingCalendar,
  getNextBusinessDay,
  type SubscriptionMethod,
  type SubscriptionConfig,
  type RedemptionMethod,
  type RedemptionConfig,
} from "./api";
import { DigiFTContractClient } from "./contract";

let args = process.argv.slice(2);
const command = args[0];

function getFlag(name: string): string | undefined {
  const i = args.indexOf(`--${name}`);
  if (i === -1) return undefined;
  const val = args[i + 1];
  if (!val || val.startsWith("--")) return undefined;
  return val;
}

function isNetworkError(err: unknown): boolean {
  return !(err instanceof DigiFTApiError);
}

function usage() {
  console.log(`Usage:
  ── Query──
  digift products                              List all available products
  digift chains                                Platform chain info, contract addresses, currencies
  digift whitelist <address> --chain <chain-id>         Check wallet whitelist status (per-chain, required)
  digift info <tokencode>                      Product chain info (token address, precision)
  digift issuance <tokencode>                  Issuance details (issuer, ISIN, date)
  digift sub-params <tokencode>                Subscription parameters (fee, min, step)
  digift red-params <tokencode>                Redemption parameters
  digift calendar <tokencode> [--type sub|red] Trading calendar & settlement cycle
  digift price <tokencode>                     Current price & yield
  digift price-history <tokencode>             Historical prices
  digift order <txhash>                         Lookup order by transaction hash
  digift orders <address> [--project <project>] [--size <n>]  List orders for wallet

  ── On-Chain──
  digift balance <token> <address> --chain <chain-id> [--rpc <rpc-url>]
      Check on-chain token balance

  digift subscribe --product <tokencode> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]
      Build subscribe transaction

  digift redeem --product <tokencode> --quantity <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]
      Build redeem transaction

  digift approve --token <address> --spender <address> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>]
      Build approve transaction

  digift --version  |  digift -V
      Show version`);
}

/** Built-in fallback RPC URLs keyed by chain ID. Used only when --rpc flag and DIGIFT_RPC_URL env are both unset. */
const CHAIN_RPC_URLS: Record<string, string> = {
  "1":      "https://ethereum-rpc.publicnode.com",
  "56":     "https://bsc-rpc.publicnode.com",
  "137":    "https://polygon-bor-rpc.publicnode.com",
  "143":    "https://rpc.monad.xyz",
  "5000":   "https://rpc.mantle.xyz",
  "98867":  "https://mainnet-rpc.plumenetwork.xyz",
  "42161":  "https://arbitrum-one-rpc.publicnode.com",
};

/** Resolve RPC URL with priority: --rpc flag > DIGIFT_RPC_URL env > built-in table by chain ID. */
function getRpcUrl(): string {
  const fromFlag = getFlag("rpc");
  if (fromFlag) return fromFlag;
  const fromEnv = process.env.DIGIFT_RPC_URL;
  if (fromEnv) return fromEnv;
  const chainId = getFlag("chain");
  if (chainId && CHAIN_RPC_URLS[chainId]) return CHAIN_RPC_URLS[chainId];
  return "";
}

function findSubscriptionMethod(
  methods: SubscriptionMethod[],
  currency: string
): { config: SubscriptionConfig; methodName: string } | undefined {
  const targetMethod = methods.find((m) => m.subscriptionMethod === "STANDARD_SUBSCRIPTION");
  if (!targetMethod) return undefined;
  const config = targetMethod.configs.find((c) => c.currency.toUpperCase() === currency.toUpperCase());
  return config ? { config, methodName: targetMethod.subscriptionMethod } : undefined;
}

function findRedemptionMethod(
  methods: RedemptionMethod[],
  currency: string
): { config: RedemptionConfig; methodName: string } | undefined {
  const targetMethod = methods.find((m) => m.redemptionMethod === "STANDARD_REDEMPTION");
  if (!targetMethod) return undefined;
  const config = targetMethod.configs.find((c) => c.currency.toUpperCase() === currency.toUpperCase());
  return config ? { config, methodName: targetMethod.redemptionMethod } : undefined;
}

/** Resolve RPC URL and chain info from flags/env, fetch platform chain data from API, instantiate DigiFTContractClient. */
async function createContract(api: DigiFTApiClient): Promise<{
  contract: DigiFTContractClient;
  chainId: number;
  chainName: string;
}> {
  const rpcUrl = getRpcUrl();
  if (!rpcUrl) {
    const chainId = getFlag("chain");
    if (chainId) {
      throw new Error(`No built-in RPC for chain ${chainId}. Set DIGIFT_RPC_URL env var or use --rpc flag`);
    }
    throw new Error("RPC URL required. Set DIGIFT_RPC_URL env var or use --rpc flag");
  }

  const platform = await api.getPlatformChains();
  const targetChain = getFlag("chain");
  if (!targetChain) {
    const knownIds = platform.chains.map(c => c.chainId).join(", ");
    throw new Error(`--chain is required. Known chain IDs: ${knownIds}.`);
  }

  let chainInfo = platform.chains.find(
    (c) => c.chainId === targetChain || c.chainName.toUpperCase() === targetChain.toUpperCase()
  );
  if (!chainInfo) throw new Error(`Chain "${targetChain}" not found. Known chain IDs: ${platform.chains.map(c => c.chainId).join(", ")}`);

  const contract = new DigiFTContractClient(
    rpcUrl,
    chainInfo.standardSubRedContractAddress
  );

  return { contract, chainId: Number(chainInfo.chainId), chainName: chainInfo.chainName };
}

// ─── Query Commands (API only) ───

async function cmdProducts() {
  const api = new DigiFTApiClient();
  const products = await api.getProducts();
  if (products.length === 0) {
    console.log("No products found.");
    return;
  }
  console.log(JSON.stringify(products, null, 2));
}

async function cmdChains() {
  const api = new DigiFTApiClient();
  const { chains } = await api.getPlatformChains();
  console.log(JSON.stringify(chains, null, 2));
}

async function cmdWhitelist() {
  const address = args[1];
  if (!address) {
    console.error("Usage: digift whitelist <address> --chain <chain-id>");
    process.exit(1);
  }
  const chainId = getFlag("chain");
  if (!chainId) {
    console.error("--chain is required for whitelist check (whitelist is per-chain)");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const status = await api.getWhitelistStatus(address, chainId);
  console.log(JSON.stringify(status, null, 2));
}

async function cmdInfo() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift info <tokencode>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const info = await api.getProductChains(tokenCode);
  console.log(JSON.stringify(info, null, 2));
}

async function cmdIssuance() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift issuance <tokencode>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const issuance = await api.getIssuance(tokenCode);
  console.log(JSON.stringify(issuance, null, 2));
}

async function cmdSubParams() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift sub-params <tokencode>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const params = await api.getSubscriptionParameters(tokenCode);
  console.log(JSON.stringify(params, null, 2));
}

async function cmdRedParams() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift red-params <tokencode>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const params = await api.getRedemptionParameters(tokenCode);
  console.log(JSON.stringify(params, null, 2));
}

async function cmdCalendar() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift calendar <tokencode> [--type sub|red]");
    process.exit(1);
  }
  const type = (getFlag("type") || "SUBSCRIPTION").toUpperCase();
  if (type !== "SUBSCRIPTION" && type !== "REDEMPTION" && type !== "SUB" && type !== "RED") {
    console.error("--type must be SUBSCRIPTION (or sub) or REDEMPTION (or red)");
    process.exit(1);
  }
  const transactionType = type === "SUB" ? "SUBSCRIPTION" : type === "RED" ? "REDEMPTION" : type as "SUBSCRIPTION" | "REDEMPTION";

  const api = new DigiFTApiClient();
  const schedule = await api.getSettlementSchedule(tokenCode, transactionType);
  const calendar = buildTradingCalendar(schedule);

  console.log(JSON.stringify({
    ...schedule,
    tradingOpen: calendar.isOpen,
    nextBusinessDay: calendar.isOpen ? undefined : getNextBusinessDay(schedule),
  }, null, 2));
}

async function cmdPrice() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift price <tokencode>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const price = await api.getPrice(tokenCode);
  console.log(JSON.stringify(price, null, 2));
}

async function cmdPriceHistory() {
  const tokenCode = args[1];
  if (!tokenCode) {
    console.error("Usage: digift price-history <tokencode>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const history = await api.getPriceHistory(tokenCode);
  console.log(JSON.stringify(history, null, 2));
}

async function cmdOrder() {
  const txHash = args[1];
  if (!txHash) {
    console.error("Usage: digift order <txhash>");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  // Try subscription first, then redemption.
  // Only swallow API "not found" errors; re-throw network/unexpected errors.
  try {
    const order = await api.getSubscriptionOrder(txHash);
    if (order && order.orderId) {
      console.log(JSON.stringify({ type: "subscription", ...order }, null, 2));
      return;
    }
  } catch (err) {
    if (isNetworkError(err)) throw err;
  }
  try {
    const order = await api.getRedemptionOrder(txHash);
    if (order && order.orderId) {
      console.log(JSON.stringify({ type: "redemption", ...order }, null, 2));
      return;
    }
  } catch (err) {
    if (isNetworkError(err)) throw err;
  }
  console.error(`Order not found for txHash: ${txHash}`);
  process.exit(1);
}

async function cmdOrders() {
  const address = args[1];
  if (!address) {
    console.error("Usage: digift orders <address> [--project <project>] [--size <n>]");
    process.exit(1);
  }
  const api = new DigiFTApiClient();
  const params: Record<string, string | number> = { walletAddress: address };
  const project = getFlag("project");
  if (project) params.project = project;
  const size = getFlag("size");
  if (size) params.size = parseInt(size);

  const [sub, red] = await Promise.all([
    api.listSubscriptionOrders(params).catch((err) => {
      if (err instanceof DigiFTApiError) return { records: [], total: 0, size: 0, pages: 0, current: 0 };
      throw err;
    }),
    api.listRedemptionOrders(params).catch((err) => {
      if (err instanceof DigiFTApiError) return { records: [], total: 0, size: 0, pages: 0, current: 0 };
      throw err;
    }),
  ]);

  console.log(JSON.stringify({
    subscription: { total: sub.total, records: sub.records },
    redemption: { total: red.total, records: red.records },
  }, null, 2));
}

// ─── On-Chain Commands ───

async function cmdBalance() {
  const tokenArg = args[1];
  const address = args[2];
  if (!tokenArg || !address) {
    console.error("Usage: digift balance <token> <address> --chain <chain-id> [--rpc <rpc-url>]");
    process.exit(1);
  }

  const api = new DigiFTApiClient();
  const { contract, chainId } = await createContract(api);
  const { address: tokenAddress } = await api.resolveToken(tokenArg, chainId);
  const bal = await contract.getTokenBalance(tokenAddress, address);
  console.log(JSON.stringify({
    token: tokenAddress,
    symbol: bal.symbol,
    balance: bal.formatted,
    raw: bal.raw.toString(),
    decimals: bal.decimals,
  }, null, 2));
}

async function cmdSubscribe() {
  const productKey = getFlag("product");
  const amount = getFlag("amount");
  const from = getFlag("from");
  const currencyKey = (getFlag("currency") || "USDC").toUpperCase();

  if (!productKey || !amount || !from) {
    console.error("Usage: digift subscribe --product <tokencode> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]");
    process.exit(1);
  }

  const api = new DigiFTApiClient();
  const { contract, chainId, chainName } = await createContract(api);

  // Get product chain info for ST token address & precision
  const chainInfo = await api.getProductChains(productKey);
  const chainEntry = chainInfo.chains.find((c) => Number(c.chainId) === chainId);
  if (!chainEntry) {
    console.error(`Product ${productKey} not deployed on chain ${chainId}`);
    process.exit(1);
  }
  const stToken = chainEntry.tokenAddress;

  // Get subscription parameters
  const subParams = await api.getSubscriptionParameters(productKey);
  const subChain = subParams.chains.find((c) => Number(c.chainId) === chainId);
  if (!subChain || subChain.subscriptionMethods.length === 0) {
    console.error(`No subscription methods for ${productKey} on chain ${chainId}`);
    process.exit(1);
  }

  const subMethod = findSubscriptionMethod(subChain.subscriptionMethods, currencyKey);
  if (!subMethod) {
    console.error(`Currency ${currencyKey} not supported for ${productKey} subscription`);
    process.exit(1);
  }

  // Get settlement schedule (informational only — tradability is determined by isSubscribable on product listing)
  const schedule = await api.getSettlementSchedule(productKey, "SUBSCRIPTION");

  // Resolve currency address from platform chains
  const platform = await api.getPlatformChains();
  const pchain = platform.chains.find((c) => Number(c.chainId) === chainId);
  const cur = pchain?.currencies.find((c) => c.currency.toUpperCase() === currencyKey);
  if (!cur || !cur.currencyAddress) {
    console.error(`Currency ${currencyKey} not found on chain ${chainId}`);
    process.exit(1);
  }
  const currencyAddress = cur.currencyAddress;

  // Get dynamic decimals from on-chain token
  const bal = await contract.getTokenBalance(currencyAddress, from);
  const currencyDecimals = bal.decimals;

  // Validate amount: min, max, and increment
  const amountRaw = ethers.parseUnits(amount, currencyDecimals);
  const minRaw = ethers.parseUnits(subMethod.config.min, currencyDecimals);
  if (amountRaw < minRaw) {
    console.error(`Amount ${amount} below minimum ${subMethod.config.min} ${currencyKey}`);
    process.exit(1);
  }
  if (subMethod.config.max) {
    const maxRaw = ethers.parseUnits(subMethod.config.max, currencyDecimals);
    if (amountRaw > maxRaw) {
      console.error(`Amount ${amount} exceeds maximum ${subMethod.config.max} ${currencyKey}`);
      process.exit(1);
    }
  }
  if (subMethod.config.increment) {
    const incRaw = ethers.parseUnits(subMethod.config.increment, currencyDecimals);
    if (incRaw > 0n && amountRaw % incRaw !== 0n) {
      console.error(`Amount ${amount} is not a multiple of increment ${subMethod.config.increment} ${currencyKey}`);
      process.exit(1);
    }
  }

  // Check whitelist via API
  const { isWhitelisted } = await api.getWhitelistStatus(from, String(chainId));
  if (!isWhitelisted) {
    console.error("Not whitelisted. Complete KYC at app.digift.com");
    process.exit(1);
  }

  // Check balance (already fetched decimals and balance above)
  const feeBreakdown = calculateTransactionFee(amount, { rate: subMethod.config.feeRate, fixedFee: subMethod.config.fixedFee, minFee: subMethod.config.minFee, gst: subMethod.config.gst }, currencyDecimals);
  if (bal.raw < feeBreakdown.totalRaw) {
    console.error(
      `Insufficient balance: have ${bal.formatted} ${bal.symbol}, need ${feeBreakdown.totalFormatted} (amount + fee)`
    );
    process.exit(1);
  }

  // Build tx (amount parameter = subscription amount + fee)
  const txBody = contract.buildSubscribeTx(stToken, currencyAddress, feeBreakdown.totalRaw, from);

  // Check allowance
  const allowance = await contract.getAllowance(currencyAddress, from, txBody.to);
  const needsApprove = allowance < feeBreakdown.totalRaw;

  console.log(
    JSON.stringify(
      {
        action: "subscribe",
        product: productKey,
        chainId,
        chainName,
        method: subMethod.methodName,
        amount,
        currency: currencyKey,
        fee: {
          rate: subMethod.config.feeRate,
          fixedFee: subMethod.config.fixedFee,
          minFee: subMethod.config.minFee,
          calculated: feeBreakdown.feeFormatted,
        },
        total: feeBreakdown.totalFormatted,
        settlementCycle: schedule.settlementRule.cycle || "unknown",
        txBody,
        needsApprove: needsApprove
          ? { token: currencyAddress, spender: txBody.to, amount: feeBreakdown.totalRaw.toString() }
          : undefined,
      },
      null,
      2
    )
  );
}

async function cmdRedeem() {
  const productKey = getFlag("product");
  const quantity = getFlag("quantity");
  const from = getFlag("from");
  const currencyKey = (getFlag("currency") || "USDC").toUpperCase();

  if (!productKey || !quantity || !from) {
    console.error("Usage: digift redeem --product <tokencode> --quantity <n> --from <address> --chain <chain-id> [--rpc <rpc-url>] [--currency <currency>]");
    process.exit(1);
  }

  const api = new DigiFTApiClient();
  const { contract, chainId, chainName } = await createContract(api);

  // Get product chain info
  const chainInfo = await api.getProductChains(productKey);
  const chainEntry = chainInfo.chains.find((c) => Number(c.chainId) === chainId);
  if (!chainEntry) {
    console.error(`Product ${productKey} not deployed on chain ${chainId}`);
    process.exit(1);
  }
  const stToken = chainEntry.tokenAddress;

  // Read ST token decimals from on-chain (NOT from API precision)
  const stBal = await contract.getTokenBalance(stToken, from);
  const stDecimals = stBal.decimals;

  // Get redemption parameters
  const redParams = await api.getRedemptionParameters(productKey);
  const redChain = redParams.chains.find((c) => Number(c.chainId) === chainId);
  if (!redChain || redChain.redemptionMethods.length === 0) {
    console.error(`No redemption methods for ${productKey} on chain ${chainId}`);
    process.exit(1);
  }

  const redMethod = findRedemptionMethod(redChain.redemptionMethods, currencyKey);
  if (!redMethod) {
    console.error(`Currency ${currencyKey} not supported for ${productKey} redemption`);
    process.exit(1);
  }

  // Get settlement schedule (informational only — tradability is determined by isRedeemable on product listing)
  const schedule = await api.getSettlementSchedule(productKey, "REDEMPTION");

  // Resolve currency address
  const platform = await api.getPlatformChains();
  const pchain = platform.chains.find((c) => Number(c.chainId) === chainId);
  const cur = pchain?.currencies.find((c) => c.currency.toUpperCase() === currencyKey);
  if (!cur || !cur.currencyAddress) {
    console.error(`Currency ${currencyKey} not found on chain ${chainId}`);
    process.exit(1);
  }
  const currencyAddress = cur.currencyAddress;

  // Read currency token decimals for fee calculation (fee is charged in settlement currency, not ST token)
  const curBal = await contract.getTokenBalance(currencyAddress, from);
  const currencyDecimals = curBal.decimals;

  // Validate quantity: min, max, and increment
  const quantityRaw = ethers.parseUnits(quantity, stDecimals);
  const minRaw = ethers.parseUnits(redMethod.config.min, stDecimals);
  if (quantityRaw < minRaw) {
    console.error(`Quantity ${quantity} below minimum ${redMethod.config.min}`);
    process.exit(1);
  }
  if (redMethod.config.max) {
    const maxRaw = ethers.parseUnits(redMethod.config.max, stDecimals);
    if (quantityRaw > maxRaw) {
      console.error(`Quantity ${quantity} exceeds maximum ${redMethod.config.max}`);
      process.exit(1);
    }
  }
  if (redMethod.config.increment) {
    const incRaw = ethers.parseUnits(redMethod.config.increment, stDecimals);
    if (incRaw > 0n && quantityRaw % incRaw !== 0n) {
      console.error(`Quantity ${quantity} is not a multiple of increment ${redMethod.config.increment}`);
      process.exit(1);
    }
  }

  // Check whitelist via API
  const { isWhitelisted } = await api.getWhitelistStatus(from, String(chainId));
  if (!isWhitelisted) {
    console.error("Not whitelisted. Complete KYC at app.digift.com");
    process.exit(1);
  }

  // Check ST token balance
  if (stBal.raw < quantityRaw) {
    console.error(`Insufficient balance: have ${stBal.formatted} ${stBal.symbol}, need ${quantity}`);
    process.exit(1);
  }

  // Build tx
  const txBody = contract.buildRedeemTx(stToken, currencyAddress, quantityRaw, from);

  // Check allowance
  const allowance = await contract.getAllowance(stToken, from, txBody.to);
  const needsApprove = allowance < quantityRaw;

  const feeBreakdown = calculateTransactionFee(quantity, { rate: redMethod.config.feeRate, fixedFee: redMethod.config.fixedFee, minFee: redMethod.config.minFee, gst: redMethod.config.gst }, currencyDecimals);

  console.log(
    JSON.stringify(
      {
        action: "redeem",
        product: productKey,
        chainId,
        chainName,
        method: redMethod.methodName,
        quantity,
        currency: currencyKey,
        fee: {
          rate: redMethod.config.feeRate,
          fixedFee: redMethod.config.fixedFee,
          minFee: redMethod.config.minFee,
          calculated: feeBreakdown.feeFormatted,
        },
        total: feeBreakdown.totalFormatted,
        settlementCycle: schedule.settlementRule.cycle || "unknown",
        txBody,
        needsApprove: needsApprove
          ? { token: stToken, spender: txBody.to, amount: quantityRaw.toString() }
          : undefined,
      },
      null,
      2
    )
  );
}

async function cmdApprove() {
  const tokenArg = getFlag("token");
  const spender = getFlag("spender");
  const amount = getFlag("amount");
  const from = getFlag("from");
  if (!tokenArg || !spender || !amount || !from) {
    console.error("Usage: digift approve --token <address> --spender <address> --amount <n> --from <address> --chain <chain-id> [--rpc <rpc-url>]");
    process.exit(1);
  }

  const api = new DigiFTApiClient();
  const { contract, chainId } = await createContract(api);

  const { address: tokenAddress } = await api.resolveToken(tokenArg, chainId);
  const tokenInfo = await contract.getTokenBalance(tokenAddress, from);
  const amountRaw = ethers.parseUnits(amount, tokenInfo.decimals);

  const txBody = contract.buildApproveTx(tokenAddress, spender, amountRaw, from);

  console.log(
    JSON.stringify(
      {
        action: "approve",
        token: tokenAddress,
        spender,
        amount,
        txBody,
      },
      null,
      2
    )
  );
}

async function main() {
  if (args.includes("--help") || args.includes("-h")) {
    usage();
    process.exit(0);
  }
  if (args.includes("--version") || args.includes("-V")) {
    console.log("1.0.0");
    return;
  }

  switch (command) {
    // Query
    case "products":      return cmdProducts();
    case "chains":        return cmdChains();
    case "whitelist":     return cmdWhitelist();
    case "info":          return cmdInfo();
    case "issuance":      return cmdIssuance();
    case "sub-params":    return cmdSubParams();
    case "red-params":    return cmdRedParams();
    case "calendar":      return cmdCalendar();
    case "price":         return cmdPrice();
    case "price-history": return cmdPriceHistory();
    case "order":         return cmdOrder();
    case "orders":        return cmdOrders();
    // On-chain
    case "balance":       return cmdBalance();
    case "subscribe":     return cmdSubscribe();
    case "redeem":        return cmdRedeem();
    case "approve":       return cmdApprove();
    default:
      usage();
      process.exit(command ? 1 : 0);
  }
}

main().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
