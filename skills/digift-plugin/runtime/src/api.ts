import { ethers } from "ethers";

/** Custom error for DigiFT API responses — HTTP errors and business errors. */
export class DigiFTApiError extends Error {
  constructor(
    message: string,
    public readonly httpStatus?: number,
    public readonly businessCode?: number,
  ) {
    super(message);
    this.name = "DigiFTApiError";
  }
}

/** Token exists on the platform but is not deployed on the requested chain. */
export class TokenNotDeployedError extends Error {
  constructor(
    token: string,
    chainId: number,
    public readonly availableChains: string[],
  ) {
    super(`Token "${token}" is not deployed on chain ${chainId}. Available chains: ${availableChains.join(", ")}`);
    this.name = "TokenNotDeployedError";
  }
}

/** API response wrapper — all endpoints return { code, msg, data } */
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

/** GET /products */
export interface ProductInfo {
  tokenCode: string;
  projectName: string;
  assetType: string;
  isSubscribable: boolean;
  isRedeemable: boolean;
}

/** GET /platform/chains */
export interface PlatformChainInfo {
  chainId: string;
  chainName: string;
  standardSubRedContractAddress: string;
  flashSubRedContractAddress?: string;
  realTimeRedemptionContractAddress?: string;
  currencies: PlatformCurrencyInfo[];
}

export interface PlatformCurrencyInfo {
  currency: string;
  currencyAddress: string;
}

/** GET /wallets/whitelist-status */
export interface WhitelistStatus {
  address: string;
  chainId: string;
  isWhitelisted: boolean;
}

/** GET /products/{tokenCode}/chains */
export interface ProductChainInfo {
  tokenCode: string;
  projectName: string;
  chains: ProductChainEntry[];
}

export interface ProductChainEntry {
  chainId: string;
  chainName: string;
  tokenAddress: string;
  /** Price/display precisions — NOT on-chain ERC20 decimals. Do NOT use these for ethers.parseUnits(). Always read actual decimals() from the ERC20 contract on-chain. */
  precision: {
    pricePrecision: number;
    stPrecision: number;
    decimal: number;
  };
}

/** GET /products/{tokenCode}/issuance */
export interface IssuanceInfo {
  projectName: string;
  tokenCode: string;
  assetType: string;
  aboutIssuer: string;
  highlights: string;
  issuer: string;
  issueDate: string;
  isin: string;
}

/** GET /products/{tokenCode}/subscription-parameters */
export interface SubscriptionParameters {
  projectName: string;
  tokenCode: string;
  chains: SubscriptionChainParams[];
}

export interface SubscriptionChainParams {
  chainId: string;
  chainName: string;
  subscriptionMethods: SubscriptionMethod[];
}

export interface SubscriptionMethod {
  subscriptionMethod: string;
  configs: SubscriptionConfig[];
}

export interface SubscriptionConfig {
  feeRate: string;
  fixedFee: string;
  minFee: string;
  gst: string;
  min: string;
  max: string;
  increment: string;
  currency: string;
}

/** GET /products/{tokenCode}/redemption-parameters */
export interface RedemptionParameters {
  projectName: string;
  tokenCode: string;
  chains: RedemptionChainParams[];
}

export interface RedemptionChainParams {
  chainId: string;
  chainName: string;
  redemptionMethods: RedemptionMethod[];
}

export interface RedemptionMethod {
  redemptionMethod: string;
  configs: RedemptionConfig[];
}

export interface RedemptionConfig {
  feeRate: string;
  fixedFee: string;
  minFee: string;
  gst: string;
  min: string;
  max: string;
  increment: string;
  currency: string;
}

/** GET /products/{tokenCode}/business-calendar */
export interface SettlementSchedule {
  projectName: string;
  tokenCode: string;
  transactionType: string;
  settlementRule: {
    cycle: string;
    days: number;
    unit: "BUSINESS_DAY" | "CALENDAR_DAY";
  };
  nonBusinessDays: string[];
}

/** Shared fee structure */
export interface TransactionFee {
  rate: string;
  fixedFee: string;
  minFee: string;
  gst: string;
}

/** GET /project/pricing/{tokenCode} */
export interface PriceInfo {
  tokenCode: string;
  indicativeSubscribePrice: string;
  indicativeRedeemPrice: string;
  price: string;
  yield: string;
  yieldType: number;
  date: number;
}

/** GET /project/pricing/history/{tokenCode} */
export interface PriceHistoryEntry {
  tokenCode: string;
  price: string;
  yield: string;
  yieldType: number;
  date: number;
}

/** Paginated response wrapper */
export interface PaginatedResponse<T> {
  records: T[];
  total: number;
  size: number;
  pages: number;
  current: number;
}

/** GET /subscription/order | GET /subscription/order/list */
export interface SubscriptionOrder {
  orderId: string;
  chainId: string;
  walletAddress: string;
  status: string;
  userId?: number;
  project: string;
  currency: string;
  indicativePrice: string;
  indicativeQty: string;
  subscriptionAmount: string;
  subscriptionFee: string;
  settlePrice: string;
  settleQty: string;
  settleAmount: string;
  settleFee: string;
  submitHash: string;
  submitTime: number;
  confirmShareTime: number | null;
  cancelHash: string | null;
  cancelTime: number | null;
  settleHash: string | null;
  settleTime: number | null;
}

/** GET /redemption/order | GET /redemption/order/list */
export interface RedemptionOrder {
  orderId: string;
  chainId: string;
  walletAddress: string;
  project: string;
  currency: string;
  status: string;
  redemptionQty: string;
  indicativePrice: string;
  indicativeAmount: string;
  indicativeFee: string;
  indicativeNetAmount: string;
  allocQty: string;
  allocPrice: string;
  allocAmount: string;
  allocFee: string;
  netAmount: string;
  submitHash: string;
  submitTime: number;
  confirmShareTime: number | null;
  cancelHash: string | null;
  cancelTime: number | null;
  settleHash: string | null;
  settleTime: number | null;
}

// ─── Fee Calculation ───

/** Detailed fee calculation result — raw (on-chain precision) and formatted (human-readable). */
export interface FeeBreakdown {
  /** Fee portion in smallest on-chain units */
  feeRaw: bigint;
  /** Fee portion (same as feeRaw — the calculated fee) */
  totalFeeRaw: bigint;
  /** Amount + fee in smallest on-chain units */
  totalRaw: bigint;
  /** Human-readable fee (e.g. "0.5") */
  feeFormatted: string;
  /** GST rate — display-only, not used in computation */
  gstFormatted: string;
  /** Human-readable fee */
  totalFeeFormatted: string;
  /** Human-readable total (amount + fee) */
  totalFormatted: string;
}

/** Validate that a string represents a non-negative decimal number (e.g. "0.05", "2000"). Empty string is treated as "0". */
function validateNumericString(value: string, fieldName: string): void {
  if (value === "") return;
  if (!/^\d+(\.\d+)?$/.test(value)) {
    throw new TypeError(`${fieldName} must be a numeric string (e.g. "0.05"), got: "${value}"`);
  }
}

/**
 * Calculate the transaction fee for a subscription or redemption.
 *
 * Formula: `max(amount × rate + fixedFee, minFee)`
 * All arithmetic uses bigint with on-chain token decimals for precision.
 *
 * @param amount — human-readable amount (e.g. "2000")
 * @param fee — fee config from subscription/redemption parameters
 * @param decimals — ERC20 decimals of the payment token
 */
export function calculateTransactionFee(
  amount: string,
  fee: TransactionFee,
  decimals: number
): FeeBreakdown {
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 255) {
    throw new TypeError(`decimals must be an integer in [0, 255], got: ${decimals}`);
  }
  validateNumericString(amount, "amount");
  validateNumericString(fee.rate, "fee.rate");
  validateNumericString(fee.fixedFee, "fee.fixedFee");
  validateNumericString(fee.minFee, "fee.minFee");
  validateNumericString(fee.gst, "fee.gst");

  const amountRaw = ethers.parseUnits(amount, decimals);

  const rateParts = fee.rate.split(".");
  const rateInt = rateParts[0];
  const rateDec = (rateParts[1] || "").padEnd(decimals, "0").slice(0, decimals);
  // Scale must stay in BigInt domain. `10 ** decimals` routes through JS Number
  // and silently loses precision once decimals > 15, which would corrupt fee
  // math for any token using >15 decimals.
  const scale = 10n ** BigInt(decimals);
  const rateRaw = BigInt(rateInt) * scale + BigInt(rateDec || "0");

  const feeRaw = (amountRaw * rateRaw) / scale;
  const fixedFeeRaw = ethers.parseUnits(fee.fixedFee || "0", decimals);
  const minFeeRaw = ethers.parseUnits(fee.minFee || "0", decimals);

  let finalFeeRaw = feeRaw + fixedFeeRaw;
  if (minFeeRaw > 0n && finalFeeRaw < minFeeRaw) {
    finalFeeRaw = minFeeRaw;
  }

  // GST is a display-only field — passed through verbatim, no computation.
  const gstRate = fee.gst || "0";

  const totalFeeRaw = finalFeeRaw;
  const totalRaw = amountRaw + finalFeeRaw;

  return {
    feeRaw: finalFeeRaw,
    totalFeeRaw,
    totalRaw,
    feeFormatted: ethers.formatUnits(finalFeeRaw, decimals),
    gstFormatted: gstRate,
    totalFeeFormatted: ethers.formatUnits(totalFeeRaw, decimals),
    totalFormatted: ethers.formatUnits(totalRaw, decimals),
  };
}

// ─── Trading Calendar ───

export interface TradingCalendar {
  isOpen: boolean;
  nonBusinessDays: string[];
  settlementRule: SettlementSchedule["settlementRule"];
}

/**
 * Format a Date as YYYY-MM-DD in a specific IANA timezone.
 * DigiFT is dual-licensed by MAS (Singapore) and SFC (Hong Kong); both regulators
 * use UTC+8 business hours. Comparing against `nonBusinessDays` strings via UTC
 * (`toISOString().slice(0,10)`) would be off-by-one for ~8 hours each day.
 */
function ymdInTz(date: Date, tz: string): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: tz,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const y = parts.find(p => p.type === "year")!.value;
  const m = parts.find(p => p.type === "month")!.value;
  const d = parts.find(p => p.type === "day")!.value;
  return `${y}-${m}-${d}`;
}

/** Check whether today is a trading day based on the settlement schedule's non-business-day list.
 *  `tz` defaults to Asia/Singapore (DigiFT business timezone). Pass a different IANA tz if the
 *  product's `nonBusinessDays` are quoted in a different timezone. */
export function buildTradingCalendar(
  schedule: SettlementSchedule,
  tz: string = "Asia/Singapore",
): TradingCalendar {
  const today = ymdInTz(new Date(), tz);
  const isHoliday = schedule.nonBusinessDays.includes(today);
  return {
    isOpen: !isHoliday,
    nonBusinessDays: schedule.nonBusinessDays,
    settlementRule: schedule.settlementRule,
  };
}

export function getNextBusinessDay(
  schedule: SettlementSchedule,
  tz: string = "Asia/Singapore",
): string {
  const holidays = new Set(schedule.nonBusinessDays);
  const d = new Date();
  for (let i = 1; i <= 14; i++) {
    d.setDate(d.getDate() + 1);
    const s = ymdInTz(d, tz);
    if (!holidays.has(s)) return s;
  }
  return ymdInTz(d, tz);
}

// ─── API Client ───

export class DigiFTApiClient {
  private baseUrl: string;
  private apiKey: string;
  private authHeader: string;

  constructor(baseUrl?: string, apiKey?: string) {
    const raw = baseUrl || process.env.DIGIFT_API_URL || "https://digift.io/api/openplatform";
    // Validate scheme + host. baseUrl is configurable via env (DIGIFT_API_URL),
    // so without an allowlist a malicious env value could exfiltrate the API
    // key on every request.
    let parsed: URL;
    try {
      parsed = new URL(raw);
    } catch {
      throw new Error(`DIGIFT_API_URL must be a valid URL, got: ${raw}`);
    }
    if (parsed.protocol !== "https:") {
      throw new Error(`DIGIFT_API_URL must use https: scheme, got: ${parsed.protocol}`);
    }
    if (parsed.hostname !== "digift.io" && !parsed.hostname.endsWith(".digift.io")) {
      throw new Error(
        `DIGIFT_API_URL host must be digift.io or a *.digift.io subdomain, got: ${parsed.hostname}`,
      );
    }
    this.baseUrl = raw;
    this.apiKey = apiKey || process.env.DIGIFT_API_KEY || "";
    this.authHeader = "X-DG-Access-Key";
    if (!this.apiKey) throw new Error("DIGIFT_API_KEY not set");
  }

  private async request<T>(path: string): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const res = await fetch(url, {
      headers: { [this.authHeader]: this.apiKey },
    });
    if (!res.ok) {
      const text = await res.text();
      throw new DigiFTApiError(`API error ${res.status}: ${text}`, res.status);
    }
    const body = await res.json() as ApiResponse<T>;
    if (body.code !== 0) {
      throw new DigiFTApiError(`API business error ${body.code}: ${body.message}`, undefined, body.code);
    }
    return body.data;
  }

  /** List available products */
  async getProducts(): Promise<ProductInfo[]> {
    return this.request("/products");
  }

  /** Get platform chain info, contract addresses, and currencies */
  async getPlatformChains(): Promise<{ chains: PlatformChainInfo[] }> {
    return this.request("/platform/chains");
  }

  /** Check wallet whitelist status on a given chain */
  async getWhitelistStatus(address: string, chainId: string): Promise<WhitelistStatus> {
    const qs = new URLSearchParams({ address, chainId });
    return this.request(`/wallets/whitelist-status?${qs.toString()}`);
  }

  /** Get product on-chain info (token address, precision per chain) */
  async getProductChains(tokenCode: string): Promise<ProductChainInfo> {
    return this.request(`/products/${tokenCode}/chains`);
  }

  /** Get issuance details (issuer, ISIN, date, highlights) */
  async getIssuance(tokenCode: string): Promise<IssuanceInfo> {
    return this.request(`/products/${tokenCode}/issuance`);
  }

  /** Get subscription parameters (fee, min, max, increment) */
  async getSubscriptionParameters(tokenCode: string): Promise<SubscriptionParameters> {
    return this.request(`/products/${tokenCode}/subscription-parameters`);
  }

  /** Get redemption parameters (fee, min, max, increment) */
  async getRedemptionParameters(tokenCode: string): Promise<RedemptionParameters> {
    return this.request(`/products/${tokenCode}/redemption-parameters`);
  }

  /** Get trading calendar and settlement schedule */
  async getSettlementSchedule(
    tokenCode: string,
    transactionType: "SUBSCRIPTION" | "REDEMPTION"
  ): Promise<SettlementSchedule> {
    return this.request(`/products/${tokenCode}/business-calendar?transactionType=${transactionType}`);
  }

  /** Get current indicative price and yield */
  async getPrice(tokenCode: string): Promise<PriceInfo> {
    return this.request(`/project/pricing/${tokenCode}`);
  }

  /** Get historical price data */
  async getPriceHistory(tokenCode: string): Promise<PriceHistoryEntry[]> {
    return this.request(`/project/pricing/history/${tokenCode}`);
  }

  /** List subscription orders (paginated) */
  async listSubscriptionOrders(params?: {
    project?: string;
    walletAddress?: string;
    startTime?: number;
    endTime?: number;
    size?: number;
    current?: number;
  }): Promise<PaginatedResponse<SubscriptionOrder>> {
    const qs = params
      ? "?" +
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
          .join("&")
      : "";
    return this.request(`/subscription/order/list${qs}`);
  }

  /** Look up a single subscription order by tx hash */
  async getSubscriptionOrder(txHash: string): Promise<SubscriptionOrder> {
    const qs = new URLSearchParams({ txHash });
    return this.request(`/subscription/order?${qs.toString()}`);
  }

  /** List redemption orders (paginated) */
  async listRedemptionOrders(params?: {
    project?: string;
    walletAddress?: string;
    startTime?: number;
    endTime?: number;
    size?: number;
    current?: number;
  }): Promise<PaginatedResponse<RedemptionOrder>> {
    const qs = params
      ? "?" +
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
          .join("&")
      : "";
    return this.request(`/redemption/order/list${qs}`);
  }

  /** Look up a single redemption order by tx hash */
  async getRedemptionOrder(txHash: string): Promise<RedemptionOrder> {
    const qs = new URLSearchParams({ txHash });
    return this.request(`/redemption/order?${qs.toString()}`);
  }

  /**
   * Resolve a token code, currency symbol, or raw address to an on-chain contract address.
   *
   * Resolution order:
   * 1. If `tokenArg` is already a valid address → return as-is.
   * 2. Try product chain info (`/products/{code}/chains`) → ST token address.
   * 3. Try platform currencies (`/platform/chains`) → currency address.
   *
   * @returns Only the address — decimals must be read from the ERC20 contract on-chain.
   */
  async resolveToken(
    tokenArg: string,
    chainId: number
  ): Promise<{ address: string }> {
    const key = tokenArg.toUpperCase();
    if (ethers.isAddress(tokenArg)) {
      return { address: tokenArg };
    }

    // 1. Try product chain info
    try {
      const info = await this.getProductChains(key);
      const chainEntry = info.chains.find((c) => Number(c.chainId) === chainId);
      if (chainEntry) {
        return { address: chainEntry.tokenAddress };
      }
      // Token exists but not on this chain — don't silently pick the wrong chain
      throw new TokenNotDeployedError(tokenArg, chainId, info.chains.map(c => c.chainId));
    } catch (err) {
      // Re-throw if the product exists but isn't on the target chain;
      // only fall through to currencies if the API didn't recognise it as a product.
      if (err instanceof TokenNotDeployedError) {
        throw err;
      }
    }

    // 2. Try platform currencies
    try {
      const platform = await this.getPlatformChains();
      for (const c of platform.chains) {
        if (Number(c.chainId) !== chainId) continue;
        const cur = c.currencies.find((x) => x.currency.toUpperCase() === key);
        if (cur && cur.currencyAddress) {
          return { address: cur.currencyAddress };
        }
      }
    } catch {
      // Fall through
    }

    throw new Error(`Unknown token "${tokenArg}" on chain ${chainId}`);
  }
}
