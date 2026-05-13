export type BirdeyeMode = 'auto' | 'apikey' | 'x402';

export function getMode(): BirdeyeMode {
  const mode = (process.env.BIRDEYE_MODE || 'auto').toLowerCase();
  if (mode === 'apikey' || mode === 'x402' || mode === 'auto') return mode;
  throw new Error(`Invalid BIRDEYE_MODE: ${mode}`);
}

export function getApiKey(): string | undefined {
  return process.env.BIRDEYE_API_KEY;
}

export function getSignerKeyFile(): string | undefined {
  return process.env.BIRDEYE_SIGNER_KEY_FILE;
}

export function getMaxDailySpend(): string | undefined {
  return process.env.MAX_DAILY_SPEND_USDC_BASE_UNITS;
}
